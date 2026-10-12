# shellcheck shell=sh
#
# Plantillas con marcas @CLAVE@. Se carga con «.».

# Copia la plantilla $1 en $2 cambiando cada @CLAVE@ por el valor de la
# variable de entorno CLAVE (tiene que estar exportada; vacía vale). Falla, y
# no deja $2, si queda alguna @…@ sin sustituir (una variable que no existe o
# un error en la plantilla). Los valores se copian tal cual: «$», «\» y «&»
# no tienen ningún significado especial.
rellenar()
{
	_plantilla=$1
	_destino=$2
	if ! awk '
		{
			linea = $0
			salida = ""
			while (match(linea, /@[A-Za-z_][A-Za-z0-9_]*@/)) {
				clave = substr(linea, RSTART + 1, RLENGTH - 2)
				if (clave in ENVIRON) {
					salida = salida substr(linea, 1, RSTART - 1) ENVIRON[clave]
				} else {
					salida = salida substr(linea, 1, RSTART + RLENGTH - 1)
				}
				linea = substr(linea, RSTART + RLENGTH)
			}
			print salida linea
		}
	' "$_plantilla" >"$_destino"; then
		rm -f "$_destino"
		echo "ERROR: no se puede rellenar $(basename "$_plantilla")" >&2
		return 1
	fi
	_restos=$(grep -o '@[A-Za-z_][A-Za-z0-9_]*@' "$_destino" | sort -u | tr '\n' ' ')
	if [ -n "$_restos" ]; then
		rm -f "$_destino"
		echo "ERROR: $(basename "$_plantilla"): sin valor para ${_restos% }" >&2
		return 1
	fi
}
