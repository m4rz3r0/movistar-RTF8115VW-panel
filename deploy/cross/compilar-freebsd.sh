#!/usr/bin/env bash
# Compila el panel para FreeBSD (x86_64-unknown-freebsd) en el PC de
# compilación, un Linux con rustup, podman y cross, y lo empaqueta para el
# buzón del servidor (deploy/freebsd-jail/recibir-precompilada.sh). Así el
# servidor no tiene que compilar: su actualizador instala lo que llega
# (deploy/freebsd-jail/README.md, «Versiones precompiladas»).
#
#   compilar-freebsd.sh --release vX.Y.Z [--bundle FICHERO] [--repo URL] [--entregar DESTINO]
#   compilar-freebsd.sh --ultima --repo URL [--entregar DESTINO]
#   compilar-freebsd.sh --prueba [REF] [--bundle FICHERO]
#
# --release   compila la etiqueta vX.Y.Z (tiene que ser una etiqueta v*, no
#             una rama ni un commit) y deja en salida/<versión>/ el archivo
#             routerpanel-<versión>-x86_64-unknown-freebsd.tar.gz y su
#             SHA256SUMS (más SHA256SUMS.minisig con RP_MINISIGN_SECRETA).
# --ultima    trae las etiquetas de --repo y compila la v* más reciente
#             (sort -V, como el actualizador) si aún no se ha entregado;
#             para el temporizador opcional (rtf-cross.timer).
# --prueba    compila REF (por defecto HEAD del bundle o del clon) para probar
#             la compilación: la versión se llama prueba-<sha corto> y el
#             buzón no la admite.
# --bundle    un `git bundle` que trae la referencia (lo manda lanzar.sh desde
#             el equipo de trabajo); sin él, el clon tiene que tenerla ya.
# --repo      URL de git de la que traer las etiquetas (como REPO_URL).
# --entregar  al terminar una versión, la sube al buzón con entregar.sh
#             (DESTINO: el usuario del buzón, p. ej. rpentrega@servidor).
#
# Todo vive en RP_CROSS_DIR (por defecto ~/rtf-cross): repo/ (clon desnudo),
# fuente/ (el código de la referencia, con target/ dentro para que la
# siguiente compilación sea incremental), salida/<versión>/ y
# compilacion.log. El código sale de git (git archive del commit), nunca de
# una copia de trabajo: un cambio sin confirmar no puede acabar en una
# versión.
#
# Cómo se compila: cargo leptos build --release --precompress, igual que el
# servidor, pero con el binario para FreeBSD (LEPTOS_BIN_TARGET_TRIPLE) y
# compilado con cross (LEPTOS_BIN_CARGO_COMMAND=cross: un contenedor con el
# enlazador y la libc de FreeBSD; las dependencias de C, como SQLite, se
# compilan dentro y van enlazadas en el binario). La web (wasm, JS y CSS) no
# depende de la plataforma y la hace cargo-leptos como en el servidor, con
# la misma versión de cargo-leptos (la de install.sh), que es la que fija
# wasm-bindgen.

set -euo pipefail

BASE=${RP_CROSS_DIR:-$HOME/rtf-cross}
AQUI=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=../lib/precompiladas.sh
. "$AQUI/../lib/precompiladas.sh"
OBJ=$PRECOMPILADA_OBJETIVO
CARGO_LEPTOS_VERSION=0.3.11
TIEMPO_MAXIMO=3600

error()
{
	echo "ERROR: $*" >&2
	exit 1
}

uso()
{
	sed -n '/^#   compilar/s/^#   //p' "$0" >&2
	exit 2
}

modo=
ref=
bundle=
repo_url=
entregar=
while [ "$#" -gt 0 ]; do
	case $1 in
	--release)
		modo=release
		ref=${2:-}
		[ -n "$ref" ] || uso
		shift 2
		;;
	--ultima)
		modo=ultima
		shift
		;;
	--prueba)
		modo=prueba
		shift
		if [ "$#" -gt 0 ] && [ "${1#--}" = "$1" ]; then
			ref=$1
			shift
		fi
		;;
	--bundle)
		bundle=${2:-}
		[ -n "$bundle" ] || uso
		shift 2
		;;
	--repo)
		repo_url=${2:-}
		[ -n "$repo_url" ] || uso
		shift 2
		;;
	--entregar)
		entregar=${2:-}
		[ -n "$entregar" ] || uso
		shift 2
		;;
	*) uso ;;
	esac
done
[ -n "$modo" ] || uso
if [ -n "$bundle" ]; then
	bundle=$(realpath -e "$bundle") || error "no existe el bundle"
fi
if [ "$modo" = ultima ] && [ -z "$repo_url" ]; then
	error "--ultima necesita --repo"
fi
if [ "$modo" = prueba ] && [ -n "$entregar" ]; then
	error "las compilaciones de --prueba no se entregan"
fi

for orden in git cross cargo podman file sha256sum tar gzip rsync flock; do
	command -v "$orden" >/dev/null 2>&1 || error "falta $orden"
done
mkdir -p "$BASE/salida"
exec 9>"$BASE/.lock"
flock -n 9 || error "ya hay otra compilación en marcha en $BASE"

REPO=$BASE/repo
if [ ! -d "$REPO" ]; then
	git init --quiet --bare "$REPO"
fi

# --- referencia --------------------------------------------------------------
if [ -n "$bundle" ]; then
	git -C "$REPO" bundle verify --quiet "$bundle" >/dev/null 2>&1 ||
		error "el bundle no es válido"
	# Etiquetas tal cual (--force: una etiqueta movida en el origen gana) y
	# el resto de referencias bajo refs/rtf/.
	git -C "$REPO" fetch --quiet --force "$bundle" \
		'+refs/tags/*:refs/tags/*' '+refs/heads/*:refs/rtf/heads/*' '+HEAD:refs/rtf/HEAD' 2>/dev/null ||
		git -C "$REPO" fetch --quiet --force "$bundle" '+refs/tags/*:refs/tags/*' ||
		error "no se pueden traer las referencias del bundle"
fi
if [ -n "$repo_url" ]; then
	git -C "$REPO" fetch --quiet --force --prune --prune-tags "$repo_url" '+refs/tags/*:refs/tags/*' ||
		error "no se pueden traer las etiquetas del repositorio"
fi
if [ "$modo" = ultima ]; then
	ref=$(git -C "$REPO" tag -l 'v*' | ultima_version)
	[ -n "$ref" ] || {
		echo "no hay ninguna etiqueta v*"
		exit 0
	}
	if [ -f "$BASE/salida/$ref/.entregada" ]; then
		echo "$ref ya está compilada y entregada"
		exit 0
	fi
	modo=release
fi

if [ "$modo" = release ]; then
	version_precompilable "$ref" || error "«$ref» no es una versión válida (vX.Y.Z…)"
	# Solo etiquetas: una rama con el nombre de la versión no vale.
	commit=$(git -C "$REPO" rev-parse --verify --quiet "refs/tags/$ref^{commit}") ||
		error "$ref no es una etiqueta del repositorio (--release solo compila etiquetas v*)"
	version=$ref
else
	if [ -z "$ref" ]; then
		if git -C "$REPO" rev-parse --verify --quiet 'refs/rtf/HEAD^{commit}' >/dev/null; then
			ref=refs/rtf/HEAD
		else
			ref=HEAD
		fi
	fi
	commit=$(git -C "$REPO" rev-parse --verify --quiet "$ref^{commit}" ||
		git -C "$REPO" rev-parse --verify --quiet "refs/rtf/heads/$ref^{commit}") ||
		error "$ref no existe en el repositorio"
	version=prueba-$(printf '%s' "$commit" | cut -c1-12)
fi
echo "compilando $version ($commit) para $OBJ"

# --- código ------------------------------------------------------------------
# git archive a una carpeta nueva y rsync --checksum a fuente/: lo que no
# cambia conserva su fecha y cargo no lo recompila.
NUEVA=$BASE/.fuente-nueva
rm -rf "$NUEVA"
mkdir -p "$NUEVA" "$BASE/fuente"
git -C "$REPO" archive --format=tar "$commit" | tar -xf - -C "$NUEVA"
rsync -rlpD --checksum --delete --exclude /target "$NUEVA/" "$BASE/fuente/"
rm -rf "$NUEVA"
FUENTE=$BASE/fuente
cd "$FUENTE"

# --- herramientas --------------------------------------------------------------
quiere=$(sed -n 's/^CARGO_LEPTOS_VERSION=//p' deploy/freebsd-jail/install.sh 2>/dev/null | head -n 1)
quiere=${quiere:-$CARGO_LEPTOS_VERSION}
tiene=$(cargo leptos --version 2>/dev/null | cut -d' ' -f2 || true)
if [ "$quiere" != "$tiene" ]; then
	echo "instalando cargo-leptos $quiere (había: ${tiene:-ninguno})"
	cargo install --locked cargo-leptos --version "$quiere"
fi
# La Rust de rust-toolchain.toml con el objetivo de FreeBSD (rustup la
# instala si falta; cross usa la misma, montada en el contenedor).
rustup target add x86_64-unknown-freebsd >/dev/null

# --- compilación ---------------------------------------------------------------
inicio=$(date +%s)
if ! LEPTOS_BIN_TARGET_TRIPLE=$OBJ LEPTOS_BIN_CARGO_COMMAND=cross \
	timeout -k 60 "$TIEMPO_MAXIMO" nice -n 10 \
	cargo leptos build --release --precompress >"$BASE/compilacion.log" 2>&1; then
	tail -n 20 "$BASE/compilacion.log" >&2
	error "falló la compilación (ver $BASE/compilacion.log)"
fi
echo "compilado en $(($(date +%s) - inicio)) s"

PANEL=$FUENTE/target/$OBJ/release/panel
SITE=$FUENTE/target/site
[ -f "$PANEL" ] || error "falta $PANEL"
case $(file -b "$PANEL") in
ELF\ 64-bit*x86-64*FreeBSD*) ;;
*) error "el binario no es de FreeBSD x86-64: $(file -b "$PANEL")" ;;
esac
# Con el objetivo de FreeBSD, cargo-leptos deja hash.txt junto a la carpeta
# del binario (hash-file-name es relativo a target/<objetivo>/release); el
# panel lo busca dentro de site/.
if [ -f "$FUENTE/target/$OBJ/site/hash.txt" ]; then
	cp "$FUENTE/target/$OBJ/site/hash.txt" "$SITE/hash.txt"
fi
[ -f "$SITE/hash.txt" ] || error "falta site/hash.txt"
for ext in js wasm css; do
	h=$(sed -n "s/^$ext: *//p" "$SITE/hash.txt")
	[ -f "$SITE/pkg/panel.$h.$ext" ] || error "falta site/pkg/panel.$h.$ext"
done

# --- archivo -------------------------------------------------------------------
NOMBRE=$(archivo_precompilada "$version")
NOMBRE=${NOMBRE%.tar.gz}
SALIDA=$BASE/salida/$version
TRABAJO=$(mktemp -d)
trap 'rm -rf "$TRABAJO"' EXIT
D=$TRABAJO/$NOMBRE
mkdir -p "$D"
cp "$PANEL" "$D/panel"
# Solo lo que corresponde a esta compilación: sin restos de otras.
cp -R "$SITE" "$D/site"
echo "$commit" >"$D/COMMIT"
for f in LICENSE AVISO-LEGAL.md; do
	if [ -f "$FUENTE/$f" ]; then cp "$FUENTE/$f" "$D/$f"; fi
done
chmod -R u=rwX,go=rX "$D"
chmod 755 "$D/panel"
if [ -n "$(find "$D" ! -type f ! -type d)" ]; then
	error "site/ trae algo que no es fichero ni directorio"
fi
SOURCE_DATE_EPOCH=$(git -C "$REPO" log -1 --format=%ct "$commit")
rm -rf "$SALIDA"
mkdir -p "$SALIDA"
tar --sort=name --format=gnu --owner=0 --group=0 --numeric-owner \
	--mtime="@$SOURCE_DATE_EPOCH" -C "$TRABAJO" -cf - "$NOMBRE" |
	gzip -n -9 >"$SALIDA/$NOMBRE.tar.gz"
(cd "$SALIDA" && LC_ALL=C sha256sum -- "$NOMBRE.tar.gz" >SHA256SUMS)
if [ -n "${RP_MINISIGN_SECRETA:-}" ]; then
	command -v minisign >/dev/null 2>&1 || error "RP_MINISIGN_SECRETA está puesta pero no hay minisign"
	minisign -S -s "$RP_MINISIGN_SECRETA" -m "$SALIDA/SHA256SUMS" -x "$SALIDA/SHA256SUMS.minisig" ||
		error "no se puede firmar SHA256SUMS"
fi
echo "$SALIDA/$NOMBRE.tar.gz"
if [ -n "$entregar" ]; then
	bash "$AQUI/entregar.sh" --destino "$entregar" "$SALIDA"
fi
