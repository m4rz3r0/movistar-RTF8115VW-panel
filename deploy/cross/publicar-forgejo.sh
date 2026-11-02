#!/usr/bin/env bash
# Compila una etiqueta para FreeBSD y la publica como la release de esa
# etiqueta en Forgejo, de donde la instala el actualizador del servidor
# (deploy/freebsd-jail/actualizar.sh). Lo lanza el flujo
# .forgejo/workflows/freebsd.yml en el runner de Actions del PC (etiqueta
# «pc», ejecutor host) al empujar una etiqueta v*, desde la copia de la
# etiqueta que el flujo trae con git.
#
#   publicar-forgejo.sh
#
# Todo llega por el entorno (lo pone el flujo):
#   FORGEJO_TOKEN       el token del trabajo (forgejo.token)
#   FORGEJO_SERVER      la URL base de Forgejo (forgejo.server_url)
#   FORGEJO_REPOSITORY  «propietario/nombre» (forgejo.repository)
#   FORGEJO_TAG         la etiqueta (forgejo.ref_name)
# Opcionales: RP_CROSS_DIR (como en compilar-freebsd.sh; por defecto
# ~/rtf-cross, así la compilación es incremental), RP_MINISIGN_SECRETA
# (firma SHA256SUMS) y RP_COMPILAR (otro guion de compilación, para las
# pruebas).
#
# Pasos:
# 1. Comprueba la etiqueta (version_precompilable) y compila con
#    compilar-freebsd.sh --release <etiqueta> --repo <servidor>/<repo>.git.
#    git entra con el token por GIT_CONFIG_* (http.extraheader), nunca en la
#    URL ni en los argumentos; compilar-freebsd.sh las borra antes de
#    compilar y FORGEJO_TOKEN no llega a la compilación.
# 2. Trae la release de la etiqueta (GET /releases/tags/<etiqueta>) o, si no
#    existe (404), la crea con el commit en la descripción.
# 3. Sube el archivo, SHA256SUMS.minisig si lo hay y, el último, SHA256SUMS
#    (el actualizador da la release por completa cuando está SHA256SUMS). Un
#    adjunto que ya estaba con el mismo nombre se borra antes: repetir el
#    trabajo lo sustituye.
# Cada curl lee la cabecera con el token de la entrada estándar (-K -).

set -euo pipefail

AQUI=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=../lib/precompiladas.sh
. "$AQUI/../lib/precompiladas.sh"
COMPILAR=${RP_COMPILAR:-$AQUI/compilar-freebsd.sh}
BASE=${RP_CROSS_DIR:-$HOME/rtf-cross}

error()
{
	echo "ERROR: $*" >&2
	exit 1
}

for v in FORGEJO_TOKEN FORGEJO_SERVER FORGEJO_REPOSITORY FORGEJO_TAG; do
	[ -n "${!v:-}" ] || error "falta $v en el entorno"
done
TAG=$FORGEJO_TAG
version_precompilable "$TAG" || error "«$TAG» no es una versión (vX.Y.Z…)"
url_forgejo_valida "$FORGEJO_SERVER" || error "FORGEJO_SERVER no es una URL base de Forgejo"
repo_forgejo_valido "$FORGEJO_REPOSITORY" || error "FORGEJO_REPOSITORY no es «propietario/nombre»"
case $FORGEJO_TOKEN in
*[!A-Za-z0-9_.-]*) error "el token no es válido" ;;
esac
TOKEN=$FORGEJO_TOKEN
unset FORGEJO_TOKEN
for orden in curl jq sha256sum tar; do
	command -v "$orden" >/dev/null 2>&1 || error "falta $orden"
done
API=$FORGEJO_SERVER/api/v1/repos/$FORGEJO_REPOSITORY
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# --- 1. compilar -------------------------------------------------------------
# Sin los demás tokens que el runner pueda poner en el entorno del trabajo.
GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=http.extraheader GIT_CONFIG_VALUE_0="Authorization: token $TOKEN" \
	RP_CROSS_DIR=$BASE env -u GITHUB_TOKEN -u GITEA_TOKEN -u ACTIONS_RUNTIME_TOKEN bash "$COMPILAR" --release "$TAG" --repo "$FORGEJO_SERVER/$FORGEJO_REPOSITORY.git"

SALIDA=$BASE/salida/$TAG
ARCH=$(archivo_precompilada "$TAG")
for f in "$ARCH" SHA256SUMS; do
	[ -f "$SALIDA/$f" ] || error "falta $SALIDA/$f"
done
(cd "$SALIDA" && sha256sum -c --status SHA256SUMS) || error "SHA256SUMS no cuadra con $ARCH"
COMMIT=$(tar -xzOf "$SALIDA/$ARCH" "${ARCH%.tar.gz}/COMMIT" | head -n 1)
case $COMMIT in
'' | *[!0-9a-f]*) error "el archivo no trae un COMMIT válido" ;;
esac

# --- API de Forgejo ----------------------------------------------------------
# api MÉTODO RUTA RESPUESTA [ARGUMENTOS DE CURL…]: llama a $API$RUTA, deja el
# cuerpo en RESPUESTA y escribe el código HTTP (000 si no hay respuesta).
api()
{
	local metodo=$1 ruta=$2 respuesta=$3
	shift 3
	printf 'header = "Authorization: token %s"\n' "$TOKEN" |
		curl -K - -sS --fail-with-body --connect-timeout 15 --max-time 1800 \
			-X "$metodo" -o "$respuesta" -w '%{http_code}' "$@" "$API$ruta" || true
}

# --- 2. la release -------------------------------------------------------------
codigo=$(api GET "/releases/tags/$TAG" "$TMP/release.json")
case $codigo in
200) ;;
404)
	jq -n --arg t "$TAG" --arg c "$COMMIT" '{
		tag_name: $t,
		name: $t,
		body: ("Compilación para FreeBSD (x86_64-unknown-freebsd) del commit " + $c + ".")
	}' >"$TMP/nueva.json"
	codigo=$(api POST /releases "$TMP/release.json" -H 'Content-Type: application/json' \
		--data-binary "@$TMP/nueva.json")
	[ "$codigo" = 201 ] || error "Forgejo no crea la release $TAG (HTTP $codigo): $(head -c 300 "$TMP/release.json")"
	echo "creada la release $TAG"
	;;
*) error "Forgejo responde HTTP $codigo a la release $TAG: $(head -c 300 "$TMP/release.json")" ;;
esac
ID=$(jq -r '.id' "$TMP/release.json")
case $ID in
'' | *[!0-9]*) error "Forgejo no da el id de la release $TAG" ;;
esac

# --- 3. los adjuntos -------------------------------------------------------------
codigo=$(api GET "/releases/$ID/assets" "$TMP/adjuntos.json")
[ "$codigo" = 200 ] || error "Forgejo no da los adjuntos de la release $TAG (HTTP $codigo)"
ficheros=("$ARCH")
if [ -f "$SALIDA/SHA256SUMS.minisig" ]; then
	ficheros+=(SHA256SUMS.minisig)
fi
ficheros+=(SHA256SUMS)
# Primero se borra lo que ya estaba, SHA256SUMS el primero, y luego se sube,
# SHA256SUMS el último: el servidor nunca ve un SHA256SUMS de otro archivo,
# solo una release a medias, y espera.
for ((i = ${#ficheros[@]} - 1; i >= 0; i--)); do
	f=${ficheros[i]}
	while read -r adjunto; do
		case $adjunto in
		'' | *[!0-9]*) error "Forgejo da un id de adjunto que no es un número" ;;
		esac
		codigo=$(api DELETE "/releases/$ID/assets/$adjunto" "$TMP/borrado.json")
		[ "$codigo" = 204 ] || error "Forgejo no borra el $f anterior (HTTP $codigo)"
		echo "borrado el $f anterior"
	done < <(jq -r --arg n "$f" '.[] | select(.name == $n) | .id' "$TMP/adjuntos.json")
done
for f in "${ficheros[@]}"; do
	codigo=$(api POST "/releases/$ID/assets?name=$f" "$TMP/subido.json" -F "attachment=@$SALIDA/$f")
	[ "$codigo" = 201 ] || error "Forgejo no acepta $f (HTTP $codigo): $(head -c 300 "$TMP/subido.json")"
	echo "subido $f"
done
echo "publicada la release $TAG ($FORGEJO_SERVER/$FORGEJO_REPOSITORY/releases/tag/$TAG)"
