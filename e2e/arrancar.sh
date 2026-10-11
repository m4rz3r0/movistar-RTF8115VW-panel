#!/bin/sh
# Arranca router-sim y un panel para las pruebas de navegador, en puertos
# propios (E2E_SIM_PORT, E2E_PORT) y con la base de datos en E2E_DIR (se
# vacía al empezar y se borra al salir; por defecto, una carpeta temporal).
# E2E_MODO:
#   instalado  (por defecto) con el usuario «e2e», las credenciales del router
#              en RP_ROUTER_* (el panel las importa, como al actualizar un
#              servidor) y un grupo de dos pasos ya aplicado (e2e_grupo);
#   nuevo      recién instalado: sin usuarios ni router (el asistente).
# Necesita antes, en la raíz del repositorio: `cargo leptos build`, `cargo
# build -p router-sim` y `cargo build -p panel --features ssr --example
# e2e_grupo`. Lo lanza playwright.config.ts (ver instancias.ts).
set -eu
cd "$(dirname "$0")/.."
SIM_PORT=${E2E_SIM_PORT:-8195}
PORT=${E2E_PORT:-3195}
MODO=${E2E_MODO:-instalado}
case $MODO in
instalado | nuevo) ;;
*) echo "E2E_MODO desconocido: $MODO" >&2; exit 1 ;;
esac
if [ -n "${E2E_DIR:-}" ]; then
	case $E2E_DIR in
	*routerpanel-e2e-*) ;;
	*) echo "E2E_DIR debe ser una carpeta routerpanel-e2e-*: $E2E_DIR" >&2; exit 1 ;;
	esac
	DIR=$E2E_DIR
	rm -rf "$DIR"
	mkdir -m 700 -p "$DIR"
else
	DIR=$(mktemp -d)
fi
SIM=
limpiar()
{
	if [ -n "$SIM" ]; then kill "$SIM" 2>/dev/null || true; fi
	rm -rf "$DIR"
}
trap limpiar EXIT
trap 'exit 143' INT TERM
for f in target/debug/panel target/debug/router-sim target/debug/examples/e2e_grupo target/site/pkg; do
	[ -e "$f" ] || {
		echo "falta $f: cargo leptos build && cargo build -p router-sim &&" \
			"cargo build -p panel --features ssr --example e2e_grupo" >&2
		exit 1
	}
done
target/debug/router-sim --listen "127.0.0.1:$SIM_PORT" --fixtures crates/hgu/fixtures &
SIM=$!
# Que el simulador escuche de verdad (y sea el nuestro) antes de seguir: con
# el puerto ocupado, el panel sólo vería «Esperando el primer dato».
n=0
while :; do
	if ! kill -0 "$SIM" 2>/dev/null; then
		echo "router-sim no arrancó en 127.0.0.1:$SIM_PORT (¿puerto ocupado? E2E_SIM_PORT)" >&2
		exit 1
	fi
	if curl -s -m 1 -o /dev/null "http://127.0.0.1:$SIM_PORT/"; then
		break
	fi
	n=$((n + 1))
	if [ "$n" -ge 50 ]; then
		echo "router-sim no responde en 127.0.0.1:$SIM_PORT" >&2
		exit 1
	fi
	sleep 0.1
done
export RP_DB="$DIR/panel.db" RP_HISTORY_KEY="$DIR/history.key" \
	RP_TRUSTED_PROXY=127.0.0.1 RP_ORIGIN="http://localhost:$PORT" RP_LOG=warn
if [ "$MODO" = instalado ]; then
	printf 'sim' >"$DIR/router.secret"
	chmod 600 "$DIR/router.secret"
	export RP_ROUTER_URL="http://127.0.0.1:$SIM_PORT" RP_ROUTER_USER=sim \
		RP_ROUTER_SECRET="$DIR/router.secret"
	printf 'e2e-contrasena\n' | target/debug/panel user add e2e
	target/debug/examples/e2e_grupo
fi
RP_LISTEN="127.0.0.1:$PORT" \
	LEPTOS_OUTPUT_NAME=panel \
	LEPTOS_SITE_ROOT=target/site \
	LEPTOS_SITE_PKG_DIR=pkg \
	LEPTOS_SITE_ADDR="127.0.0.1:$PORT" \
	target/debug/panel || {
	echo "el panel no arrancó en 127.0.0.1:$PORT (¿puerto ocupado? E2E_PORT)" >&2
	exit 1
}
