#!/usr/bin/env bash
# Empaqueta una versión del panel para FreeBSD en el formato que acepta el
# actualizador del servidor (preparar_precompilada, deploy/lib/precompiladas.sh).
# Lo usan el PC de compilación (compilar-freebsd.sh) y la release de GitHub
# (.github/workflows/release.yml), así que los dos dejan el mismo archivo.
#
#   empaquetar-freebsd.sh <versión> <commit> <panel> <site> <fuente> <salida>
#
# <versión>  la etiqueta (vX.Y.Z…; version_precompilable) o, para probar la
#            compilación, prueba-<sha corto> (el actualizador no la admite)
# <commit>   el commit completo de la etiqueta (40 cifras hex): va en COMMIT
# <panel>    el binario para x86_64-unknown-freebsd
# <site>     su web (target/site de cargo-leptos, con hash.txt y
#            pkg/panel.<hash>.{js,wasm,css})
# <fuente>   el código de la etiqueta, de donde salen LICENSE y AVISO-LEGAL.md
# <salida>   carpeta donde deja routerpanel-<versión>-x86_64-unknown-freebsd.tar.gz
#            (SHA256SUMS lo escribe quien llama: el PC, de un archivo; la
#            release, de todos)
#
# El archivo tiene una única carpeta del mismo nombre (sin .tar.gz) con panel,
# site/, COMMIT, LICENSE y AVISO-LEGAL.md. Dueños, modos y fechas fijos
# (SOURCE_DATE_EPOCH, o la hora actual): con los mismos ficheros, el mismo
# archivo.

set -euo pipefail

AQUI=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=../lib/precompiladas.sh
. "$AQUI/../lib/precompiladas.sh"

error()
{
	echo "ERROR: $*" >&2
	exit 1
}

[ "$#" -eq 6 ] || {
	sed -n '/^#   empaquetar/s/^#   //p' "$0" >&2
	exit 2
}
version=$1
commit=$2
panel=$3
site=$4
fuente=$5
salida=$6

case $version in
prueba-*[!0-9a-f]* | prueba-) error "«$version» no es una versión válida (vX.Y.Z… o prueba-<sha>)" ;;
prueba-*) ;;
*) version_precompilable "$version" || error "«$version» no es una versión válida (vX.Y.Z…)" ;;
esac
case $commit in
'' | *[!0-9a-f]*) error "el commit no es válido" ;;
esac
[ "${#commit}" -eq 40 ] || error "el commit no es válido"
[ -f "$panel" ] || error "falta $panel"
case $(file -b "$panel") in
ELF\ 64-bit*x86-64*FreeBSD*) ;;
*) error "el binario no es de FreeBSD x86-64: $(file -b "$panel")" ;;
esac
[ -f "$site/hash.txt" ] || error "falta site/hash.txt"
for ext in js wasm css; do
	h=$(sed -n "s/^$ext: *//p" "$site/hash.txt")
	[ -n "$h" ] && [ -f "$site/pkg/panel.$h.$ext" ] || error "falta site/pkg/panel.$h.$ext"
done

nombre=$(archivo_precompilada "$version")
nombre=${nombre%.tar.gz}
trabajo=$(mktemp -d)
trap 'rm -rf "$trabajo"' EXIT
d=$trabajo/$nombre
mkdir -p "$d"
cp "$panel" "$d/panel"
cp -R "$site" "$d/site"
echo "$commit" >"$d/COMMIT"
for f in LICENSE AVISO-LEGAL.md; do
	if [ -f "$fuente/$f" ]; then cp "$fuente/$f" "$d/$f"; fi
done
chmod -R u=rwX,go=rX "$d"
chmod 755 "$d/panel"
if [ -n "$(find "$d" ! -type f ! -type d)" ]; then
	error "site/ trae algo que no es fichero ni directorio"
fi
mkdir -p "$salida"
tar --sort=name --format=gnu --owner=0 --group=0 --numeric-owner \
	--mtime="@${SOURCE_DATE_EPOCH:-$(date +%s)}" -C "$trabajo" -cf - "$nombre" |
	gzip -n -9 >"$salida/$nombre.tar.gz"
echo "$salida/$nombre.tar.gz"
