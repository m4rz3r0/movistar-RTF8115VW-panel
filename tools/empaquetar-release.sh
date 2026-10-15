#!/bin/sh
# Empaqueta una release para Linux tal y como la espera
# deploy/linux/actualizar-release.sh. Lo usan .github/workflows/release.yml y
# deploy/pruebas/release-local.sh, así que ambos construyen el mismo archivo.
#
#   tools/empaquetar-release.sh <versión> <objetivo> <panel> <site> <salida>
#   tools/empaquetar-release.sh --sumas <salida>
#
# <versión>  vX.Y.Z (solo cifras, sin sufijo: las únicas que se publican)
# <objetivo> x86_64-unknown-linux-gnu o aarch64-unknown-linux-gnu
# <panel>    el binario (target/release/panel de `cargo leptos build --release`)
# <site>     su web, la salida de cargo-leptos (target/site, con pkg/panel.js,
#            pkg/panel.wasm y pkg/panel.css; con hash-files, pkg/panel.<hash>.js…
#            y los hashes en hash.txt)
# <salida>   carpeta donde deja routerpanel-<versión>-<objetivo>.tar.gz
#
# El archivo tiene una única carpeta, routerpanel-<versión>-<objetivo>/, con
# panel, site/, LICENSE, AVISO-LEGAL.md y deploy/linux/ (la unidad de systemd
# apunta a current/deploy/linux/README.md). LICENSE, AVISO-LEGAL.md y
# deploy/linux/ salen del commit HEAD del repositorio donde está este script
# (git archive), nunca de la copia de trabajo: un fichero sin seguimiento, uno
# ignorado o un cambio sin confirmar no acaban en la release. Solo site/ (y
# el binario) vienen de fuera de git, de la salida de cargo-leptos.
# Dueños, modos y fechas fijos (SOURCE_DATE_EPOCH, o la fecha del último
# commit): con los mismos ficheros, el mismo archivo.
#
# Con --sumas escribe <salida>/SHA256SUMS con los routerpanel-*.tar.gz que
# haya: uno por release, con los archivos de todos los objetivos, en el
# formato de sha256sum («<hash>  <fichero>»), que es el que lee el
# actualizador.

set -eu

USO="uso: $0 <versión> <objetivo> <panel> <site> <salida>
       $0 --sumas <salida>"
if [ "$#" -eq 2 ] && [ "$1" = --sumas ]; then
	cd "$2"
	set -- routerpanel-*.tar.gz
	[ -f "$1" ] || {
		echo "no hay routerpanel-*.tar.gz en $2" >&2
		exit 1
	}
	LC_ALL=C sha256sum -- "$@" >SHA256SUMS
	exit 0
fi
if [ "$#" -ne 5 ]; then
	echo "$USO" >&2
	exit 2
fi
VER=$1
OBJ=$2
PANEL=$3
SITE=$4
SALIDA=$5
RAIZ=$(cd "$(dirname "$0")/.." && pwd)

if ! printf '%s\n' "$VER" | grep -Eqx 'v[0-9]+\.[0-9]+\.[0-9]+'; then
	echo "versión no válida (vX.Y.Z): $VER" >&2
	exit 2
fi
case $OBJ in
x86_64-unknown-linux-gnu | aarch64-unknown-linux-gnu) ;;
*)
	echo "objetivo no válido: $OBJ" >&2
	exit 2
	;;
esac

[ -f "$PANEL" ] || {
	echo "falta $PANEL" >&2
	exit 1
}
for ext in js wasm css; do
	f=pkg/panel.$ext
	if [ -f "$SITE/hash.txt" ]; then
		# «js: <hash>» y así: el nombre que pide la página.
		h=$(sed -n "s/^$ext: *//p" "$SITE/hash.txt")
		f=pkg/panel.$h.$ext
	fi
	[ -f "$SITE/$f" ] || {
		echo "falta $SITE/$f: <site> tiene que ser la salida de cargo leptos build --release" >&2
		exit 1
	}
done
DE_GIT="LICENSE AVISO-LEGAL.md deploy/linux/README.md deploy/linux/routerpanel.service deploy/linux/actualizar-release.sh"
for f in $DE_GIT; do
	git -C "$RAIZ" cat-file -e "HEAD:$f" 2>/dev/null || {
		echo "falta $f en el commit HEAD del repositorio" >&2
		exit 1
	}
done

if [ -z "${SOURCE_DATE_EPOCH:-}" ]; then
	SOURCE_DATE_EPOCH=$(git -C "$RAIZ" log -1 --format=%ct 2>/dev/null || date +%s)
fi

NOMBRE=routerpanel-$VER-$OBJ
TRABAJO=$(mktemp -d)
trap 'rm -rf "$TRABAJO"' EXIT
trap 'exit 143' INT TERM

D=$TRABAJO/$NOMBRE
mkdir -p "$D"
git -C "$RAIZ" archive --format=tar -o "$TRABAJO/git.tar" HEAD LICENSE AVISO-LEGAL.md deploy/linux
tar -xf "$TRABAJO/git.tar" -C "$D"
rm -f "$TRABAJO/git.tar"
cp "$PANEL" "$D/panel"
cp -R "$SITE" "$D/site"
chmod -R u=rwX,go=rX "$D"
chmod 755 "$D/panel" "$D/deploy/linux/actualizar-release.sh"
# Solo ficheros y carpetas: el actualizador rechaza cualquier otra cosa.
if [ -n "$(find "$D" ! -type f ! -type d)" ]; then
	echo "site/ o deploy/linux/ traen algo que no es fichero ni directorio" >&2
	exit 1
fi

mkdir -p "$SALIDA"
SALIDA=$(cd "$SALIDA" && pwd)
tar --sort=name --format=gnu --owner=0 --group=0 --numeric-owner \
	--mtime="@$SOURCE_DATE_EPOCH" -C "$TRABAJO" -cf - "$NOMBRE" |
	gzip -n -9 >"$SALIDA/$NOMBRE.tar.gz"
echo "$SALIDA/$NOMBRE.tar.gz"
