#!/bin/sh
# Pruebas de la instalación en jail con la configuración de ejemplo, sin
# tocar ningún servidor:
#
# - sintaxis (sh -n) de todos los guiones de deploy/ y shellcheck si está;
# - install.sh --plantillas con config.example no deja ninguna @CLAVE@;
# - comprobar-publicacion no encuentra nada en deploy/ (sin listas locales);
# - una configuración sin ROUTER_IP falla y el mensaje nombra la clave;
# - TRABAJOS_COMPILACION (opcional) llega a actualizador.conf y un valor que
#   no es un entero positivo falla nombrando la clave.
#
# Uso: deploy/pruebas/plantillas.sh (desde cualquier sitio). El comprobador
# se toma de $COMPROBAR (ruta al binario) o, si no, de «cargo run».

set -eu

DEPLOY=$(cd "$(dirname "$0")/.." && pwd)
RAIZ=$(dirname "$DEPLOY")
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

fallos=0
fallo()
{
	echo "FALLO: $*" >&2
	fallos=$((fallos + 1))
}

# --- sintaxis ---------------------------------------------------------------
for f in "$DEPLOY"/*/*.sh "$DEPLOY"/freebsd-jail/routerpanel.rc; do
	sh -n "$f" || fallo "sh -n: ${f#"$RAIZ"/}"
done
if command -v shellcheck >/dev/null 2>&1; then
	shellcheck -s sh "$DEPLOY"/*/*.sh "$DEPLOY"/freebsd-jail/routerpanel.rc ||
		fallo "shellcheck"
else
	echo "aviso: no hay shellcheck; solo sh -n" >&2
fi

# --- plantillas con el ejemplo ----------------------------------------------
if sh "$DEPLOY/freebsd-jail/install.sh" --config "$DEPLOY/config.example" \
	--plantillas "$TMP/plantillas" >"$TMP/salida" 2>&1; then
	if [ -z "$(ls -A "$TMP/plantillas" 2>/dev/null)" ]; then
		fallo "--plantillas no ha generado ningún fichero"
	fi
	if grep -rn '@[A-Za-z_][A-Za-z0-9_]*@' "$TMP/plantillas" >&2; then
		fallo "quedan @CLAVE@ sin sustituir"
	fi
	for f in jail.conf pf.conf rc.conf actualizador.conf resolv.conf; do
		[ -s "$TMP/plantillas/$f" ] || fallo "--plantillas no genera $f"
	done
else
	cat "$TMP/salida" >&2
	fallo "install.sh --plantillas con config.example"
fi

# --- comprobador de publicación ---------------------------------------------
if [ -n "${COMPROBAR:-}" ]; then
	set -- "$COMPROBAR"
else
	set -- cargo run -q --manifest-path "$RAIZ/Cargo.toml" -p comprobar-publicacion --
fi
if ! (cd "$RAIZ" && "$@" --arbol --solo deploy/ --sin-locales) >"$TMP/comprobar" 2>&1; then
	cat "$TMP/comprobar" >&2
	fallo "comprobar-publicacion encuentra algo en deploy/"
fi

# --- clave que falta --------------------------------------------------------
grep -v '^ROUTER_IP=' "$DEPLOY/config.example" >"$TMP/sin-router.conf"
if ROUTER_IP=192.168.1.1 sh "$DEPLOY/freebsd-jail/install.sh" --config "$TMP/sin-router.conf" \
	--plantillas "$TMP/sin-router" >"$TMP/salida" 2>&1; then
	fallo "una configuración sin ROUTER_IP no falla"
elif ! grep -q 'ROUTER_IP' "$TMP/salida"; then
	cat "$TMP/salida" >&2
	fallo "el error de una configuración sin ROUTER_IP no nombra la clave"
fi

# --- trabajos de compilación -----------------------------------------------
# Vacía por defecto (cargo decide); un número llega a actualizador.conf; lo
# que no es un número entero positivo falla y nombra la clave.
grep -q "^TRABAJOS_COMPILACION=''" "$TMP/plantillas/actualizador.conf" 2>/dev/null ||
	fallo "actualizador.conf no lleva TRABAJOS_COMPILACION vacía con el ejemplo"
{ cat "$DEPLOY/config.example"; echo 'TRABAJOS_COMPILACION=3'; } >"$TMP/trabajos.conf"
if sh "$DEPLOY/freebsd-jail/install.sh" --config "$TMP/trabajos.conf" \
	--plantillas "$TMP/trabajos" >"$TMP/salida" 2>&1; then
	grep -q "^TRABAJOS_COMPILACION='3'" "$TMP/trabajos/actualizador.conf" ||
		fallo "TRABAJOS_COMPILACION no llega a actualizador.conf"
else
	cat "$TMP/salida" >&2
	fallo "install.sh --plantillas con TRABAJOS_COMPILACION=3"
fi
for malo in x 0 -2; do
	{ cat "$DEPLOY/config.example"; echo "TRABAJOS_COMPILACION=$malo"; } >"$TMP/trabajos.conf"
	if sh "$DEPLOY/freebsd-jail/install.sh" --config "$TMP/trabajos.conf" \
		--plantillas "$TMP/trabajos-malo" >"$TMP/salida" 2>&1; then
		fallo "TRABAJOS_COMPILACION=$malo no falla"
	elif ! grep -q TRABAJOS_COMPILACION "$TMP/salida"; then
		cat "$TMP/salida" >&2
		fallo "el error de TRABAJOS_COMPILACION=$malo no nombra la clave"
	fi
done

if [ "$fallos" -ne 0 ]; then
	echo "$fallos fallos" >&2
	exit 1
fi
echo "plantillas: ok"
