#!/bin/sh
# traer_ultima_etiqueta (deploy/lib/git.sh) con un remoto cuyo historial se
# reescribe: el clon tiene que acabar en la etiqueta nueva y sin la vieja,
# sin merge ni conflicto.
#
#   1. remoto con v0.1.0; se clona y se trae la última etiqueta;
#   2. el remoto se reescribe (rama huérfana con v0.2.0; v0.1.0 desaparece);
#   3. se vuelve a traer: el clon está en v0.2.0 y ya no tiene v0.1.0.
#
# Uso: deploy/pruebas/historial-reescrito.sh (solo necesita git y sh).

set -eu

DEPLOY=$(cd "$(dirname "$0")/.." && pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# Un git sin la configuración de quien lo ejecuta.
HOME=$TMP
GIT_CONFIG_NOSYSTEM=1
GIT_AUTHOR_NAME=prueba
GIT_AUTHOR_EMAIL=prueba
GIT_COMMITTER_NAME=prueba
GIT_COMMITTER_EMAIL=prueba
export HOME GIT_CONFIG_NOSYSTEM GIT_AUTHOR_NAME GIT_AUTHOR_EMAIL GIT_COMMITTER_NAME GIT_COMMITTER_EMAIL

. "$DEPLOY/lib/git.sh"

fallo()
{
	echo "FALLO: $*" >&2
	exit 1
}

git init -q -b main "$TMP/remoto"
echo uno >"$TMP/remoto/fichero"
git -C "$TMP/remoto" add fichero
git -C "$TMP/remoto" commit -q -m uno
git -C "$TMP/remoto" tag -a -m v0.1.0 v0.1.0

git clone -q "$TMP/remoto" "$TMP/clon"
etiqueta=$(traer_ultima_etiqueta "$TMP/clon") || fallo "primera vez: traer_ultima_etiqueta falla"
[ "$etiqueta" = v0.1.0 ] || fallo "primera vez: etiqueta «$etiqueta», se esperaba v0.1.0"

# Historial reescrito: rama huérfana, la rama principal apunta a ella y la
# etiqueta vieja desaparece.
git -C "$TMP/remoto" checkout -q --orphan nueva
git -C "$TMP/remoto" rm -q -rf .
echo dos >"$TMP/remoto/otro"
git -C "$TMP/remoto" add otro
git -C "$TMP/remoto" commit -q -m dos
git -C "$TMP/remoto" branch -q -M nueva main
git -C "$TMP/remoto" tag -d v0.1.0 >/dev/null
git -C "$TMP/remoto" tag -a -m v0.2.0 v0.2.0
git -C "$TMP/remoto" gc -q --prune=now

etiqueta=$(traer_ultima_etiqueta "$TMP/clon") || fallo "tras reescribir: traer_ultima_etiqueta falla"
[ "$etiqueta" = v0.2.0 ] || fallo "tras reescribir: etiqueta «$etiqueta», se esperaba v0.2.0"
[ "$(git -C "$TMP/clon" rev-parse HEAD)" = "$(git -C "$TMP/remoto" rev-parse 'v0.2.0^{commit}')" ] ||
	fallo "el clon no está en el commit de v0.2.0"
[ -z "$(git -C "$TMP/clon" tag -l v0.1.0)" ] || fallo "el clon conserva v0.1.0"
[ -f "$TMP/clon/otro" ] && [ ! -e "$TMP/clon/fichero" ] || fallo "el árbol del clon no es el de v0.2.0"

echo "historial reescrito: ok"
