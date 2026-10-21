#!/bin/sh
# Prueba de que la release que publica la CI (.github/workflows/release.yml)
# y el actualizador (deploy/linux/actualizar-release.sh) casan, sin tocar el
# sistema:
#
# 1. Empaqueta una v1.0.0 con tools/empaquetar-release.sh, como el workflow:
#    el archivo de este equipo con el panel de verdad, otro del otro objetivo
#    (para que SHA256SUMS tenga las dos líneas) y SHA256SUMS con --sumas.
#    Empaquetar dos veces da el mismo archivo. Se empaqueta desde un clon
#    del commit HEAD (lo que empaqueta el workflow) con un fichero sin
#    seguimiento y otro ignorado en deploy/linux/, que no deben acabar en el
#    archivo; deploy/linux/ trae exactamente lo de git.
# 2. Los sirve por HTTPS (python3 y un certificado de usar y tirar hecho con
#    openssl) en un puerto propio (RP_PRUEBA_PUERTO, por defecto 8502), con
#    las rutas de GitHub: latest/download/SHA256SUMS y download/v1.0.0/….
# 3. Pasa el actualizador con todo bajo una carpeta temporal y un systemctl
#    de mentira que arranca de verdad current/panel con el entorno de la
#    unidad que trae el archivo (Environment= y ExecStart, con
#    /opt/routerpanel cambiado por la carpeta temporal) y el fichero de
#    entorno del panel (RP_LISTEN en 127.0.0.1:RP_PRUEBA_PUERTO_PANEL, por
#    defecto 3502). El actualizador espera a /salud del panel de verdad.
# 4. Comprueba lo desplegado (panel, site/, LICENSE, AVISO-LEGAL.md,
#    deploy/linux/README.md), que el panel sirve su web desde site/, que no
#    atiende RP_E2E_VIGILANCIA_S (la vigilancia corta de las pruebas, que
#    solo existe con debug-assertions) y que --comprobar dice que está al día.
#
# Necesita el panel compilado: RP_PRUEBA_PANEL (por defecto
# target/release/panel) y RP_PRUEBA_SITE (por defecto target/site), de
# `cargo leptos build --release`. Prueba lo confirmado (HEAD), no los cambios
# sin confirmar. Si LICENSE o AVISO-LEGAL.md aún no están en HEAD, el clon
# los añade de relleno en un commit propio y avisa.
#
# Uso: deploy/pruebas/release-local.sh (desde cualquier sitio).

set -eu

RAIZ=$(cd "$(dirname "$0")/../.." && pwd)
PUERTO=${RP_PRUEBA_PUERTO:-8502}
PUERTO_PANEL=${RP_PRUEBA_PUERTO_PANEL:-3502}
PANEL=${RP_PRUEBA_PANEL:-$RAIZ/target/release/panel}
SITE=${RP_PRUEBA_SITE:-$RAIZ/target/site}
VER=v1.0.0
case "$(uname -m)" in
x86_64) OBJ=x86_64-unknown-linux-gnu OTRO=aarch64-unknown-linux-gnu ;;
aarch64 | arm64) OBJ=aarch64-unknown-linux-gnu OTRO=x86_64-unknown-linux-gnu ;;
*)
	echo "arquitectura sin releases: $(uname -m)" >&2
	exit 1
	;;
esac

for orden in python3 openssl curl sha256sum tar; do
	command -v "$orden" >/dev/null 2>&1 || {
		echo "FALLO: falta $orden" >&2
		exit 1
	}
done
if [ ! -x "$PANEL" ] || [ ! -d "$SITE/pkg" ]; then
	echo "FALLO: falta el panel compilado ($PANEL y $SITE/pkg): cargo leptos build --release" >&2
	exit 1
fi

TMP=$(mktemp -d)
SERVIDOR=
limpiar()
{
	if [ -x "$TMP/systemctl" ]; then "$TMP/systemctl" stop routerpanel 2>/dev/null || true; fi
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

# --- 1. empaquetar como el workflow -----------------------------------------
CLON=$TMP/repo
git clone -q "$RAIZ" "$CLON" 2>"$TMP/clon.log" || {
	cat "$TMP/clon.log" >&2
	echo "FALLO: no se puede clonar el repositorio" >&2
	exit 1
}
relleno=
for f in LICENSE AVISO-LEGAL.md; do
	if [ ! -f "$CLON/$f" ]; then
		printf 'relleno\n' >"$CLON/$f"
		relleno="$relleno $f"
	fi
done
if [ -n "$relleno" ]; then
	echo "aviso: faltan en HEAD$relleno; se empaqueta con unos de relleno" >&2
	git -C "$CLON" add LICENSE AVISO-LEGAL.md
	git -C "$CLON" -c user.name=prueba -c user.email=prueba \
		commit -q -m relleno
fi
# Un fichero sin seguimiento y otro ignorado (*.secret) en deploy/linux/.
printf 'sin seguimiento\n' >"$CLON/deploy/linux/sin-seguimiento.txt"
printf 'ignorado\n' >"$CLON/deploy/linux/prueba.secret"
EMPAQUETAR=$CLON/tools/empaquetar-release.sh
DIST=$TMP/dist
export SOURCE_DATE_EPOCH=1700000000
sh "$EMPAQUETAR" "$VER" "$OBJ" "$PANEL" "$SITE" "$DIST" >/dev/null
sh "$EMPAQUETAR" "$VER" "$OTRO" "$PANEL" "$SITE" "$DIST" >/dev/null
sh "$EMPAQUETAR" --sumas "$DIST"
ARCHIVO=routerpanel-$VER-$OBJ.tar.gz

sh "$EMPAQUETAR" "$VER" "$OBJ" "$PANEL" "$SITE" "$TMP/otra-vez" >/dev/null
cmp -s "$DIST/$ARCHIVO" "$TMP/otra-vez/$ARCHIVO" || fallo "empaquetar dos veces no da el mismo archivo"

[ "$(wc -l <"$DIST/SHA256SUMS")" -eq 2 ] || fallo "SHA256SUMS no tiene una línea por objetivo"
# Lo que hay en la raíz del archivo: una carpeta con esto y nada más.
tar -tzf "$DIST/$ARCHIVO" | awk -F/ '{print $1 "/" $2}' | sort -u >"$TMP/raiz.txt"
printf '%s\n' "routerpanel-$VER-$OBJ/" "routerpanel-$VER-$OBJ/AVISO-LEGAL.md" \
	"routerpanel-$VER-$OBJ/LICENSE" "routerpanel-$VER-$OBJ/deploy" \
	"routerpanel-$VER-$OBJ/panel" "routerpanel-$VER-$OBJ/site" | sort >"$TMP/esperado.txt"
cmp -s "$TMP/raiz.txt" "$TMP/esperado.txt" || {
	diff "$TMP/esperado.txt" "$TMP/raiz.txt" >&2 || true
	fallo "el archivo no trae exactamente panel, site/, LICENSE, AVISO-LEGAL.md y deploy/"
}
# deploy/linux/: exactamente lo que hay en git (sin el fichero sin
# seguimiento ni el ignorado).
tar -tzf "$DIST/$ARCHIVO" | sed -n "s|^routerpanel-$VER-$OBJ/\(deploy/linux/..*[^/]\)\$|\1|p" |
	sort >"$TMP/linux-archivo.txt"
git -C "$CLON" ls-files deploy/linux | sort >"$TMP/linux-git.txt"
cmp -s "$TMP/linux-archivo.txt" "$TMP/linux-git.txt" || {
	diff "$TMP/linux-git.txt" "$TMP/linux-archivo.txt" >&2 || true
	fallo "deploy/linux/ del archivo no es exactamente el de git"
}
if grep -q 'sin-seguimiento\|prueba\.secret' "$TMP/linux-archivo.txt"; then
	fallo "el archivo trae ficheros sin seguimiento o ignorados de deploy/linux/"
fi

# --- 2. servir las releases con las rutas de GitHub -------------------------
SRV=$TMP/srv
mkdir -p "$SRV/latest/download" "$SRV/download/$VER"
cp "$DIST"/routerpanel-*.tar.gz "$DIST/SHA256SUMS" "$SRV/download/$VER/"
cp "$DIST/SHA256SUMS" "$SRV/latest/download/SHA256SUMS"
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
until curl -s -m 1 -o /dev/null --cacert "$TMP/cert.pem" "https://127.0.0.1:$PUERTO/"; do
	n=$((n + 1))
	if [ "$n" -ge 50 ] || ! kill -0 "$SERVIDOR" 2>/dev/null; then
		echo "FALLO: el servidor de la prueba no responde en 127.0.0.1:$PUERTO (¿puerto ocupado? RP_PRUEBA_PUERTO)" >&2
		exit 1
	fi
	sleep 0.1
done

# --- 3. el actualizador con un systemctl que arranca el panel de verdad ------
PREFIJO=$TMP/opt
DATOS=$TMP/datos
mkdir -p "$PREFIJO" "$DATOS"
cat >"$TMP/env" <<EOF
RP_LISTEN=127.0.0.1:$PUERTO_PANEL
RP_DB=$DATOS/panel.db
RP_HISTORY_KEY=$DATOS/history.key
RP_ORIGIN=http://localhost:$PUERTO_PANEL
RP_LOG=warn
RP_E2E_VIGILANCIA_S=1
EOF
# start: el entorno de la unidad desplegada (Environment=), luego el fichero
# de entorno (EnvironmentFile=) y ExecStart, en segundo plano; stop: lo para.
cat >"$TMP/systemctl" <<EOF
#!/bin/sh
set -eu
UNIDAD=$PREFIJO/current/deploy/linux/routerpanel.service
PID=$TMP/panel.pid
case \$1 in
start)
	[ -f "\$UNIDAD" ] || { echo "la versión desplegada no trae la unidad" >&2; exit 1; }
	sed -n 's|/opt/routerpanel|$PREFIJO|g; s/^Environment=/export /p' "\$UNIDAD" >$TMP/unidad.env
	EXEC=\$(sed -n 's|/opt/routerpanel|$PREFIJO|g; s/^ExecStart=//p' "\$UNIDAD")
	(
		. $TMP/unidad.env
		set -a
		. $TMP/env
		set +a
		cd $DATOS
		exec "\$EXEC"
	) >>$TMP/panel.log 2>&1 &
	echo \$! >"\$PID"
	;;
stop)
	if [ -f "\$PID" ]; then
		p=\$(cat "\$PID")
		kill "\$p" 2>/dev/null || true
		n=0
		while kill -0 "\$p" 2>/dev/null && [ "\$n" -lt 100 ]; do n=\$((n + 1)); sleep 0.1; done
		rm -f "\$PID"
	fi
	;;
esac
EOF
chmod 755 "$TMP/systemctl"

actualizar()
{
	CURL_CA_BUNDLE=$TMP/cert.pem \
		RP_ACTUALIZAR_CONF=$TMP/no-existe \
		RP_RELEASES_URL="https://127.0.0.1:$PUERTO" \
		RP_PREFIJO=$PREFIJO \
		RP_SYSTEMCTL=$TMP/systemctl \
		RP_ENV_PANEL=$TMP/env \
		RP_SALUD_ESPERA=30 \
		sh "$RAIZ/deploy/linux/actualizar-release.sh" "$@" >"$TMP/salida" 2>&1
}

if ! actualizar; then
	cat "$TMP/salida" "$TMP/panel.log" >&2 2>/dev/null || true
	fallo "el actualizador no despliega la release empaquetada"
fi

# --- 4. lo desplegado --------------------------------------------------------
[ "$(readlink "$PREFIJO/current" 2>/dev/null)" = "releases/$VER" ] || fallo "current no apunta a releases/$VER"
for f in panel LICENSE AVISO-LEGAL.md deploy/linux/README.md \
	deploy/linux/routerpanel.service deploy/linux/actualizar-release.sh; do
	[ -f "$PREFIJO/current/$f" ] || fallo "falta $f en la versión desplegada"
done
[ -x "$PREFIJO/current/panel" ] || fallo "panel no es ejecutable"
cmp -s "$PANEL" "$PREFIJO/current/panel" || fallo "el panel desplegado no es el empaquetado"
if ! curl -fsS -m 5 "http://127.0.0.1:$PUERTO_PANEL/salud" | grep -q '"ok":true'; then
	fallo "el panel desplegado no responde en /salud"
fi
# Release no tiene debug-assertions: la vigilancia corta de las pruebas no
# se activa (el binario de depuración lo anuncia por la salida de error).
if grep -q 'vigilancia de pruebas' "$TMP/panel.log" 2>/dev/null; then
	fallo "el panel release atiende RP_E2E_VIGILANCIA_S (¿debug-assertions en [profile.release]?)"
fi
# La hoja de estilos, con el hash en el nombre si la web lo lleva (hash.txt).
CSS=pkg/panel.css
H=$(sed -n 's/^css: *//p' "$PREFIJO/current/site/hash.txt" 2>/dev/null || true)
[ -z "$H" ] || CSS=pkg/panel.$H.css
[ -f "$PREFIJO/current/site/$CSS" ] || fallo "falta site/$CSS en la versión desplegada"
if ! curl -fsS -m 5 -o /dev/null "http://127.0.0.1:$PUERTO_PANEL/$CSS"; then
	fallo "el panel desplegado no sirve su web desde site/ (/$CSS)"
fi
if ! actualizar --comprobar || ! grep -q "al día: $VER" "$TMP/salida"; then
	cat "$TMP/salida" >&2
	fallo "--comprobar no dice que está al día"
fi

if [ "$fallos" -ne 0 ]; then
	echo "$fallos fallos" >&2
	exit 1
fi
echo "release-local: ok"
