#!/bin/sh
# Pruebas de las versiones precompiladas para FreeBSD (deploy/cross/,
# deploy/lib/precompiladas.sh y la parte del actualizador), sin servidor, sin
# PC de compilación, sin Forgejo y sin tocar el sistema: todo bajo una
# carpeta temporal.
#
# 1. Nombres: archivo_precompilada, version_precompilable,
#    repo_forgejo_valido y url_forgejo_valida.
# 2. ultima_version elige la misma etiqueta que ultima_etiqueta_remota
#    (deploy/lib/git.sh, lo que busca el servidor con git ls-remote) en un
#    repositorio de verdad: la última vX.Y.Z, sin rc.
# 3. preparar_github con fetch y gh de mentira (jq de verdad): acepta una
#    release buena (sha256, atestación de este repositorio, de
#    refs/tags/vX.Y.Z y de release.yml, y COMMIT de la etiqueta) y distingue
#    «aún no se puede usar» (sale con 1: sin publicar, sin atestaciones, sin
#    gh) de «rechazada» (sale con 2: sha256, atestación de otra etiqueta o de
#    otro flujo, otro commit, repositorio no válido), sin dejar nada.
# 4. preparar_forgejo con un Forgejo de mentira (curl en $FALSOS): acepta
#    una release buena; sale con 1 si la release no existe (404) o le falta
#    SHA256SUMS, con 2 si SHA256SUMS no cuadra y con 3 si el token no vale
#    (no es de root, lo leen otros, no existe), sin llamar a curl. El token
#    va siempre por la entrada estándar de curl, nunca en sus argumentos.
# 5. preparar_precompilada acepta un archivo bueno (deja solo panel, site/ y
#    COMMIT) y rechaza, sin dejar nada: sha256 que no cuadra, otro commit, un
#    binario de Linux, carpeta con otro nombre o algo más al lado, un enlace
#    simbólico dentro, sin site/, un enlace en la carpeta y, con minisign (si
#    está instalado), una firma mala.
# 6. actualizar.sh de verdad, con jexec, service, /salud, sockstat, ps,
#    fetch, curl y gh de mentira, y cargo, rustc y una jail de compilación
#    que apuntan si alguien los llama (nadie debe hacerlo):
#    a. sin la release en Forgejo (404) ni en GitHub, espera: lo dice una
#       sola vez por etiqueta, sale con 0, no despliega ni cuenta un fallo;
#    b. con la release en Forgejo, la instala;
#    c. con un SHA256SUMS que no cuadra en Forgejo, la rechaza sin contar un
#       fallo y lo registra una sola vez en dos pasadas;
#    d. con una release de otro commit en Forgejo, la rechaza;
#    e. sin la release en Forgejo, la instala de GitHub;
#    f. con el token de Forgejo en modo 0644, Forgejo no está disponible (lo
#       dice una vez), no se manda el token y se instala de GitHub;
#    g. sin FORGEJO_URL, lo dice una sola vez y usa GitHub;
#    h. con una atestación de otra etiqueta en GitHub, la rechaza;
#    i. con GITHUB_REPO y la release aún sin publicar, espera sin fallo;
#    j. sin acceso al repositorio, lo registra y sale con 0;
#    k. --ref ya no existe: sale con 2 sin tocar nada;
#    y en ninguno se ha llamado a cargo, rustc ni a la jail de compilación.
# 7. Lo que corre en el PC: compilar-freebsd.sh se niega con una rama
#    (aunque se llame como una versión) y ya no conoce --entregar ni
#    --ultima; publicar-forgejo.sh crea la release si falta, sube el archivo
#    y SHA256SUMS, al repetir sustituye los que ya estaban, y el token nunca
#    está en los argumentos de curl ni en el entorno de la compilación.
#    empaquetar-freebsd.sh deja un archivo que preparar_precompilada acepta.
# 8. sh -n de todo y shellcheck (si está).
#
# Uso: deploy/pruebas/precompiladas.sh (desde cualquier sitio; necesita git,
# tar, gzip, file, sha256sum y jq). RP_PRUEBA_ACTUALIZAR cambia el actualizador
# que se prueba en 6 (para comparar con otra versión a mano).

set -eu

DEPLOY=$(cd "$(dirname "$0")/.." && pwd)
LIB=$DEPLOY/lib/precompiladas.sh
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

# Construye en $1 (la carpeta de la versión $2) un archivo con
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
for r in dueno/panel Dueno.x/mi_panel-2 a/b; do
	repo_forgejo_valido "$r" || fallo "repo_forgejo_valido rechaza $r"
done
for r in '' dueno dueno/ /panel dueno/panel/x .dueno/panel dueno/-panel 'dueno/pa nel' 'dueno/$(id)' dueno/..; do
	if repo_forgejo_valido "$r"; then fallo "repo_forgejo_valido acepta «$r»"; fi
done
for u in https://forge.example http://forge.example:3000 https://forge.example/git; do
	url_forgejo_valida "$u" || fallo "url_forgejo_valida rechaza $u"
done
# La arroba aparte: no es un correo.
ARROBA=@
for u in '' forge.example https://forge.example/ "https://u${ARROBA}forge.example" 'https://forge.example?x' \
	https:// ftp://forge.example 'https://forge.example/a b' https://forge.example/../x; do
	if url_forgejo_valida "$u"; then fallo "url_forgejo_valida acepta «$u»"; fi
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

# --- 4. la release de Forgejo -----------------------------------------------------
# Un Forgejo de mentira: curl (en $FALSOS) atiende la API de releases
# (/api/v1/repos/<repo>/releases…) y las descargas
# (/<repo>/releases/download/<etiqueta>/<fichero>) de $FORJA_URL desde
# $FORJA/<etiqueta>/ (id: el de la release; adjuntos/: los ficheros;
# adjuntos.ids: «<id> <nombre>»). Solo responde con la cabecera
# «Authorization: token <$TMP/token-esperado>» leída de -K -; si no, 401.
# Apunta sus argumentos en $TMP/curl-argv, lo que lee por -K - en
# $TMP/curl-entrada y lo que cambia en $TMP/forja-log.
FORJA=$TMP/forja
FORJA_URL=https://forge.example
REPO_FJ=dueno/panel
mkdir -p "$FORJA"
cat >"$FALSOS/curl" <<EOF
#!/bin/sh
FORJA=$FORJA
TMP=$TMP
EOF
cat >>"$FALSOS/curl" <<'EOF'
printf '%s\n' "$*" >>"$TMP/curl-argv"
metodo=
salida=-
formato=
datos=
adjunto=
url=
conf=
while [ "$#" -gt 0 ]; do
	case $1 in
	-K)
		[ "$2" = - ] && conf=$(cat)
		shift 2
		;;
	-X) metodo=$2 && shift 2 ;;
	-o) salida=$2 && shift 2 ;;
	-w) formato=$2 && shift 2 ;;
	--data-binary) datos=${2#@} && shift 2 ;;
	-F) adjunto=${2#attachment=@} && shift 2 ;;
	-H | --connect-timeout | --max-time | --max-filesize | --max-redirs | --proto | --proto-redir) shift 2 ;;
	http://* | https://*) url=$1 && shift ;;
	*) shift ;;
	esac
done
if [ -z "$metodo" ]; then
	if [ -n "$datos$adjunto" ]; then metodo=POST; else metodo=GET; fi
fi
printf '%s\n' "$conf" >>"$TMP/curl-entrada"
codigo=404
cuerpo='{"message":"not found"}'
ruta=${url#https://forge.example}
api=/api/v1/repos/dueno/panel
etiqueta_de_id()
{
	for _d in "$FORJA"/*/; do
		[ "$(cat "$_d/id" 2>/dev/null)" = "$1" ] && basename "$_d" && return 0
	done
	return 1
}
if [ "$conf" != "header = \"Authorization: token $(cat "$TMP/token-esperado")\"" ]; then
	codigo=401
	cuerpo='{"message":"token"}'
else
	case "$metodo $ruta" in
	"GET $api/releases/tags/"*)
		t=${ruta##*/}
		if [ -f "$FORJA/$t/id" ]; then
			codigo=200
			cuerpo="{\"id\":$(cat "$FORJA/$t/id"),\"tag_name\":\"$t\"}"
		fi
		;;
	"POST $api/releases")
		t=$(jq -r .tag_name "$datos")
		if [ -f "$FORJA/$t/id" ]; then
			codigo=409
		else
			n=$(($(find "$FORJA" -name id | wc -l) + 1))
			mkdir -p "$FORJA/$t/adjuntos"
			echo "$n" >"$FORJA/$t/id"
			: >"$FORJA/$t/adjuntos.ids"
			jq -r .body "$datos" >"$FORJA/$t/cuerpo"
			echo "creada $t" >>"$TMP/forja-log"
			codigo=201
			cuerpo="{\"id\":$n}"
		fi
		;;
	"GET $api/releases/"*/assets)
		r=${ruta#"$api/releases/"}
		if t=$(etiqueta_de_id "${r%/assets}"); then
			codigo=200
			cuerpo=$(while read -r i nom; do printf '{"id":%s,"name":"%s"}\n' "$i" "$nom"; done \
				<"$FORJA/$t/adjuntos.ids" | jq -s -c .)
		fi
		;;
	"DELETE $api/releases/"*/assets/*)
		r=${ruta#"$api/releases/"}
		a=${r##*/}
		if t=$(etiqueta_de_id "${r%%/*}") && nom=$(awk -v a="$a" '$1 == a { print $2 }' "$FORJA/$t/adjuntos.ids") &&
			[ -n "$nom" ]; then
			rm -f "$FORJA/$t/adjuntos/$nom"
			awk -v a="$a" '$1 != a' "$FORJA/$t/adjuntos.ids" >"$FORJA/$t/ids.tmp"
			mv "$FORJA/$t/ids.tmp" "$FORJA/$t/adjuntos.ids"
			echo "borrado $nom" >>"$TMP/forja-log"
			codigo=204
			cuerpo=
		fi
		;;
	"POST $api/releases/"*/assets\?name=*)
		r=${ruta#"$api/releases/"}
		nom=${r#*\?name=}
		if t=$(etiqueta_de_id "${r%%/*}") && [ -f "$adjunto" ]; then
			if grep -q " $nom\$" "$FORJA/$t/adjuntos.ids"; then
				codigo=409
			else
				cp "$adjunto" "$FORJA/$t/adjuntos/$nom"
				n=$(($(cat "$TMP/forja-contador" 2>/dev/null || echo 100) + 1))
				echo "$n" >"$TMP/forja-contador"
				echo "$n $nom" >>"$FORJA/$t/adjuntos.ids"
				echo "subido $nom" >>"$TMP/forja-log"
				codigo=201
				cuerpo="{\"id\":$n,\"name\":\"$nom\"}"
			fi
		fi
		;;
	"GET /dueno/panel/releases/download/"*)
		r=${ruta#/dueno/panel/releases/download/}
		if [ -f "$FORJA/${r%%/*}/adjuntos/${r#*/}" ]; then
			codigo=200
			cuerpo=
			if [ "$salida" = - ]; then
				cat "$FORJA/${r%%/*}/adjuntos/${r#*/}"
			else
				cp "$FORJA/${r%%/*}/adjuntos/${r#*/}" "$salida"
			fi
			salida=
		fi
		;;
	esac
fi
if [ -n "$salida" ]; then
	if [ "$salida" = - ]; then
		printf '%s' "$cuerpo"
	else
		printf '%s' "$cuerpo" >"$salida"
	fi
fi
case $formato in
*http_code*) printf '%s' "$codigo" ;;
esac
[ "$codigo" -lt 400 ] || exit 22
EOF
chmod 755 "$FALSOS/curl"
TOKEN_FJ=$(printf '%040d' 0 | tr 0 7)
printf '%s' "$TOKEN_FJ" >"$TMP/token-esperado"
mkdir -p "$TMP/claves-fj"
printf '%s\n' "$TOKEN_FJ" >"$TMP/claves-fj/token"
chmod 600 "$TMP/claves-fj/token"
YO=$(id -u)

# Publica en el Forgejo de mentira la release $1 con COMMIT $2; $3: variante
# de hacer_archivo; «sin-sumas» la deja sin SHA256SUMS.
publicar_forgejo()
{
	_d=$FORJA/$1
	rm -rf "$_d"
	hacer_archivo "$_d/adjuntos" "$1" "$2" "${3:-buena}"
	if [ "${3:-}" = sin-sumas ]; then rm -f "$_d/adjuntos/SHA256SUMS"; fi
	echo "$(($(find "$FORJA" -name id | wc -l) + 1))" >"$_d/id"
	(cd "$_d/adjuntos" && ls | awk '{ print NR + 10, $0 }') >"$_d/adjuntos.ids"
}
# forjar ESPERADO MOTIVO VERSION [FICHERO_TOKEN]: preparar_forgejo sale con
# ESPERADO y, si no es 0, dice MOTIVO y no deja $TMP/fj-destino.
forjar()
{
	rm -rf "$TMP/fj-destino"
	_rc=0
	PATH="$FALSOS:$PATH" preparar_forgejo "$FORJA_URL" "$REPO_FJ" "${4:-$TMP/claves-fj/token}" "$YO" \
		"$3" "$C" '' "$TMP/fj-destino" 2>"$TMP/err" || _rc=$?
	if [ "$_rc" != "$1" ]; then
		fallo "forgejo ($2): sale con $_rc, no con $1: $(cat "$TMP/err")"
	elif [ "$1" != 0 ] && ! grep -q "$2" "$TMP/err"; then
		fallo "forgejo: el motivo no dice «$2»: $(cat "$TMP/err")"
	elif [ "$1" != 0 ] && [ -e "$TMP/fj-destino" ]; then
		fallo "forgejo ($2): deja la carpeta de destino"
	fi
}
publicar_forgejo v1.2.3 "$C"
forjar 0 bien v1.2.3
[ "$(find "$TMP/fj-destino" -mindepth 1 -maxdepth 1 -printf '%f\n' 2>/dev/null | sort | tr '\n' ' ')" = "COMMIT panel site " ] ||
	fallo "forgejo: la versión no queda solo con panel, site/ y COMMIT"
grep -q "/dueno/panel/releases/download/v1.2.3/SHA256SUMS" "$TMP/curl-argv" ||
	fallo "forgejo: no descarga de FORGEJO_URL/FORGEJO_REPO/releases/download"
forjar 1 "aún no está publicada" v9.9.9
publicar_forgejo v1.2.4 "$C" sin-sumas
forjar 1 SHA256SUMS v1.2.4
publicar_forgejo v1.2.5 "$C" sha-malo
forjar 2 sha256 v1.2.5
publicar_forgejo v1.2.6 "$C" linux
forjar 2 FreeBSD v1.2.6
forjar 2 "no válida" v1.2
# El token: sin fichero, de otro dueño o legible por otros, no se usa.
: >"$TMP/curl-argv"
forjar 3 "no existe" v1.2.3 "$TMP/claves-fj/no-existe"
cp "$TMP/claves-fj/token" "$TMP/claves-fj/abierto"
chmod 644 "$TMP/claves-fj/abierto"
forjar 3 "0600" v1.2.3 "$TMP/claves-fj/abierto"
chmod 640 "$TMP/claves-fj/abierto"
forjar 3 "0600" v1.2.3 "$TMP/claves-fj/abierto"
rc=0
PATH="$FALSOS:$PATH" preparar_forgejo "$FORJA_URL" "$REPO_FJ" "$TMP/claves-fj/token" $((YO + 1)) \
	v1.2.3 "$C" '' "$TMP/fj-destino" 2>"$TMP/err" || rc=$?
[ "$rc" = 3 ] && grep -q "dueño" "$TMP/err" || fallo "forgejo con un token de otro dueño: sale con $rc: $(cat "$TMP/err")"
printf 'a b\n' >"$TMP/claves-fj/raro"
chmod 600 "$TMP/claves-fj/raro"
forjar 3 "no es válido" v1.2.3 "$TMP/claves-fj/raro"
[ ! -s "$TMP/curl-argv" ] || fallo "forgejo llama a curl con un token que no vale: $(cat "$TMP/curl-argv")"
# Con el token bueno, pero el servidor lo rechaza (otro token): no se puede usar.
printf '%s\n' "$(printf '%040d' 0 | tr 0 8)" >"$TMP/claves-fj/otro"
chmod 600 "$TMP/claves-fj/otro"
forjar 1 "HTTP 401" v1.2.3 "$TMP/claves-fj/otro"
if grep -q "$TOKEN_FJ" "$TMP/curl-argv"; then fallo "el token de Forgejo aparece en los argumentos de curl"; fi
grep -q "Authorization: token $TOKEN_FJ" "$TMP/curl-entrada" || fallo "forgejo no manda el token por -K -"

# --- 5. comprobar un archivo ----------------------------------------------------
B=$TMP/carpeta
V=v1.2.3

hacer_archivo "$B/$V" "$V" "$C"

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
if preparar_precompilada "$B" "$V" "$C" '' "$TMP/malo" 2>/dev/null; then
	fallo "acepta un archivo de la carpeta que es un enlace"
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
		"$H/state" "$H/log" "$H/run" "$H/claves" "$H/jails/routerpanel/usr/local/routerpanel"
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
		-e "s|^DUENO_TOKEN=0\$|DUENO_TOKEN=$YO|" \
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
	rm -rf "$WEB/github.com/$REPO_GH/releases/download/v1.0.0" "$FORJA/v1.0.0"
	: >"$TMP/curl-argv"
	: >"$TMP/curl-entrada"
}

# Configura en el servidor la release de Forgejo, con su token (de quien
# corre las pruebas, que hace de root, y 0600).
con_forgejo()
{
	{
		echo "FORGEJO_URL='$FORJA_URL'"
		echo "FORGEJO_REPO='$REPO_FJ'"
	} >>"$H/actualizar.conf"
	printf '%s\n' "$TOKEN_FJ" >"$H/claves/forgejo-token"
	chmod 600 "$H/claves/forgejo-token"
}

con_github()
{
	echo "GITHUB_REPO='$REPO_GH'" >>"$H/actualizar.conf"
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

# a. Ni en Forgejo (404) ni en GitHub: espera, lo dice una vez, sin fallo.
preparar_servidor
con_forgejo
con_github
actualizar
[ "$RC" = 0 ] || fallo "a: sale con $RC: $(cat "$H/salida")"
[ -z "$(desplegada)" ] || fallo "a: despliega algo mientras espera"
actualizar
[ "$RC" = 0 ] || fallo "a: la segunda pasada sale con $RC"
[ "$(registro | grep -c 'esperando la versión precompilada de v1.0.0')" = 1 ] ||
	fallo "a: no dice una sola vez que espera: $(registro)"
registro | grep -q 'no está en Forgejo ni en GitHub' || fallo "a: no dice dónde ha mirado: $(registro)"
if grep -q service "$H/llamadas" 2>/dev/null; then fallo "a: toca el servicio mientras espera"; fi
[ ! -e "$H/state/intentos" ] && [ ! -e "$H/state/fallida" ] || fallo "a: esperar cuenta como fallo"
sin_compilar a

# b. Con la release en Forgejo: la instala.
preparar_servidor
con_forgejo
publicar_forgejo v1.0.0 "$COMMIT_ETIQUETA"
actualizar
[ "$RC" = 0 ] || fallo "b: sale con $RC: $(cat "$H/salida") $(registro)"
[ "$(desplegada)" = v1.0.0 ] || fallo "b: no despliega v1.0.0"
file -b "$H/releases/v1.0.0/panel" 2>/dev/null | grep -q FreeBSD || fallo "b: lo desplegado no es la precompilada"
[ "$(cat "$H/releases/v1.0.0/COMMIT" 2>/dev/null)" = "$COMMIT_ETIQUETA" ] || fallo "b: COMMIT no es el de la etiqueta"
[ -d "$H/releases/v1.0.0/site" ] && [ ! -e "$H/releases/v1.0.0/LICENSE" ] ||
	fallo "b: la versión no tiene solo panel, site/ y COMMIT"
[ "$(stat -c %a "$H/releases/v1.0.0/panel")" = 755 ] || fallo "b: el binario no queda 755"
registro | grep -q 'instalada la versión de Forgejo de v1.0.0' || fallo "b: no lo registra: $(registro)"
grep -q 'service start' "$H/llamadas" || fallo "b: no arranca el servicio"
if grep -q "$TOKEN_FJ" "$TMP/curl-argv"; then fallo "b: el token aparece en los argumentos de curl"; fi
# Ya desplegada: no hace nada.
: >"$H/llamadas"
actualizar
[ "$RC" = 0 ] && ! grep -q service "$H/llamadas" || fallo "b: con la versión ya desplegada hace algo"
sin_compilar b

# c. SHA256SUMS que no cuadra en Forgejo: rechazada, sin contar fallo, y
# registrada una sola vez en dos pasadas.
preparar_servidor
con_forgejo
publicar_forgejo v1.0.0 "$COMMIT_ETIQUETA" sha-malo
actualizar
[ "$RC" = 1 ] || fallo "c: sale con $RC, no con 1"
actualizar
[ -z "$(desplegada)" ] || fallo "c: despliega una versión rechazada"
[ ! -e "$H/releases/v1.0.0" ] || fallo "c: deja la versión rechazada en releases"
[ "$(registro | grep -c 'versión de Forgejo de v1.0.0 rechazada: el sha256')" = 1 ] ||
	fallo "c: no registra el rechazo una sola vez: $(registro)"
[ ! -e "$H/state/intentos" ] && [ ! -e "$H/state/fallida" ] || fallo "c: cuenta la release rechazada como fallo"
# Arreglada la release, se instala en la pasada siguiente.
publicar_forgejo v1.0.0 "$COMMIT_ETIQUETA"
actualizar
[ "$RC" = 0 ] && [ "$(desplegada)" = v1.0.0 ] || fallo "c: no instala la release arreglada: $(registro)"
sin_compilar c

# d. En Forgejo, pero de otro commit: rechazada.
preparar_servidor
con_forgejo
publicar_forgejo v1.0.0 "$OTRO"
actualizar
[ "$RC" = 1 ] || fallo "d: sale con $RC, no con 1"
[ -z "$(desplegada)" ] || fallo "d: despliega una versión de otro commit"
registro | grep -q 'rechazada: el archivo es de otro commit' || fallo "d: no dice por qué la rechaza: $(registro)"
[ ! -e "$H/state/intentos" ] && [ ! -e "$H/state/fallida" ] || fallo "d: cuenta la release rechazada como fallo"
sin_compilar d

# e. Sin la release en Forgejo: la instala de GitHub, con su atestación.
preparar_servidor
con_forgejo
con_github
publicar_github "$REPO_GH" v1.0.0 "$COMMIT_ETIQUETA"
actualizar
[ "$RC" = 0 ] || fallo "e: sale con $RC: $(cat "$H/salida") $(registro)"
[ "$(desplegada)" = v1.0.0 ] || fallo "e: no despliega v1.0.0 de GitHub"
[ "$(cat "$H/releases/v1.0.0/COMMIT" 2>/dev/null)" = "$COMMIT_ETIQUETA" ] || fallo "e: COMMIT no es el de la etiqueta"
registro | grep -q "instalada la versión de GitHub de v1.0.0" || fallo "e: no lo registra: $(registro)"
grep -q "releases/tags/v1.0.0" "$TMP/curl-argv" || fallo "e: no mira Forgejo antes que GitHub"
sin_compilar e

# f. Token de Forgejo legible por otros: Forgejo no disponible (dicho una
# vez), el token no se manda y se instala de GitHub.
preparar_servidor
con_forgejo
chmod 644 "$H/claves/forgejo-token"
con_github
actualizar
actualizar
[ "$RC" = 0 ] || fallo "f: sale con $RC: $(registro)"
[ "$(registro | grep -c 'Forgejo no está disponible')" = 1 ] || fallo "f: no dice una vez que Forgejo no está disponible: $(registro)"
[ ! -s "$TMP/curl-argv" ] && [ ! -s "$TMP/curl-entrada" ] || fallo "f: llama a curl con un token que no vale"
publicar_github "$REPO_GH" v1.0.0 "$COMMIT_ETIQUETA"
publicar_forgejo v1.0.0 "$COMMIT_ETIQUETA"
actualizar
[ "$(desplegada)" = v1.0.0 ] && registro | grep -q "instalada la versión de GitHub de v1.0.0" ||
	fallo "f: no instala de GitHub: $(registro)"
[ ! -s "$TMP/curl-entrada" ] || fallo "f: manda el token de Forgejo"
sin_compilar f

# g. Sin FORGEJO_URL: lo dice una sola vez y mira GitHub.
preparar_servidor
con_github
actualizar
actualizar
[ "$RC" = 0 ] || fallo "g: sale con $RC"
[ "$(registro | grep -c 'Forgejo: sin FORGEJO_URL o FORGEJO_REPO')" = 1 ] || fallo "g: no lo dice una sola vez: $(registro)"
[ ! -s "$TMP/curl-argv" ] || fallo "g: llama a curl sin Forgejo configurado"
publicar_github "$REPO_GH" v1.0.0 "$COMMIT_ETIQUETA"
actualizar
[ "$(desplegada)" = v1.0.0 ] || fallo "g: no instala de GitHub: $(registro)"
sin_compilar g

# h. De GitHub, con la atestación de otra etiqueta: rechazada, una vez.
preparar_servidor
con_github
publicar_github "$REPO_GH" v1.0.0 "$COMMIT_ETIQUETA" refs/tags/v0.9.0
actualizar
[ "$RC" = 1 ] || fallo "h: sale con $RC, no con 1"
actualizar
[ -z "$(desplegada)" ] || fallo "h: despliega una versión con la atestación de otra etiqueta"
[ "$(registro | grep -c 'versión de GitHub de v1.0.0 rechazada: la atestación')" = 1 ] ||
	fallo "h: no dice una sola vez por qué: $(registro)"
[ ! -e "$H/state/intentos" ] || fallo "h: el rechazo cuenta como fallo"
sin_compilar h

# i. GitHub configurado pero la release aún no está: espera, sin fallo.
preparar_servidor
con_github
actualizar
[ "$RC" = 0 ] || fallo "i: sale con $RC: $(cat "$H/salida")"
[ -z "$(desplegada)" ] || fallo "i: despliega algo sin release"
registro | grep -q 'esperando la versión precompilada de v1.0.0' || fallo "i: no dice que espera: $(registro)"
[ ! -e "$H/state/intentos" ] || fallo "i: esperar a GitHub cuenta como fallo"
sin_compilar i

# j. Sin acceso al repositorio.
preparar_servidor
echo "REPO_URL='$H/no-existe'" >>"$H/actualizar.conf"
actualizar
[ "$RC" = 0 ] || fallo "j: sale con $RC"
registro | grep -q 'sin acceso al repositorio' || fallo "j: no lo dice: $(registro)"
sin_compilar j

# k. --ref ya no existe.
preparar_servidor
actualizar --ref main
[ "$RC" = 2 ] || fallo "k: --ref sale con $RC, no con 2"
grep -q 'no compila' "$H/salida" || fallo "k: --ref no explica por qué: $(cat "$H/salida")"
[ -z "$(desplegada)" ] && [ ! -e "$H/llamadas" ] || fallo "k: --ref toca algo"
sin_compilar k

# --- 7. lo que corre en el PC ------------------------------------------------------
L=$TMP/repo-pc
mkdir -p "$L"
cp -R "$DEPLOY" "$L/deploy"
git init -q -b main "$L"
git -C "$L" add deploy
git -C "$L" commit -q -m deploy
git -C "$L" tag v1.0.0
git -C "$L" branch v2.0.0

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
# El buzón ya no existe: ni --entregar ni --ultima.
for opcion in "--release v1.0.0 --entregar x" "--ultima --repo $L"; do
	st=0
	# shellcheck disable=SC2086 # opciones separadas por espacios
	compilar $opcion || st=$?
	[ "$st" = 2 ] || fallo "compilar-freebsd.sh $opcion: sale con $st, no con 2: $(cat "$TMP/salida")"
done

# publicar-forgejo.sh con el Forgejo de mentira y una compilación de
# mentira (RP_COMPILAR) que deja en RP_CROSS_DIR/salida/<versión>/ lo de
# $TMP/pub-fuente/<versión>/ y apunta sus argumentos y su entorno.
P=$TMP/rtf-pub
mkdir -p "$P"
cat >"$TMP/compilar-falso.sh" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$*" >"$TMP/compilar-argv"
env >"$TMP/compilar-env"
mkdir -p "\$RP_CROSS_DIR/salida"
rm -rf "\$RP_CROSS_DIR/salida/\$2"
cp -R "$TMP/pub-fuente/\$2" "\$RP_CROSS_DIR/salida/\$2"
EOF
publicar()
{
	FORGEJO_TOKEN=$TOKEN_FJ FORGEJO_SERVER=$FORJA_URL FORGEJO_REPOSITORY=$REPO_FJ FORGEJO_TAG=$1 \
		RP_CROSS_DIR=$P RP_COMPILAR=$TMP/compilar-falso.sh PATH="$FALSOS:$PATH" \
		bash "$DEPLOY/cross/publicar-forgejo.sh" >"$TMP/salida" 2>&1
}
ARCH2=$(archivo_precompilada v2.0.0)
rm -rf "$FORJA/v2.0.0" "$TMP/pub-fuente"
: >"$TMP/curl-argv"
: >"$TMP/forja-log"
hacer_archivo "$TMP/pub-fuente/v2.0.0" v2.0.0 "$C"
if publicar v2.0.0; then
	grep -qx "creada v2.0.0" "$TMP/forja-log" || fallo "publicar-forgejo.sh no crea la release que falta: $(cat "$TMP/forja-log")"
	[ "$(grep '^subido' "$TMP/forja-log" | tr '\n' ' ')" = "subido $ARCH2 subido SHA256SUMS " ] ||
		fallo "publicar-forgejo.sh no sube el archivo y, el último, SHA256SUMS: $(cat "$TMP/forja-log")"
	cmp -s "$FORJA/v2.0.0/adjuntos/$ARCH2" "$TMP/pub-fuente/v2.0.0/$ARCH2" ||
		fallo "publicar-forgejo.sh no sube el archivo compilado"
	grep -q "$C" "$FORJA/v2.0.0/cuerpo" || fallo "la release no dice de qué commit es: $(cat "$FORJA/v2.0.0/cuerpo")"
	[ "$(cat "$TMP/compilar-argv")" = "--release v2.0.0 --repo $FORJA_URL/$REPO_FJ.git" ] ||
		fallo "publicar-forgejo.sh compila con otros argumentos: $(cat "$TMP/compilar-argv")"
	grep -qx "GIT_CONFIG_VALUE_0=Authorization: token $TOKEN_FJ" "$TMP/compilar-env" &&
		grep -qx "GIT_CONFIG_KEY_0=http.extraheader" "$TMP/compilar-env" ||
		fallo "publicar-forgejo.sh no pasa a git el token por GIT_CONFIG_*"
	if grep -q '^FORGEJO_TOKEN=' "$TMP/compilar-env"; then
		fallo "FORGEJO_TOKEN llega al entorno de la compilación"
	fi
else
	fallo "publicar-forgejo.sh falla: $(cat "$TMP/salida")"
fi
# Otra vez (el runner repite el trabajo): la misma release, los adjuntos
# sustituidos.
: >"$TMP/forja-log"
hacer_archivo "$TMP/pub-fuente/v2.0.0" v2.0.0 "$OTRO"
if publicar v2.0.0; then
	if grep -q '^creada' "$TMP/forja-log"; then fallo "publicar-forgejo.sh crea otra vez la release"; fi
	grep -qx "borrado $ARCH2" "$TMP/forja-log" && grep -qx "borrado SHA256SUMS" "$TMP/forja-log" ||
		fallo "publicar-forgejo.sh no borra los adjuntos que ya estaban: $(cat "$TMP/forja-log")"
	cmp -s "$FORJA/v2.0.0/adjuntos/$ARCH2" "$TMP/pub-fuente/v2.0.0/$ARCH2" ||
		fallo "publicar-forgejo.sh no sustituye el archivo"
	[ "$(awk '{ print $2 }' "$FORJA/v2.0.0/adjuntos.ids" | sort | tr '\n' ' ')" = "$ARCH2 SHA256SUMS " ] ||
		fallo "la release no queda con un archivo y un SHA256SUMS: $(cat "$FORJA/v2.0.0/adjuntos.ids")"
else
	fallo "publicar-forgejo.sh falla al repetir: $(cat "$TMP/salida")"
fi
if grep -q "$TOKEN_FJ" "$TMP/curl-argv" "$TMP/compilar-argv"; then
	fallo "el token aparece en los argumentos de curl o de la compilación"
fi
grep -q "Authorization: token $TOKEN_FJ" "$TMP/curl-entrada" || fallo "publicar-forgejo.sh no manda el token por -K -"
# Lo que no es una versión no se compila.
rm -f "$TMP/compilar-argv"
if publicar main; then fallo "publicar-forgejo.sh publica «main»"; fi
grep -q "no es una versión" "$TMP/salida" || fallo "publicar-forgejo.sh main: $(cat "$TMP/salida")"
[ ! -e "$TMP/compilar-argv" ] || fallo "publicar-forgejo.sh compila «main»"

# empaquetar-freebsd.sh (el runner del PC y la release de GitHub): lo que deja lo acepta
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
for f in "$LIB" "$DEPLOY/lib/git.sh" "$DEPLOY/freebsd-jail/actualizar.sh"; do
	sh -n "$f" || fallo "sh -n ${f#"$DEPLOY"/}"
done
for f in "$DEPLOY"/cross/*.sh; do
	bash -n "$f" || fallo "bash -n ${f#"$DEPLOY"/}"
done
if command -v shellcheck >/dev/null 2>&1; then
	shellcheck -S warning -x "$LIB" "$DEPLOY/lib/git.sh" "$DEPLOY/freebsd-jail/actualizar.sh" "$DEPLOY"/cross/*.sh ||
		fallo "shellcheck"
else
	echo "aviso: no hay shellcheck" >&2
fi

if [ "$fallos" -ne 0 ]; then
	echo "$fallos fallos" >&2
	exit 1
fi
echo "precompiladas: todo bien"
