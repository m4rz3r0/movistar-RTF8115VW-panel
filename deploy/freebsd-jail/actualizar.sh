#!/bin/sh
# shellcheck disable=SC2016 # los guiones de in_build se expanden en la jail
#
# Actualizador del panel del router. Corre EN EL HOST como root; se instala
# como /usr/local/sbin/routerpanel-actualizar y lo lanza cron cada 15 minutos
# (/usr/local/etc/cron.d/routerpanel).
#
#   routerpanel-actualizar              despliega la etiqueta v* más reciente
#   routerpanel-actualizar --ref REF    compila y despliega REF (rama o commit);
#                                       para el primer despliegue, antes de que
#                                       exista ninguna etiqueta
#
# Pasos: trae el repositorio (REPO_URL) en la jail de compilación (usuario sin
# privilegios rpbuild) con traer_ultima_etiqueta (deploy/lib/git.sh: nunca
# hace merge, así que tolera un historial reescrito en el remoto); si la
# última v* (sort -V) no es la desplegada, cargo leptos build
# --release y las pruebas; copia binario y web a releases/<versión>; cambia el
# enlace «current» de la jail del panel; reinicia el servicio y espera a que
# /salud responda 200 (60 s como mucho). Si no responde, vuelve a la versión
# anterior. Se conservan las 3 últimas versiones.
#
# Configuración: /usr/local/etc/routerpanel-actualizar.conf (JAILS_RAIZ,
# JAIL_PANEL, JAIL_COMPILAR, REPO_URL; la escribe install.sh) y
# deploy/lib/git.sh en /usr/local/libexec/routerpanel-actualizar. La URL de
# /salud sale de routerpanel_listen en el rc.conf de la jail del panel.
#
# Registro: /var/log/routerpanel-update.log (salida de cargo de la última
# compilación en /var/log/routerpanel-build.log).
#
# Todo lo que hay bajo la jail de compilación lo controla el código del
# repositorio (build.rs, macros, pruebas), así que este guion, que corre como
# root, nunca abre rutas de esa jail desde el host: los artefactos salen como
# un tar que se genera DENTRO de la jail, y del lado del host sólo se aceptan
# ficheros y directorios normales. Lo mismo con la base de datos del panel:
# la copia previa y su restauración se hacen dentro de la jail como el
# usuario del servicio.

set -u
umask 027

JEXEC=/usr/sbin/jexec
# git por ruta absoluta en todo lo que el host usa: ~rpbuild/.cargo/bin (lo
# primero del PATH de la compilación) lo puede escribir cualquier build.rs.
GIT=/usr/local/bin/git
CONF=/usr/local/etc/routerpanel-actualizar.conf
GIT_SH=/usr/local/libexec/routerpanel-actualizar/git.sh
# Valores por defecto; los de la instalación vienen de $CONF.
JAILS_RAIZ=/usr/local/jails/containers
JAIL_PANEL=routerpanel
JAIL_COMPILAR=routerpanel-build
REPO_URL=
# Trabajos de cargo a la vez; vacía, los que cargo decida (uno por núcleo).
TRABAJOS_COMPILACION=
if [ -f "$CONF" ]; then
	# shellcheck source=/dev/null
	. "$CONF"
fi
BUILD_JAIL=$JAIL_COMPILAR
RUN_JAIL=$JAIL_PANEL
BUILD_USER=rpbuild
SRC=/home/rpbuild/panel
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
BUILD_LOG=/var/log/routerpanel-build.log
LOCK=/var/run/routerpanel-actualizar.lock
KEEP=3
# Fallos seguidos de una etiqueta antes de darla por fallida.
MAX_TRIES=2

# Una sola ejecución a la vez: una compilación puede durar más de 15 minutos.
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

# Ejecuta el guion $2 en la jail de compilación como $BUILD_USER, dentro del
# clon y con PATH=$1; el resto de argumentos le llegan como $1, $2…
in_jail_sh()
{
	_path=$1
	_script=$2
	shift 2
	"$JEXEC" -l -U "$BUILD_USER" "$BUILD_JAIL" /bin/sh -c "
		PATH=$_path
		GIT_SSH_COMMAND='ssh -o BatchMode=yes -o ConnectTimeout=15'
		export PATH GIT_SSH_COMMAND
		cd $SRC || exit 1
		$_script" sh "$@"
}

# Para compilar y probar: con ~/.cargo/bin delante.
in_build()
{
	in_jail_sh "\$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin" "$@"
}

# Para lo que el host usa (nombres, commits): PATH del sistema y `git` es
# siempre $GIT. Aun así, la salida no es de fiar (el clon lo puede tocar el
# código que se compila): el host valida todo lo que sale de aquí antes de
# usarlo en una ruta (valid_name, valid_sha).
in_build_git()
{
	_script=$1
	shift
	in_jail_sh /usr/local/bin:/usr/bin:/bin "git() { $GIT \"\$@\"; }
		$_script" "$@"
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

# Commit abreviado o completo: 7 a 40 cifras hexadecimales en minúscula.
valid_sha()
{
	case "$1" in
	'' | *[!0-9a-f]*) return 1 ;;
	esac
	[ "${#1}" -ge 7 ] && [ "${#1}" -le 40 ]
}

deployed()
{
	_t=$(readlink "$CURRENT" 2>/dev/null) || return 0
	basename "$_t"
}

# Dirección de escucha del panel (routerpanel_listen en el rc.conf de su
# jail; la misma por defecto que routerpanel.rc).
listen_addr()
{
	_l=$(/usr/sbin/sysrc -R "$JAILS_RAIZ/$RUN_JAIL" -n routerpanel_listen 2>/dev/null)
	echo "${_l:-127.0.0.1:3000}"
}

# URL de /salud a partir de routerpanel_listen. Se pregunta desde la jail de
# compilación: si el panel escucha en 0.0.0.0 o en 127.0.0.1 (que en una
# jail es su propia IP), se usa la IP de la jail del panel.
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
		# Desde la jail de compilación: pf sólo deja llegar al panel desde
		# los proxies y desde ella. fetch falla con cualquier respuesta que
		# no sea 2xx.
		if /usr/sbin/jexec -U "$BUILD_USER" "$BUILD_JAIL" \
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
	/usr/sbin/jexec "$RUN_JAIL" /usr/sbin/service routerpanel "$1" >>"$LOG" 2>&1
}

# Guion $1 en la jail del panel como el usuario del servicio, en $DB_DIR.
in_run()
{
	/usr/sbin/jexec -U "$RUN_USER" "$RUN_JAIL" /bin/sh -c "cd $DB_DIR || exit 1
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

# Compila $1 (referencia de git) y deja el resultado en releases/$2.
build_release()
{
	_ref=$1
	_name=$2
	if ! in_build_git 'git -c advice.detachedHead=false checkout --quiet --force --detach "$1" &&
		git clean -fdxq -e target' "$_ref" >"$BUILD_LOG" 2>&1; then
		log "ERROR: no se puede sacar $_ref del repositorio (ver $BUILD_LOG)"
		return 1
	fi
	log "compilando $_name ($(in_build_git 'git rev-parse --short HEAD'))"
	# wasm-bindgen-cli tiene que ser exactamente la versión de wasm-bindgen
	# del Cargo.lock; si el repositorio la cambia, se instala la nueva.
	if ! in_build '
		want=$(awk "/^name = \"wasm-bindgen\"\$/ { getline; gsub(/version = |\"/, \"\"); print; exit }" Cargo.lock)
		have=$(wasm-bindgen --version 2>/dev/null | cut -d" " -f2)
		if [ "$want" != "$have" ]; then
			echo "instalando wasm-bindgen-cli $want (había: ${have:-ninguno})"
			cargo install --locked wasm-bindgen-cli --version "$want" || exit 1
		fi
		# Trabajos a la vez (TRABAJOS_COMPILACION, si se fijó; menos trabajos,
		# menos memoria) y prioridad baja, para que el resto de servicios del
		# servidor no lo note.
		case $1 in
		"") ;;
		*[!0-9]* | 0*) echo "TRABAJOS_COMPILACION no es válida; se usa el valor de cargo" ;;
		*)
			CARGO_BUILD_JOBS=$1
			export CARGO_BUILD_JOBS
			;;
		esac
		renice -n 15 $$ >/dev/null 2>&1
		cargo leptos build --release &&
		# Red de seguridad: una prueba colgada no deja el actualizador
		# esperando para siempre (con lockf, tampoco correría el siguiente).
		# Falla como cualquier prueba; cargo deja en el registro cuál llevaba
		# más de 60 s («has been running for over 60 seconds»).
		timeout -k 60 3600 cargo test --workspace &&
		timeout -k 60 3600 cargo test --workspace --features panel/ssr' "$TRABAJOS_COMPILACION" >>"$BUILD_LOG" 2>&1; then
		log "ERROR: falló la compilación o las pruebas de $_name (ver $BUILD_LOG)"
		return 1
	fi
	_tmp=$RELEASES/.$_name.tmp
	rm -rf "$_tmp"
	if ! extract_release "$_tmp"; then
		log "ERROR: no se pueden copiar los artefactos de $_name a $RELEASES"
		rm -rf "$_tmp"
		return 1
	fi
	mv "$_tmp" "$RELEASES/$_name"
}

# Saca target/release/panel y target/site de la jail de compilación a $1. El
# tar se genera dentro de la jail (los enlaces que haya ahí se resuelven, si
# acaso, dentro de la jail); bsdtar no extrae rutas absolutas ni con «..»; y
# después se rechaza todo lo que no sea fichero o directorio normal.
extract_release()
{
	_dst=$1
	mkdir -m 755 "$_dst" || return 1
	# pipefail (en un subshell, para no cambiar el resto del guion): si el tar
	# de la jail falla a medias, el del host puede terminar bien con lo que
	# le llegó, y la versión quedaría incompleta.
	if ! (
		# shellcheck disable=SC3040 # el sh de FreeBSD (14+) tiene pipefail
		set -o pipefail
		in_build 'tar -cf - -C target release/panel site' |
			tar -xf - -C "$_dst" --no-same-owner --no-same-permissions
	); then
		log "ERROR: falló el tar de los artefactos (en la jail o en el host)"
		return 1
	fi
	if [ -n "$(find "$_dst" ! -type f ! -type d)" ]; then
		log "ERROR: los artefactos traen algo que no es fichero ni directorio"
		return 1
	fi
	if [ ! -f "$_dst/release/panel" ] || [ ! -d "$_dst/site" ] ||
		[ "$(find "$_dst" -type f -links +1)" ]; then
		log "ERROR: faltan release/panel o site/, o hay enlaces duros"
		return 1
	fi
	mv "$_dst/release/panel" "$_dst/panel" && rmdir "$_dst/release" || return 1
	in_build_git 'git rev-parse HEAD' >"$_dst/COMMIT" || return 1
	chown -R root:wheel "$_dst" &&
		find "$_dst" -type d -exec chmod 755 {} + &&
		find "$_dst" -type f -exec chmod 644 {} + &&
		chmod 755 "$_dst/panel"
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
# Casos sin versión anterior a la que volver:
# - $1 ya es la desplegada (sólo con --ref y el mismo commit): no se cambia
#   nada; se reinicia el servicio y se comprueba /salud. Si no responde, se
#   queda como está (en marcha; daemon -r la relanza) y no se borra: es la
#   que estaba funcionando, y lo normal es que falle otra cosa (el router).
# - Primer despliegue (o «current» apunta a una versión que ya no existe): si
#   falla, el servicio queda parado, se quita «current» (rc.d se niega a
#   arrancar sin versión, también al reiniciar el host) y se borra la versión
#   fallida, igual que cuando hay vuelta atrás. Nunca queda «current»
#   apuntando a una versión que no respondió.
deploy()
{
	_name=$1
	_prev=$(deployed)
	if [ "$_name" = "$_prev" ]; then
		log "$_name ya está desplegada; se reinicia"
		service_ctl restart
		if healthy "$_name"; then
			log "reiniciada $_name"
			return 0
		fi
		log "ERROR: $_name no responde en /salud tras reiniciarla; sigue desplegada (no hay otra a la que volver)"
		return 1
	fi
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

# Un fallo de la etiqueta $1 (nada si está vacía: --ref). Un fallo suelto
# puede ser transitorio (red, una prueba sensible a tiempos): la etiqueta sólo
# queda en «fallida» (y el cron deja de intentarlo) tras $MAX_TRIES fallos
# seguidos; hasta entonces se reintenta en la ejecución siguiente.
tag_failed()
{
	[ -n "$1" ] || return 0
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
	_ref=
	_tag=
	case "${1:-}" in
	--ref)
		_ref=${2:?falta la referencia tras --ref}
		;;
	"") ;;
	*)
		echo "uso: $0 [--ref REF]" >&2
		exit 2
		;;
	esac

	if ! in_build 'true' 2>/dev/null; then
		log "ERROR: no existe el clon $SRC en $BUILD_JAIL; sin acceso al repositorio"
		exit 0
	fi
	if [ -z "$REPO_URL" ]; then
		log "ERROR: falta REPO_URL en $CONF"
		exit 1
	fi
	if [ ! -f "$GIT_SH" ]; then
		log "ERROR: falta $GIT_SH (se instala con deploy/freebsd-jail/install.sh)"
		exit 1
	fi
	# Funciones de git.sh: se mandan a la jail como texto (el host nunca
	# ejecuta nada del clon). Sin --ref, traer_ultima_etiqueta trae y deja
	# el clon en la última v*; con --ref, sólo se traen las referencias.
	_git_sh=$(cat "$GIT_SH")
	if [ -n "$_ref" ]; then
		_traer=traer_referencias
	else
		_traer=traer_ultima_etiqueta
	fi
	_err=$(mktemp)
	if ! _salida=$(in_build_git "$_git_sh
		git remote set-url origin \"\$1\" && $_traer ." "$REPO_URL" 2>"$_err"); then
		log "sin acceso al repositorio: $(head -n 1 "$_err")"
		rm -f "$_err"
		if [ -z "$_ref" ]; then
			exit 0
		fi
		log "se compila $_ref con lo que ya hay en el clon"
	fi
	rm -f "$_err"

	if [ -n "$_ref" ]; then
		# Las ramas se toman del remoto si existe; si no, la local.
		if in_build_git 'git rev-parse --verify --quiet "origin/$1^{commit}"' "$_ref" >/dev/null; then
			_ref=origin/$_ref
		fi
		_sha=$(in_build_git 'git rev-parse --short "$1^{commit}"' "$_ref") || {
			log "ERROR: $_ref no existe en el repositorio"
			exit 1
		}
		if ! valid_sha "$_sha"; then
			log "ERROR: git devolvió un commit no válido para $_ref; no se hace nada"
			exit 1
		fi
		_name=$(printf '%s' "${_ref#origin/}" | tr -c 'A-Za-z0-9._-' '_')-$_sha
	else
		_tag=$(printf '%s\n' "$_salida" | tail -n 1)
		if [ -z "$_tag" ]; then
			exit 0
		fi
		_name=$_tag
		_ref=refs/tags/$_tag
	fi
	# Lo que sale de la jail de compilación no es de fiar: el nombre acaba en
	# rutas que root borra (rm -rf "$RELEASES/$_name").
	if ! valid_name "$_name"; then
		log "ERROR: nombre de versión no válido ($(printf '%s' "$_name" | head -c 100 | tr -c 'A-Za-z0-9._-' '?')); no se hace nada"
		exit 1
	fi
	if [ -n "$_tag" ]; then
		if [ "$_name" = "$(deployed)" ]; then
			exit 0
		fi
		# Una etiqueta que ya falló no se recompila cada 15 minutos.
		if [ "$_name" = "$(cat "$STATE/fallida" 2>/dev/null)" ]; then
			exit 0
		fi
		log "etiqueta nueva $_tag (desplegada: $(deployed))"
	fi

	# «fallida» sólo guarda etiquetas (lo que mira el cron): un --ref que falla
	# no debe pisar una etiqueta fallida y hacer que se reintente.
	if [ ! -d "$RELEASES/$_name" ] && ! build_release "$_ref" "$_name"; then
		tag_failed "$_tag"
		exit 1
	fi
	if ! deploy "$_name"; then
		tag_failed "$_tag"
		exit 1
	fi
	if [ -n "$_tag" ]; then rm -f "$STATE/fallida" "$STATE/intentos"; fi
	prune
}

main "$@"
