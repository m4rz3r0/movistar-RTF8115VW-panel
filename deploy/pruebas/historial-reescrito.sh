#!/bin/sh
# ultima_etiqueta_remota (deploy/lib/git.sh) con `git ls-remote --tags`, como
# la usa el actualizador, contra un remoto cuyo historial se reescribe: sin
# clon, la etiqueta y el commit son siempre los que el remoto tiene ahora.
#
#   1. remoto con v0.1.0 (anotada): da v0.1.0 y el commit de la etiqueta
#      (la línea pelada ^{}, no el objeto de la etiqueta);
#   2. el remoto se reescribe (rama huérfana con v0.2.0; v0.1.0 desaparece):
#      da v0.2.0 y su commit;
#   3. una etiqueta ligera (sin ^{}) da el commit al que apunta;
#   4. sort -V (v0.10.0 va después de v0.9.0) y solo vX.Y.Z: ni v1.0.0-rc1,
#      ni «v2», ni otras; sin ninguna vX.Y.Z no da nada.
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

ultima()
{
	git ls-remote --tags "$1" | ultima_etiqueta_remota
}

git init -q -b main "$TMP/remoto"
echo uno >"$TMP/remoto/fichero"
git -C "$TMP/remoto" add fichero
git -C "$TMP/remoto" commit -q -m uno
git -C "$TMP/remoto" tag -a -m v0.1.0 v0.1.0

[ "$(ultima "$TMP/remoto")" = "v0.1.0 $(git -C "$TMP/remoto" rev-parse 'v0.1.0^{commit}')" ] ||
	fallo "primera vez: «$(ultima "$TMP/remoto")», se esperaba v0.1.0 y su commit"

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

[ "$(ultima "$TMP/remoto")" = "v0.2.0 $(git -C "$TMP/remoto" rev-parse 'v0.2.0^{commit}')" ] ||
	fallo "tras reescribir: «$(ultima "$TMP/remoto")», se esperaba v0.2.0 y su commit"

# Etiqueta ligera: sin línea pelada.
git -C "$TMP/remoto" tag v0.3.0
[ "$(ultima "$TMP/remoto")" = "v0.3.0 $(git -C "$TMP/remoto" rev-parse HEAD)" ] ||
	fallo "etiqueta ligera: «$(ultima "$TMP/remoto")»"

# Orden y nombres.
for t in v0.9.0 v0.10.0 v1.0.0-rc1 v2 otra; do
	git -C "$TMP/remoto" tag "$t"
done
[ "$(ultima "$TMP/remoto" | cut -d' ' -f1)" = v0.10.0 ] ||
	fallo "orden: «$(ultima "$TMP/remoto")», se esperaba v0.10.0"
git init -q -b main "$TMP/sin"
git -C "$TMP/sin" commit -q --allow-empty -m nada
git -C "$TMP/sin" tag otra
git -C "$TMP/sin" tag v1.0.0-rc1
[ -z "$(ultima "$TMP/sin")" ] || fallo "sin vX.Y.Z da «$(ultima "$TMP/sin")»"
# Líneas raras (commit corto, mayúsculas, espacios de más) no cuentan.
[ -z "$(printf 'abc\trefs/tags/v9.9.9\n%s\trefs/tags/v9.9.9 x\n' "$(printf '%040d' 0 | tr 0 A)" | ultima_etiqueta_remota)" ] ||
	fallo "acepta líneas que no son de ls-remote"

echo "historial reescrito: ok"
