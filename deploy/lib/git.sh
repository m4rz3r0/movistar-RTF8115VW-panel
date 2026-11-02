# shellcheck shell=sh
#
# La última versión del repositorio sin clonarlo: el actualizador
# (deploy/freebsd-jail/actualizar.sh) lanza `git ls-remote --tags` contra
# REPO_URL desde el host y le pasa la salida a ultima_etiqueta_remota. No hay
# clon ni checkout, y no se ejecuta nada del repositorio. Se carga con «.»;
# solo funciones, sin efectos al cargarse.
#
# Cada pasada lee lo que el remoto tiene en ese momento, así que da igual que
# haya reescrito su historial (rama huérfana, etiquetas borradas o movidas).

# Lee por la entrada estándar la salida de `git ls-remote --tags` («<commit>
# TAB refs/tags/<nombre>», más «refs/tags/<nombre>^{}» en las anotadas) y
# escribe «<etiqueta> <commit>» de la última vX.Y.Z (solo cifras, por
# sort -V: la misma que ultima_version de deploy/lib/precompiladas.sh). El
# commit es el de la línea pelada (^{}) si la etiqueta es anotada y, si no,
# el de la propia referencia; siempre 40 cifras hexadecimales en minúscula.
# Sin ninguna vX.Y.Z no escribe nada.
ultima_etiqueta_remota()
{
	awk '
		function sha(s) { return length(s) == 40 && s !~ /[^0-9a-f]/ }
		NF == 2 && sha($1) && $2 ~ /^refs\/tags\/v[0-9]+\.[0-9]+\.[0-9]+(\^\{\})?$/ {
			n = substr($2, 11)
			if (n ~ /\^\{\}$/) {
				pelada[substr(n, 1, length(n) - 3)] = $1
			} else {
				directa[n] = $1
			}
		}
		END {
			for (n in directa) print n, (n in pelada ? pelada[n] : directa[n])
		}
	' | sort -V | tail -n 1
}
