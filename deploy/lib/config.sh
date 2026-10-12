# shellcheck shell=sh
#
# Configuración de la instalación (deploy/config.example, deploy/config.local).
# Se carga con «.» después de poner DEPLOY_DIR (el directorio deploy/).
#
# Los mensajes de error nombran la clave, nunca su valor: la configuración
# lleva datos de la red de quien instala.

CLAVES_OBLIGATORIAS="SERVIDOR_SSH JAILS_RAIZ ZFS_DATASET JAIL_PANEL JAIL_PANEL_IP
	JAIL_COMPILAR JAIL_COMPILAR_IP UID_COMPILAR INTERFAZ_EXT RED_LAN ROUTER_IP DNS_IP
	PANEL_ORIGEN PANEL_ESCUCHA PROXIES_CONFIANZA REPO_URL MODULOS"
CLAVES_OPCIONALES="RED_VPN REPO_HOST_CLAVE COMPROBAR_URLS TRABAJOS_COMPILACION"
MODULOS_CONOCIDOS="pf rc actualizador caddyfile unbound"

# ¿Es $1 una IPv4 escrita como cuatro números de 0 a 255?
_es_ipv4()
{
	echo "$1" | awk -F. '
		NF != 4 { exit 1 }
		{ for (i = 1; i <= 4; i++) if ($i !~ /^[0-9]+$/ || $i > 255) exit 1 }
	'
}

# Carga la configuración $1 (por defecto $DEPLOY_DIR/config.local) y exporta
# sus claves. Antes borra las del entorno, para que una clave que falta en el
# fichero no se tome de fuera. El fichero puede empezar con
#   . "$CONFIG_EJEMPLO"
# para tomar los valores del ejemplo y cambiar solo los propios. Falla, con
# un mensaje que nombra cada clave que falta o no es válida, si alguna
# obligatoria está vacía o tiene una forma imposible.
cargar_config()
{
	_config=${1:-$DEPLOY_DIR/config.local}
	if [ ! -r "$_config" ]; then
		echo "ERROR: no se puede leer la configuración ($(basename "$_config")); copia deploy/config.example a deploy/config.local y ajústala" >&2
		return 1
	fi
	for _k in $CLAVES_OBLIGATORIAS $CLAVES_OPCIONALES; do
		unset "$_k"
	done
	CONFIG_EJEMPLO=$DEPLOY_DIR/config.example
	case $_config in
	*/*) . "$_config" ;;
	*) . "./$_config" ;;
	esac

	_faltan=
	for _k in $CLAVES_OBLIGATORIAS; do
		eval "_v=\${$_k-}"
		[ -n "$_v" ] || _faltan="$_faltan $_k"
	done
	if [ -n "$_faltan" ]; then
		echo "ERROR: falta en la configuración:$_faltan" >&2
		return 1
	fi

	_malas=
	for _k in JAIL_PANEL_IP JAIL_COMPILAR_IP ROUTER_IP DNS_IP; do
		eval "_v=\$$_k"
		_es_ipv4 "$_v" || _malas="$_malas $_k"
	done
	case $UID_COMPILAR in
	*[!0-9]*) _malas="$_malas UID_COMPILAR" ;;
	esac
	# Vacía (cargo decide) o un entero positivo.
	case ${TRABAJOS_COMPILACION-} in
	'') ;;
	*[!0-9]* | 0*) _malas="$_malas TRABAJOS_COMPILACION" ;;
	esac
	case $PANEL_ORIGEN in
	http://?* | https://?*) ;;
	*) _malas="$_malas PANEL_ORIGEN" ;;
	esac
	# IP:puerto (la IPv6 entre corchetes), como RP_LISTEN.
	case ${PANEL_ESCUCHA##*:} in
	'' | *[!0-9]*) _malas="$_malas PANEL_ESCUCHA" ;;
	esac
	case $PANEL_ESCUCHA in
	?*:?*) ;;
	*) _malas="$_malas PANEL_ESCUCHA" ;;
	esac
	for _m in $MODULOS; do
		case " $MODULOS_CONOCIDOS " in
		*" $_m "*) ;;
		*) _malas="$_malas MODULOS" ;;
		esac
	done
	if [ -n "$_malas" ]; then
		echo "ERROR: no es válido en la configuración:$_malas" >&2
		return 1
	fi

	for _k in $CLAVES_OBLIGATORIAS $CLAVES_OPCIONALES; do
		eval "$_k=\${$_k-}"
		# shellcheck disable=SC2163 # se exporta la variable que nombra $_k
		export "$_k"
	done
}

# ¿Está el módulo $1 en MODULOS?
tiene_modulo()
{
	case " $MODULOS " in
	*" $1 "*) return 0 ;;
	esac
	return 1
}

# Escribe las claves $@ (todas si no se nombra ninguna) como CLAVE='valor',
# listas para cargarlas con «.» en otra máquina.
volcar_config()
{
	if [ "$#" -eq 0 ]; then
		# shellcheck disable=SC2086 # lista de nombres separados por espacios
		set -- $CLAVES_OBLIGATORIAS $CLAVES_OPCIONALES
	fi
	for _k in "$@"; do
		eval "_v=\${$_k-}"
		printf "%s='%s'\n" "$_k" "$(printf '%s' "$_v" | sed "s/'/'\\\\''/g")"
	done
}
