#!/bin/sh
# Pruebas de la instalación en jail con la configuración de ejemplo, sin
# tocar ningún servidor:
#
# - sintaxis (sh -n) de todos los guiones de deploy/ y shellcheck si está;
# - install.sh --plantillas con config.example no deja ninguna @CLAVE@;
# - comprobar-publicacion no encuentra nada en deploy/ (sin listas locales);
# - una configuración sin ROUTER_IP falla y el mensaje nombra la clave;
# - sin jail de compilación: jail.conf solo trae la del panel, pf.conf no
#   tiene reglas de compilación y deja a la jail del panel llegar a su
#   propio puerto (/salud); no se genera resolv.conf ni ssh_config;
# - actualizador.conf lleva REPO_URL, CLAVE_DESPLIEGUE y HOSTS_CONOCIDOS (en
#   el host), y GITHUB_REPO y PRECOMPILADAS_MINISIGN, vacías con el ejemplo;
#   un GITHUB_REPO que no es «propietario/nombre» falla nombrando la clave;
#   REPO_HOST_CLAVE acaba en known_hosts;
# - una configuración de antes, con las claves de la jail de compilación
#   (JAIL_COMPILAR, TRABAJOS_COMPILACION, PRECOMPILADAS_ESPERA…), sigue
#   valiendo y no las pasa al actualizador.
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
# Los de deploy/cross/ son de bash (corren en el PC de compilación, Linux).
for f in "$DEPLOY"/*/*.sh "$DEPLOY"/freebsd-jail/routerpanel.rc; do
	case $f in
	"$DEPLOY"/cross/*) bash -n "$f" || fallo "bash -n: ${f#"$RAIZ"/}" ;;
	*) sh -n "$f" || fallo "sh -n: ${f#"$RAIZ"/}" ;;
	esac
done
if command -v shellcheck >/dev/null 2>&1; then
	set --
	for f in "$DEPLOY"/*/*.sh; do
		case $f in
		"$DEPLOY"/cross/*) ;;
		*) set -- "$@" "$f" ;;
		esac
	done
	# Solo avisos y errores: las notas de estilo («info») no hacen fallar.
	shellcheck -S warning -s sh "$@" "$DEPLOY"/freebsd-jail/routerpanel.rc ||
		fallo "shellcheck"
	shellcheck -S warning -s bash "$DEPLOY"/cross/*.sh || fallo "shellcheck de deploy/cross"
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
	for f in jail.conf pf.conf rc.conf actualizador.conf; do
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

# --- sin jail de compilación -----------------------------------------------
P=$TMP/plantillas
[ "$(grep -c '^[^ #].* {$' "$P/jail.conf" 2>/dev/null)" = 1 ] && grep -q '^routerpanel {' "$P/jail.conf" ||
	fallo "jail.conf no trae solo la jail del panel: $(cat "$P/jail.conf")"
if grep -qi 'build\|compil\|192\.0\.2\.21' "$P/jail.conf" "$P/pf.conf"; then
	fallo "jail.conf o pf.conf hablan todavía de la jail de compilación"
fi
grep -q '^pass quick proto tcp from 192.0.2.20 to 192.0.2.20 port 3000$' "$P/pf.conf" ||
	fallo "pf.conf no deja a la jail del panel llegar a su /salud"
for f in resolv.conf ssh_config; do
	[ ! -e "$P/$f" ] || fallo "--plantillas genera todavía $f"
done

# --- actualizador.conf -------------------------------------------------------
for k in GITHUB_REPO PRECOMPILADAS_MINISIGN; do
	grep -q "^$k=''" "$P/actualizador.conf" 2>/dev/null ||
		fallo "actualizador.conf no lleva $k vacía con el ejemplo"
done
grep -q "^REPO_URL='https://git.example/routerpanel.git'" "$P/actualizador.conf" ||
	fallo "actualizador.conf no lleva REPO_URL"
grep -q "^CLAVE_DESPLIEGUE='/usr/local/etc/routerpanel-actualizar/id_ed25519'" "$P/actualizador.conf" &&
	grep -q "^HOSTS_CONOCIDOS='/usr/local/etc/routerpanel-actualizar/known_hosts'" "$P/actualizador.conf" ||
	fallo "actualizador.conf no lleva CLAVE_DESPLIEGUE y HOSTS_CONOCIDOS del host"
[ ! -e "$P/known_hosts" ] || fallo "sin REPO_HOST_CLAVE se genera known_hosts"
{
	cat "$DEPLOY/config.example"
	echo 'GITHUB_REPO=dueno/movistar-RTF8115VW-panel'
	echo "REPO_HOST_CLAVE='[192.0.2.30]:2222 clave-de-prueba'"
} >"$TMP/github.conf"
if sh "$DEPLOY/freebsd-jail/install.sh" --config "$TMP/github.conf" \
	--plantillas "$TMP/github" >"$TMP/salida" 2>&1; then
	grep -q "^GITHUB_REPO='dueno/movistar-RTF8115VW-panel'" "$TMP/github/actualizador.conf" ||
		fallo "GITHUB_REPO no llega a actualizador.conf"
	[ "$(cat "$TMP/github/known_hosts" 2>/dev/null)" = '[192.0.2.30]:2222 clave-de-prueba' ] ||
		fallo "REPO_HOST_CLAVE no llega a known_hosts"
else
	cat "$TMP/salida" >&2
	fallo "install.sh --plantillas con GITHUB_REPO"
fi
for malo in dueno 'dueno/a/b' '/x' 'dueno/' 'du eno/x' '-x/y' 'x/..'; do
	{ cat "$DEPLOY/config.example"; echo "GITHUB_REPO='$malo'"; } >"$TMP/github-malo.conf"
	if sh "$DEPLOY/freebsd-jail/install.sh" --config "$TMP/github-malo.conf" \
		--plantillas "$TMP/github-malo" >"$TMP/salida" 2>&1; then
		fallo "GITHUB_REPO=$malo no falla"
	elif ! grep -q GITHUB_REPO "$TMP/salida"; then
		cat "$TMP/salida" >&2
		fallo "el error de GITHUB_REPO=$malo no nombra la clave"
	fi
done

# --- configuración de antes ----------------------------------------------------
{
	cat "$DEPLOY/config.example"
	echo 'JAIL_COMPILAR=routerpanel-build'
	echo 'JAIL_COMPILAR_IP=192.0.2.21'
	echo 'UID_COMPILAR=3160'
	echo 'RED_LAN=192.0.2.0/24'
	echo 'DNS_IP=192.0.2.10'
	echo 'TRABAJOS_COMPILACION=3'
	echo 'PRECOMPILADAS_ESPERA=20'
} >"$TMP/antes.conf"
if sh "$DEPLOY/freebsd-jail/install.sh" --config "$TMP/antes.conf" \
	--plantillas "$TMP/antes" >"$TMP/salida" 2>&1; then
	if grep -q 'COMPILAR\|ESPERA\|TRABAJOS' "$TMP/antes/actualizador.conf"; then
		fallo "una configuración de antes pasa claves de compilación al actualizador"
	fi
	cmp -s "$TMP/antes/pf.conf" "$P/pf.conf" && cmp -s "$TMP/antes/jail.conf" "$P/jail.conf" ||
		fallo "una configuración de antes cambia jail.conf o pf.conf"
else
	cat "$TMP/salida" >&2
	fallo "install.sh --plantillas con una configuración de antes"
fi

# --- plantillas en el servidor: temporal de root -----------------------------
# servidor() y pf_cargar() corren como root y no pueden probarse enteras sin
# root; se prueba plantillas_de_root() extraída del guion (con generar_plantillas
# y die de mentira) y que las dos ramas la usan y no escriben en DEPLOY_DIR.
INSTALL=$DEPLOY/freebsd-jail/install.sh
awk '/^plantillas_de_root\(\)/ {d=1} d {print} d && /^}/ {exit}' "$INSTALL" >"$TMP/pdr.sh"
[ -s "$TMP/pdr.sh" ] || fallo "install.sh no define plantillas_de_root"
# El temporal es siempre /tmp/routerpanel-plantillas.*, aunque TMPDIR apunte
# a otro sitio (aquí, dentro de deploy/): en producción no hay variable que
# desvíe la ruta. Cada modo termina de una forma y el temporal debe borrarse.
cat >"$TMP/drv.sh" <<'DRV'
. "$1"
die() { echo "$2" >&2; exit 1; }
set -eu
PLANTILLAS=
generar_plantillas() { echo secreto >"$1/jail.conf"; }
modo=$2
[ "$modo" != mktemp ] || mktemp() { return 1; }
plantillas_de_root
printf '%s\n' "$PLANTILLAS" >"$3"
ls -ld "$PLANTILLAS" | cut -c1-10 >"$3.modo"
case $modo in
ok) ;;
falla) exit 3 ;;
diez) exit 10 ;;
doble) plantillas_de_root ;;
hup) kill -HUP $$ ;;
int) kill -INT $$ ;;
term) kill -TERM $$ ;;
esac
DRV
sed -i 's/^die() .*/die() { echo "ERROR: $*" >\&2; exit 1; }/' "$TMP/drv.sh"
# Una señal que llega ya ignorada (con nohup, o lanzando las pruebas en
# segundo plano desde un sh no interactivo, que ignora SIGINT) no se puede
# volver a capturar en sh: el trap no hace nada y el modo de esa señal
# fallaría sin que el guion tenga culpa. Se comprueba antes y se salta.
senal_capturable()
{
	rc=0
	sh -c "trap 'exit 7' $1; kill -$1 \$\$; exit 0" >/dev/null 2>&1 || rc=$?
	[ "$rc" = 7 ]
}
for par in ok:0 falla:3 diez:10 doble:1 hup:1 int:1 term:1 mktemp:1; do
	modo=${par%:*}
	esperado=${par#*:}
	case $modo in
	hup | int | term)
		senal=$(printf '%s' "$modo" | tr '[:lower:]' '[:upper:]')
		if ! senal_capturable "$senal"; then
			echo "aviso: SIG$senal llega ignorada (nohup o segundo plano); no se prueba plantillas_de_root ($modo)" >&2
			continue
		fi
		;;
	esac
	rm -f "$TMP/ruta" "$TMP/ruta.modo"
	rc=0
	TMPDIR=$DEPLOY sh "$TMP/drv.sh" "$TMP/pdr.sh" "$modo" "$TMP/ruta" >/dev/null 2>&1 || rc=$?
	[ "$rc" = "$esperado" ] || fallo "plantillas_de_root ($modo): sale con $rc, no con $esperado"
	if [ "$modo" = mktemp ]; then
		[ ! -e "$TMP/ruta" ] || fallo "plantillas_de_root (mktemp falla): sigue adelante"
		continue
	fi
	ruta=$(cat "$TMP/ruta" 2>/dev/null || true)
	[ -n "$ruta" ] || { fallo "plantillas_de_root ($modo) no fija PLANTILLAS"; continue; }
	[ ! -e "$ruta" ] || { fallo "plantillas_de_root ($modo): el temporal no se borra al salir"; rm -rf "$ruta"; }
	[ "$(cat "$TMP/ruta.modo")" = drwx------ ] || fallo "plantillas_de_root ($modo): el temporal no es 0700"
	case $ruta in /tmp/routerpanel-plantillas.*) ;; *) fallo "plantillas_de_root ($modo): ruta fuera de /tmp ($ruta)" ;; esac
done
for f in servidor pf_cargar; do
	cuerpo=$(awk -v f="$f" '$0 == f "()" {d=1; next} d && /^}/ {exit} d {print}' "$INSTALL")
	case $cuerpo in *plantillas_de_root*) ;; *) fallo "$f no usa plantillas_de_root" ;; esac
	case $cuerpo in *'rm -rf "$PLANTILLAS"'* | *DEPLOY_DIR*) fallo "$f toca la carpeta de subida" ;; esac
done
grep -q '^PLANTILLAS=\$DEPLOY_DIR' "$INSTALL" && fallo "PLANTILLAS apunta dentro de DEPLOY_DIR"

if [ "$fallos" -ne 0 ]; then
	echo "$fallos fallos" >&2
	exit 1
fi
echo "plantillas: ok"
