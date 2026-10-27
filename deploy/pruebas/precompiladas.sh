#!/bin/sh
# Pruebas de las versiones precompiladas para FreeBSD (deploy/cross/,
# deploy/lib/precompiladas.sh, deploy/freebsd-jail/recibir-precompilada.sh y
# la parte del actualizador), sin servidor, sin PC de compilación y sin tocar
# el sistema: todo bajo una carpeta temporal.
#
# 1. Nombres: archivo_precompilada, version_precompilable, fichero_de_buzon.
# 2. ultima_version elige la misma etiqueta que traer_ultima_etiqueta
#    (deploy/lib/git.sh) en un clon de verdad.
# 3. decidir_precompilada: esperar la primera vez y mientras no pase la
#    espera, compilar después, instalar si ya llegó; una etiqueta nueva vuelve
#    a empezar la cuenta.
# 4. preparar_precompilada acepta un archivo bueno (deja solo panel, site/ y
#    COMMIT) y rechaza, sin dejar nada: sha256 que no cuadra, otro commit, un
#    binario de Linux, carpeta con otro nombre o algo más al lado, un enlace
#    simbólico dentro, sin site/, un enlace en el buzón y, con minisign (si
#    está instalado), una firma mala. precompilada_llegada no da por llegada
#    una versión sin SHA256SUMS ni, con clave, sin firma.
# 5. recibir-precompilada.sh guarda lo admitido y rechaza versiones y
#    nombres no válidos, órdenes con más argumentos, ficheros vacíos o
#    demasiado grandes (sin dejar restos); conserva 3 versiones.
# 6. actualizar.sh de verdad, con jexec, service, /salud, sockstat y ps de
#    mentira y cargo que finge compilar:
#    a. sin PRECOMPILADAS_ESPERA hace lo de siempre (compila y despliega) y
#       no mira el buzón, aunque haya en él una versión buena;
#    b. configurado y sin nada en el buzón, espera: no compila ni despliega;
#    c. pasada la espera, compila;
#    d. con la versión en el buzón, la instala sin compilar y vacía el buzón;
#    e. con una versión de otro commit, la rechaza y compila.
# 7. Lo que corre fuera del servidor se niega antes de compilar nada:
#    lanzar.sh con algo que no es una etiqueta v*, con la copia de trabajo
#    sucia o sin PRECOMPILADAS_BUZON_SSH; compilar-freebsd.sh --release con
#    una rama (aunque se llame como una versión) y --prueba con --entregar;
#    entregar.sh con una versión de --prueba. Con un PC que no responde,
#    lanzar.sh sale con 3 y lo dice.
# 8. sh -n de todo y shellcheck (si está).
#
# Uso: deploy/pruebas/precompiladas.sh (desde cualquier sitio; necesita git,
# tar, gzip, file y sha256sum). RP_PRUEBA_ACTUALIZAR cambia el actualizador
# que se prueba en 6 (para comparar con otra versión a mano).

set -eu

DEPLOY=$(cd "$(dirname "$0")/.." && pwd)
LIB=$DEPLOY/lib/precompiladas.sh
RECIBIR=$DEPLOY/freebsd-jail/recibir-precompilada.sh
ACTUALIZAR=${RP_PRUEBA_ACTUALIZAR:-$DEPLOY/freebsd-jail/actualizar.sh}
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
trap 'exit 143' INT TERM

HOME=$TMP/casa
mkdir -p "$HOME"
GIT_CONFIG_NOSYSTEM=1
GIT_AUTHOR_NAME=prueba
GIT_AUTHOR_EMAIL=prueba
GIT_COMMITTER_NAME=prueba
GIT_COMMITTER_EMAIL=prueba
export HOME GIT_CONFIG_NOSYSTEM GIT_AUTHOR_NAME GIT_AUTHOR_EMAIL GIT_COMMITTER_NAME GIT_COMMITTER_EMAIL

for orden in git tar gzip file sha256sum; do
	command -v "$orden" >/dev/null 2>&1 || {
		echo "FALLO: falta $orden" >&2
		exit 1
	}
done

fallos=0
fallo()
{
	echo "FALLO: $*" >&2
	fallos=$((fallos + 1))
}

# shellcheck source=../lib/precompiladas.sh
. "$LIB"
# shellcheck source=../lib/git.sh
. "$DEPLOY/lib/git.sh"

# Cabecera ELF mínima de x86-64 con el ABI $1 (9: FreeBSD, 0: Linux/SYSV);
# file la reconoce igual que un binario de verdad.
elf()
{
	printf '\177ELF\002\001\001'
	printf "\\$(printf '%03o' "$1")"
	printf '\000\000\000\000\000\000\000\000\002\000\076\000\001\000\000\000'
	head -c 40 /dev/zero
}

# Construye en $1 (carpeta del buzón para la versión $2) un archivo con
# COMMIT $3. Variantes ($4): buena, linux, otro-nombre, extra, enlace,
# sin-site, sha-malo.
hacer_archivo()
{
	_dir=$1
	_ver=$2
	_commit=$3
	_variante=${4:-buena}
	_arch=$(archivo_precompilada "$_ver")
	_raiz=${_arch%.tar.gz}
	_t=$TMP/hacer
	rm -rf "$_t"
	if [ "$_variante" = otro-nombre ]; then _raiz=routerpanel-otra; fi
	mkdir -p "$_t/$_raiz/site/pkg" "$_dir"
	if [ "$_variante" = linux ]; then
		elf 0 >"$_t/$_raiz/panel"
	else
		elf 9 >"$_t/$_raiz/panel"
	fi
	echo 'js: abc' >"$_t/$_raiz/site/hash.txt"
	echo js >"$_t/$_raiz/site/pkg/panel.abc.js"
	echo "$_commit" >"$_t/$_raiz/COMMIT"
	echo licencia >"$_t/$_raiz/LICENSE"
	case $_variante in
	extra) echo x >"$_t/suelto" ;;
	enlace) ln -s /etc/passwd "$_t/$_raiz/site/pkg/enlace" ;;
	sin-site) rm -rf "$_t/$_raiz/site" ;;
	esac
	(cd "$_t" && tar -czf "$_dir/$_arch" -- *)
	(cd "$_dir" && sha256sum -- "$_arch" >SHA256SUMS)
	if [ "$_variante" = sha-malo ]; then
		echo basura >>"$_dir/$_arch"
	fi
}

# --- 1. nombres ---------------------------------------------------------------
[ "$(archivo_precompilada v1.2.3)" = routerpanel-v1.2.3-x86_64-unknown-freebsd.tar.gz ] ||
	fallo "archivo_precompilada v1.2.3: $(archivo_precompilada v1.2.3)"
for v in v1.2.3 v1.0.0-rc1 v0.9.10; do
	version_precompilable "$v" || fallo "version_precompilable rechaza $v"
done
largo=v$(printf '%0100d' 1)
for v in '' 1.2.3 prueba-abc v1/../x 'v1 2' va.b ../v1 "$largo" 'v1
x'; do
	if version_precompilable "$v"; then fallo "version_precompilable acepta «$v»"; fi
done
for f in routerpanel-v1.2.3-x86_64-unknown-freebsd.tar.gz SHA256SUMS SHA256SUMS.minisig; do
	fichero_de_buzon v1.2.3 "$f" || fallo "fichero_de_buzon rechaza $f"
done
for f in routerpanel-v1.2.4-x86_64-unknown-freebsd.tar.gz routerpanel-v1.2.3-x86_64-unknown-linux-gnu.tar.gz \
	panel ../SHA256SUMS .SHA256SUMS; do
	if fichero_de_buzon v1.2.3 "$f"; then fallo "fichero_de_buzon acepta $f"; fi
done

# --- 2. la misma última versión que el servidor ---------------------------------
git init -q -b main "$TMP/versiones"
echo uno >"$TMP/versiones/f"
git -C "$TMP/versiones" add f
git -C "$TMP/versiones" commit -q -m uno
for t in v0.9.9 v0.9.10 v0.10.0 v1.0.0 v1.0.0-rc1 otra; do
	git -C "$TMP/versiones" tag "$t"
done
git clone -q "$TMP/versiones" "$TMP/versiones-clon"
servidor=$(traer_ultima_etiqueta "$TMP/versiones-clon")
pc=$(git -C "$TMP/versiones" tag -l | ultima_version)
[ "$pc" = "$servidor" ] || fallo "ultima_version da $pc y el servidor elige $servidor"
[ "$(printf 'v0.9.9\nv0.9.10\n' | ultima_version)" = v0.9.10 ] || fallo "ultima_version no ordena v0.9.10 después de v0.9.9"
[ -z "$(printf 'otra\n' | ultima_version)" ] || fallo "ultima_version da algo sin etiquetas v*"

# --- 3. cuándo esperar ----------------------------------------------------------
E=$TMP/estado
[ "$(decidir_precompilada "$E" v1.0.0 1000 20 no)" = esperar ] || fallo "primera vez: no espera"
[ "$(cat "$E")" = "v1.0.0 1000" ] || fallo "primera vez: no apunta la hora ($(cat "$E"))"
[ "$(decidir_precompilada "$E" v1.0.0 2199 20 no)" = esperar ] || fallo "a los 19 min 59 s: no espera"
[ "$(decidir_precompilada "$E" v1.0.0 2200 20 no)" = compilar ] || fallo "a los 20 min: no compila"
[ "$(cat "$E")" = "v1.0.0 1000" ] || fallo "la hora de la primera vez cambia"
[ "$(decidir_precompilada "$E" v1.0.0 9999 20 si)" = instalar ] || fallo "llegada: no instala"
[ "$(decidir_precompilada "$E" v1.0.1 5000 20 no)" = esperar ] || fallo "etiqueta nueva: no vuelve a esperar"
[ "$(cat "$E")" = "v1.0.1 5000" ] || fallo "etiqueta nueva: no apunta su hora"
[ "$(decidir_precompilada "$E" v1.0.2 5000 0 no)" = compilar ] || fallo "espera 0: no compila enseguida"
[ "$(decidir_precompilada "$E" v1.0.2 5000 0 si)" = instalar ] || fallo "espera 0 y llegada: no instala"

# --- 4. comprobar un archivo ----------------------------------------------------
B=$TMP/buzon
# Commits de 40 cifras hex, generados (sin literales largos en el repositorio).
C=$(printf "%040d" 0 | tr 0 a)
OTRO=$(printf "%040d" 0 | tr 0 b)
V=v1.2.3

precompilada_llegada "$B" "$V" '' && fallo "llegada sin nada en el buzón"
hacer_archivo "$B/$V" "$V" "$C"
rm -f "$B/$V/SHA256SUMS"
precompilada_llegada "$B" "$V" '' && fallo "llegada sin SHA256SUMS"
hacer_archivo "$B/$V" "$V" "$C"
precompilada_llegada "$B" "$V" '' || fallo "no da por llegada una versión completa"
precompilada_llegada "$B" "$V" 'RWQclave' && fallo "llegada sin firma, con clave"

if preparar_precompilada "$B" "$V" "$C" '' "$TMP/bueno" 2>"$TMP/err"; then
	[ "$(find "$TMP/bueno" -mindepth 1 -maxdepth 1 -printf '%f\n' | sort | tr '\n' ' ')" = "COMMIT panel site " ] ||
		fallo "el archivo bueno no queda solo con panel, site/ y COMMIT"
	[ "$(cat "$TMP/bueno/COMMIT")" = "$C" ] || fallo "COMMIT no es el de la etiqueta"
else
	fallo "rechaza el archivo bueno: $(cat "$TMP/err")"
fi

# rechazar NOMBRE VARIANTE COMMIT_ESPERADO MOTIVO_ESPERADO
rechazar()
{
	rm -rf "$B/$V" "$TMP/malo"
	hacer_archivo "$B/$V" "$V" "$C" "$2"
	if preparar_precompilada "$B" "$V" "$3" '' "$TMP/malo" 2>"$TMP/err"; then
		fallo "$1: lo acepta"
	elif ! grep -q "$4" "$TMP/err"; then
		fallo "$1: el motivo no dice «$4»: $(cat "$TMP/err")"
	fi
	[ ! -e "$TMP/malo" ] || fallo "$1: deja la carpeta de destino"
}
rechazar "sha256 alterado" sha-malo "$C" sha256
rechazar "otro commit" buena "$OTRO" "otro commit"
rechazar "binario de Linux" linux "$C" FreeBSD
rechazar "carpeta con otro nombre" otro-nombre "$C" "solo la carpeta"
rechazar "algo al lado de la carpeta" extra "$C" "solo la carpeta"
rechazar "enlace simbólico dentro" enlace "$C" "ni directorio"
rechazar "sin site/" sin-site "$C" "faltan"
rechazar "commit no válido" buena "abc" "commit no válido"

rm -rf "$B/$V"
hacer_archivo "$TMP/fuera" "$V" "$C"
mkdir -p "$B/$V"
ln -s "$TMP/fuera/$(archivo_precompilada "$V")" "$B/$V/$(archivo_precompilada "$V")"
cp "$TMP/fuera/SHA256SUMS" "$B/$V/"
precompilada_llegada "$B" "$V" '' && fallo "da por llegada un archivo que es un enlace"
if preparar_precompilada "$B" "$V" "$C" '' "$TMP/malo" 2>/dev/null; then
	fallo "acepta un archivo del buzón que es un enlace"
fi
rm -rf "$B/$V" "$TMP/malo"

if command -v minisign >/dev/null 2>&1; then
	minisign -G -W -f -p "$TMP/clave.pub" -s "$TMP/clave.sec" >/dev/null
	minisign -G -W -f -p "$TMP/otra.pub" -s "$TMP/otra.sec" >/dev/null
	PUB=$(tail -n 1 "$TMP/clave.pub")
	hacer_archivo "$B/$V" "$V" "$C"
	minisign -S -s "$TMP/clave.sec" -m "$B/$V/SHA256SUMS" -x "$B/$V/SHA256SUMS.minisig" >/dev/null
	preparar_precompilada "$B" "$V" "$C" "$PUB" "$TMP/firmado" 2>"$TMP/err" ||
		fallo "rechaza un archivo bien firmado: $(cat "$TMP/err")"
	minisign -S -s "$TMP/otra.sec" -m "$B/$V/SHA256SUMS" -x "$B/$V/SHA256SUMS.minisig" >/dev/null
	if preparar_precompilada "$B" "$V" "$C" "$PUB" "$TMP/malfirmado" 2>"$TMP/err"; then
		fallo "acepta una firma de otra clave"
	fi
	rm -rf "$B/$V"
else
	echo "aviso: no hay minisign; no se prueban las firmas" >&2
fi

# --- 5. el receptor del buzón ---------------------------------------------------
R=$TMP/receptor
mkdir -p "$R"
recibir()
{
	SSH_ORIGINAL_COMMAND=$1 RP_BUZON=$R RP_BUZON_LIB=$LIB RP_BUZON_MAXIMO=${MAXIMO:-1000} \
		sh "$RECIBIR" >"$TMP/salida" 2>&1
}
ARCH=$(archivo_precompilada "$V")
echo contenido | recibir "recibir $V $ARCH" || fallo "el receptor rechaza $ARCH: $(cat "$TMP/salida")"
[ "$(cat "$R/$V/$ARCH" 2>/dev/null)" = contenido ] || fallo "el receptor no guarda $ARCH"
echo sumas | recibir "recibir $V SHA256SUMS" || fallo "el receptor rechaza SHA256SUMS"
for orden in "recibir ../x SHA256SUMS" "recibir $V panel" "recibir $V ../SHA256SUMS" \
	"recibir $V SHA256SUMS extra" "borrar $V SHA256SUMS" "recibir $V" \
	"recibir v1.0.0;id SHA256SUMS" "recibir \$(id) SHA256SUMS" "recibir * *"; do
	if echo x | recibir "$orden"; then fallo "el receptor acepta «$orden»"; fi
done
if printf '' | recibir "recibir v2.0.0 SHA256SUMS"; then fallo "el receptor acepta un fichero vacío"; fi
if head -c 2000 /dev/zero | recibir "recibir v2.0.0 SHA256SUMS"; then
	fallo "el receptor acepta un fichero de más de RP_BUZON_MAXIMO"
fi
[ -z "$(find "$R" -name '.*' -type f)" ] || fallo "el receptor deja temporales"
[ ! -e "$R/v2.0.0/SHA256SUMS" ] || fallo "el receptor guarda un fichero rechazado"
[ ! -e "$R/x" ] && [ ! -e "$TMP/x" ] || fallo "el receptor escribe fuera de su versión"
for v in v3.0.0 v3.0.1 v3.0.2 v3.0.3; do
	sleep 1
	echo s | recibir "recibir $v SHA256SUMS" || fallo "el receptor rechaza $v"
done
quedan=$(find "$R" -mindepth 1 -maxdepth 1 -type d -name 'v3*' | wc -l)
[ "$quedan" -eq 3 ] && [ ! -e "$R/v3.0.0" ] && [ ! -e "$R/$V" ] ||
	fallo "el receptor no deja solo las 3 versiones más recientes ($(ls "$R" | tr '\n' ' '))"

# --- 6. el actualizador ----------------------------------------------------------
# Un «servidor» bajo $H: el actualizador con sus rutas cambiadas por sed (y
# las órdenes de BSD por las de GNU), el clon de la jail de compilación, el
# remoto con la etiqueta, las releases y el «current» de la jail del panel.
GIT_ABS=$(command -v git)
preparar_servidor()
{
	H=$TMP/servidor
	rm -rf "$H"
	mkdir -p "$H/bin" "$H/casa/.cargo/bin" "$H/libexec" "$H/releases" "$H/db" \
		"$H/state" "$H/log" "$H/run" "$H/buzon/entrada" "$H/jails/routerpanel/usr/local/routerpanel"
	ln -s "$H/releases" "$H/jails/routerpanel/usr/local/routerpanel/releases"
	cp "$DEPLOY/lib/git.sh" "$LIB" "$H/libexec/"
	sed -e "s|/usr/sbin/jexec|$H/bin/jexec|g" \
		-e "s|^GIT=/usr/local/bin/git|GIT=$GIT_ABS|" \
		-e "s|/usr/local/etc/routerpanel-actualizar.conf|$H/actualizar.conf|" \
		-e "s|/usr/local/libexec/routerpanel-actualizar|$H/libexec|g" \
		-e "s|^SRC=/home/rpbuild/panel|SRC=$H/clon|" \
		-e "s|^RELEASES=/usr/local/routerpanel/releases|RELEASES=$H/releases|" \
		-e "s|^DB_DIR=/var/db/routerpanel\$|DB_DIR=$H/db|" \
		-e "s|^STATE=/var/db/routerpanel-actualizar|STATE=$H/state|" \
		-e "s|^BUZON=/var/db/routerpanel-buzon/entrada|BUZON=$H/buzon/entrada|" \
		-e "s|/var/log/|$H/log/|g" -e "s|/var/run/|$H/run/|g" \
		-e "s|/usr/sbin/sysrc|$H/bin/sysrc|" -e "s|/usr/sbin/jls|$H/bin/jls|" \
		-e 's|ln -sfh|ln -sfn|g' -e 's|mv -fh|mv -fT|g' \
		-e 's|chown -R root:wheel|true|' \
		"$ACTUALIZAR" >"$H/actualizar.sh"
	cat >"$H/bin/jexec" <<-EOF
		#!/bin/sh
		while :; do
			case \$1 in
			-l) shift ;;
			-U) shift 2 ;;
			*) break ;;
			esac
		done
		shift
		case \$1 in
		/usr/sbin/service) echo "service \$3" >>"$H/llamadas"; exit 0 ;;
		/usr/bin/fetch) echo '{"ok":true}'; exit 0 ;;
		esac
		exec "\$@"
	EOF
	cat >"$H/bin/sockstat" <<-'EOF'
		#!/bin/sh
		echo "routerpanel panel 4242 3 tcp4 *:3000 *:*"
	EOF
	cat >"$H/bin/ps" <<-EOF
		#!/bin/sh
		echo "/usr/local/routerpanel/releases/\$(basename "\$(readlink "$H/jails/routerpanel/usr/local/routerpanel/current")")/panel"
	EOF
	printf '#!/bin/sh\necho 127.0.0.1:3000\n' >"$H/bin/sysrc"
	printf '#!/bin/sh\necho 127.0.0.1\n' >"$H/bin/jls"
	cat >"$H/casa/.cargo/bin/cargo" <<-EOF
		#!/bin/sh
		echo "cargo \$*" >>"$H/llamadas"
		if [ "\$*" = "leptos build --release --precompress" ]; then
			mkdir -p target/release target/site/pkg
			echo "panel compilado aquí" >target/release/panel
			echo js >target/site/pkg/panel.js
		fi
	EOF
	printf '#!/bin/sh\necho "wasm-bindgen 0.2.129"\n' >"$H/casa/.cargo/bin/wasm-bindgen"
	chmod 755 "$H"/bin/* "$H"/casa/.cargo/bin/*

	git init -q -b main "$H/origen"
	printf '[[package]]\nname = "wasm-bindgen"\nversion = "0.2.129"\n' >"$H/origen/Cargo.lock"
	git -C "$H/origen" add Cargo.lock
	git -C "$H/origen" commit -q -m uno
	git -C "$H/origen" tag -a -m v1.0.0 v1.0.0
	git clone -q "$H/origen" "$H/clon"
	COMMIT_ETIQUETA=$(git -C "$H/origen" rev-parse 'v1.0.0^{commit}')
	{
		echo "JAILS_RAIZ='$H/jails'"
		echo "REPO_URL='$H/origen'"
		echo "TRABAJOS_COMPILACION=''"
	} >"$H/actualizar.conf"
}

# Ejecuta el actualizador; su salida, en $H/salida.
actualizar()
{
	ROUTERPANEL_LOCKED=1 HOME=$H/casa PATH="$H/bin:$PATH" sh "$H/actualizar.sh" >"$H/salida" 2>&1
}

compilo()
{
	grep -q '^cargo leptos build' "$H/llamadas" 2>/dev/null
}

desplegada()
{
	basename "$(readlink "$H/jails/routerpanel/usr/local/routerpanel/current" 2>/dev/null)" 2>/dev/null || true
}

# a. Sin configurar: lo de siempre, aunque haya una versión buena en el buzón.
preparar_servidor
hacer_archivo "$H/buzon/entrada/v1.0.0" v1.0.0 "$COMMIT_ETIQUETA"
actualizar || fallo "a: el actualizador falla: $(cat "$H/salida" "$H/log/routerpanel-update.log")"
compilo || fallo "a: sin configurar no compila"
[ "$(desplegada)" = v1.0.0 ] || fallo "a: no despliega v1.0.0"
[ "$(cat "$H/releases/v1.0.0/panel" 2>/dev/null)" = "panel compilado aquí" ] ||
	fallo "a: lo desplegado no es lo compilado aquí"
[ "$(cat "$H/releases/v1.0.0/COMMIT" 2>/dev/null)" = "$COMMIT_ETIQUETA" ] || fallo "a: COMMIT no es el de la etiqueta"
[ -f "$H/buzon/entrada/v1.0.0/SHA256SUMS" ] || fallo "a: sin configurar toca el buzón"
[ ! -e "$H/state/precompilada" ] || fallo "a: sin configurar apunta estado de precompiladas"
if grep -qi precompilada "$H/log/routerpanel-update.log"; then fallo "a: sin configurar habla de precompiladas"; fi
grep -q 'service start' "$H/llamadas" || fallo "a: no arranca el servicio"

# b. Configurado y sin nada en el buzón: espera.
preparar_servidor
echo "PRECOMPILADAS_ESPERA='20'" >>"$H/actualizar.conf"
actualizar || fallo "b: el actualizador falla: $(cat "$H/salida")"
if compilo; then fallo "b: compila sin esperar"; fi
[ -z "$(desplegada)" ] || fallo "b: despliega algo mientras espera"
grep -q 'esperando la versión precompilada de v1.0.0' "$H/log/routerpanel-update.log" || fallo "b: no dice que espera"
if grep -q service "$H/llamadas" 2>/dev/null; then fallo "b: toca el servicio mientras espera"; fi

# c. Pasada la espera: compila.
echo "v1.0.0 1" >"$H/state/precompilada"
actualizar || fallo "c: el actualizador falla: $(cat "$H/salida")"
compilo || fallo "c: pasada la espera no compila"
[ "$(desplegada)" = v1.0.0 ] || fallo "c: no despliega v1.0.0"
grep -q 'no ha llegado la versión precompilada de v1.0.0' "$H/log/routerpanel-update.log" || fallo "c: no dice por qué compila"

# d. Con la versión en el buzón: la instala sin compilar.
preparar_servidor
echo "PRECOMPILADAS_ESPERA='20'" >>"$H/actualizar.conf"
hacer_archivo "$H/buzon/entrada/v1.0.0" v1.0.0 "$COMMIT_ETIQUETA"
actualizar || fallo "d: el actualizador falla: $(cat "$H/salida" "$H/log/routerpanel-update.log")"
if compilo; then fallo "d: compila teniendo la precompilada"; fi
[ "$(desplegada)" = v1.0.0 ] || fallo "d: no despliega v1.0.0"
file -b "$H/releases/v1.0.0/panel" 2>/dev/null | grep -q FreeBSD || fallo "d: lo desplegado no es la precompilada"
[ -d "$H/releases/v1.0.0/site" ] && [ ! -e "$H/releases/v1.0.0/LICENSE" ] ||
	fallo "d: la versión no tiene solo panel, site/ y COMMIT"
[ "$(stat -c %a "$H/releases/v1.0.0/panel")" = 755 ] || fallo "d: el binario no queda 755"
[ ! -e "$H/buzon/entrada/v1.0.0" ] || fallo "d: no vacía el buzón"
grep -q 'instalada la versión precompilada de v1.0.0' "$H/log/routerpanel-update.log" || fallo "d: no lo registra"

# e. Con una versión de otro commit: la rechaza y compila.
preparar_servidor
echo "PRECOMPILADAS_ESPERA='20'" >>"$H/actualizar.conf"
hacer_archivo "$H/buzon/entrada/v1.0.0" v1.0.0 "$OTRO"
actualizar || fallo "e: el actualizador falla: $(cat "$H/salida")"
compilo || fallo "e: con una precompilada mala no compila"
[ "$(cat "$H/releases/v1.0.0/panel" 2>/dev/null)" = "panel compilado aquí" ] || fallo "e: no despliega lo compilado"
grep -q 'rechazada: el archivo es de otro commit' "$H/log/routerpanel-update.log" || fallo "e: no dice por qué la rechaza"
[ ! -e "$H/buzon/entrada/v1.0.0" ] || fallo "e: deja la precompilada mala en el buzón"

# --- 7. negativas fuera del servidor ---------------------------------------------
L=$TMP/lanzar
mkdir -p "$L"
cp -R "$DEPLOY" "$L/deploy"
git init -q -b main "$L"
git -C "$L" add deploy
git -C "$L" commit -q -m deploy
git -C "$L" tag v1.0.0
git -C "$L" branch v2.0.0
# Destinos SSH de mentira (la arroba aparte: no son correos).
ARROBA=@
PC_FALSO=nadie${ARROBA}pc.invalid
BUZON_FALSO=rpentrega${ARROBA}servidor.invalid
printf '. "$CONFIG_EJEMPLO"\nPC_COMPILAR_SSH=%s\nPRECOMPILADAS_BUZON_SSH=%s\n' "$PC_FALSO" "$BUZON_FALSO" >"$TMP/lanzar.conf"
lanzar()
{
	RP_CONFIG=$TMP/lanzar.conf bash "$L/deploy/cross/lanzar.sh" "$@" >"$TMP/salida" 2>&1
}
# negar MOTIVO_ESPERADO ARGUMENTOS…
negar()
{
	_motivo=$1
	shift
	if lanzar "$@"; then
		fallo "lanzar.sh $*: no se niega"
	elif ! grep -q "$_motivo" "$TMP/salida"; then
		fallo "lanzar.sh $*: el motivo no dice «$_motivo»: $(cat "$TMP/salida")"
	fi
}
negar "no es una etiqueta" v2.0.0
negar "no es una versión" main
negar "no es una etiqueta" v9.9.9
echo cambio >>"$L/deploy/README.md"
negar "copia sucia" v1.0.0
git -C "$L" checkout -q -- deploy/README.md
st=0
lanzar v1.0.0 || st=$?
[ "$st" -eq 3 ] && grep -q "no responde" "$TMP/salida" ||
	fallo "lanzar.sh con el PC apagado: sale con $st: $(cat "$TMP/salida")"
printf '. "$CONFIG_EJEMPLO"\nPC_COMPILAR_SSH=%s\n' "$PC_FALSO" >"$TMP/lanzar.conf"
negar PRECOMPILADAS_BUZON_SSH v1.0.0

# compilar-freebsd.sh con cross, podman y compañía de mentira (no llega a
# compilar: se niega antes).
mkdir -p "$TMP/falsos"
for o in cross podman cargo; do
	printf '#!/bin/sh\nexit 1\n' >"$TMP/falsos/$o"
	chmod 755 "$TMP/falsos/$o"
done
git -C "$L" bundle create -q "$TMP/l.bundle" --all 2>/dev/null
compilar()
{
	RP_CROSS_DIR=$TMP/rtf-cross PATH="$TMP/falsos:$PATH" \
		bash "$DEPLOY/cross/compilar-freebsd.sh" "$@" >"$TMP/salida" 2>&1
}
if compilar --release v2.0.0 --bundle "$TMP/l.bundle"; then
	fallo "compilar-freebsd.sh --release con una rama: no se niega"
elif ! grep -q "no es una etiqueta" "$TMP/salida"; then
	fallo "compilar-freebsd.sh --release con una rama: $(cat "$TMP/salida")"
fi
if compilar --release main --bundle "$TMP/l.bundle"; then
	fallo "compilar-freebsd.sh --release main: no se niega"
fi
if compilar --prueba --entregar "$BUZON_FALSO"; then
	fallo "compilar-freebsd.sh --prueba --entregar: no se niega"
fi
mkdir -p "$TMP/prueba-abc"
: >"$TMP/clave-falsa"
if RP_BUZON_CLAVE=$TMP/clave-falsa bash "$DEPLOY/cross/entregar.sh" --destino "$BUZON_FALSO" \
	"$TMP/prueba-abc" >"$TMP/salida" 2>&1 || ! grep -q "no es una versión" "$TMP/salida"; then
	fallo "entregar.sh con una versión de --prueba: $(cat "$TMP/salida")"
fi

# --- 8. sintaxis ------------------------------------------------------------------
for f in "$LIB" "$RECIBIR" "$DEPLOY/freebsd-jail/actualizar.sh"; do
	sh -n "$f" || fallo "sh -n ${f#"$DEPLOY"/}"
done
for f in "$DEPLOY"/cross/*.sh; do
	bash -n "$f" || fallo "bash -n ${f#"$DEPLOY"/}"
done
if command -v shellcheck >/dev/null 2>&1; then
	shellcheck -x "$LIB" "$RECIBIR" "$DEPLOY/freebsd-jail/actualizar.sh" "$DEPLOY"/cross/*.sh ||
		fallo "shellcheck"
else
	echo "aviso: no hay shellcheck" >&2
fi

if [ "$fallos" -ne 0 ]; then
	echo "$fallos fallos" >&2
	exit 1
fi
echo "precompiladas: todo bien"
