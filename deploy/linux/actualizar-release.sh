#!/bin/sh
# Actualizador opcional del panel en Linux con systemd, desde las releases
# publicadas. Corre como root (lo lanzas tú o un temporizador; no se instala
# ninguno por defecto: ver README.md). Se instala como
# /usr/local/sbin/routerpanel-actualizar.
#
#   routerpanel-actualizar              despliega la última versión publicada
#   routerpanel-actualizar --comprobar  solo dice si hay una versión nueva
#   routerpanel-actualizar --forzar     despliega la última publicada aunque
#                                       no sea posterior a la desplegada
#
# Configuración: /etc/routerpanel/actualizar.env (root, 600; o el entorno):
#   RP_RELEASES_URL    base de las releases, p. ej.
#                      https://github.com/<usuario>/<repo>/releases; tiene
#                      que ser https:// salvo que haya RP_MINISIGN_CLAVE
#   RP_OBJETIVO        x86_64-unknown-linux-gnu o aarch64-unknown-linux-gnu
#                      (por defecto, el de este equipo)
#   RP_MINISIGN_CLAVE  opcional: clave pública de minisign; si está, la firma
#                      SHA256SUMS.minisig tiene que ser válida
#
# Pasos:
# 1. Lee la última versión de <base>/latest/download/SHA256SUMS (la línea de
#    routerpanel-<versión>-<objetivo>.tar.gz) y, si es posterior a la
#    desplegada (sort -V; con --forzar, basta con que sea otra), descarga de <base>/download/<versión>/ el archivo y su SHA256SUMS (y la
#    firma si hay clave).
# 2. Comprueba la firma (con clave) y el sha256; si algo no cuadra, se para
#    sin tocar nada.
# 3. Lo desempaqueta en /opt/routerpanel/releases/<versión>; para el
#    servicio, copia la base de datos, cambia el enlace «current» y arranca.
# 4. Espera a que /salud responda 200 con "ok":true (60 s como mucho); si no,
#    vuelve a la versión anterior con la base de datos de antes y borra la
#    nueva.
# Se conservan las 3 últimas versiones (nunca se borra la desplegada).

set -u
umask 022

CONF=${RP_ACTUALIZAR_CONF:-/etc/routerpanel/actualizar.env}
if [ -f "$CONF" ]; then
	# shellcheck source=/dev/null
	. "$CONF"
fi
RP_RELEASES_URL=${RP_RELEASES_URL:-}
RP_MINISIGN_CLAVE=${RP_MINISIGN_CLAVE:-}
# Rutas y órdenes: las cambian las pruebas (deploy/pruebas/linux.sh).
PREFIJO=${RP_PREFIJO:-/opt/routerpanel}
SERVICIO=${RP_SERVICIO:-routerpanel}
SYSTEMCTL=${RP_SYSTEMCTL:-systemctl}
ENV_PANEL=${RP_ENV_PANEL:-/etc/routerpanel/env}
ESPERA=${RP_SALUD_ESPERA:-60}
RELEASES=$PREFIJO/releases
CURRENT=$PREFIJO/current
KEEP=3

log()
{
	echo "$(date '+%Y-%m-%d %H:%M:%S') $*"
}

error()
{
	log "ERROR: $*" >&2
}

# Nombre de versión aceptable como componente de ruta: empieza por letra o
# cifra, solo [A-Za-z0-9._-], sin «..» y como mucho 100 caracteres.
nombre_valido()
{
	case "$1" in
	'' | [!A-Za-z0-9]* | *[!A-Za-z0-9._-]* | *..*) return 1 ;;
	esac
	[ "${#1}" -le 100 ]
}

objetivo_por_defecto()
{
	case "$(uname -m)" in
	x86_64) echo x86_64-unknown-linux-gnu ;;
	aarch64 | arm64) echo aarch64-unknown-linux-gnu ;;
	*) echo "" ;;
	esac
}

desplegada()
{
	_t=$(readlink "$CURRENT" 2>/dev/null) || return 0
	basename "$_t"
}

# Valor de la variable $1 en el fichero de entorno del panel (sin comillas).
valor_env()
{
	[ -f "$ENV_PANEL" ] || return 0
	sed -n "s/^$1=//p" "$ENV_PANEL" | tail -n 1 | tr -d "\"'"
}

# URL de /salud: RP_SALUD_URL o, si no, a partir de RP_LISTEN del panel.
url_salud()
{
	if [ -n "${RP_SALUD_URL:-}" ]; then
		echo "$RP_SALUD_URL"
		return
	fi
	_l=$(valor_env RP_LISTEN)
	_l=${_l:-127.0.0.1:3000}
	_equipo=${_l%:*}
	case $_equipo in
	0.0.0.0) _equipo=127.0.0.1 ;;
	'[::]') _equipo='[::1]' ;;
	esac
	echo "http://$_equipo:${_l##*:}/salud"
}

# Carpeta de panel.db (de RP_DB en el entorno del panel).
dir_db()
{
	_db=$(valor_env RP_DB)
	dirname "${_db:-/var/lib/routerpanel/panel.db}"
}

# Descarga $1 en $2. Solo https, salvo con firma (RP_MINISIGN_CLAVE): ahí la
# integridad la da la firma y vale también http (también en redirecciones).
descargar()
{
	_proto='=https'
	if [ -n "$RP_MINISIGN_CLAVE" ]; then _proto='=https,http'; fi
	curl -fsSL --proto "$_proto" --proto-redir "$_proto" --max-time 600 -o "$2" "$1"
}

# ¿Es $1 posterior a $2 según sort -V? (Con $2 vacía, sí.)
posterior()
{
	[ -n "$2" ] || return 0
	[ "$1" != "$2" ] &&
		[ "$(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -n 1)" = "$1" ]
}

# /salud responde 200 con "ok":true en $ESPERA segundos.
sano()
{
	_fin=$(($(date +%s) + ESPERA))
	_url=$(url_salud)
	while [ "$(date +%s)" -lt "$_fin" ]; do
		if curl -fsS -m 2 "$_url" 2>/dev/null | grep -q '"ok":true'; then
			return 0
		fi
		sleep 1
	done
	return 1
}

servicio()
{
	"$SYSTEMCTL" "$1" "$SERVICIO"
}

# Con el servicio parado: copia panel.db (y -wal/-shm) a antes-de-actualizar.
copiar_db()
{
	_d=$(dir_db)
	[ -d "$_d" ] || return 0
	rm -rf "$_d/antes-de-actualizar" && mkdir -m 700 "$_d/antes-de-actualizar" || return 1
	for _f in panel.db panel.db-wal panel.db-shm; do
		if [ -f "$_d/$_f" ]; then cp -p "$_d/$_f" "$_d/antes-de-actualizar/" || return 1; fi
	done
}

# Con el servicio parado: deja la base de datos como antes del despliegue
# (una versión nueva puede haber aplicado una migración que la anterior no
# conoce, y no arrancaría).
restaurar_db()
{
	_d=$(dir_db)
	[ -d "$_d/antes-de-actualizar" ] || return 0
	rm -f "$_d/panel.db" "$_d/panel.db-wal" "$_d/panel.db-shm" || return 1
	for _f in "$_d"/antes-de-actualizar/*; do
		if [ -f "$_f" ]; then cp -p "$_f" "$_d/" || return 1; fi
	done
}

# Apunta «current» a releases/$1 de golpe (rename).
fijar_current()
{
	ln -sfn "releases/$1" "$CURRENT.nuevo" && mv -fT "$CURRENT.nuevo" "$CURRENT"
}

# Desempaqueta $1 (archivo comprobado) en releases/$2. El archivo trae
# panel, site/ y el resto, en la raíz o dentro de una única carpeta.
desempaquetar()
{
	_arch=$1
	_dest=$RELEASES/$2
	_tmp=$RELEASES/.$2.tmp
	rm -rf "$_tmp"
	mkdir -p "$_tmp/x" || return 1
	# Como root, tar conservaría dueños y modos del archivo (setuid incluido).
	if ! tar -xzf "$_arch" -C "$_tmp/x" --no-same-owner --no-same-permissions; then
		error "no se puede desempaquetar $(basename "$_arch")"
		rm -rf "$_tmp"
		return 1
	fi
	if [ -n "$(find "$_tmp/x" ! -type f ! -type d)" ]; then
		error "el archivo trae algo que no es fichero ni directorio"
		rm -rf "$_tmp"
		return 1
	fi
	_raiz=$_tmp/x
	if [ ! -f "$_raiz/panel" ]; then
		set -- "$_tmp"/x/*
		if [ "$#" -eq 1 ] && [ -f "$1/panel" ]; then
			_raiz=$1
		fi
	fi
	if [ ! -f "$_raiz/panel" ] || [ ! -d "$_raiz/site" ]; then
		error "al archivo le faltan panel o site/"
		rm -rf "$_tmp"
		return 1
	fi
	# touch: podar ordena por fecha, y tar deja la del archivo (con builds
	# reproducibles, la misma en todas las versiones).
	mv "$_raiz" "$_dest" && chmod -R go-w,a-s "$_dest" && chmod 755 "$_dest/panel" &&
		touch "$_dest" && rm -rf "$_tmp"
}

# Borra las versiones más antiguas; nunca la desplegada.
podar()
{
	_keep=$(desplegada)
	# shellcheck disable=SC2012 # nombres validados: sin espacios ni saltos
	ls -1t "$RELEASES" | tail -n +$((KEEP + 1)) | while read -r _vieja; do
		if [ "$_vieja" != "$_keep" ]; then
			rm -rf "${RELEASES:?}/$_vieja"
			log "borrada la versión antigua $_vieja"
		fi
	done
}

# Despliega releases/$1; si /salud no responde, vuelve a la anterior.
desplegar()
{
	_nueva=$1
	_antes=$(desplegada)
	# Sin parar el servicio, la copia de la base de datos no sería fiable.
	if ! servicio stop; then
		error "no se puede parar $SERVICIO; no se despliega $_nueva"
		return 1
	fi
	if ! copiar_db; then
		error "no se puede copiar la base de datos; no se despliega $_nueva"
		servicio start
		return 1
	fi
	if ! fijar_current "$_nueva"; then
		error "no se puede cambiar current a $_nueva; sigue ${_antes:-sin versión}"
		servicio start
		return 1
	fi
	servicio start
	if sano; then
		log "desplegada $_nueva (antes: ${_antes:-ninguna})"
		return 0
	fi
	error "$_nueva no responde en /salud tras $ESPERA s"
	servicio stop
	restaurar_db || error "no se puede restaurar la base de datos de antes de $_nueva"
	if [ -n "$_antes" ] && [ -d "$RELEASES/$_antes" ]; then
		fijar_current "$_antes"
		servicio start
		if sano; then
			log "vuelta atrás a $_antes"
		else
			error "vuelta atrás a $_antes, pero tampoco responde"
		fi
	else
		rm -f "$CURRENT"
		error "no hay versión anterior; el servicio queda parado y sin versión desplegada"
	fi
	rm -rf "${RELEASES:?}/$_nueva"
	log "borrada la versión fallida $_nueva"
	return 1
}

main()
{
	_comprobar=
	_forzar=
	for _arg in "$@"; do
		case $_arg in
		--comprobar) _comprobar=1 ;;
		--forzar) _forzar=1 ;;
		*)
			echo "uso: $0 [--comprobar] [--forzar]" >&2
			exit 2
			;;
		esac
	done
	RP_OBJETIVO=${RP_OBJETIVO:-$(objetivo_por_defecto)}
	case $RP_OBJETIVO in
	x86_64-unknown-linux-gnu | aarch64-unknown-linux-gnu) ;;
	*)
		error "RP_OBJETIVO no válido o arquitectura sin releases: «$RP_OBJETIVO»"
		exit 1
		;;
	esac
	if [ -z "$RP_RELEASES_URL" ]; then
		error "falta RP_RELEASES_URL (en $CONF o en el entorno)"
		exit 1
	fi
	_base=${RP_RELEASES_URL%/}
	case $_base in
	https://*) ;;
	http://*)
		if [ -z "$RP_MINISIGN_CLAVE" ]; then
			error "RP_RELEASES_URL tiene que ser https:// (http:// solo con RP_MINISIGN_CLAVE: sin firma, nada garantiza lo que se descarga)"
			exit 1
		fi
		;;
	*)
		error "RP_RELEASES_URL tiene que empezar por https://"
		exit 1
		;;
	esac
	for _orden in curl sha256sum tar; do
		command -v "$_orden" >/dev/null 2>&1 || {
			error "falta $_orden"
			exit 1
		}
	done
	if [ -n "$RP_MINISIGN_CLAVE" ] && ! command -v minisign >/dev/null 2>&1; then
		error "RP_MINISIGN_CLAVE está puesta pero no hay minisign"
		exit 1
	fi

	_tmp=$(mktemp -d) || exit 1
	trap 'rm -rf "$_tmp"' EXIT
	trap 'exit 143' INT TERM

	if ! descargar "$_base/latest/download/SHA256SUMS" "$_tmp/ultima"; then
		error "no se puede descargar $_base/latest/download/SHA256SUMS"
		exit 1
	fi
	_ver=$(sed -n "s/^[0-9a-f]\{64\} [ *]routerpanel-\(.*\)-$RP_OBJETIVO\.tar\.gz\$/\1/p" "$_tmp/ultima" | head -n 1)
	if ! nombre_valido "$_ver"; then
		error "la última release no trae un archivo válido para $RP_OBJETIVO"
		exit 1
	fi
	_hay=$(desplegada)
	if [ "$_ver" = "$_hay" ]; then
		log "al día: $_ver"
		exit 0
	fi
	# La versión «última» no va firmada: sin esto, quien controle la descarga
	# podría hacer volver a una versión antigua (aunque esté firmada).
	if ! posterior "$_ver" "$_hay"; then
		if [ -n "$_comprobar" ]; then
			log "la última publicada ($_ver) no es posterior a la desplegada ($_hay)"
			exit 0
		fi
		if [ -z "$_forzar" ]; then
			error "la última publicada ($_ver) no es posterior a la desplegada ($_hay); no se despliega (--forzar para hacerlo igualmente)"
			exit 1
		fi
		log "se despliega $_ver, anterior a $_hay, por --forzar"
	fi
	if [ -n "$_comprobar" ]; then
		log "hay una versión nueva: $_ver (desplegada: ${_hay:-ninguna})"
		exit 0
	fi

	# Una ejecución a la vez.
	mkdir -p "$RELEASES" || exit 1
	if command -v flock >/dev/null 2>&1; then
		exec 9>"$PREFIJO/.actualizar.lock"
		if ! flock -n 9; then
			log "ya hay otra actualización en marcha"
			exit 0
		fi
	fi
	find "$RELEASES" -maxdepth 1 -name '.*.tmp' -exec rm -rf {} +

	_archivo=routerpanel-$_ver-$RP_OBJETIVO.tar.gz
	_url=$_base/download/$_ver
	log "descargando $_ver"
	if ! descargar "$_url/SHA256SUMS" "$_tmp/SHA256SUMS" ||
		! descargar "$_url/$_archivo" "$_tmp/$_archivo"; then
		error "no se puede descargar $_url/$_archivo o su SHA256SUMS"
		exit 1
	fi
	if [ -n "$RP_MINISIGN_CLAVE" ]; then
		if ! descargar "$_url/SHA256SUMS.minisig" "$_tmp/SHA256SUMS.minisig" ||
			! minisign -Vq -P "$RP_MINISIGN_CLAVE" -m "$_tmp/SHA256SUMS" \
				-x "$_tmp/SHA256SUMS.minisig"; then
			error "la firma de SHA256SUMS de $_ver no es válida; no se toca nada"
			exit 1
		fi
	fi
	grep " [ *]$_archivo\$" "$_tmp/SHA256SUMS" | head -n 1 |
		sed 's/ \*/  /' >"$_tmp/suma"
	if [ ! -s "$_tmp/suma" ] || ! (cd "$_tmp" && sha256sum -c --status suma); then
		error "el sha256 de $_archivo no coincide con SHA256SUMS; no se toca nada"
		exit 1
	fi

	if [ ! -d "$RELEASES/$_ver" ] && ! desempaquetar "$_tmp/$_archivo" "$_ver"; then
		exit 1
	fi
	desplegar "$_ver" || exit 1
	podar
}

main "$@"
