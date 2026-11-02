#!/bin/sh
# shellcheck disable=SC2016 # los guiones de in_run se expanden en la jail
#
# Actualizador del panel del router. Corre EN EL HOST como root; se instala
# como /usr/local/sbin/routerpanel-actualizar y lo lanza cron cada 15 minutos
# (/usr/local/etc/cron.d/routerpanel).
#
#   routerpanel-actualizar           despliega la última etiqueta vX.Y.Z en
#                                    cuanto hay una versión precompilada que
#                                    pasa las comprobaciones
#   routerpanel-actualizar --ayuda   esta ayuda
#
# El servidor nunca compila: no llama a cargo ni a rustc, ni tiene jail de
# compilación. Pasos:
#
# 1. La última etiqueta vX.Y.Z (sort -V) y su commit, con
#    `git ls-remote --tags REPO_URL` desde el host (ultima_etiqueta_remota,
#    deploy/lib/git.sh): sin clon ni checkout, y sin ejecutar nada del
#    repositorio. Por SSH, con la clave de despliegue de solo lectura
#    (CLAVE_DESPLIEGUE) y la clave del servidor git fijada (HOSTS_CONOCIDOS).
# 2. Si ya está desplegada, o ya falló, termina.
# 3. La versión, ya compilada, de la primera fuente que la tenga y pase las
#    comprobaciones (deploy/lib/precompiladas.sh):
#    - la release de la etiqueta en Forgejo (FORGEJO_URL y FORGEJO_REPO), que
#      publica el runner de Actions del PC (.forgejo/workflows/freebsd.yml):
#      con el token de solo lectura de FORGEJO_TOKEN_FICHERO (de root y
#      0600; si no, no se usa), sha256 de SHA256SUMS, firma de minisign si
#      hay PRECOMPILADAS_MINISIGN, COMMIT igual al commit de la etiqueta y
#      panel ejecutable de FreeBSD x86-64;
#    - la release de GitHub (si hay GITHUB_REPO): sha256 de SHA256SUMS,
#      atestación de origen (gh attestation verify: de GITHUB_REPO, de
#      refs/tags/vX.Y.Z y de .github/workflows/release.yml) y lo mismo que
#      Forgejo salvo la firma.
#    Si ninguna la tiene todavía, registra una vez por etiqueta «esperando la
#    versión precompilada de vX.Y.Z» y sale con 0: lo vuelve a mirar en la
#    siguiente pasada, sin límite de tiempo y sin contarlo como fallo. Lo que
#    no cuadra se rechaza (nunca se instala nada sin comprobar), se registra
#    una vez por etiqueta y fuente, y no cuenta como fallo de la etiqueta.
# 4. La deja en releases/<versión>, de root; cambia el enlace «current» de la
#    jail del panel, reinicia el servicio y espera a que /salud responda 200
#    (60 s como mucho). Si no responde, vuelve a la versión anterior. Se
#    conservan las 3 últimas versiones.
#
# Configuración: /usr/local/etc/routerpanel-actualizar.conf (JAILS_RAIZ,
# JAIL_PANEL, REPO_URL, CLAVE_DESPLIEGUE, HOSTS_CONOCIDOS, FORGEJO_URL,
# FORGEJO_REPO, FORGEJO_TOKEN_FICHERO, GITHUB_REPO, PRECOMPILADAS_MINISIGN;
# la escribe install.sh) y deploy/lib/git.sh y deploy/lib/precompiladas.sh en
# /usr/local/libexec/routerpanel-actualizar. La URL de /salud sale de
# routerpanel_listen en el rc.conf de la jail del panel. Las claves de
# instalaciones anteriores (JAIL_COMPILAR, TRABAJOS_COMPILACION,
# PRECOMPILADAS_ESPERA, BUZON) se ignoran.
#
# Registro: /var/log/routerpanel-update.log.

set -u
umask 027

# cron da un PATH corto: curl, gh y jq están en /usr/local/bin.
PATH=/sbin:/bin:/usr/sbin:/usr/bin:/usr/local/sbin:/usr/local/bin
export PATH

JEXEC=/usr/sbin/jexec
GIT=/usr/local/bin/git
CONF=/usr/local/etc/routerpanel-actualizar.conf
GIT_SH=/usr/local/libexec/routerpanel-actualizar/git.sh
PRECOMPILADAS_SH=/usr/local/libexec/routerpanel-actualizar/precompiladas.sh
# Valores por defecto; los de la instalación vienen de $CONF.
JAILS_RAIZ=/usr/local/jails/containers
JAIL_PANEL=routerpanel
REPO_URL=
# Clave de despliegue (de solo lectura en el servidor git) y known_hosts con
# la clave del servidor git: de root, en el host.
CLAVE_DESPLIEGUE=/usr/local/etc/routerpanel-actualizar/id_ed25519
HOSTS_CONOCIDOS=/usr/local/etc/routerpanel-actualizar/known_hosts
# URL base del Forgejo (https://forge.example) y «propietario/nombre» del
# repositorio; con alguna vacía, no se mira Forgejo. El token, de solo
# lectura, de root y 0600.
FORGEJO_URL=
FORGEJO_REPO=
FORGEJO_TOKEN_FICHERO=/usr/local/etc/routerpanel-actualizar/forgejo-token
# «propietario/nombre» del repositorio en GitHub; vacía, no se mira GitHub.
GITHUB_REPO=
# Clave pública de minisign con la que el PC firma SHA256SUMS en Forgejo;
# vacía, sin firma.
PRECOMPILADAS_MINISIGN=
if [ -f "$CONF" ]; then
	# shellcheck source=/dev/null
	. "$CONF"
fi
# Dueño que tiene que tener el fichero del token (root).
DUENO_TOKEN=0
RUN_JAIL=$JAIL_PANEL
RUN_USER=routerpanel
RELEASES=/usr/local/routerpanel/releases
CURRENT=$JAILS_RAIZ/$RUN_JAIL/usr/local/routerpanel/current
HEALTH_TIMEOUT=60
# Copia de la base de datos antes de cada despliegue (dentro de la jail del
# panel, del usuario del servicio). Se conserva la última.
DB_DIR=/var/db/routerpanel
DB_BACKUP=$DB_DIR/antes-de-actualizar
STATE=/var/db/routerpanel-actualizar
LOG=/var/log/routerpanel-update.log
LOCK=/var/run/routerpanel-actualizar.lock
KEEP=3
# Fallos seguidos de una etiqueta antes de darla por fallida.
MAX_TRIES=2

ayuda()
{
	sed -n '/^#   routerpanel-actualizar/s/^#   //p' "$0"
}

case "${1:-}" in
"") ;;
--ayuda | -h | --help)
	ayuda
	exit 0
	;;
--ref)
	echo "--ref ya no existe: el servidor no compila. Crea una etiqueta vX.Y.Z: el runner" \
		"del PC la compila y la publica en la release de Forgejo (o la publica GitHub)." >&2
	exit 2
	;;
*)
	ayuda >&2
	exit 2
	;;
esac

# Una sola ejecución a la vez: una descarga o un despliegue pueden durar más
# que el intervalo del cron.
if [ -z "${ROUTERPANEL_LOCKED:-}" ]; then
	ROUTERPANEL_LOCKED=1 exec /usr/bin/lockf -s -t 0 "$LOCK" "$0" "$@"
fi

log()
{
	_line="$(date '+%Y-%m-%d %H:%M:%S') $*"
	echo "$_line" >>"$LOG"
	if [ -t 1 ]; then
		echo "$_line"
	fi
}

# Nombre de versión aceptable como componente de ruta bajo $RELEASES: empieza
# por letra o cifra, sólo [A-Za-z0-9._-], sin «..» y como mucho 100 caracteres
# (los saltos de línea caen en la clase prohibida).
valid_name()
{
	case "$1" in
	'' | [!A-Za-z0-9]* | *[!A-Za-z0-9._-]* | *..*) return 1 ;;
	esac
	[ "${#1}" -le 100 ]
}

# Ruta que se puede poner tal cual en GIT_SSH_COMMAND (lo interpreta un sh):
# absoluta y solo con [A-Za-z0-9/._-].
ruta_simple()
{
	case "$1" in
	/*) ;;
	*) return 1 ;;
	esac
	case "$1" in
	*[!A-Za-z0-9/._-]*) return 1 ;;
	esac
	return 0
}

deployed()
{
	_t=$(readlink "$CURRENT" 2>/dev/null) || return 0
	basename "$_t"
}

# `git ls-remote --tags REPO_URL` desde el host, sin clon. Por SSH: solo la
# clave de despliegue, la clave del servidor git tiene que estar ya en
# HOSTS_CONOCIDOS (nunca se acepta una nueva) y sin preguntar nada. Sin la
# configuración de git ni la de ssh de root (-F /dev/null), y sin
# transportes que ejecuten órdenes (ext::).
ls_remote()
{
	_ssh="ssh -F /dev/null -o BatchMode=yes -o ConnectTimeout=15 -o IdentitiesOnly=yes"
	_ssh="$_ssh -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$HOSTS_CONOCIDOS"
	# Siempre -i: sin él, ssh probaría las claves de root (~/.ssh/id_*). Si
	# falta la clave de despliegue, ssh falla y se registra sin acceso.
	_ssh="$_ssh -o GlobalKnownHostsFile=/dev/null -i $CLAVE_DESPLIEGUE"
	GIT_SSH_COMMAND=$_ssh GIT_SSH_VARIANT=ssh GIT_TERMINAL_PROMPT=0 \
		GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null \
		GIT_ALLOW_PROTOCOL=https:http:ssh:git:file \
		timeout 120 "$GIT" ls-remote --tags -- "$REPO_URL"
}

# Dirección de escucha del panel (routerpanel_listen en el rc.conf de su
# jail; la misma por defecto que routerpanel.rc).
listen_addr()
{
	_l=$(/usr/sbin/sysrc -R "$JAILS_RAIZ/$RUN_JAIL" -n routerpanel_listen 2>/dev/null)
	echo "${_l:-127.0.0.1:3000}"
}

# URL de /salud a partir de routerpanel_listen. Si el panel escucha en
# 0.0.0.0 o en 127.0.0.1 (que en una jail es su propia IP), se usa la IP de
# la jail del panel.
health_url()
{
	_l=$(listen_addr)
	_equipo=${_l%:*}
	case $_equipo in
	0.0.0.0 | 127.* | localhost)
		_equipo=$(/usr/sbin/jls -j "$RUN_JAIL" ip4.addr 2>/dev/null | cut -d, -f1)
		;;
	esac
	echo "http://$_equipo:${_l##*:}/salud"
}

# ¿Quién escucha en el puerto del panel en su jail es el binario de
# releases/$1? Tras un restart el proceso viejo ya no debería estar, pero si
# no hubiera muerto seguiría contestando a /salud con la versión anterior.
serving()
{
	_l=$(listen_addr)
	_pid=$(sockstat -j "$RUN_JAIL" -l -p "${_l##*:}" -q | awk '{ print $3; exit }')
	[ -n "$_pid" ] &&
		[ "$(ps -o command= -p "$_pid")" = "/usr/local/routerpanel/releases/$1/panel" ]
}

# /salud responde 200 con "ok":true y lo sirve releases/$1, como mucho en
# $HEALTH_TIMEOUT segundos de reloj.
healthy()
{
	_deadline=$(($(date +%s) + HEALTH_TIMEOUT))
	_url=$(health_url)
	while [ "$(date +%s)" -lt "$_deadline" ]; do
		# Desde dentro de la jail del panel, como el usuario del servicio: pf
		# solo deja llegar a su puerto a los proxies y a la propia jail.
		# fetch falla con cualquier respuesta que no sea 2xx.
		if "$JEXEC" -U "$RUN_USER" "$RUN_JAIL" \
			/usr/bin/fetch -T 2 -qo - "$_url" 2>/dev/null |
			grep -q '"ok":true' && serving "$1"; then
			return 0
		fi
		sleep 2
	done
	return 1
}

service_ctl()
{
	"$JEXEC" "$RUN_JAIL" /usr/sbin/service routerpanel "$1" >>"$LOG" 2>&1
}

# Guion $1 en la jail del panel como el usuario del servicio, en $DB_DIR.
in_run()
{
	"$JEXEC" -U "$RUN_USER" "$RUN_JAIL" /bin/sh -c "cd $DB_DIR || exit 1
		$1" sh "$DB_BACKUP"
}

# Con el servicio parado: copia panel.db (y -wal/-shm si los hay).
backup_db()
{
	in_run 'rm -rf "$1" && mkdir -m 700 "$1" || exit 1
		for f in panel.db panel.db-wal panel.db-shm; do
			if [ -f "$f" ]; then cp -p "$f" "$1/" || exit 1; fi
		done'
}

# Con el servicio parado: deja la base de datos como estaba antes del
# despliegue (una versión nueva puede haber aplicado una migración que la
# anterior no conoce, y sqlx se negaría a arrancar).
restore_db()
{
	in_run '[ -d "$1" ] || exit 1
		rm -f panel.db panel.db-wal panel.db-shm || exit 1
		for f in "$1"/*; do
			if [ -f "$f" ]; then cp -p "$f" . || exit 1; fi
		done'
}

# Deja la versión $1 de root, con 755/644 (el binario, 755).
asentar_release()
{
	chown -R root:wheel "$1" &&
		find "$1" -type d -exec chmod 755 {} + &&
		find "$1" -type f -exec chmod 644 {} + &&
		chmod 755 "$1/panel"
}

# Registra $3 solo si $STATE/$1 no tiene ya el valor $2 (y lo deja); sale
# con 0 si lo ha registrado. Para no repetir lo mismo en cada pasada del
# cron: una vez por etiqueta, o por etiqueta y fuente.
una_vez()
{
	if [ "$(cat "$STATE/$1" 2>/dev/null)" = "$2" ]; then
		return 1
	fi
	echo "$2" >"$STATE/$1"
	log "$3"
}

# Prueba la fuente $1 («forgejo» o «github»; $2, su nombre para el
# registro) con la función $3 y sus argumentos (el último, la carpeta
# temporal $_tmp). Sale con 0 si deja releases/$_et instalada. Si no, según
# lo que diga la función: 1, aún no la tiene (el motivo, a _motivos); 2, la
# rechaza (FUENTE=rechazada, una vez por etiqueta y fuente); 3, la fuente no
# está disponible (una vez por etiqueta y fuente).
probar_fuente()
{
	_fuente=$1
	_nombre=$2
	shift 2
	rm -rf "$_tmp"
	_err=$("$@" 2>&1)
	_rf=$?
	_err=$(printf '%s' "${_err:-no se puede instalar}" | head -n 1)
	if [ "$_rf" -eq 0 ] && asentar_release "$_tmp" && mv "$_tmp" "$RELEASES/$_et"; then
		FUENTE=instalada
		return 0
	fi
	rm -rf "$_tmp"
	case $_rf in
	1) _motivos="$_motivos
$_nombre: $_err" ;;
	3) una_vez "no-disponible-$_fuente" "$_et" "$_nombre no está disponible: $_err" || true ;;
	*)
		FUENTE=rechazada
		una_vez "rechazada-$_fuente" "$_et" "ERROR: versión de $_nombre de $_et rechazada: $_err" || true
		;;
	esac
	return 1
}

# Deja en releases/$1 la versión de la etiqueta $1 (commit $2) de la primera
# fuente que la tenga: Forgejo y después GitHub. Pone FUENTE a «instalada»,
# «esperar» (ninguna fuente la tiene aún) o «rechazada» (alguna la tenía y no
# cuadraba; no se instala nada).
obtener()
{
	_et=$1
	_commit=$2
	_tmp=$RELEASES/.$_et.tmp
	FUENTE=esperar
	_motivos=
	_donde=

	if [ -n "$FORGEJO_URL" ] && [ -n "$FORGEJO_REPO" ]; then
		rm -f "$STATE/forgejo-desactivado"
		_donde=Forgejo
		if probar_fuente forgejo Forgejo preparar_forgejo "$FORGEJO_URL" "$FORGEJO_REPO" \
			"$FORGEJO_TOKEN_FICHERO" "$DUENO_TOKEN" "$_et" "$_commit" "$PRECOMPILADAS_MINISIGN" "$_tmp"; then
			log "instalada la versión de Forgejo de $_et ($FORGEJO_REPO, $(echo "$_commit" | cut -c1-12))"
			return 0
		fi
	else
		una_vez forgejo-desactivado si "Forgejo: sin FORGEJO_URL o FORGEJO_REPO; no se mira su release" || true
	fi

	if [ -n "$GITHUB_REPO" ]; then
		_donde=${_donde:+$_donde ni en }GitHub
		if probar_fuente github GitHub preparar_github "$GITHUB_REPO" "$_et" "$_commit" "$_tmp"; then
			log "instalada la versión de GitHub de $_et ($GITHUB_REPO, atestación comprobada)"
			return 0
		fi
	fi

	# Una vez por etiqueta: el cron vuelve a mirar cada 15 minutos.
	if [ "$FUENTE" = esperar ] && una_vez esperando "$_et" \
		"esperando la versión precompilada de $_et (no está en ${_donde:-ninguna fuente}; se vuelve a mirar en cada ejecución)"; then
		printf '%s\n' "$_motivos" | while IFS= read -r _m; do
			if [ -n "$_m" ]; then log "$_m"; fi
		done
	fi
}

# Borra las versiones más antiguas; nunca la desplegada.
prune()
{
	_keep=$(deployed)
	# shellcheck disable=SC2012 # nombres propios: sin espacios ni saltos
	ls -1t "$RELEASES" | tail -n +$((KEEP + 1)) | while read -r _old; do
		if [ "$_old" != "$_keep" ]; then
			rm -rf "${RELEASES:?}/$_old"
			log "borrada la versión antigua $_old"
		fi
	done
}

# Apunta «current» a releases/$1, de golpe (rename). El enlace es relativo:
# vale igual desde el host que dentro de la jail.
set_current()
{
	ln -sfh "releases/$1" "$CURRENT.new" && mv -fh "$CURRENT.new" "$CURRENT"
}

# Despliega releases/$1; si /salud no responde, vuelve a la anterior con la
# base de datos de antes y borra la versión fallida.
#
# Primer despliegue (o «current» apunta a una versión que ya no existe): si
# falla, el servicio queda parado, se quita «current» (rc.d se niega a
# arrancar sin versión, también al reiniciar el host) y se borra la versión
# fallida, igual que cuando hay vuelta atrás. Nunca queda «current»
# apuntando a una versión que no respondió.
deploy()
{
	_name=$1
	_prev=$(deployed)
	service_ctl stop
	if ! backup_db; then
		log "ERROR: no se puede copiar la base de datos; no se despliega $_name"
		service_ctl start
		return 1
	fi
	set_current "$_name"
	service_ctl start
	if healthy "$_name"; then
		log "desplegada $_name (antes: ${_prev:-ninguna})"
		return 0
	fi
	log "ERROR: $_name no responde en /salud tras $HEALTH_TIMEOUT s"
	service_ctl stop
	if ! restore_db; then
		log "ERROR: no se puede restaurar la base de datos de antes de $_name"
	fi
	if [ -n "$_prev" ] && [ -d "$RELEASES/$_prev" ]; then
		set_current "$_prev"
		service_ctl start
		if healthy "$_prev"; then
			log "vuelta atrás a $_prev"
		else
			log "ERROR: vuelta atrás a $_prev, pero tampoco responde"
		fi
	else
		rm -f "$CURRENT"
		log "ERROR: no hay versión anterior a la que volver; el servicio queda parado y sin versión desplegada"
	fi
	rm -rf "${RELEASES:?}/$_name"
	log "borrada la versión fallida $_name"
	return 1
}

# Un fallo de la etiqueta $1. Un fallo suelto puede ser transitorio (una
# descarga a medias, la red): la etiqueta sólo queda en «fallida» (y el cron
# deja de intentarlo) tras $MAX_TRIES fallos seguidos; hasta entonces se
# reintenta en la ejecución siguiente. Esperar a la versión, o que se
# rechace una release, no cuenta.
tag_failed()
{
	_n=1
	if [ -f "$STATE/intentos" ] && [ "$(sed -n 1p "$STATE/intentos")" = "$1" ]; then
		_n=$(($(sed -n 2p "$STATE/intentos" | tr -cd '0-9') + 1))
	fi
	if [ "$_n" -ge "$MAX_TRIES" ]; then
		echo "$1" >"$STATE/fallida"
		rm -f "$STATE/intentos"
		log "ERROR: $1 ha fallado $_n veces seguidas; no se reintenta hasta la próxima etiqueta"
	else
		printf '%s\n%s\n' "$1" "$_n" >"$STATE/intentos"
		log "fallo $_n de $MAX_TRIES de $1; se reintenta en la próxima ejecución"
	fi
}

main()
{
	mkdir -p "$STATE" "$RELEASES"
	# Restos de una ejecución cortada a medias.
	find "$RELEASES" -maxdepth 1 -name '.*.tmp' -exec rm -rf {} +

	if [ -z "$REPO_URL" ]; then
		log "ERROR: falta REPO_URL en $CONF"
		exit 1
	fi
	for _f in "$GIT_SH" "$PRECOMPILADAS_SH"; do
		if [ ! -f "$_f" ]; then
			log "ERROR: falta $_f (se instala con deploy/freebsd-jail/install.sh)"
			exit 1
		fi
	done
	if ! ruta_simple "$CLAVE_DESPLIEGUE" || ! ruta_simple "$HOSTS_CONOCIDOS"; then
		log "ERROR: CLAVE_DESPLIEGUE y HOSTS_CONOCIDOS tienen que ser rutas absolutas con solo [A-Za-z0-9/._-]"
		exit 1
	fi
	# shellcheck source=../lib/git.sh
	. "$GIT_SH"
	# shellcheck source=../lib/precompiladas.sh
	. "$PRECOMPILADAS_SH"

	_err=$(mktemp) || exit 1
	if ! _remoto=$(ls_remote 2>"$_err"); then
		log "sin acceso al repositorio: $(grep -v '^[[:space:]]*$' "$_err" | tail -n 1 | cut -c1-200)"
		rm -f "$_err"
		exit 0
	fi
	rm -f "$_err"
	_ultima=$(printf '%s\n' "$_remoto" | ultima_etiqueta_remota)
	if [ -z "$_ultima" ]; then
		exit 0
	fi
	_tag=${_ultima% *}
	_commit=${_ultima#* }
	# ultima_etiqueta_remota ya lo garantiza, pero el nombre acaba en rutas
	# que root borra (rm -rf "$RELEASES/$_tag").
	if ! valid_name "$_tag" || ! version_final "$_tag"; then
		log "ERROR: nombre de versión no válido ($(printf '%s' "$_tag" | head -c 100 | tr -c 'A-Za-z0-9._-' '?')); no se hace nada"
		exit 1
	fi
	if [ "$_tag" = "$(deployed)" ]; then
		exit 0
	fi
	# Una etiqueta que ya falló no se reintenta cada 15 minutos.
	if [ "$_tag" = "$(cat "$STATE/fallida" 2>/dev/null)" ]; then
		exit 0
	fi

	if [ ! -d "$RELEASES/$_tag" ]; then
		obtener "$_tag" "$_commit"
		case $FUENTE in
		esperar) exit 0 ;;
		# Una release rechazada no cuenta como fallo de la etiqueta: otra
		# buena (el runner repite el trabajo, o sale la de GitHub) tiene que
		# poder instalarse en la pasada siguiente.
		rechazada) exit 1 ;;
		esac
	fi
	log "etiqueta nueva $_tag (desplegada: $(deployed))"
	if ! deploy "$_tag"; then
		tag_failed "$_tag"
		exit 1
	fi
	rm -f "$STATE/fallida" "$STATE/intentos" "$STATE/esperando" \
		"$STATE"/rechazada-* "$STATE"/no-disponible-*
	prune
}

main "$@"
