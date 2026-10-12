#!/bin/sh
# Prueba de humo de deploy/docker con Compose (docker compose, podman compose
# o podman-compose; si no hay ninguno, se salta con un aviso):
#
# 1. levanta el proyecto RP_PRUEBA_PROYECTO (por defecto routerpanel-prueba)
#    con el perfil «prueba» (router-sim) y el proxy en el puerto
#    RP_PRUEBA_PUERTO (por defecto 8500) de este equipo;
# 2. espera a que /salud responda 200 a través del proxy (HTTPS interno);
# 3. comprueba que existe /data/instalar.token en el contenedor del panel;
# 4. una petición directa al panel desde la puerta de enlace de la red de
#    Compose (no es el proxy de confianza), aunque diga en X-Forwarded-For
#    venir de la red local, no puede crear el administrador; a través de
#    el proxy tampoco sin el token;
# 5. down -v, solo de ese proyecto (también si algo falla).
#
# Las imágenes que compila (<proyecto>_panel y <proyecto>_router-sim) se
# quedan, para que la siguiente vez la compilación salga de la caché.
#
# Uso: deploy/pruebas/docker.sh (desde cualquier sitio).

set -eu

DEPLOY=$(cd "$(dirname "$0")/.." && pwd)
PROYECTO=${RP_PRUEBA_PROYECTO:-routerpanel-prueba}
PUERTO=${RP_PRUEBA_PUERTO:-8500}
DOMINIO=panel.home.arpa

if command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1; then
	set -- docker compose
	MOTOR=docker
elif command -v podman >/dev/null 2>&1 && podman compose version >/dev/null 2>&1; then
	set -- podman compose
	MOTOR=podman
elif command -v podman >/dev/null 2>&1 && command -v podman-compose >/dev/null 2>&1; then
	set -- podman-compose
	MOTOR=podman
else
	echo "aviso: no hay docker compose, podman compose ni podman-compose; se salta la prueba de Compose" >&2
	exit 0
fi

# Variables de compose.yaml para la prueba.
export RP_PUERTO_HTTPS="$PUERTO" PANEL_DOMINIO="$DOMINIO" \
	RP_ORIGIN="https://$DOMINIO:$PUERTO"
# Las conexiones a 127.0.0.1 no llegan al proxy con la IP del cliente: con
# Docker, el proxy de puertos las hace desde la puerta de enlace de la red
# (198.51.100.1); con podman sin root, el proxy ve llegar todo desde su
# propia IP (198.51.100.10; ver deploy/docker/README.md). Ninguna es privada:
# se dejan pasar a propósito. Con podman sin root, además, el asistente tiene
# que aceptar la IP del proxy (RP_INSTALL_NETS, como dice el README) para
# poder comprobar que, aun así, sin token no se crea el administrador.
if [ "$MOTOR" = podman ] && [ "$(podman info --format '{{.Host.Security.Rootless}}')" = true ]; then
	export PROXY_REDES_EXTRA=198.51.100.10/32 RP_INSTALL_NETS=198.51.100.10
	SIN_ROOT=1
else
	export PROXY_REDES_EXTRA=198.51.100.1/32
	SIN_ROOT=
fi

COMPOSE="$*"
# Compose sobre el proyecto de la prueba y nada más.
c()
{
	# shellcheck disable=SC2086 # COMPOSE son palabras sueltas a propósito
	$COMPOSE -p "$PROYECTO" -f "$DEPLOY/docker/compose.yaml" --profile prueba "$@"
}

TMP=$(mktemp -d)
limpiar()
{
	c down -v >"$TMP/down.log" 2>&1 || cat "$TMP/down.log" >&2
	rm -rf "$TMP"
}
trap limpiar EXIT
trap 'exit 143' INT TERM

fallos=0
fallo()
{
	echo "FALLO: $*" >&2
	fallos=$((fallos + 1))
}

# --- 1. arrancar ------------------------------------------------------------
if ! c up -d --build >"$TMP/up.log" 2>&1; then
	tail -n 40 "$TMP/up.log" >&2
	echo "FALLO: compose up" >&2
	exit 1
fi

# --- 2. /salud a través del proxy -----------------------------------------
salud()
{
	curl -sk -m 3 -o "$TMP/salud" -w '%{http_code}' \
		--resolve "$DOMINIO:$PUERTO:127.0.0.1" "https://$DOMINIO:$PUERTO/salud" || true
}
n=0
until [ "$(salud)" = 200 ]; do
	n=$((n + 1))
	if [ "$n" -ge 60 ]; then
		c logs >&2 2>&1 || true
		echo "FALLO: /salud no responde 200 a través del proxy en 120 s" >&2
		exit 1
	fi
	sleep 2
done
grep -q '"ok":true' "$TMP/salud" || fallo "/salud no dice \"ok\":true"

# --- 3. token del asistente -------------------------------------------------
if ! c exec -T panel test -s /data/instalar.token; then
	fallo "no existe /data/instalar.token en el panel"
fi

# --- 4. el asistente no se abre sin token -----------------------------------
# POST a crear_admin sin token; el cuerpo de la respuesta, en $1; delante de
# curl, $RED (vacía, o la orden que entra en el espacio de red de podman).
RED=
crear_admin()
{
	_cuerpo=$1
	shift
	# shellcheck disable=SC2086 # RED son palabras sueltas a propósito
	$RED curl -s -m 5 -o "$_cuerpo" -w '%{http_code}' -X POST \
		-H 'Content-Type: application/json' -H 'Accept-Language: es' "$@" \
		--data '{"usuario":"intruso","contrasena":"una contraseña larga"}' || true
}
id=$($MOTOR ps -q --filter "label=com.docker.compose.project=$PROYECTO" \
	--filter label=com.docker.compose.service=panel | head -n 1)
ip=
if [ -n "$id" ]; then
	ip=$($MOTOR inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' "$id" || true)
fi
if [ -z "$ip" ]; then
	fallo "no se sabe la IP del panel en la red de Compose"
else
	# Desde la puerta de enlace de la red de Compose (con podman sin root, en
	# su espacio de red; con root, desde este equipo), diciendo venir de la
	# red local. La puerta de enlace no es el proxy de confianza: el panel no
	# cree la cabecera y responde que no es la red local, antes de mirar el
	# token. (Si confiara en la subred entera, la creería y solo le pararía
	# el token.)
	if [ -n "$SIN_ROOT" ]; then RED="podman unshare --rootless-netns"; fi
	directo=$(crear_admin "$TMP/directo" -H 'X-Forwarded-For: 10.255.0.1' \
		"http://$ip:3000/api/crear_admin")
	RED=
	[ "$directo" = 403 ] || fallo "petición directa desde la puerta de enlace: $directo, no 403"
	grep -q 'red local' "$TMP/directo" 2>/dev/null ||
		fallo "la petición directa desde la puerta de enlace no se para por la red"
fi
# A través del proxy, desde la red local (o PROXY_REDES_EXTRA): sin token, no.
via_proxy=$(crear_admin "$TMP/via-proxy" --resolve "$DOMINIO:$PUERTO:127.0.0.1" -k \
	-H "Origin: https://$DOMINIO:$PUERTO" "https://$DOMINIO:$PUERTO/api/crear_admin")
[ "$via_proxy" = 403 ] || fallo "a través del proxy sin token: $via_proxy, no 403"
if [ -n "$SIN_ROOT" ] && ! grep -q 'Token de instalación no válido' "$TMP/via-proxy"; then
	fallo "a través del proxy sin token, el 403 no es por el token"
fi
c exec -T panel test -s /data/instalar.token || fallo "tras las peticiones sin token ya no hay token"

if [ "$fallos" -ne 0 ]; then
	c logs panel >&2 2>&1 || true
	echo "$fallos fallos" >&2
	exit 1
fi
echo "docker ($COMPOSE): ok"
