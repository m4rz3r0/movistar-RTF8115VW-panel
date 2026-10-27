# shellcheck shell=sh
#
# Versiones precompiladas para FreeBSD (deploy/cross/): las compila el PC de
# compilación y las deja en el buzón del servidor; el actualizador
# (deploy/freebsd-jail/actualizar.sh) las instala en vez de compilar. Se carga
# con «.»; solo funciones, sin efectos al cargarse. Las usan el actualizador
# (en el host, como root), el receptor del buzón
# (deploy/freebsd-jail/recibir-precompilada.sh, como el usuario del buzón) y
# las pruebas (deploy/pruebas/precompiladas.sh).
#
# El archivo es el de las releases de Linux (tools/empaquetar-release.sh) con
# el objetivo de FreeBSD: routerpanel-<versión>-x86_64-unknown-freebsd.tar.gz,
# con una única carpeta del mismo nombre (sin .tar.gz) que trae panel, site/ y
# COMMIT (el commit completo de la etiqueta), y SHA256SUMS en el formato de
# sha256sum. Si hay clave de minisign, también SHA256SUMS.minisig.

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

# La versión más reciente de las etiquetas que llegan por la entrada
# estándar, una por línea: la última v* según sort -V, la misma que elige
# traer_ultima_etiqueta (deploy/lib/git.sh) en el servidor. Así el PC compila
# justo la que el actualizador va a buscar.
ultima_version()
{
	{ grep '^v' || true; } | sort -V | tail -n 1
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

# Qué hacer con la etiqueta $2 según el estado $1 (fichero «etiqueta
# segundos» de cuándo se vio por primera vez sin precompilada), la hora $3
# (segundos), la espera $4 (minutos) y si ya ha llegado al buzón ($5: «si» o
# «no»). Escribe instalar, esperar o compilar. La primera vez que ve una
# etiqueta sin precompilada apunta la hora en $1.
decidir_precompilada()
{
	_estado=$1
	_etiqueta=$2
	_ahora=$3
	_espera=$4
	if [ "$5" = si ]; then
		echo instalar
		return 0
	fi
	case $_espera in
	'' | *[!0-9]*) _espera=0 ;;
	esac
	_vista=
	if [ -f "$_estado" ] && [ "$(cut -d' ' -f1 "$_estado")" = "$_etiqueta" ]; then
		_vista=$(cut -d' ' -f2 "$_estado" | tr -cd '0-9')
	fi
	if [ -z "$_vista" ]; then
		echo "$_etiqueta $_ahora" >"$_estado"
		_vista=$_ahora
	fi
	if [ "$((_ahora - _vista))" -lt "$((_espera * 60))" ]; then
		echo esperar
	else
		echo compilar
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
# completo de la etiqueta en el clon del servidor y $4 la clave pública de
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
