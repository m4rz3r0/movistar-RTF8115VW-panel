# shellcheck shell=sh
#
# Versiones precompiladas para FreeBSD: el servidor nunca compila. El
# actualizador (deploy/freebsd-jail/actualizar.sh) instala la versión de una
# etiqueta de una de estas dos fuentes, en este orden:
#
# - la release de la etiqueta en Forgejo (FORGEJO_URL y FORGEJO_REPO), que
#   publica el runner de Actions del PC con .forgejo/workflows/freebsd.yml y
#   deploy/cross/publicar-forgejo.sh (preparar_forgejo);
# - la release vX.Y.Z del repositorio en GitHub (GITHUB_REPO), que publica
#   .github/workflows/release.yml con su atestación de origen
#   (preparar_github).
#
# Se carga con «.»; solo funciones, sin efectos al cargarse. Las usan el
# actualizador (en el host, como root), el PC de compilación
# (deploy/cross/) y las pruebas (deploy/pruebas/precompiladas.sh).
#
# El archivo es el de las releases de Linux (tools/empaquetar-release.sh) con
# el objetivo de FreeBSD: routerpanel-<versión>-x86_64-unknown-freebsd.tar.gz,
# con una única carpeta del mismo nombre (sin .tar.gz) que trae panel, site/ y
# COMMIT (el commit completo de la etiqueta); lo hace
# deploy/cross/empaquetar-freebsd.sh. SHA256SUMS va en el formato de
# sha256sum. En Forgejo, si el PC firma con minisign, también
# SHA256SUMS.minisig.

PRECOMPILADA_OBJETIVO=x86_64-unknown-freebsd

# Nombre del archivo de la versión $1.
archivo_precompilada()
{
	echo "routerpanel-$1-$PRECOMPILADA_OBJETIVO.tar.gz"
}

# ¿Es $1 una versión que se puede precompilar? Una etiqueta v* con nombre
# válido como componente de ruta: empieza por «v» y una cifra, solo
# [A-Za-z0-9._-], sin «..» y como mucho 100 caracteres.
version_precompilable()
{
	case "$1" in
	v[0-9]*) ;;
	*) return 1 ;;
	esac
	case "$1" in
	*[!A-Za-z0-9._-]* | *..*) return 1 ;;
	esac
	[ "${#1}" -le 100 ]
}

# ¿Es $1 una versión final vX.Y.Z (solo cifras)? Las únicas que publica la
# release de GitHub y que busca el actualizador.
version_final()
{
	printf '%s\n' "$1" | grep -Eqx 'v[0-9]+\.[0-9]+\.[0-9]+' && [ "${#1}" -le 40 ]
}

# La versión más reciente de las etiquetas que llegan por la entrada
# estándar, una por línea: la última vX.Y.Z según sort -V, la misma que elige
# ultima_etiqueta_remota (deploy/lib/git.sh) en el servidor. Así el PC compila
# justo la que el actualizador va a buscar.
ultima_version()
{
	{ grep -Ex 'v[0-9]+\.[0-9]+\.[0-9]+' || true; } | sort -V | tail -n 1
}

# sha256 de $1 en hexadecimal (sha256sum en Linux y en FreeBSD 14+; si no,
# sha256 -q de FreeBSD).
suma_sha256()
{
	if command -v sha256sum >/dev/null 2>&1; then
		sha256sum <"$1" | cut -d' ' -f1
	else
		sha256 -q <"$1"
	fi
}

# Comprueba la versión $2 de la carpeta $1 (descargada en $1/$2/) y la deja
# desempaquetada en $5 (que no debe existir), con panel, site/ y COMMIT y nada
# más. $3 es el commit completo de la etiqueta (git ls-remote de REPO_URL) y
# $4 la clave pública de minisign (vacía: sin firma). Escribe el motivo en la
# salida de error y sale con 1 si algo no cuadra; $5 no queda creada en ese
# caso.
#
# Antes de leer nada se copia a una carpeta de trabajo propia (con un tiempo
# máximo, por si hubiera una FIFO en lugar de un fichero) y todo se comprueba
# sobre la copia.
preparar_precompilada()
{
	_origen=$1
	_ver=$2
	_commit=$3
	_clave=$4
	_dst=$5
	_arch=$(archivo_precompilada "$_ver")
	_raiz=${_arch%.tar.gz}
	if ! version_precompilable "$_ver"; then
		echo "versión no válida" >&2
		return 1
	fi
	case $_commit in
	'' | *[!0-9a-f]*)
		echo "commit no válido" >&2
		return 1
		;;
	esac
	if [ "${#_commit}" -ne 40 ]; then
		echo "commit no válido" >&2
		return 1
	fi
	_trabajo=$(mktemp -d) || return 1
	for _f in "$_arch" SHA256SUMS SHA256SUMS.minisig; do
		if [ "$_f" = SHA256SUMS.minisig ] && [ -z "$_clave" ]; then
			continue
		fi
		if [ ! -f "$_origen/$_ver/$_f" ] || [ -L "$_origen/$_ver/$_f" ] ||
			! timeout 300 cp "$_origen/$_ver/$_f" "$_trabajo/$_f" ||
			[ ! -f "$_trabajo/$_f" ] || [ -L "$_trabajo/$_f" ]; then
			echo "falta $_f o no se puede copiar" >&2
			rm -rf "$_trabajo"
			return 1
		fi
	done
	if [ -n "$_clave" ] && ! minisign -Vq -P "$_clave" -m "$_trabajo/SHA256SUMS" \
		-x "$_trabajo/SHA256SUMS.minisig" >/dev/null 2>&1; then
		echo "la firma de SHA256SUMS no es válida" >&2
		rm -rf "$_trabajo"
		return 1
	fi
	# La línea del archivo: «<64 hex>  <archivo>» (o « *<archivo>»).
	_esperada=$(sed -n "s/^\([0-9a-f]\{64\}\) [ *]$_arch\$/\1/p" "$_trabajo/SHA256SUMS" | head -n 1)
	if [ -z "$_esperada" ] || [ "$(suma_sha256 "$_trabajo/$_arch")" != "$_esperada" ]; then
		echo "el sha256 de $_arch no coincide con SHA256SUMS" >&2
		rm -rf "$_trabajo"
		return 1
	fi
	mkdir "$_trabajo/x" || return 1
	if ! tar -xzf "$_trabajo/$_arch" -C "$_trabajo/x" --no-same-owner --no-same-permissions 2>/dev/null; then
		echo "no se puede desempaquetar $_arch" >&2
		rm -rf "$_trabajo"
		return 1
	fi
	if [ -n "$(find "$_trabajo/x" ! -type f ! -type d)" ] ||
		[ -n "$(find "$_trabajo/x" -type f -links +1)" ]; then
		echo "el archivo trae algo que no es fichero ni directorio, o enlaces duros" >&2
		rm -rf "$_trabajo"
		return 1
	fi
	# Exactamente una carpeta, la de la versión.
	if [ "$(find "$_trabajo/x" -mindepth 1 -maxdepth 1 | wc -l)" -ne 1 ] ||
		[ ! -d "$_trabajo/x/$_raiz" ]; then
		echo "el archivo no trae solo la carpeta $_raiz" >&2
		rm -rf "$_trabajo"
		return 1
	fi
	_d=$_trabajo/x/$_raiz
	if [ ! -f "$_d/panel" ] || [ ! -d "$_d/site" ] || [ ! -f "$_d/COMMIT" ]; then
		echo "al archivo le faltan panel, site/ o COMMIT" >&2
		rm -rf "$_trabajo"
		return 1
	fi
	if [ "$(head -n 1 "$_d/COMMIT")" != "$_commit" ]; then
		echo "el archivo es de otro commit ($(head -n 1 "$_d/COMMIT" | tr -cd '0-9a-f' | cut -c1-12)), no del de $_ver" >&2
		rm -rf "$_trabajo"
		return 1
	fi
	_tipo=$(file -b "$_d/panel")
	case $_tipo in
	ELF\ 64-bit*x86-64*FreeBSD*) ;;
	*)
		echo "panel no es un ejecutable de FreeBSD x86-64 ($(printf '%s' "$_tipo" | cut -c1-60))" >&2
		rm -rf "$_trabajo"
		return 1
		;;
	esac
	if ! mkdir "$_dst" || ! mv "$_d/panel" "$_d/site" "$_d/COMMIT" "$_dst/"; then
		echo "no se puede preparar $_dst" >&2
		rm -rf "$_trabajo" "$_dst"
		return 1
	fi
	rm -rf "$_trabajo"
}

# ¿Es $1 un repositorio de GitHub «propietario/nombre»? El propietario, de
# letras, cifras y «-» (39 como mucho); el nombre, también con «.» y «_»
# (100 como mucho); ninguno empieza por «-» ni el nombre es «.» o «..».
repo_github_valido()
{
	case $1 in
	*/*/* | -* | */-* | */ | /* | */. | */.. | *[!A-Za-z0-9._/-]*) return 1 ;;
	*/*) ;;
	*) return 1 ;;
	esac
	_propietario=${1%%/*}
	_nombre=${1#*/}
	case $_propietario in
	'' | *[!A-Za-z0-9-]*) return 1 ;;
	esac
	[ "${#_propietario}" -le 39 ] && [ "${#_nombre}" -le 100 ]
}

# Descarga $1 (https) en $2 con fetch: como mucho $3 bytes y 15 minutos
# (60 s sin datos). Sale con 1 si fetch falla o si trae más de $3 bytes.
descargar_https()
{
	_salida=$2
	{
		timeout 900 fetch -q -T 60 -o - "$1" 2>/dev/null
		echo $? >"$_salida.rc"
	} | head -c "$(($3 + 1))" >"$_salida"
	_rc=$(cat "$_salida.rc" 2>/dev/null)
	rm -f "$_salida.rc"
	[ "$_rc" = 0 ] && [ "$(wc -c <"$_salida" | tr -d ' ')" -le "$3" ]
}

# Trae de la release $2 (vX.Y.Z) del repositorio de GitHub $1 la versión para
# FreeBSD y la deja desempaquetada en $4 (que no debe existir), como
# preparar_precompilada; $3 es el commit completo de la etiqueta. Pasos, en
# una carpeta de trabajo propia:
# 1. Descarga el archivo y SHA256SUMS de
#    https://github.com/$1/releases/download/$2/ y comprueba el sha256 con
#    la línea del archivo (SHA256SUMS trae una por archivo publicado).
# 2. Trae, sin token, las atestaciones de ese sha256 de
#    https://api.github.com/repos/$1/attestations/sha256:<sha256> y saca con
#    jq cada «bundle» a una línea (JSON lines, lo que acepta gh --bundle).
# 3. gh attestation verify --bundle (sin sesión de gh ni conexión a su API):
#    la atestación tiene que ser de $1, de la etiqueta refs/tags/$2 y firmada
#    por .github/workflows/release.yml.
# 4. preparar_precompilada sobre una copia: sha256, solo la carpeta de la
#    versión con panel, site/ y COMMIT, COMMIT igual a $3 y panel ejecutable
#    de FreeBSD x86-64.
# Nada de lo descargado se ejecuta: jq y gh solo lo leen. Sale con 0 si
# queda en $4; 1 si aún no se puede usar (sin publicar, sin red, sin
# atestación, falta gh, jq o fetch); 2 si se rechaza (sumas, atestación o
# archivo que no cuadran). En los dos casos dice el motivo en la salida de
# error y no deja $4. GITHUB_MAX_ARCHIVO: bytes como mucho del archivo.
preparar_github()
{
	_grepo=$1
	_gver=$2
	_gcommit=$3
	_gdst=$4
	_gmax=${GITHUB_MAX_ARCHIVO:-268435456}
	case $_gmax in
	'' | *[!0-9]*) _gmax=268435456 ;;
	esac
	if ! repo_github_valido "$_grepo"; then
		echo "repositorio de GitHub no válido" >&2
		return 2
	fi
	if ! version_final "$_gver"; then
		echo "versión no válida (vX.Y.Z)" >&2
		return 2
	fi
	for _go in gh jq fetch; do
		if ! command -v "$_go" >/dev/null 2>&1; then
			echo "falta $_go (pkg install gh jq)" >&2
			return 1
		fi
	done
	_garch=$(archivo_precompilada "$_gver")
	_gurl=https://github.com/$_grepo/releases/download/$_gver
	_gtrabajo=$(mktemp -d) || return 1
	if ! descargar_https "$_gurl/$_garch" "$_gtrabajo/$_garch" "$_gmax" ||
		! descargar_https "$_gurl/SHA256SUMS" "$_gtrabajo/SHA256SUMS.github" 65536; then
		echo "no se puede descargar $_garch o SHA256SUMS de la release $_gver de github.com/$_grepo (¿aún no está publicada?)" >&2
		rm -rf "$_gtrabajo"
		return 1
	fi
	_gre=$(printf '%s' "$_garch" | sed 's/\./\\./g')
	_gesperada=$(sed -n "s/^\([0-9a-f]\{64\}\)  $_gre\$/\1/p" "$_gtrabajo/SHA256SUMS.github")
	if [ "${#_gesperada}" -ne 64 ]; then
		echo "SHA256SUMS de GitHub no tiene una sola línea «<sha256>  $_garch»" >&2
		rm -rf "$_gtrabajo"
		return 2
	fi
	_gsuma=$(suma_sha256 "$_gtrabajo/$_garch")
	if [ "$_gsuma" != "$_gesperada" ]; then
		echo "el sha256 de $_garch no coincide con SHA256SUMS de GitHub" >&2
		rm -rf "$_gtrabajo"
		return 2
	fi
	if ! descargar_https "https://api.github.com/repos/$_grepo/attestations/sha256:$_gsuma" \
		"$_gtrabajo/atestaciones.json" 16777216; then
		echo "no se pueden traer las atestaciones de $_garch de api.github.com (¿no tiene?)" >&2
		rm -rf "$_gtrabajo"
		return 1
	fi
	if ! jq -c '.attestations | arrays | .[] | objects | .bundle | objects' \
		<"$_gtrabajo/atestaciones.json" >"$_gtrabajo/atestaciones.jsonl" 2>/dev/null ||
		[ ! -s "$_gtrabajo/atestaciones.jsonl" ]; then
		echo "api.github.com no da ninguna atestación de $_garch" >&2
		rm -rf "$_gtrabajo"
		return 1
	fi
	# gh con una casa vacía: ni sesión, ni configuración, ni avisos.
	if ! mkdir "$_gtrabajo/casa" "$_gtrabajo/carpeta" "$_gtrabajo/carpeta/$_gver"; then
		rm -rf "$_gtrabajo"
		return 1
	fi
	if ! HOME=$_gtrabajo/casa GH_CONFIG_DIR=$_gtrabajo/casa/gh XDG_CACHE_HOME=$_gtrabajo/casa/cache \
		GH_TOKEN='' GITHUB_TOKEN='' GH_NO_UPDATE_NOTIFIER=1 GH_PROMPT_DISABLED=1 NO_COLOR=1 \
		timeout 300 gh attestation verify "$_gtrabajo/$_garch" --bundle "$_gtrabajo/atestaciones.jsonl" \
		--repo "$_grepo" --source-ref "refs/tags/$_gver" \
		--signer-workflow "$_grepo/.github/workflows/release.yml" >"$_gtrabajo/gh.txt" 2>&1; then
		echo "la atestación de $_garch no es de $_grepo, refs/tags/$_gver y release.yml ($(grep -v '^[[:space:]]*$' "$_gtrabajo/gh.txt" | tail -n 1 | tr -cd '[:alnum:][:space:].,:;()/_-' | cut -c1-160))" >&2
		rm -rf "$_gtrabajo"
		return 2
	fi
	# El archivo, con su línea de SHA256SUMS, a una carpeta solo suya: el resto
	# lo comprueba preparar_precompilada, igual que con el de Forgejo.
	if ! mv "$_gtrabajo/$_garch" "$_gtrabajo/carpeta/$_gver/$_garch" ||
		! printf '%s  %s\n' "$_gesperada" "$_garch" >"$_gtrabajo/carpeta/$_gver/SHA256SUMS" ||
		! preparar_precompilada "$_gtrabajo/carpeta" "$_gver" "$_gcommit" '' "$_gdst"; then
		rm -rf "$_gtrabajo"
		return 2
	fi
	rm -rf "$_gtrabajo"
}

# ¿Es $1 un repositorio de Forgejo «propietario/nombre»? Cada parte, de
# [A-Za-z0-9._-], sin empezar por «.» ni «-» y de 100 caracteres como mucho.
repo_forgejo_valido()
{
	case $1 in
	*/*/* | *[!A-Za-z0-9._/-]*) return 1 ;;
	?*/?*) ;;
	*) return 1 ;;
	esac
	for _parte in "${1%%/*}" "${1#*/}"; do
		case $_parte in
		[.-]*) return 1 ;;
		esac
		[ "${#_parte}" -le 100 ] || return 1
	done
	return 0
}

# ¿Es $1 la URL base de un Forgejo? http(s)://equipo[:puerto][/ruta], sin
# usuario, consulta, «..» ni «/» final: solo [A-Za-z0-9.:/_-].
url_forgejo_valida()
{
	case $1 in
	http://?* | https://?*) ;;
	*) return 1 ;;
	esac
	case $1 in
	*[!A-Za-z0-9.:/_-]* | */ | *..*) return 1 ;;
	esac
	case ${1#*://} in
	/* | :* | .*) return 1 ;;
	esac
	return 0
}

# Lee el token de Forgejo del fichero $1 en FORGEJO_TOKEN_LEIDO. Solo si $1
# es un fichero normal (no un enlace), del usuario con uid $2 (root en el
# servidor), sin permisos para el grupo ni para otros, y su primera línea es
# un token de [A-Za-z0-9_.-] (200 caracteres como mucho). Si no, dice por
# qué en la salida de error y sale con 1 sin dejar nada en
# FORGEJO_TOKEN_LEIDO.
leer_token_forgejo()
{
	FORGEJO_TOKEN_LEIDO=
	if [ ! -f "$1" ] || [ -L "$1" ]; then
		echo "no existe el fichero del token ($1)" >&2
		return 1
	fi
	# stat de GNU (pruebas) o de FreeBSD.
	_st=$(stat -c '%u %a' "$1" 2>/dev/null || stat -f '%u %Lp' "$1" 2>/dev/null) || _st=
	case $_st in
	[0-9]*' '[0-7]*) ;;
	*)
		echo "no se pueden leer el dueño y los permisos de $1" >&2
		return 1
		;;
	esac
	if [ "${_st% *}" != "$2" ]; then
		echo "$1 no es del dueño que toca (uid $2)" >&2
		return 1
	fi
	case ${_st#* } in
	*00) ;;
	*)
		echo "$1 lo pueden leer el grupo u otros (modo ${_st#* }; tiene que ser 0600)" >&2
		return 1
		;;
	esac
	_tok=$(head -n 1 "$1" | tr -d '\r')
	case $_tok in
	'' | *[!A-Za-z0-9_.-]*)
		echo "el token de $1 no es válido" >&2
		return 1
		;;
	esac
	if [ "${#_tok}" -gt 200 ]; then
		echo "el token de $1 no es válido" >&2
		return 1
	fi
	FORGEJO_TOKEN_LEIDO=$_tok
}

# curl a la URL $1 con el token de FORGEJO_TOKEN_LEIDO en la cabecera
# Authorization, que curl lee de la entrada estándar (-K -): nunca va en la
# línea de órdenes (la vería cualquiera con ps) ni en la URL. Sin seguir
# redirecciones: el token solo va al Forgejo configurado. Guarda la
# respuesta en $2 (como mucho $3 bytes) y escribe el código HTTP (000 si no
# hay respuesta). El resto de argumentos van a curl.
curl_forgejo()
{
	_cu=$1
	_cs=$2
	_cm=$3
	shift 3
	printf 'header = "Authorization: token %s"\n' "$FORGEJO_TOKEN_LEIDO" |
		timeout 960 curl -K - -sS --fail-with-body --connect-timeout 15 --max-time 900 \
			--proto =https,http --max-filesize "$_cm" -o "$_cs" -w '%{http_code}' "$@" "$_cu" 2>/dev/null ||
		true
}

# Descarga $1 del Forgejo en $2 (como mucho $3 bytes). Sale con 0 si llega
# entera (HTTP 200 y no más de $3 bytes); si no, deja el código HTTP en
# _fcodigo y sale con 1.
descargar_forgejo()
{
	_fcodigo=$(curl_forgejo "$1" "$2" "$3")
	[ "$_fcodigo" = 200 ] && [ -f "$2" ] && [ "$(wc -c <"$2" | tr -d ' ')" -le "$3" ]
}

# Trae de la release $5 (vX.Y.Z) del repositorio $2 («propietario/nombre»)
# del Forgejo $1 la versión para FreeBSD y la deja desempaquetada en $8 (que
# no debe existir), como preparar_precompilada. $3 es el fichero del token
# de solo lectura (leer_token_forgejo, del uid $4), $6 el commit completo de
# la etiqueta y $7 la clave pública de minisign (vacía: sin firma). Pasos, en
# una carpeta de trabajo propia:
# 1. GET $1/api/v1/repos/$2/releases/tags/$5: 404 es que aún no está.
# 2. Descarga SHA256SUMS, el archivo y, con clave, SHA256SUMS.minisig, con
#    los nombres que se esperan, de $1/$2/releases/download/$5/<nombre>: la
#    URL sale de la configuración, nunca de la respuesta, así que el token
#    solo va a $1.
# 3. preparar_precompilada: sha256, firma, solo la carpeta de la versión con
#    panel, site/ y COMMIT, COMMIT igual a $6 y panel ejecutable de FreeBSD
#    x86-64.
# Nada de lo descargado se ejecuta. Sale con 0 si queda en $8; 1 si aún no
# se puede usar (sin publicar, a medias, sin red, falta curl); 2 si se
# rechaza (configuración, sumas o archivo que no cuadran); 3 si el token no
# vale (entonces no llama a curl). En todos los casos dice el motivo en la
# salida de error y no deja $8. FORGEJO_MAX_ARCHIVO: bytes como mucho del
# archivo.
preparar_forgejo()
{
	_furl=$1
	_frepo=$2
	_fver=$5
	_fcommit=$6
	_fclave=$7
	_fdst=$8
	_fmax=${FORGEJO_MAX_ARCHIVO:-268435456}
	case $_fmax in
	'' | *[!0-9]*) _fmax=268435456 ;;
	esac
	if ! url_forgejo_valida "$_furl" || ! repo_forgejo_valido "$_frepo"; then
		echo "FORGEJO_URL o FORGEJO_REPO no son válidos" >&2
		return 2
	fi
	if ! version_final "$_fver"; then
		echo "versión no válida (vX.Y.Z)" >&2
		return 2
	fi
	if ! command -v curl >/dev/null 2>&1; then
		echo "falta curl (pkg install curl)" >&2
		return 1
	fi
	leer_token_forgejo "$3" "$4" || return 3
	_farch=$(archivo_precompilada "$_fver")
	_ftrabajo=$(mktemp -d) || return 1
	mkdir "$_ftrabajo/carpeta" "$_ftrabajo/carpeta/$_fver" || {
		rm -rf "$_ftrabajo"
		return 1
	}
	_fcodigo=$(curl_forgejo "$_furl/api/v1/repos/$_frepo/releases/tags/$_fver" "$_ftrabajo/release.json" 1048576)
	case $_fcodigo in
	200) ;;
	404)
		echo "la release $_fver aún no está publicada en Forgejo" >&2
		rm -rf "$_ftrabajo"
		return 1
		;;
	*)
		echo "Forgejo responde HTTP $_fcodigo a la release $_fver" >&2
		rm -rf "$_ftrabajo"
		return 1
		;;
	esac
	_fbase=$_furl/$_frepo/releases/download/$_fver
	_fdir=$_ftrabajo/carpeta/$_fver
	if ! descargar_forgejo "$_fbase/SHA256SUMS" "$_fdir/SHA256SUMS" 65536; then
		echo "la release $_fver de Forgejo aún no tiene SHA256SUMS (HTTP $_fcodigo)" >&2
		rm -rf "$_ftrabajo"
		return 1
	fi
	if ! descargar_forgejo "$_fbase/$_farch" "$_fdir/$_farch" "$_fmax"; then
		echo "no se puede descargar $_farch de la release $_fver de Forgejo (HTTP $_fcodigo)" >&2
		rm -rf "$_ftrabajo"
		return 1
	fi
	if [ -n "$_fclave" ] && ! descargar_forgejo "$_fbase/SHA256SUMS.minisig" "$_fdir/SHA256SUMS.minisig" 4096; then
		echo "la release $_fver de Forgejo no tiene SHA256SUMS.minisig (HTTP $_fcodigo)" >&2
		rm -rf "$_ftrabajo"
		return 1
	fi
	if ! preparar_precompilada "$_ftrabajo/carpeta" "$_fver" "$_fcommit" "$_fclave" "$_fdst"; then
		rm -rf "$_ftrabajo"
		return 2
	fi
	rm -rf "$_ftrabajo"
}
