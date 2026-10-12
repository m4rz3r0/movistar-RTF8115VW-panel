# shellcheck shell=sh
#
# Clon del repositorio que sigue al remoto por etiquetas. Se carga con «.»;
# solo usa git y sort, así que el actualizador también puede mandar este
# fichero tal cual a la jail de compilación.
#
# Nunca hace merge: el clon se pone en la etiqueta con checkout --detach, así
# que da igual que el remoto haya reescrito su historial (rama huérfana,
# etiquetas borradas o movidas).

# Trae ramas y etiquetas de origin al clon $1: --force acepta etiquetas
# movidas y --prune/--prune-tags borran las que el remoto ya no tiene.
traer_referencias()
{
	git -C "$1" fetch --quiet --force --prune --prune-tags --tags origin
}

# Trae las referencias, pone el clon $1 en la última etiqueta v* (sort -V) y
# la escribe en la salida estándar. Sin ninguna etiqueta v* no escribe nada
# ni cambia el clon. Sale con error si no se puede traer o sacar.
traer_ultima_etiqueta()
{
	traer_referencias "$1" || return 1
	_etiqueta=$(git -C "$1" tag -l 'v*' | sort -V | tail -n 1)
	if [ -z "$_etiqueta" ]; then
		return 0
	fi
	git -C "$1" -c advice.detachedHead=false checkout --quiet --force --detach \
		"refs/tags/$_etiqueta" || return 1
	echo "$_etiqueta"
}
