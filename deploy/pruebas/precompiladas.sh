#!/bin/sh
# Pruebas de las versiones precompiladas para FreeBSD (deploy/cross/,
# deploy/lib/precompiladas.sh, deploy/freebsd-jail/recibir-precompilada.sh y
# la parte del actualizador), sin servidor, sin PC de compilación y sin tocar
# el sistema: todo bajo una carpeta temporal.
#
# 1. Nombres: archivo_precompilada, version_precompilable, fichero_de_buzon.
# 2. ultima_version (lo que compila el PC) elige la misma etiqueta que
#    ultima_etiqueta_remota (deploy/lib/git.sh, lo que busca el servidor con
#    git ls-remote) en un repositorio de verdad: la última vX.Y.Z, sin rc.
# 3. preparar_github con fetch y gh de mentira (jq de verdad): acepta una
#    release buena (sha256, atestación de este repositorio, de
#    refs/tags/vX.Y.Z y de release.yml, y COMMIT de la etiqueta) y distingue
#    «aún no se puede usar» (sale con 1: sin publicar, sin atestaciones, sin
#    gh) de «rechazada» (sale con 2: sha256, atestación de otra etiqueta o de
#    otro flujo, otro commit, repositorio no válido), sin dejar nada.
# 4. preparar_precompilada acepta un archivo bueno (deja solo panel, site/ y
#    COMMIT) y rechaza, sin dejar nada: sha256 que no cuadra, otro commit, un
#    binario de Linux, carpeta con otro nombre o algo más al lado, un enlace
#    simbólico dentro, sin site/, un enlace en el buzón y, con minisign (si
#    está instalado), una firma mala. precompilada_llegada no da por llegada
#    una versión sin SHA256SUMS ni, con clave, sin firma.
# 5. recibir-precompilada.sh guarda lo admitido y rechaza versiones y
#    nombres no válidos, órdenes con más argumentos, ficheros vacíos o
#    demasiado grandes (sin dejar restos); conserva 3 versiones.
# 6. actualizar.sh de verdad, con jexec, service, /salud, sockstat, ps,
#    fetch y gh de mentira, y cargo, rustc y una jail de compilación que
#    apuntan si alguien los llama (nadie debe hacerlo):
#    a. sin nada en el buzón ni en GitHub, espera: lo dice una sola vez por
#       etiqueta, no despliega ni cuenta un fallo;
#    b. con la versión en el buzón, la instala y vacía el buzón;
#    c. con una versión de otro commit en el buzón, la rechaza, la borra,
#       no despliega y cuenta un fallo;
#    d. con GITHUB_REPO y la release publicada, la instala de GitHub;
#    e. con una atestación de otra etiqueta, la rechaza y no despliega;
#    f. con GITHUB_REPO y la release aún sin publicar, espera sin fallo;
#    g. sin acceso al repositorio, lo registra y sale con 0;
#    h. --ref ya no existe: sale con 2 sin tocar nada;
#    y en ninguno se ha llamado a cargo, rustc ni a la jail de compilación.
# 7. Lo que corre fuera del servidor se niega antes de compilar nada:
#    lanzar.sh con algo que no es una etiqueta v*, con la copia de trabajo
#    sucia o sin PRECOMPILADAS_BUZON_SSH; compilar-freebsd.sh --release con
#    una rama (aunque se llame como una versión) y --prueba con --entregar;
#    entregar.sh con una versión de --prueba. Con un PC que no responde,
#    lanzar.sh sale con 3 y lo dice. empaquetar-freebsd.sh (el que usan el
#    PC y la release de GitHub) deja un archivo que preparar_precompilada
#    acepta.
# 8. sh -n de todo y shellcheck (si está).
#
# Uso: deploy/pruebas/precompiladas.sh (desde cualquier sitio; necesita git,
# tar, gzip, file, sha256sum y jq). RP_PRUEBA_ACTUALIZAR cambia el actualizador
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

for orden in git tar gzip file sha256sum jq; do
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
for t in v0.9.9 v0.9.10 v0.10.0 v1.0.0 v1.0.0-rc1 v2 otra; do
	git -C "$TMP/versiones" tag "$t"
done
servidor=$(git ls-remote --tags "$TMP/versiones" | ultima_etiqueta_remota | cut -d' ' -f1)
pc=$(git -C "$TMP/versiones" tag -l | ultima_version)
[ "$pc" = "$servidor" ] || fallo "ultima_version da $pc y el servidor elige $servidor"
[ "$pc" = v1.0.0 ] || fallo "ultima_version da $pc, no v1.0.0"
[ "$(printf 'v0.9.9\nv0.9.10\n' | ultima_version)" = v0.9.10 ] || fallo "ultima_version no ordena v0.9.10 después de v0.9.9"
[ -z "$(printf 'otra\nv1.0.0-rc1\n' | ultima_version)" ] || fallo "ultima_version da algo sin etiquetas vX.Y.Z"

# --- 3. la release de GitHub -------------------------------------------------------
# Una web de mentira bajo $WEB: fetch (en $FALSOS) sirve
# https://<equipo>/<ruta> desde $WEB/<equipo>/<ruta>. gh de mentira: da por
# buena la atestación si algún bundle dice «firmado-por» con el repositorio,
# la referencia y el flujo que le piden.
WEB=$TMP/web
FALSOS=$TMP/bin-falsos
mkdir -p "$WEB" "$FALSOS"
cat >"$FALSOS/fetch" <<EOF
#!/bin/sh
for a; do u=\$a; done
echo "fetch \$u" >>"$TMP/descargas"
f="$WEB/\${u#https://}"
[ -f "\$f" ] || exit 1
cat "\$f"
EOF
cat >"$FALSOS/gh" <<'EOF'
#!/bin/sh
[ "$1 $2" = "attestation verify" ] || exit 9
shift 3
while [ "$#" -gt 0 ]; do
	case $1 in
	--bundle) b=$2 ;;
	--repo) r=$2 ;;
	--source-ref) ref=$2 ;;
	--signer-workflow) w=$2 ;;
	esac
	shift 2
done
grep -qF "\"firmado-por\":\"$r@$ref@$w\"" "$b" || { echo "no casa"; exit 1; }
echo "ok"
EOF
chmod 755 "$FALSOS/fetch" "$FALSOS/gh"
REPO_GH=dueno/movistar-RTF8115VW-panel
# Commits de 40 cifras hex, generados (sin literales largos en el repositorio).
C=$(printf "%040d" 0 | tr 0 a)
OTRO=$(printf "%040d" 0 | tr 0 b)

# Publica en la web de mentira la release $2 del repositorio $1 con COMMIT
# $3; $4: la referencia de la atestación (por defecto refs/tags/$2), «sin»
# para no publicar atestaciones; $5: variante de hacer_archivo.
publicar_github()
{
	_r=$1
	_v=$2
	_d=$WEB/github.com/$_r/releases/download/$_v
	rm -rf "$_d"
	hacer_archivo "$_d" "$_v" "$3" "${5:-buena}"
	# Como la de verdad, una línea por paquete publicado.
	printf '%064d  routerpanel-%s-x86_64-unknown-linux-gnu.tar.gz\n' 0 "$_v" >>"$_d/SHA256SUMS"
	_s=$(sha256sum <"$_d/$(archivo_precompilada "$_v")" | cut -d' ' -f1)
	_a=$WEB/api.github.com/repos/$_r/attestations
	mkdir -p "$_a"
	case ${4:-} in
	sin) printf '{"attestations":[]}' >"$_a/sha256:$_s" ;;
	*)
		printf '{"attestations":[{"bundle":{"firmado-por":"%s@%s@%s"}}]}' \
			"$_r" "${4:-refs/tags/$_v}" "$_r/.github/workflows/release.yml" >"$_a/sha256:$_s"
		;;
	esac
}
# github ESPERADO MOTIVO REPO VERSION COMMIT: preparar_github sale con
# ESPERADO y, si no es 0, dice MOTIVO y no deja $TMP/gh-destino.
github()
{
	rm -rf "$TMP/gh-destino"
	_rc=0
	PATH="$FALSOS:$PATH" preparar_github "$3" "$4" "$5" "$TMP/gh-destino" 2>"$TMP/err" || _rc=$?
	if [ "$_rc" != "$1" ]; then
		fallo "github ($2): sale con $_rc, no con $1: $(cat "$TMP/err")"
	elif [ "$1" != 0 ] && ! grep -q "$2" "$TMP/err"; then
		fallo "github: el motivo no dice «$2»: $(cat "$TMP/err")"
	elif [ "$1" != 0 ] && [ -e "$TMP/gh-destino" ]; then
		fallo "github ($2): deja la carpeta de destino"
	fi
}
publicar_github "$REPO_GH" v1.2.3 "$C"
github 0 bien "$REPO_GH" v1.2.3 "$C"
[ "$(find "$TMP/gh-destino" -mindepth 1 -maxdepth 1 -printf '%f\n' 2>/dev/null | sort | tr '\n' ' ')" = "COMMIT panel site " ] ||
	fallo "github: la versión no queda solo con panel, site/ y COMMIT"
github 2 "otro commit" "$REPO_GH" v1.2.3 "$OTRO"
github 1 "aún no está publicada" "$REPO_GH" v9.9.9 "$C"
github 2 "no válido" "dueno/../x" v1.2.3 "$C"
github 2 "no válida" "$REPO_GH" v1.2 "$C"
github 2 "no válida" "$REPO_GH" v1.2.3-rc1 "$C"
publicar_github "$REPO_GH" v1.2.4 "$C" refs/tags/v1.2.3
github 2 "atestación" "$REPO_GH" v1.2.4 "$C"
publicar_github "$REPO_GH" v1.2.5 "$C" sin
github 1 "ninguna atestación" "$REPO_GH" v1.2.5 "$C"
publicar_github "$REPO_GH" v1.2.6 "$C" "" sha-malo
github 2 sha256 "$REPO_GH" v1.2.6 "$C"
publicar_github "$REPO_GH" v1.2.7 "$C" "" linux
github 2 FreeBSD "$REPO_GH" v1.2.7 "$C"
# Atestación de otro flujo (no release.yml).
publicar_github "$REPO_GH" v1.2.8 "$C"
_s=$(sha256sum <"$WEB/github.com/$REPO_GH/releases/download/v1.2.8/$(archivo_precompilada v1.2.8)" | cut -d' ' -f1)
printf '{"attestations":[{"bundle":{"firmado-por":"%s@refs/tags/v1.2.8@%s/.github/workflows/otro.yml"}}]}' \
	"$REPO_GH" "$REPO_GH" >"$WEB/api.github.com/repos/$REPO_GH/attestations/sha256:$_s"
github 2 "atestación" "$REPO_GH" v1.2.8 "$C"
# Sin gh: no se puede usar todavía (pkg install gh jq).
mkdir -p "$TMP/sin-gh"
ln -sf "$FALSOS/fetch" "$TMP/sin-gh/fetch"
for o in sh sed head tail cut tr grep find wc mktemp rm mv mkdir cat tar gzip file sha256sum timeout printf jq awk sort ls; do
	ruta=$(command -v "$o" 2>/dev/null) && ln -sf "$ruta" "$TMP/sin-gh/$o"
done
rc=0
PATH=$TMP/sin-gh preparar_github "$REPO_GH" v1.2.3 "$C" "$TMP/gh-destino" 2>"$TMP/err" || rc=$?
[ "$rc" = 1 ] && grep -q "falta gh" "$TMP/err" || fallo "github sin gh: sale con $rc: $(cat "$TMP/err")"
rm -rf "$TMP/gh-destino"

# --- 4. comprobar un archivo ----------------------------------------------------
B=$TMP/buzon
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
	rm -rf "${B:?}/${V:?}" "$TMP/malo"
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

rm -rf "${B:?}/${V:?}"
hacer_archivo "$TMP/fuera" "$V" "$C"
mkdir -p "$B/$V"
ln -s "$TMP/fuera/$(archivo_precompilada "$V")" "$B/$V/$(archivo_precompilada "$V")"
cp "$TMP/fuera/SHA256SUMS" "$B/$V/"
precompilada_llegada "$B" "$V" '' && fallo "da por llegada un archivo que es un enlace"
if preparar_precompilada "$B" "$V" "$C" '' "$TMP/malo" 2>/dev/null; then
	fallo "acepta un archivo del buzón que es un enlace"
fi
rm -rf "${B:?}/${V:?}" "$TMP/malo"

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
	rm -rf "${B:?}/${V:?}"
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
# Por SSH de verdad (SSH_CONNECTION) no valen las rutas de las variables: con
# un AcceptEnv permisivo en sshd_config, el cliente podría cargar otro script.
if echo x | SSH_CONNECTION="192.0.2.1 50000 192.0.2.2 22" SSH_ORIGINAL_COMMAND="recibir v4.0.0 SHA256SUMS" \
	RP_BUZON=$R RP_BUZON_LIB=$LIB RP_BUZON_MAXIMO=1000 sh "$RECIBIR" >"$TMP/salida" 2>&1 ||
	[ -e "$R/v4.0.0/SHA256SUMS" ]; then
	fallo "el receptor usa RP_BUZON/RP_BUZON_LIB por SSH: $(cat "$TMP/salida")"
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
# las órdenes de BSD por las de GNU), el remoto con la etiqueta, las releases
# y el «current» de la jail del panel. No hay clon ni jail de compilación:
# cargo, rustc y cualquier jexec que no sea a la jail del panel quedan
# apuntados en $H/prohibido.
GIT_ABS=$(command -v git)
preparar_servidor()
{
	H=$TMP/servidor
	rm -rf "$H"
	mkdir -p "$H/bin" "$H/libexec" "$H/releases" "$H/db" \
		"$H/state" "$H/log" "$H/run" "$H/buzon/entrada" "$H/jails/routerpanel/usr/local/routerpanel"
	ln -s "$H/releases" "$H/jails/routerpanel/usr/local/routerpanel/releases"
	cp "$DEPLOY/lib/git.sh" "$LIB" "$H/libexec/"
	sed -e "s|/usr/sbin/jexec|$H/bin/jexec|g" \
		-e "s|^GIT=/usr/local/bin/git|GIT=$GIT_ABS|" \
		-e "s|^PATH=/sbin:.*|PATH=$H/bin:$FALSOS:$PATH|" \
		-e "s|/usr/local/etc/routerpanel-actualizar.conf|$H/actualizar.conf|" \
		-e "s|/usr/local/etc/routerpanel-actualizar/|$H/claves/|g" \
		-e "s|/usr/local/libexec/routerpanel-actualizar|$H/libexec|g" \
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
		echo "jexec \$*" >>"$H/llamadas"
		while :; do
			case \$1 in
			-l) shift ;;
			-U) shift 2 ;;
			*) break ;;
			esac
		done
		if [ "\$1" != routerpanel ]; then
			echo "jexec a \$1" >>"$H/prohibido"
			exit 1
		fi
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
	for o in cargo rustc cargo-leptos rustup; do
		printf '#!/bin/sh\necho "%s $*" >>"%s/prohibido"\nexit 1\n' "$o" "$H" >"$H/bin/$o"
	done
	chmod 755 "$H"/bin/*

	git init -q -b main "$H/origen"
	echo uno >"$H/origen/f"
	git -C "$H/origen" add f
	git -C "$H/origen" commit -q -m uno
	git -C "$H/origen" tag -a -m v1.0.0 v1.0.0
	COMMIT_ETIQUETA=$(git -C "$H/origen" rev-parse 'v1.0.0^{commit}')
	{
		echo "JAILS_RAIZ='$H/jails'"
		echo "REPO_URL='$H/origen'"
	} >"$H/actualizar.conf"
	rm -rf "$WEB/github.com/$REPO_GH/releases/download/v1.0.0"
}

# Ejecuta el actualizador; su salida, en $H/salida, y su código, en $RC.
actualizar()
{
	RC=0
	ROUTERPANEL_LOCKED=1 sh "$H/actualizar.sh" "$@" >"$H/salida" 2>&1 || RC=$?
}

registro()
{
	cat "$H/log/routerpanel-update.log" 2>/dev/null
}

desplegada()
{
	basename "$(readlink "$H/jails/routerpanel/usr/local/routerpanel/current" 2>/dev/null)" 2>/dev/null || true
}

# Nadie ha llamado a cargo, rustc ni a una jail que no sea la del panel.
sin_compilar()
{
	if [ -e "$H/prohibido" ]; then
		fallo "$1: se llama a lo que compila: $(tr '\n' ';' <"$H/prohibido")"
	fi
}

# a. Nada en el buzón ni en GitHub: espera, lo dice una vez, sin fallo.
preparar_servidor
actualizar
[ "$RC" = 0 ] || fallo "a: sale con $RC: $(cat "$H/salida")"
[ -z "$(desplegada)" ] || fallo "a: despliega algo mientras espera"
actualizar
[ "$(registro | grep -c 'esperando la versión precompilada de v1.0.0')" = 1 ] ||
	fallo "a: no dice una sola vez que espera: $(registro)"
if grep -q service "$H/llamadas" 2>/dev/null; then fallo "a: toca el servicio mientras espera"; fi
[ ! -e "$H/state/intentos" ] && [ ! -e "$H/state/fallida" ] || fallo "a: esperar cuenta como fallo"
sin_compilar a

# b. Con la versión en el buzón: la instala y vacía el buzón.
hacer_archivo "$H/buzon/entrada/v1.0.0" v1.0.0 "$COMMIT_ETIQUETA"
actualizar
[ "$RC" = 0 ] || fallo "b: sale con $RC: $(cat "$H/salida") $(registro)"
[ "$(desplegada)" = v1.0.0 ] || fallo "b: no despliega v1.0.0"
file -b "$H/releases/v1.0.0/panel" 2>/dev/null | grep -q FreeBSD || fallo "b: lo desplegado no es la precompilada"
[ "$(cat "$H/releases/v1.0.0/COMMIT" 2>/dev/null)" = "$COMMIT_ETIQUETA" ] || fallo "b: COMMIT no es el de la etiqueta"
[ -d "$H/releases/v1.0.0/site" ] && [ ! -e "$H/releases/v1.0.0/LICENSE" ] ||
	fallo "b: la versión no tiene solo panel, site/ y COMMIT"
[ "$(stat -c %a "$H/releases/v1.0.0/panel")" = 755 ] || fallo "b: el binario no queda 755"
[ ! -e "$H/buzon/entrada/v1.0.0" ] || fallo "b: no vacía el buzón"
registro | grep -q 'instalada la versión precompilada de v1.0.0' || fallo "b: no lo registra"
grep -q 'service start' "$H/llamadas" || fallo "b: no arranca el servicio"
# Ya desplegada: no hace nada.
: >"$H/llamadas"
actualizar
[ "$RC" = 0 ] && ! grep -q service "$H/llamadas" || fallo "b: con la versión ya desplegada hace algo"
sin_compilar b

# c. Del buzón, pero de otro commit: rechazada, borrada, un fallo.
preparar_servidor
hacer_archivo "$H/buzon/entrada/v1.0.0" v1.0.0 "$OTRO"
actualizar
[ "$RC" = 1 ] || fallo "c: sale con $RC, no con 1"
[ -z "$(desplegada)" ] || fallo "c: despliega una versión rechazada"
[ ! -e "$H/releases/v1.0.0" ] || fallo "c: deja la versión rechazada en releases"
registro | grep -q 'rechazada: el archivo es de otro commit' || fallo "c: no dice por qué la rechaza: $(registro)"
[ ! -e "$H/buzon/entrada/v1.0.0" ] || fallo "c: deja la precompilada mala en el buzón"
[ "$(sed -n 2p "$H/state/intentos" 2>/dev/null)" = 1 ] || fallo "c: no cuenta el fallo"
sin_compilar c

# d. De GitHub, con su atestación.
preparar_servidor
echo "GITHUB_REPO='$REPO_GH'" >>"$H/actualizar.conf"
publicar_github "$REPO_GH" v1.0.0 "$COMMIT_ETIQUETA"
actualizar
[ "$RC" = 0 ] || fallo "d: sale con $RC: $(cat "$H/salida") $(registro)"
[ "$(desplegada)" = v1.0.0 ] || fallo "d: no despliega v1.0.0 de GitHub"
[ "$(cat "$H/releases/v1.0.0/COMMIT" 2>/dev/null)" = "$COMMIT_ETIQUETA" ] || fallo "d: COMMIT no es el de la etiqueta"
registro | grep -q "instalada la versión de GitHub de v1.0.0" || fallo "d: no lo registra: $(registro)"
sin_compilar d

# e. De GitHub, con la atestación de otra etiqueta: rechazada.
preparar_servidor
echo "GITHUB_REPO='$REPO_GH'" >>"$H/actualizar.conf"
publicar_github "$REPO_GH" v1.0.0 "$COMMIT_ETIQUETA" refs/tags/v0.9.0
actualizar
[ "$RC" = 1 ] || fallo "e: sale con $RC, no con 1"
[ -z "$(desplegada)" ] || fallo "e: despliega una versión con la atestación de otra etiqueta"
registro | grep -q 'versión de GitHub de v1.0.0 rechazada: la atestación' || fallo "e: no dice por qué: $(registro)"
sin_compilar e

# f. GitHub configurado pero la release aún no está: espera, sin fallo.
preparar_servidor
echo "GITHUB_REPO='$REPO_GH'" >>"$H/actualizar.conf"
actualizar
[ "$RC" = 0 ] || fallo "f: sale con $RC: $(cat "$H/salida")"
[ -z "$(desplegada)" ] || fallo "f: despliega algo sin release"
registro | grep -q 'esperando la versión precompilada de v1.0.0' || fallo "f: no dice que espera: $(registro)"
[ ! -e "$H/state/intentos" ] || fallo "f: esperar a GitHub cuenta como fallo"
sin_compilar f

# g. Sin acceso al repositorio.
preparar_servidor
echo "REPO_URL='$H/no-existe'" >>"$H/actualizar.conf"
actualizar
[ "$RC" = 0 ] || fallo "g: sale con $RC"
registro | grep -q 'sin acceso al repositorio' || fallo "g: no lo dice: $(registro)"
sin_compilar g

# h. --ref ya no existe.
preparar_servidor
actualizar --ref main
[ "$RC" = 2 ] || fallo "h: --ref sale con $RC, no con 2"
grep -q 'no compila' "$H/salida" || fallo "h: --ref no explica por qué: $(cat "$H/salida")"
[ -z "$(desplegada)" ] && [ ! -e "$H/llamadas" ] || fallo "h: --ref toca algo"
sin_compilar h

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

# empaquetar-freebsd.sh (el PC y la release de GitHub): lo que deja lo acepta
# preparar_precompilada, y se niega con un binario que no es de FreeBSD.
E=$TMP/empaquetar
mkdir -p "$E/site/pkg" "$E/fuente" "$E/salida"
elf 9 >"$E/panel"
printf 'js: h1\nwasm: h2\ncss: h3\n' >"$E/site/hash.txt"
for x in js:h1 wasm:h2 css:h3; do echo "${x%%:*}" >"$E/site/pkg/panel.${x#*:}.${x%%:*}"; done
echo licencia >"$E/fuente/LICENSE"
if SOURCE_DATE_EPOCH=1700000000 bash "$DEPLOY/cross/empaquetar-freebsd.sh" v3.2.1 "$C" "$E/panel" "$E/site" \
	"$E/fuente" "$E/salida/v3.2.1" >"$TMP/salida" 2>&1; then
	(cd "$E/salida/v3.2.1" && sha256sum -- "$(archivo_precompilada v3.2.1)" >SHA256SUMS)
	preparar_precompilada "$E/salida" v3.2.1 "$C" '' "$E/bien" 2>"$TMP/err" ||
		fallo "preparar_precompilada rechaza lo de empaquetar-freebsd.sh: $(cat "$TMP/err")"
	tar -tzf "$E/salida/v3.2.1/$(archivo_precompilada v3.2.1)" | grep -q '/LICENSE$' ||
		fallo "empaquetar-freebsd.sh no mete LICENSE"
else
	fallo "empaquetar-freebsd.sh falla: $(cat "$TMP/salida")"
fi
elf 0 >"$E/panel"
if bash "$DEPLOY/cross/empaquetar-freebsd.sh" v3.2.2 "$C" "$E/panel" "$E/site" "$E/fuente" "$E/salida/v3.2.2" \
	>"$TMP/salida" 2>&1; then
	fallo "empaquetar-freebsd.sh acepta un binario de Linux"
fi

# --- 8. sintaxis ------------------------------------------------------------------
for f in "$LIB" "$DEPLOY/lib/git.sh" "$RECIBIR" "$DEPLOY/freebsd-jail/actualizar.sh"; do
	sh -n "$f" || fallo "sh -n ${f#"$DEPLOY"/}"
done
for f in "$DEPLOY"/cross/*.sh; do
	bash -n "$f" || fallo "bash -n ${f#"$DEPLOY"/}"
done
if command -v shellcheck >/dev/null 2>&1; then
	shellcheck -S warning -x "$LIB" "$DEPLOY/lib/git.sh" "$RECIBIR" "$DEPLOY/freebsd-jail/actualizar.sh" "$DEPLOY"/cross/*.sh ||
		fallo "shellcheck"
else
	echo "aviso: no hay shellcheck" >&2
fi

if [ "$fallos" -ne 0 ]; then
	echo "$fallos fallos" >&2
	exit 1
fi
echo "precompiladas: todo bien"
