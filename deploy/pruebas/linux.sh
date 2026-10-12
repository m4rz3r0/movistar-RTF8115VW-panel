#!/bin/sh
# Pruebas de deploy/linux sin tocar el sistema:
#
# - systemd-analyze verify de routerpanel.service (si está), con ExecStart
#   apuntando a un ejecutable de mentira (verify exige que exista);
# - sh -n y shellcheck (si está) de actualizar-release.sh;
# - actualizar-release.sh contra releases servidas por HTTPS (python3 con un
#   certificado propio de usar y tirar, hecho con openssl) en un puerto propio
#   (RP_PRUEBA_PUERTO, por defecto 8501), con systemctl y /salud de mentira y
#   todo bajo una carpeta temporal:
#   1. un SHA256SUMS alterado se rechaza y no toca «current»;
#   2. una URL http:// sin RP_MINISIGN_CLAVE se rechaza;
#   3. --comprobar informa de la versión nueva y no toca nada;
#   4. con la suma buena, despliega y cambia «current»;
#   5. si /salud no responde, vuelve a la versión anterior y borra la nueva;
#   6. un archivo con «../» o que escribe a través de un enlace simbólico se
#      rechaza sin escribir fuera;
#   7. una versión anterior a la desplegada no se despliega, salvo con
#      --forzar.
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

# SHA256SUMS del archivo de la versión $1, que pasa a ser la última.
sumar()
{
	(cd "$SRV/download/$1" && sha256sum "routerpanel-$1-$OBJ.tar.gz" >SHA256SUMS)
	cp "$SRV/download/$1/SHA256SUMS" "$SRV/latest/download/SHA256SUMS"
}

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
	sumar "$_v"
}

# Publica la versión $1 con un archivo malicioso (con sumas válidas): $2 es
# «punto» (un miembro ../../../../escape-punto, que acabaría en $TMP) o
# «enlace» (lnk -> $TMP/fuera y luego lnk/escape-enlace).
publicar_malo()
{
	mkdir -p "$SRV/download/$1"
	python3 -I - "$SRV/download/$1/routerpanel-$1-$OBJ.tar.gz" "routerpanel-$1-$OBJ" \
		"$2" "$TMP/fuera" <<'PY'
import io, sys, tarfile
salida, raiz, tipo, fuera = sys.argv[1:]
def fichero(t, nombre, datos=b"x\n", modo=0o644):
    i = tarfile.TarInfo(nombre)
    i.size = len(datos)
    i.mode = modo
    t.addfile(i, io.BytesIO(datos))
with tarfile.open(salida, "w:gz", format=tarfile.GNU_FORMAT) as t:
    d = tarfile.TarInfo(raiz + "/site")
    d.type = tarfile.DIRTYPE
    d.mode = 0o755
    t.addfile(d)
    fichero(t, raiz + "/panel", b"panel malo\n", 0o755)
    if tipo == "punto":
        fichero(t, "../../../../escape-punto")
    else:
        e = tarfile.TarInfo(raiz + "/lnk")
        e.type = tarfile.SYMTYPE
        e.linkname = fuera
        t.addfile(e)
        fichero(t, raiz + "/lnk/escape-enlace")
PY
	sumar "$1"
}

actualizar()
{
	CURL_CA_BUNDLE=$TMP/cert.pem \
		RP_ACTUALIZAR_CONF=$TMP/no-existe \
		RP_RELEASES_URL=${URL:-"https://127.0.0.1:$PUERTO"} \
		RP_OBJETIVO=$OBJ \
		RP_PREFIJO=$PREFIJO \
		RP_SYSTEMCTL=$TMP/systemctl \
		RP_ENV_PANEL=$TMP/env \
		RP_SALUD_URL=${SALUD:-"https://127.0.0.1:$PUERTO/salud"} \
		RP_SALUD_ESPERA=4 \
		sh "$ACTUALIZAR" "$@" >"$TMP/salida" 2>&1
}

actual()
{
	readlink "$PREFIJO/current"
}

if command -v python3 >/dev/null 2>&1 && command -v openssl >/dev/null 2>&1; then
	mkdir -p "$TMP/fuera"
	openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
		-days 1 -subj /CN=127.0.0.1 -addext subjectAltName=IP:127.0.0.1 \
		-keyout "$TMP/clave.pem" -out "$TMP/cert.pem" 2>"$TMP/openssl.log" || {
		cat "$TMP/openssl.log" >&2
		echo "FALLO: openssl no genera el certificado de la prueba" >&2
		exit 1
	}
	python3 -I - "$SRV" "$PUERTO" "$TMP/cert.pem" "$TMP/clave.pem" >"$TMP/http.log" 2>&1 <<'PY' &
import functools, http.server, ssl, sys
carpeta, puerto, cert, clave = sys.argv[1:]
manejador = functools.partial(http.server.SimpleHTTPRequestHandler, directory=carpeta)
servidor = http.server.ThreadingHTTPServer(("127.0.0.1", int(puerto)), manejador)
ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
ctx.load_cert_chain(cert, clave)
servidor.socket = ctx.wrap_socket(servidor.socket, server_side=True)
servidor.serve_forever()
PY
	SERVIDOR=$!
	n=0
	until curl -s -m 1 -o /dev/null --cacert "$TMP/cert.pem" "https://127.0.0.1:$PUERTO/salud"; do
		n=$((n + 1))
		if [ "$n" -ge 50 ] || ! kill -0 "$SERVIDOR" 2>/dev/null; then
			echo "FALLO: el servidor de la prueba no responde en 127.0.0.1:$PUERTO (¿puerto ocupado? RP_PRUEBA_PUERTO)" >&2
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

	# 2. http:// sin clave de minisign: se rechaza.
	publicar v1.0.0
	if URL="http://127.0.0.1:$PUERTO" actualizar; then
		fallo "una URL http:// sin RP_MINISIGN_CLAVE no falla"
	fi
	grep -q 'https://' "$TMP/salida" || fallo "el error de una URL http:// no pide https://"
	[ "$(actual)" = releases/v0.9.0 ] || fallo "una URL http:// ha cambiado current"

	# 3. --comprobar: informa y no toca nada.
	if ! actualizar --comprobar; then
		cat "$TMP/salida" >&2
		fallo "--comprobar falla"
	fi
	grep -q 'v1.0.0' "$TMP/salida" || fallo "--comprobar no dice la versión nueva"
	[ "$(actual)" = releases/v0.9.0 ] || fallo "--comprobar ha cambiado current"
	[ ! -e "$PREFIJO/releases/v1.0.0" ] || fallo "--comprobar ha descargado la versión"

	# 4. Suma buena: despliega.
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

	# 5. /salud no responde: vuelve a la anterior.
	publicar v1.1.0
	cp "$TMP/datos/panel.db" "$TMP/db-antes"
	if SALUD="https://127.0.0.1:$PUERTO/no-existe" actualizar; then
		fallo "una versión que no responde en /salud no falla"
	fi
	[ "$(actual)" = releases/v1.0.0 ] || fallo "no vuelve a v1.0.0 ($(actual))"
	[ ! -e "$PREFIJO/releases/v1.1.0" ] || fallo "no borra la versión fallida"
	cmp -s "$TMP/db-antes" "$TMP/datos/panel.db" || fallo "no restaura la base de datos de antes"

	# 6. Archivos maliciosos con sumas válidas: no escriben fuera.
	for caso in punto:v2.1.0 enlace:v2.2.0; do
		tipo=${caso%%:*}
		v=${caso#*:}
		publicar_malo "$v" "$tipo"
		if actualizar; then
			fallo "un archivo con «$tipo» no falla"
		fi
		[ "$(actual)" = releases/v1.0.0 ] || fallo "un archivo con «$tipo» ha cambiado current ($(actual))"
		[ ! -e "$PREFIJO/releases/$v" ] || fallo "un archivo con «$tipo» ha dejado releases/$v"
	done
	if [ -n "$(find "$TMP" -name 'escape-*' -print)" ]; then
		fallo "un archivo malicioso ha escrito fuera de su carpeta"
	fi

	# 7. Versión anterior a la desplegada: no, salvo con --forzar.
	publicar v0.9.5
	if actualizar; then
		fallo "se despliega una versión anterior sin --forzar"
	fi
	grep -q -- '--forzar' "$TMP/salida" || fallo "el rechazo de una versión anterior no menciona --forzar"
	[ "$(actual)" = releases/v1.0.0 ] || fallo "una versión anterior ha cambiado current ($(actual))"
	[ ! -e "$PREFIJO/releases/v0.9.5" ] || fallo "una versión anterior se ha descargado sin --forzar"
	if ! actualizar --comprobar || ! grep -q 'no es posterior' "$TMP/salida"; then
		fallo "--comprobar no dice que la última no es posterior"
	fi
	if ! actualizar --forzar; then
		cat "$TMP/salida" >&2
		fallo "--forzar no despliega la versión anterior"
	fi
	[ "$(actual)" = releases/v0.9.5 ] || fallo "--forzar no cambia current a v0.9.5 ($(actual))"
else
	echo "aviso: no hay python3 u openssl; no se prueba el actualizador" >&2
fi

if [ "$fallos" -ne 0 ]; then
	echo "$fallos fallos" >&2
	exit 1
fi
echo "linux: ok"
