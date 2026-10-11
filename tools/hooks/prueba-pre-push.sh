#!/bin/sh
# Prueba del gancho pre-push con repositorios temporales.
#
# Necesita RP_COMPROBAR_PUBLICACION (el binario de comprobar-publicacion);
# la lanza la prueba de Rust `pre_push_rechaza_historial_privado`.
#
# 1. Un repositorio A con un remoto local vacío y su raíz guardada en
#    .git/routerpanel-raices-privadas: su push falla.
# 2. Una rama huérfana con un árbol limpio: su push pasa.
# 3. Un commit encima con un valor de la lista de prohibidos: el push falla.
# 4. Un commit limpio que además borra un fichero: el push (por --diff) pasa.
# 5. Una etiqueta anotada con un valor prohibido en su mensaje: el push falla.
set -eu

: "${RP_COMPROBAR_PUBLICACION:?falta RP_COMPROBAR_PUBLICACION}"
export RP_COMPROBAR_PUBLICACION

ganchos=$(cd "$(dirname "$0")" && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# Entorno aislado: sin configuración del usuario ni listas del dueño.
export HOME="$tmp/casa"
unset XDG_CONFIG_HOME
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=prueba GIT_AUTHOR_EMAIL=prueba@users.noreply.github.com
export GIT_COMMITTER_NAME=prueba GIT_COMMITTER_EMAIL=prueba@users.noreply.github.com
mkdir -p "$HOME"
printf 'zafiro-escondido\n' >"$tmp/prohibidos.txt"
export RP_PUBLICACION_PROHIBIDOS="$tmp/prohibidos.txt"

fallo() {
    echo "FALLO: $*" >&2
    exit 1
}

git init -q --bare "$tmp/remoto.git"
git init -q -b main "$tmp/a"
cd "$tmp/a"
git config core.hooksPath "$ganchos"
git remote add origin "$tmp/remoto.git"
echo 'texto neutro' >a.txt
echo 'otro texto neutro' >b.txt
git add -A
git commit -qm 'historial privado'
git rev-list --max-parents=0 HEAD >"$(git rev-parse --git-common-dir)/routerpanel-raices-privadas"

# 1. El historial privado no sale.
if git push -q origin main 2>"$tmp/err"; then
    fallo "el push con la raíz privada pasó"
fi
grep -q 'esta rama contiene el historial privado' "$tmp/err" ||
    fallo "el rechazo no lo explica: $(cat "$tmp/err")"
if git ls-remote "$tmp/remoto.git" | grep -q .; then
    fallo "el remoto recibió algo"
fi

# 2. Rama huérfana con un árbol limpio.
git checkout -q --orphan publica
git commit -qm 'Panel web'
git push -q origin publica 2>"$tmp/err" || fallo "el push limpio falló: $(cat "$tmp/err")"

# 3. Un valor prohibido encima de lo ya publicado.
echo 'zafiro-escondido' >c.txt
git add -A
git commit -qm 'con un dato'
if git push -q origin publica 2>"$tmp/err" >"$tmp/out"; then
    fallo "el push con un valor prohibido pasó"
fi
if grep -qi zafiro "$tmp/out" "$tmp/err"; then
    fallo "la salida contiene el valor"
fi
git reset -q --hard HEAD~1

# 4. Un commit limpio que borra un fichero: el gancho no falla por el borrado.
git rm -q b.txt
echo 'más texto neutro' >d.txt
git add -A
git commit -qm 'borra b y añade d'
git push -q origin publica 2>"$tmp/err" || fallo "el push con un borrado falló: $(cat "$tmp/err")"

# 5. Una etiqueta anotada con el valor en su mensaje: el push falla.
git tag -a v1 -m 'versión con zafiro-escondido'
if git push -q origin v1 2>"$tmp/err" >"$tmp/out"; then
    fallo "el push de una etiqueta con un valor prohibido pasó"
fi
if grep -qi zafiro "$tmp/out" "$tmp/err"; then
    fallo "la salida contiene el valor"
fi

# Borrar una rama remota tampoco necesita revisión.
git push -q origin :publica 2>"$tmp/err" || fallo "el borrado de la rama remota falló: $(cat "$tmp/err")"

echo "prueba-pre-push: correcto"
