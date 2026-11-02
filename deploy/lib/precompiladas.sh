# shellcheck shell=sh
#
# Versiones precompiladas para FreeBSD: el servidor nunca compila. El
# actualizador (deploy/freebsd-jail/actualizar.sh) instala la versión de una
# etiqueta de una de estas dos fuentes:
#
# - el buzón del servidor, donde la deja el PC de compilación (deploy/cross/);
# - la release vX.Y.Z del repositorio en GitHub (GITHUB_REPO), que publica
#   .github/workflows/release.yml con su atestación de origen
#   (preparar_github).
#
# Se carga con «.»; solo funciones, sin efectos al cargarse. Las usan el
# actualizador (en el host, como root), el receptor del buzón
# (deploy/freebsd-jail/recibir-precompilada.sh, como el usuario del buzón), el
# PC de compilación y las pruebas (deploy/pruebas/precompiladas.sh).
#
# El archivo es el de las releases de Linux (tools/empaquetar-release.sh) con
# el objetivo de FreeBSD: routerpanel-<versión>-x86_64-unknown-freebsd.tar.gz,
# con una única carpeta del mismo nombre (sin .tar.gz) que trae panel, site/ y
# COMMIT (el commit completo de la etiqueta); lo hace
# deploy/cross/empaquetar-freebsd.sh. SHA256SUMS va en el formato de
# sha256sum. En el buzón, si hay clave de minisign, también
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

# ¿Es $2 un fichero que se acepta en el buzón para la versión $1?
fichero_de_buzon()
{
	case "$2" in
	"$(archivo_precompilada "$1")" | SHA256SUMS | SHA256SUMS.minisig) return 0 ;;
	esac
	return 1
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

# ¿Ha llegado ya al buzón $1 la versión $2 completa? Lo está cuando están el
# archivo, SHA256SUMS (el PC lo sube el último) y, si $3 (la clave de
# minisign) no está vacía, la firma. Solo cuentan los ficheros normales, nunca
# los enlaces.
precompilada_llegada()
{
	_dir=$1/$2
	for _f in "$(archivo_precompilada "$2")" SHA256SUMS; do
		[ -f "$_dir/$_f" ] && [ ! -L "$_dir/$_f" ] || return 1
	done
	if [ -n "$3" ]; then
		[ -f "$_dir/SHA256SUMS.minisig" ] && [ ! -L "$_dir/SHA256SUMS.minisig" ] || return 1
	fi
	return 0
}

# Comprueba la versión $2 del buzón $1 y la deja desempaquetada en $5 (que no
# debe existir), con panel, site/ y COMMIT y nada más. $3 es el commit
# completo de la etiqueta (git ls-remote de REPO_URL) y $4 la clave pública de
# minisign (vacía: sin firma). Escribe el motivo en la salida de error y sale
# con 1 si algo no cuadra; $5 no queda creada en ese caso.
#
# Quien escribe en el buzón es otro usuario: antes de leer nada se copia a
# una carpeta propia (con un tiempo máximo, por si hubiera una FIFO en lugar
# de un fichero) y todo se comprueba sobre la copia.
preparar_precompilada()
{
	_buzon=$1
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
		if [ ! -f "$_buzon/$_ver/$_f" ] || [ -L "$_buzon/$_ver/$_f" ] ||
			! timeout 300 cp "$_buzon/$_ver/$_f" "$_trabajo/$_f" ||
			[ ! -f "$_trabajo/$_f" ] || [ -L "$_trabajo/$_f" ]; then
			echo "no se puede copiar $_f del buzón" >&2
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
	if ! mkdir "$_gtrabajo/casa" "$_gtrabajo/buzon" "$_gtrabajo/buzon/$_gver"; then
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
	# El archivo, con su línea de SHA256SUMS, a un «buzón» solo suyo: el resto
	# lo comprueba preparar_precompilada, igual que con el del PC.
	if ! mv "$_gtrabajo/$_garch" "$_gtrabajo/buzon/$_gver/$_garch" ||
		! printf '%s  %s\n' "$_gesperada" "$_garch" >"$_gtrabajo/buzon/$_gver/SHA256SUMS" ||
		! preparar_precompilada "$_gtrabajo/buzon" "$_gver" "$_gcommit" '' "$_gdst"; then
		rm -rf "$_gtrabajo"
		return 2
	fi
	rm -rf "$_gtrabajo"
}
