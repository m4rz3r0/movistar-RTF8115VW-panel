#!/bin/sh
# Arranca router-sim y el panel para las pruebas de navegador, en puertos
# propios (E2E_SIM_PORT, E2E_PORT) y con una base de datos temporal con el
# usuario «e2e». Necesita antes `cargo leptos build` y `cargo build -p
# router-sim` en la raíz del repositorio. Lo lanza playwright.config.ts.
set -eu
cd "$(dirname "$0")/.."
SIM_PORT=${E2E_SIM_PORT:-8195}
PORT=${E2E_PORT:-3195}
DIR=$(mktemp -d)
SIM=
limpiar()
{
	if [ -n "$SIM" ]; then kill "$SIM" 2>/dev/null || true; fi
	rm -rf "$DIR"
}
trap limpiar EXIT
trap 'exit 143' INT TERM
for f in target/debug/panel target/debug/router-sim target/site/pkg; do
	[ -e "$f" ] || { echo "falta $f: cargo leptos build && cargo build -p router-sim" >&2; exit 1; }
done
printf 'sim' >"$DIR/router.secret"
chmod 600 "$DIR/router.secret"
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
printf 'e2e-contrasena\n' | RP_DB="$DIR/panel.db" target/debug/panel user add e2e
RP_LISTEN="127.0.0.1:$PORT" \
	RP_DB="$DIR/panel.db" \
	RP_ROUTER_URL="http://127.0.0.1:$SIM_PORT" \
	RP_ROUTER_USER=sim \
	RP_ROUTER_SECRET="$DIR/router.secret" \
	RP_HISTORY_KEY="$DIR/history.key" \
	RP_TRUSTED_PROXY=127.0.0.1 \
	RP_ORIGIN="http://localhost:$PORT" \
	RP_LOG=warn \
	LEPTOS_OUTPUT_NAME=panel \
	LEPTOS_SITE_ROOT=target/site \
	LEPTOS_SITE_PKG_DIR=pkg \
	LEPTOS_SITE_ADDR="127.0.0.1:$PORT" \
	target/debug/panel || {
	echo "el panel no arrancó en 127.0.0.1:$PORT (¿puerto ocupado? E2E_PORT)" >&2
	exit 1
}
