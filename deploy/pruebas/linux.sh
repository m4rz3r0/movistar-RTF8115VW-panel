#!/bin/sh
# Pruebas de deploy/linux sin tocar el sistema:
#
# - systemd-analyze verify de routerpanel.service (si está), con ExecStart
#   apuntando a un ejecutable de mentira (verify exige que exista);
# - sh -n y shellcheck (si está) de actualizar-release.sh;
# - actualizar-release.sh contra releases servidas con python3 -m http.server
#   en un puerto propio (RP_PRUEBA_PUERTO, por defecto 8501), con systemctl
#   y /salud de mentira y todo bajo una carpeta temporal:
#   1. un SHA256SUMS alterado se rechaza y no toca «current»;
#   2. --comprobar informa de la versión nueva y no toca nada;
#   3. con la suma buena, despliega y cambia «current»;
#   4. si /salud no responde, vuelve a la versión anterior y borra la nueva.
#
# Uso: deploy/pruebas/linux.sh (desde cualquier sitio).

set -eu

DEPLOY=$(cd "$(dirname "$0")/.." && pwd)
LINUX=$DEPLOY/linux
ACTUALIZAR=$LINUX/actualizar-release.sh
PUERTO=${RP_PRUEBA_PUERTO:-8501}
OBJ=x86_64-unknown-linux-gnu
TMP=$(mktemp -d)
SERVIDOR=
limpiar()
{
	if [ -n "$SERVIDOR" ]; then kill "$SERVIDOR" 2>/dev/null || true; fi
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

for f in "$LINUX/routerpanel.service" "$LINUX/env.example" "$ACTUALIZAR" \
	"$LINUX/nftables-routerpanel.nft"; do
	[ -f "$f" ] || {
		echo "FALLO: falta ${f#"$DEPLOY"/}" >&2
		exit 1
	}
done

# --- unidad de systemd ------------------------------------------------------
if command -v systemd-analyze >/dev/null 2>&1; then
	mkdir -p "$TMP/unidad"
	printf '#!/bin/sh\n' >"$TMP/unidad/panel"
	chmod 755 "$TMP/unidad/panel"
	sed "s|/opt/routerpanel/current/panel|$TMP/unidad/panel|" \
		"$LINUX/routerpanel.service" >"$TMP/unidad/routerpanel.service"
	if ! systemd-analyze verify --man=no "$TMP/unidad/routerpanel.service"; then
		fallo "systemd-analyze verify routerpanel.service"
	fi
else
	echo "aviso: no hay systemd-analyze; no se verifica la unidad" >&2
fi
for clave in 'DynamicUser=yes' 'StateDirectory=routerpanel' \
	'ConfigurationDirectory=routerpanel' 'EnvironmentFile=/etc/routerpanel/env' \
	'ExecStart=/opt/routerpanel/current/panel' 'NoNewPrivileges=yes' \
	'ProtectSystem=strict' 'ProtectHome=yes' 'PrivateTmp=yes' \
	'RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX' 'IPAddressDeny=any' \
	'IPAddressAllow=localhost 192.168.1.1/32'; do
	grep -qx "$clave" "$LINUX/routerpanel.service" || fallo "la unidad no tiene «$clave»"
done

# --- env.example: todas las RP_* que lee el panel ---------------------------
for v in RP_LISTEN RP_DB RP_HISTORY_KEY RP_ORIGIN RP_TRUSTED_PROXY \
	RP_INSTALL_NETS RP_ROUTER_URL RP_ROUTER_USER RP_ROUTER_SECRET RP_LOG; do
	grep -q "^#* *$v=" "$LINUX/env.example" || fallo "env.example no tiene $v"
done
grep -qx 'RP_ORIGIN=' "$LINUX/env.example" || fallo "env.example: RP_ORIGIN debe ir sin valor"

# --- sintaxis del actualizador ----------------------------------------------
sh -n "$ACTUALIZAR" || fallo "sh -n actualizar-release.sh"
if command -v shellcheck >/dev/null 2>&1; then
	shellcheck -s sh "$ACTUALIZAR" "$0" || fallo "shellcheck"
else
	echo "aviso: no hay shellcheck; solo sh -n" >&2
fi

# --- actualizador contra releases locales -----------------------------------
SRV=$TMP/srv
PREFIJO=$TMP/opt
mkdir -p "$SRV/latest/download" "$PREFIJO/releases/v0.9.0/site" "$TMP/datos"
printf 'viejo\n' >"$PREFIJO/releases/v0.9.0/panel"
ln -s releases/v0.9.0 "$PREFIJO/current"
printf 'base de datos\n' >"$TMP/datos/panel.db"
printf 'RP_LISTEN=127.0.0.1:3999\nRP_DB=%s\n' "$TMP/datos/panel.db" >"$TMP/env"
printf '{"ok":true}' >"$SRV/salud"
# systemctl de mentira: apunta lo que se le pide y, al arrancar v1.1.0,
# «migra» la base de datos (como una versión con una migración nueva).
cat >"$TMP/systemctl" <<EOF
#!/bin/sh
echo "\$*" >>"$TMP/systemctl.log"
if [ "\$1" = start ] && [ "\$(readlink "$PREFIJO/current")" = releases/v1.1.0 ]; then
	echo migrada >>"$TMP/datos/panel.db"
fi
EOF
chmod 755 "$TMP/systemctl"

# Publica la versión $1 (como hará la CI) y la deja como la última.
publicar()
{
	_v=$1
	_d=$TMP/build/routerpanel-$_v-$OBJ
	mkdir -p "$_d/site/pkg" "$SRV/download/$_v"
	printf 'panel %s\n' "$_v" >"$_d/panel"
	chmod 755 "$_d/panel"
	printf 'css\n' >"$_d/site/pkg/panel.css"
	tar -czf "$SRV/download/$_v/routerpanel-$_v-$OBJ.tar.gz" -C "$TMP/build" "routerpanel-$_v-$OBJ"
	(cd "$SRV/download/$_v" && sha256sum "routerpanel-$_v-$OBJ.tar.gz" >SHA256SUMS)
	cp "$SRV/download/$_v/SHA256SUMS" "$SRV/latest/download/SHA256SUMS"
}

actualizar()
{
	RP_ACTUALIZAR_CONF=$TMP/no-existe \
		RP_RELEASES_URL="http://127.0.0.1:$PUERTO" \
		RP_OBJETIVO=$OBJ \
		RP_PREFIJO=$PREFIJO \
		RP_SYSTEMCTL=$TMP/systemctl \
		RP_ENV_PANEL=$TMP/env \
		RP_SALUD_URL=${SALUD:-"http://127.0.0.1:$PUERTO/salud"} \
		RP_SALUD_ESPERA=4 \
		sh "$ACTUALIZAR" "$@" >"$TMP/salida" 2>&1
}

actual()
{
	readlink "$PREFIJO/current"
}

if command -v python3 >/dev/null 2>&1; then
	python3 -m http.server --bind 127.0.0.1 --directory "$SRV" "$PUERTO" \
		>"$TMP/http.log" 2>&1 &
	SERVIDOR=$!
	n=0
	until curl -s -m 1 -o /dev/null "http://127.0.0.1:$PUERTO/salud"; do
		n=$((n + 1))
		if [ "$n" -ge 50 ] || ! kill -0 "$SERVIDOR" 2>/dev/null; then
			echo "FALLO: http.server no responde en 127.0.0.1:$PUERTO (¿puerto ocupado? RP_PRUEBA_PUERTO)" >&2
			exit 1
		fi
		sleep 0.1
	done

	# 1. Suma alterada: se rechaza y no se toca nada.
	publicar v1.0.0
	sed 's/^[0-9a-f]\{8\}/00000000/' "$SRV/download/v1.0.0/SHA256SUMS" >"$TMP/sumas"
	cp "$TMP/sumas" "$SRV/download/v1.0.0/SHA256SUMS"
	if actualizar; then
		cat "$TMP/salida" >&2
		fallo "una suma alterada no falla"
	fi
	[ "$(actual)" = releases/v0.9.0 ] || fallo "una suma alterada ha cambiado current ($(actual))"
	[ ! -e "$PREFIJO/releases/v1.0.0" ] || fallo "una suma alterada ha dejado releases/v1.0.0"
	[ ! -e "$TMP/systemctl.log" ] || fallo "una suma alterada ha llamado a systemctl"
	grep -qi 'sha256' "$TMP/salida" || {
		cat "$TMP/salida" >&2
		fallo "el error de una suma alterada no habla de sha256"
	}

	# 2. --comprobar: informa y no toca nada.
	publicar v1.0.0
	if ! actualizar --comprobar; then
		cat "$TMP/salida" >&2
		fallo "--comprobar falla"
	fi
	grep -q 'v1.0.0' "$TMP/salida" || fallo "--comprobar no dice la versión nueva"
	[ "$(actual)" = releases/v0.9.0 ] || fallo "--comprobar ha cambiado current"
	[ ! -e "$PREFIJO/releases/v1.0.0" ] || fallo "--comprobar ha descargado la versión"

	# 3. Suma buena: despliega.
	if ! actualizar; then
		cat "$TMP/salida" >&2
		fallo "no despliega una versión buena"
	fi
	[ "$(actual)" = releases/v1.0.0 ] || fallo "current no apunta a v1.0.0 ($(actual))"
	grep -qx 'panel v1.0.0' "$PREFIJO/releases/v1.0.0/panel" 2>/dev/null ||
		fallo "releases/v1.0.0/panel no es el del archivo"
	[ -x "$PREFIJO/releases/v1.0.0/panel" ] || fallo "releases/v1.0.0/panel no es ejecutable"
	[ -f "$PREFIJO/releases/v1.0.0/site/pkg/panel.css" ] || fallo "falta site/ en v1.0.0"
	grep -q 'start routerpanel' "$TMP/systemctl.log" 2>/dev/null || fallo "no arranca el servicio"
	[ -f "$TMP/datos/antes-de-actualizar/panel.db" ] || fallo "no copia la base de datos antes de desplegar"

	# 4. /salud no responde: vuelve a la anterior.
	publicar v1.1.0
	cp "$TMP/datos/panel.db" "$TMP/db-antes"
	if SALUD="http://127.0.0.1:$PUERTO/no-existe" actualizar; then
		fallo "una versión que no responde en /salud no falla"
	fi
	[ "$(actual)" = releases/v1.0.0 ] || fallo "no vuelve a v1.0.0 ($(actual))"
	[ ! -e "$PREFIJO/releases/v1.1.0" ] || fallo "no borra la versión fallida"
	cmp -s "$TMP/db-antes" "$TMP/datos/panel.db" || fallo "no restaura la base de datos de antes"
else
	echo "aviso: no hay python3; no se prueba el actualizador" >&2
fi

if [ "$fallos" -ne 0 ]; then
	echo "$fallos fallos" >&2
	exit 1
fi
echo "linux: ok"
