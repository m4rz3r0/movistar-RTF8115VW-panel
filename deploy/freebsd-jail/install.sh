#!/bin/sh
# shellcheck disable=SC2016,SC2029 # guiones que se expanden en la jail o en el servidor
#
# Instala el panel del router en una jail de un servidor FreeBSD, con su
# actualizador en el host. El servidor nunca compila: las versiones llegan ya
# compiladas (buzón del PC de compilación o release de GitHub). Desde el
# equipo de trabajo, en el repositorio:
#
#   deploy/freebsd-jail/install.sh [--config FICHERO]
#   deploy/freebsd-jail/install.sh [--config FICHERO] --plantillas DIR
#
# La configuración sale de deploy/config.local (o de --config; ver
# deploy/config.example). Con --plantillas solo genera en DIR los ficheros
# del host (jail.conf, reglas de pf, rc.conf de la jail…) para revisarlos, sin
# conectarse a nada.
#
# Sin --plantillas es idempotente: se puede repetir y sólo hace lo que falte.
# Sube deploy/ (con la configuración ya resuelta) a ~/routerpanel-deploy en
# SERVIDOR_SSH, y allí, como root (doas), según MODULOS:
#
#   --servidor      siempre: la jail del panel en /etc/jail.conf.
#                   rc: base PkgBase de la jail del panel, usuario, nullfs de
#                   releases, rc.d con sus variables y newsyslog; si el panel
#                   aún no tiene usuarios, dice dónde está el token del
#                   asistente de instalación (/instalar).
#                   actualizador: en el host, el actualizador y su cron, la
#                   clave de despliegue y el known_hosts del servidor git
#                   (para git ls-remote), y gh y jq si hay GITHUB_REPO.
#                   unbound: el nombre del panel en el unbound del host.
#                   caddyfile: el sitio para el Caddyfile (lo imprime; no lo instala).
#   --pf-cargar     (pf) mete las reglas en /etc/pf.conf y las carga con un
#                   interruptor de hombre muerto: si nadie lo desarma en 180 s,
#                   vuelve a cargar la copia anterior.
#   --pf-confirmar  desarma el interruptor.
#   --comprobar-red qué alcanzan la jail del panel y el actualizador con las
#                   reglas cargadas (el equipo de trabajo lo lanza antes de
#                   desarmar).
#
# Entre --pf-cargar y --pf-confirmar, el equipo de trabajo comprueba por una
# conexión SSH NUEVA que el servidor sigue respondiendo (y las webs de
# COMPROBAR_URLS), y con --comprobar-red lo que alcanzan la jail del panel y
# el actualizador; si algo falla no confirma y el servidor vuelve solo a las
# reglas anteriores.
#
# El primer despliegue es el de la primera etiqueta vX.Y.Z con su versión
# precompilada; después se abre PANEL_ORIGEN/instalar con el token que
# imprime install.sh si se repite (o el registro del panel).

set -eu

DIR=$(cd "$(dirname "$0")" && pwd)
DEPLOY_DIR=$(dirname "$DIR")
. "$DEPLOY_DIR/lib/config.sh"
. "$DEPLOY_DIR/lib/plantillas.sh"

STAGE=routerpanel-deploy
# En el servidor, los ficheros generados con la configuración (pueden llevar
# datos del dueño). Los fija plantillas_de_root(): un temporal de root, nunca la
# carpeta de subida del usuario.
PLANTILLAS=
RELEASES=/usr/local/routerpanel/releases
# uid/gid del usuario del servicio en la jail del panel.
UID_PANEL=3159
# Token del asistente de instalación: lo escribe el panel al arrancar sin
# usuarios, junto a su base de datos (ruta en la jail).
TOKEN=/var/db/routerpanel/instalar.token
DEADMAN_PID=/var/run/routerpanel-pf-deadman.pid
# Lo crea el interruptor al saltar (daemon -p borra el pidfile cuando el sh
# termina, así que sin esto no se distingue «saltó» de «nunca se armó»).
DEADMAN_FIRED=/var/run/routerpanel-pf-deadman.saltado
PF_BACKUP=/etc/pf.conf.bak-routerpanel
# Lo que el actualizador necesita en el host.
ACTUALIZADOR_CONF=/usr/local/etc/routerpanel-actualizar.conf
ACTUALIZADOR_LIB=/usr/local/libexec/routerpanel-actualizar
# Clave de despliegue y known_hosts del servidor git, de root (0700).
ACTUALIZADOR_CLAVES=/usr/local/etc/routerpanel-actualizar
# Jail de compilación de instalaciones anteriores: solo para llevarse su
# clave de despliegue al host y avisar de que ya no se usa.
JAIL_COMPILAR_ANTIGUA=routerpanel-build

say()
{
	echo "==> $*"
}

die()
{
	echo "ERROR: $*" >&2
	exit 1
}

uso()
{
	echo "uso: $0 [--config FICHERO] [--plantillas DIR]" >&2
	exit 2
}

# ----------------------------------------------------------- configuración --

# Carga la configuración $1 (vacío: deploy/config.local) y calcula lo que
# sale de ella: nombre y ruta de la jail y los valores derivados que usan las
# plantillas (exportados, como las claves).
configurar()
{
	cargar_config "$1" || exit 1
	RUN_JAIL=$JAIL_PANEL
	RJ=$JAILS_RAIZ/$RUN_JAIL

	PANEL_PUERTO=${PANEL_ESCUCHA##*:}
	# Lista de pf: una sola dirección tal cual; varias, entre llaves.
	_proxies=$(echo "$PROXIES_CONFIANZA" | tr ',' ' ' | xargs)
	case $_proxies in
	*' '*) PROXIES_PF="{ $_proxies }" ;;
	*) PROXIES_PF=$_proxies ;;
	esac
	# El primer proxy, si es una IPv4 que no sea del propio equipo: a él
	# apunta el nombre del panel (unbound) y no le deben llegar las jails.
	PROXY_IP=${_proxies%% *}
	case $PROXY_IP in
	127.* | */*) PROXY_IP= ;;
	esac
	if [ -n "$PROXY_IP" ] && ! _es_ipv4 "$PROXY_IP"; then
		PROXY_IP=
	fi
	export PANEL_PUERTO PROXIES_PF
}

# Genera en $1 los ficheros del host a partir de la configuración cargada.
generar_plantillas()
{
	_dst=$1
	mkdir -p "$_dst"
	rellenar "$DIR/jail.conf.plantilla" "$_dst/jail.conf" || exit 1
	rellenar "$DIR/pf.conf.plantilla" "$_dst/pf.conf" || exit 1
	# Variables de rc.conf de la jail del panel (sysrc, una por línea).
	{
		echo "routerpanel_enable=YES"
		echo "routerpanel_origin=$PANEL_ORIGEN"
		echo "routerpanel_listen=$PANEL_ESCUCHA"
		echo "routerpanel_trusted_proxy=$PROXIES_CONFIANZA"
	} >"$_dst/rc.conf"
	{
		volcar_config JAILS_RAIZ JAIL_PANEL REPO_URL GITHUB_REPO PRECOMPILADAS_MINISIGN
		echo "CLAVE_DESPLIEGUE='$ACTUALIZADOR_CLAVES/id_ed25519'"
		echo "HOSTS_CONOCIDOS='$ACTUALIZADOR_CLAVES/known_hosts'"
	} >"$_dst/actualizador.conf"
	# La clave del servidor git fijada: la de REPO_HOST_CLAVE o, sin ella, la
	# primera que presente (setup_updater).
	if [ -n "$REPO_HOST_CLAVE" ]; then
		echo "$REPO_HOST_CLAVE" >"$_dst/known_hosts"
	else
		rm -f "$_dst/known_hosts"
	fi
	if tiene_modulo caddyfile; then
		{
			echo "# Sitio del panel del router: añádelo al Caddyfile."
			echo "$(echo "$PANEL_ORIGEN" | tr ',' ' ' | xargs | sed 's/ /, /g') {"
			echo "	reverse_proxy $JAIL_PANEL_IP:$PANEL_PUERTO"
			echo "}"
		} >"$_dst/Caddyfile.routerpanel"
	fi
	if tiene_modulo unbound; then
		_ip=${PROXY_IP:-$JAIL_PANEL_IP}
		{
			echo "# Nombre del panel del router (lo instala install.sh)."
			echo "server:"
			for _origen in $(echo "$PANEL_ORIGEN" | tr ',' ' '); do
				_equipo=${_origen#*://}
				_equipo=${_equipo%:*}
				echo "	local-data: \"$_equipo. 3600 IN A $_ip\""
			done
		} >"$_dst/unbound.conf"
	fi
}

# ---------------------------------------------------------------- servidor --

check_syntax()
{
	for f in "$DIR/install.sh" "$DIR/actualizar.sh" "$DIR/recibir-precompilada.sh" \
		"$DIR/routerpanel.rc" "$DEPLOY_DIR"/lib/*.sh; do
		sh -n "$f" || die "error de sintaxis en $(basename "$f")"
	done
}

setup_jail_conf()
{
	if grep -q "^$RUN_JAIL {" /etc/jail.conf; then
		return
	fi
	say "jail.conf: añadiendo $RUN_JAIL"
	cp -p /etc/jail.conf /etc/jail.conf.bak-routerpanel
	cat "$PLANTILLAS/jail.conf" >>/etc/jail.conf
}

# Crea el dataset de la jail $1 y le instala el conjunto de base $2 (PkgBase).
create_jail()
{
	_name=$1
	_set=$2
	_root=$JAILS_RAIZ/$_name
	if ! zfs list "$ZFS_DATASET/$_name" >/dev/null 2>&1; then
		say "$_name: creando $ZFS_DATASET/$_name"
		zfs create "$ZFS_DATASET/$_name"
	fi
	if ! pkg --rootdir "$_root" info -e "$_set" 2>/dev/null; then
		say "$_name: instalando $_set"
		# pkg busca las claves de PkgBase dentro de la raíz nueva.
		mkdir -p "$_root/usr/share/keys"
		cp -Rp "/usr/share/keys/pkgbase-$(freebsd-version -u | cut -d. -f1)" "$_root/usr/share/keys/"
		pkg --rootdir "$_root" install -y -r FreeBSD-base "$_set"
	fi
	mkdir -p "$_root/usr/local/etc/pkg/repos"
	cp /usr/local/etc/pkg/repos/FreeBSD.conf "$_root/usr/local/etc/pkg/repos/FreeBSD.conf"
	cp /etc/localtime "$_root/etc/localtime"
}

start_jail()
{
	if ! jls -j "$1" jid >/dev/null 2>&1; then
		say "$1: arrancando"
		service jail start "$1"
	fi
}

# uid:gid del usuario $2 en la jail con raíz $1.
user_ids()
{
	pw -R "$1" usershow "$2" | cut -d: -f3,4
}

setup_runtime_jail()
{
	create_jail "$RUN_JAIL" FreeBSD-set-minimal-jail
	if ! pw -R "$RJ" usershow routerpanel >/dev/null 2>&1; then
		say "$RUN_JAIL: usuario routerpanel"
		# uid/gid propios, que no coincidan con ningún usuario del host.
		pw -R "$RJ" groupadd routerpanel -g "$UID_PANEL"
		pw -R "$RJ" useradd routerpanel -u "$UID_PANEL" -g routerpanel -c "Panel del router" \
			-d /nonexistent -s /usr/sbin/nologin
	fi
	_ids=$(user_ids "$RJ" routerpanel)

	# releases vive en el host y entra en la jail por nullfs, sólo lectura.
	mkdir -p "$RELEASES" "$RJ/usr/local/routerpanel/releases"
	_fstab="$RELEASES	$RJ/usr/local/routerpanel/releases	nullfs	ro,late		0	0"
	if ! grep -q "^${RELEASES}[[:space:]]" /etc/fstab; then
		say "fstab: nullfs de $RELEASES"
		echo "$_fstab" >>/etc/fstab
	fi
	if ! mount -p | grep -q "[[:space:]]$RJ/usr/local/routerpanel/releases[[:space:]]"; then
		mount -t nullfs -o ro "$RELEASES" "$RJ/usr/local/routerpanel/releases"
	fi

	# La base de datos y la clave del historial las crea el panel: sus
	# directorios son del usuario del servicio.
	install -d -m 0700 -o "${_ids%:*}" -g "${_ids#*:}" "$RJ/var/db/routerpanel" "$RJ/usr/local/etc/routerpanel"
	# Ese directorio es del usuario del servicio: lo que se escribe dentro se
	# escribe como él y desde dentro de la jail, para que un enlace simbólico
	# puesto ahí no pueda llevar una escritura de root a otro sitio.
	start_jail "$RUN_JAIL"
	# router.secret ya no se crea: las credenciales del router las guarda el
	# asistente de instalación en la base de datos. El de una instalación
	# anterior se respeta (routerpanel.rc lo pasa al panel, que lo importa
	# una vez; después se puede borrar).
	if [ -e "$RJ/usr/local/etc/routerpanel/router.secret" ]; then
		jexec -U routerpanel "$RUN_JAIL" /bin/chmod 0600 /usr/local/etc/routerpanel/router.secret
	fi

	mkdir -p "$RJ/usr/local/etc/rc.d"
	install -m 0755 -o root -g wheel "$DIR/routerpanel.rc" "$RJ/usr/local/etc/rc.d/routerpanel"
	while IFS= read -r _var; do
		sysrc -q -R "$RJ" "$_var" >/dev/null
	done <"$PLANTILLAS/rc.conf"
	# El log lo escribe daemon(8) como root; -H lo reabre tras rotar.
	mkdir -p "$RJ/etc/newsyslog.conf.d"
	echo "/var/log/routerpanel.log	root:wheel	600	7	1000	*	JC	/var/run/routerpanel.pid	1" \
		>"$RJ/etc/newsyslog.conf.d/routerpanel.conf"
}

# git ls-remote de REPO_URL desde el host, como lo hace el actualizador, con
# StrictHostKeyChecking=$1 (yes, o accept-new para fijar la primera clave).
ls_remote_host()
{
	GIT_SSH_COMMAND="ssh -F /dev/null -o BatchMode=yes -o ConnectTimeout=10 -o IdentitiesOnly=yes -i $ACTUALIZADOR_CLAVES/id_ed25519 -o StrictHostKeyChecking=$1 -o UserKnownHostsFile=$ACTUALIZADOR_CLAVES/known_hosts" \
		GIT_TERMINAL_PROMPT=0 GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null \
		timeout 60 git ls-remote --tags -- "$REPO_URL"
}

# Clave de despliegue y known_hosts del servidor git, en el host y de root.
# La clave: la que ya hubiera; si no, la de la jail de compilación de una
# instalación anterior (leída dentro de esa jail como su usuario, para que un
# enlace simbólico de allí no lleve a root a otro fichero, y comprobada con
# ssh-keygen); si no, una nueva.
setup_clave_despliegue()
{
	install -d -m 0700 -o root -g wheel "$ACTUALIZADOR_CLAVES"
	_k=$ACTUALIZADOR_CLAVES/id_ed25519
	if [ ! -f "$_k" ] && jls -j "$JAIL_COMPILAR_ANTIGUA" jid >/dev/null 2>&1; then
		_t=$(mktemp "$ACTUALIZADOR_CLAVES/.clave.XXXXXX")
		if jexec -U rpbuild "$JAIL_COMPILAR_ANTIGUA" /bin/cat /home/rpbuild/.ssh/id_ed25519 >"$_t" 2>/dev/null &&
			ssh-keygen -y -f "$_t" >"$_t.pub" 2>/dev/null; then
			say "actualizador: clave de despliegue traída de $JAIL_COMPILAR_ANTIGUA"
			mv "$_t.pub" "$_k.pub"
			mv "$_t" "$_k"
		fi
		rm -f "$_t" "$_t.pub"
	fi
	if [ ! -f "$_k" ]; then
		say "actualizador: clave de despliegue nueva"
		ssh-keygen -q -t ed25519 -N "" -C "routerpanel-actualizar@$(hostname -s)" -f "$_k"
	fi
	chmod 600 "$_k"
	chmod 644 "$_k.pub"

	_h=$ACTUALIZADOR_CLAVES/known_hosts
	if [ -f "$PLANTILLAS/known_hosts" ]; then
		install -m 0644 -o root -g wheel "$PLANTILLAS/known_hosts" "$_h"
	elif [ ! -s "$_h" ]; then
		case $REPO_URL in
		https://* | http://*) : >"$_h" ;;
		*)
			# Sin REPO_HOST_CLAVE: se fija la primera clave que presente.
			if ls_remote_host accept-new >/dev/null 2>&1 || [ -s "$_h" ]; then
				say "actualizador: clave del servidor git fijada en $_h (compárala con la del servidor):"
				ssh-keygen -lf "$_h" || true
			else
				echo "    aviso: no se llega al servidor git para fijar su clave; repite install.sh o pon REPO_HOST_CLAVE"
			fi
			;;
		esac
	fi
}

# Herramientas del host que necesita el actualizador: git siempre; gh y jq
# solo con GITHUB_REPO (comprobar la atestación de la release).
setup_paquetes_host()
{
	_p=git
	if [ -n "$GITHUB_REPO" ]; then
		_p="$_p gh jq"
	fi
	for _x in $_p; do
		if ! pkg info -e "$_x" 2>/dev/null; then
			say "host: instalando $_x"
			pkg install -y "$_x"
		fi
	done
}

# Si queda la jail de compilación de una instalación anterior, avisa: ya no
# se usa (README, «Migrar una instalación existente»).
aviso_jail_compilar()
{
	if jls -j "$JAIL_COMPILAR_ANTIGUA" jid >/dev/null 2>&1 ||
		grep -q "^$JAIL_COMPILAR_ANTIGUA {" /etc/jail.conf 2>/dev/null; then
		say "la jail $JAIL_COMPILAR_ANTIGUA ya no se usa: el servidor no compila. Cuando el actualizador funcione sin ella, bórrala (deploy/freebsd-jail/README.md, «Migrar una instalación existente»)"
	fi
}

setup_updater()
{
	setup_paquetes_host
	setup_clave_despliegue
	install -m 0755 -o root -g wheel "$DIR/actualizar.sh" /usr/local/sbin/routerpanel-actualizar
	install -d -m 0755 -o root -g wheel "$ACTUALIZADOR_LIB"
	install -m 0644 -o root -g wheel "$DEPLOY_DIR/lib/git.sh" "$ACTUALIZADOR_LIB/git.sh"
	install -m 0644 -o root -g wheel "$DEPLOY_DIR/lib/precompiladas.sh" "$ACTUALIZADOR_LIB/precompiladas.sh"
	install -m 0755 -o root -g wheel "$DIR/recibir-precompilada.sh" "$ACTUALIZADOR_LIB/recibir-precompilada.sh"
	install -m 0600 -o root -g wheel "$PLANTILLAS/actualizador.conf" "$ACTUALIZADOR_CONF"
	mkdir -p /usr/local/etc/cron.d /usr/local/etc/newsyslog.conf.d
	cat >/usr/local/etc/cron.d/routerpanel <<-'EOF'
		# Actualizador del panel del router (repositorio movistar-RTF8115VW-panel,
		# deploy/freebsd-jail/actualizar.sh). Registro en /var/log/routerpanel-update.log.
		*/15	*	*	*	*	root	/usr/local/sbin/routerpanel-actualizar
	EOF
	cat >/usr/local/etc/newsyslog.conf.d/routerpanel.conf <<-'EOF'
		/var/log/routerpanel-update.log	root:wheel	640	5	1000	*	JC
	EOF
}

setup_unbound()
{
	_conf=/var/unbound/conf.d/routerpanel.conf
	if cmp -s "$PLANTILLAS/unbound.conf" "$_conf"; then
		return
	fi
	say "unbound: nombre del panel"
	mkdir -p /var/unbound/conf.d
	if [ -f "$_conf" ]; then
		cp -p "$_conf" "$_conf.bak-routerpanel"
	fi
	install -m 0644 -o root -g wheel "$PLANTILLAS/unbound.conf" "$_conf"
	if ! local-unbound-checkconf >/dev/null; then
		if [ -f "$_conf.bak-routerpanel" ]; then
			cp -p "$_conf.bak-routerpanel" "$_conf"
		else
			rm -f "$_conf"
		fi
		die "unbound-checkconf falla; restaurado $_conf"
	fi
	service local_unbound reload
}

setup_caddyfile()
{
	say "caddyfile: añade este sitio a tu Caddyfile (no se instala solo):"
	cat "$PLANTILLAS/Caddyfile.routerpanel"
}

# Sin usuarios en el panel, dónde está el token del asistente de
# instalación (no lo imprime: se lee con doas en el servidor).
aviso_asistente()
{
	if [ ! -e "$RJ/usr/local/routerpanel/current" ]; then
		say "panel: aún sin versión desplegada; tras el primer despliegue, el token del asistente estará en $RJ$TOKEN"
		return
	fi
	# El panel lo escribe al arrancar: unos segundos como mucho.
	_n=0
	while [ ! -e "$RJ$TOKEN" ] && [ "$_n" -lt 10 ]; do
		sleep 1
		_n=$((_n + 1))
	done
	if [ -e "$RJ$TOKEN" ]; then
		say "panel sin usuarios: abre ${PANEL_ORIGEN%%,*}/instalar con el token de $RJ$TOKEN (doas cat …)"
	fi
}

# Genera las plantillas en un temporal propio de root (mktemp -d: 0700) y lo
# borra al salir, también si algo falla o llega HUP, INT o TERM (salida 1).
# La ruta es fija en /tmp: no se hereda TMPDIR, que podría apuntar a una
# carpeta del usuario. No se generan dentro de ~/routerpanel-deploy: lo que
# crea root ahí no lo puede borrar el usuario en la subida siguiente. Solo la
# usan las ramas que corren como root y generan plantillas (servidor y
# pf_cargar); pf_confirmar y comprobar_red no las usan. Se llama una sola vez.
plantillas_de_root()
{
	[ -z "$PLANTILLAS" ] || die "plantillas_de_root: ya se llamó antes"
	PLANTILLAS=$(mktemp -d /tmp/routerpanel-plantillas.XXXXXX) ||
		die "no puedo crear el directorio temporal de las plantillas"
	trap 'rm -rf "$PLANTILLAS"' EXIT
	trap 'exit 1' HUP INT TERM
	generar_plantillas "$PLANTILLAS"
}

servidor()
{
	check_syntax
	plantillas_de_root
	setup_jail_conf
	if tiene_modulo rc; then
		setup_runtime_jail
	fi
	if tiene_modulo actualizador; then
		setup_updater
	fi
	if tiene_modulo unbound; then
		setup_unbound
	fi
	if tiene_modulo caddyfile; then
		setup_caddyfile
	fi
	if tiene_modulo rc; then
		if [ -e "$RJ/usr/local/routerpanel/current" ] &&
			! jexec "$RUN_JAIL" service routerpanel status >/dev/null 2>&1; then
			jexec "$RUN_JAIL" service routerpanel start
		fi
		aviso_asistente
	fi
	if tiene_modulo actualizador; then
		aviso_jail_compilar
		echo
		echo "Clave pública de despliegue (darla de alta en el servidor git como clave de sólo lectura):"
		cat "$ACTUALIZADOR_CLAVES/id_ed25519.pub"
	fi
}

# /etc/pf.conf con los bloques de las reglas generadas en su sitio (salida
# estándar): el de nat antes de la primera regla nat, el de filtro antes del
# primer «block all».
pf_render()
{
	awk '
		FNR == NR {
			if ($0 ~ /^# @nat/) { part = "nat"; next }
			if ($0 ~ /^# @filtro/) { part = "filtro"; next }
			if (part != "") block[part] = block[part] $0 "\n"
			next
		}
		/^# >>> routerpanel/ { skip = 1; next }
		/^# <<< routerpanel/ { skip = 0; next }
		skip { next }
		!n && /^nat[ \t]/ {
			printf "# >>> routerpanel nat (deploy/freebsd-jail/pf.conf.plantilla del panel del router)\n%s# <<< routerpanel nat\n", block["nat"]
			n = 1
		}
		!f && /^block[ \t]+(log[ \t]+)?all([ \t#]|$)/ {
			printf "# >>> routerpanel filtro (deploy/freebsd-jail/pf.conf.plantilla del panel del router)\n%s# <<< routerpanel filtro\n", block["filtro"]
			f = 1
		}
		{ print }
		END { if (!n || !f) exit 1 }
	' "$PLANTILLAS/pf.conf" /etc/pf.conf
}

pf_cargar()
{
	plantillas_de_root
	pf_render >/etc/pf.conf.routerpanel-new ||
		die "no encuentro dónde insertar las reglas en /etc/pf.conf (hace falta una regla nat y un «block all»)"
	if cmp -s /etc/pf.conf.routerpanel-new /etc/pf.conf; then
		rm -f /etc/pf.conf.routerpanel-new
		echo "pf: /etc/pf.conf ya tiene las reglas"
		exit 10
	fi
	pfctl -nf /etc/pf.conf.routerpanel-new || die "pfctl -nf rechaza las reglas nuevas"
	cp -p /etc/pf.conf "$PF_BACKUP"
	# Hombre muerto: si nadie lo desarma, en 180 s vuelven las reglas
	# anteriores (en el núcleo y en el fichero). daemon(8) lo desliga de SSH.
	# Con «&&»: si el sleep muere por una señal, no se restaura nada (con «;»
	# el sh seguiría con cp y pfctl aunque se le desarmara a tiempo).
	# La marca se crea antes de restaurar: si se ve, ya no se puede desarmar.
	rm -f "$DEADMAN_FIRED"
	daemon -f -p "$DEADMAN_PID" /bin/sh -c \
		"sleep 180 && : >$DEADMAN_FIRED && cp -p $PF_BACKUP /etc/pf.conf && pfctl -f /etc/pf.conf &&
		logger -t routerpanel 'pf: reglas anteriores restauradas (hombre muerto)'"
	sleep 1
	[ -f "$DEADMAN_PID" ] || die "no arranca el interruptor de hombre muerto"
	mv /etc/pf.conf.routerpanel-new /etc/pf.conf
	pfctl -f /etc/pf.conf
	echo "pf: reglas cargadas; se restauran solas en 180 s si no se confirman"
}

pf_confirmar()
{
	if [ -f "$DEADMAN_FIRED" ]; then
		rm -f "$DEADMAN_PID"
		die "el interruptor ya saltó: pf tiene otra vez las reglas anteriores (ver syslog); repetir --pf-cargar y las comprobaciones"
	fi
	[ -f "$DEADMAN_PID" ] || die "no hay interruptor armado (falta --pf-cargar, o se paró a mano): revisar pfctl -sr"
	# El pidfile es el del sh (daemon -p). Primero muere el sh, para que nadie
	# siga tras el sleep; después el sleep, que ya no tiene a quién avisar.
	_pid=$(cat "$DEADMAN_PID")
	# El sh muerto sin marca (p. ej. matado a mano): no se sabe qué hay
	# cargado.
	if ! kill -0 "$_pid" 2>/dev/null; then
		rm -f "$DEADMAN_PID"
		die "el interruptor ya no está y no consta que saltara: revisar pfctl -sr y repetir --pf-cargar"
	fi
	_sleep=$(pgrep -P "$_pid" sleep || true)
	kill "$_pid" 2>/dev/null || true
	if [ -n "$_sleep" ]; then
		kill "$_sleep" 2>/dev/null || true
	fi
	sleep 1
	if kill -0 "$_pid" 2>/dev/null; then
		die "el interruptor sigue vivo (pid $_pid)"
	fi
	rm -f "$DEADMAN_PID"
	echo "pf: confirmadas; copia anterior en $PF_BACKUP"
}

# Lo que pf debe dejar (y no dejar) a la jail del panel, y que el
# actualizador llega al repositorio. Se ejecuta con las reglas nuevas
# cargadas y el interruptor aún armado.
comprobar_red()
{
	_ok=0
	_check()
	{
		_want=$1
		_what=$2
		shift 2
		if "$@" >/dev/null 2>&1; then _got=sale; else _got=bloqueado; fi
		echo "  $_what: $_got"
		[ "$_got" = "$_want" ] || _ok=1
	}
	_check sale "panel -> router :80" jexec "$RUN_JAIL" fetch -T 5 -qo /dev/null "http://$ROUTER_IP/"
	_check bloqueado "panel -> Internet" jexec "$RUN_JAIL" fetch -T 5 -qo /dev/null http://1.1.1.1/
	if [ -n "$PROXY_IP" ]; then
		_check bloqueado "panel -> proxy" jexec "$RUN_JAIL" fetch -T 5 -qo /dev/null "http://$PROXY_IP/"
	fi
	if [ -e "$RJ/usr/local/routerpanel/current" ]; then
		_check sale "panel -> su /salud (el actualizador)" jexec -U routerpanel "$RUN_JAIL" \
			fetch -T 5 -qo /dev/null "http://$JAIL_PANEL_IP:$PANEL_PUERTO/salud"
	fi
	if tiene_modulo actualizador; then
		# Sin la clave dada de alta en el servidor git, «Permission denied
		# (publickey)» también demuestra que se llega.
		_git=$(ls_remote_host yes 2>&1 >/dev/null; echo "rc=$?")
		case $_git in
		*rc=0* | *"Permission denied (publickey)"*) echo "  host -> repositorio: sale" ;;
		*)
			echo "  host -> repositorio: NO ($(echo "$_git" | head -n 1))"
			_ok=1
			;;
		esac
	fi
	return $_ok
}

# ------------------------------------------------------- equipo de trabajo --

services_ok()
{
	ssh -o ControlPath=none -o ConnectTimeout=10 -o BatchMode=yes "$HOST" true ||
		{ echo "  SSH: una conexión nueva NO entra"; return 1; }
	echo "  SSH: una conexión nueva entra"
	_ok=0
	for _url in $COMPROBAR_URLS; do
		_code=$(curl -s -o /dev/null -m 10 -w '%{http_code}' "$_url" || true)
		echo "  $_url: HTTP $_code"
		case $_code in
		2?? | 3?? | 401 | 403) ;;
		*) _ok=1 ;;
		esac
	done
	ssh -o ControlPath=none "$HOST" "cd ~/$STAGE && doas sh freebsd-jail/install.sh --comprobar-red" </dev/null ||
		_ok=1
	return $_ok
}

instalar()
{
	HOST=$SERVIDOR_SSH
	_tmp=$(mktemp -d)
	trap 'rm -rf "$_tmp"' EXIT
	# Las plantillas se generan también aquí, para fallar antes de subir nada.
	generar_plantillas "$_tmp/plantillas"
	rm -rf "$_tmp/plantillas"
	mkdir -p "$_tmp/freebsd-jail" "$_tmp/lib"
	cp "$DIR"/install.sh "$DIR"/actualizar.sh "$DIR"/recibir-precompilada.sh \
		"$DIR"/routerpanel.rc "$DIR"/*.plantilla "$_tmp/freebsd-jail/"
	cp "$DEPLOY_DIR"/lib/*.sh "$_tmp/lib/"
	# La configuración ya resuelta: en el servidor no hace falta config.example.
	volcar_config >"$_tmp/config.local"

	say "subiendo a ~/$STAGE del servidor"
	tar cf - -C "$_tmp" . | ssh "$HOST" "rm -rf ~/$STAGE && mkdir -p ~/$STAGE && tar xf - -C ~/$STAGE"

	say "instalando"
	if [ -t 0 ]; then
		ssh -t -o ServerAliveInterval=30 "$HOST" "cd ~/$STAGE && doas sh freebsd-jail/install.sh --servidor"
	else
		ssh -o ServerAliveInterval=30 "$HOST" "cd ~/$STAGE && doas sh freebsd-jail/install.sh --servidor"
	fi

	if tiene_modulo pf; then
		say "pf"
		_rc=0
		ssh "$HOST" "cd ~/$STAGE && doas sh freebsd-jail/install.sh --pf-cargar" </dev/null || _rc=$?
		case $_rc in
		0)
			say "comprobando el servidor con las reglas nuevas"
			if services_ok; then
				ssh -o ControlPath=none "$HOST" "cd ~/$STAGE && doas sh freebsd-jail/install.sh --pf-confirmar" </dev/null
			else
				die "algo no responde: NO se confirma; el servidor vuelve a las reglas anteriores en 180 s"
			fi
			;;
		10) ;;
		*) die "pf: no se han cargado las reglas" ;;
		esac
	fi
	say "hecho"
}

CONFIG=
if [ "${1:-}" = --config ]; then
	[ "$#" -ge 2 ] || uso
	CONFIG=$2
	shift 2
fi

case "${1:-}" in
--plantillas)
	[ "$#" -eq 2 ] || uso
	configurar "$CONFIG"
	generar_plantillas "$2"
	;;
--servidor | --pf-cargar | --pf-confirmar | --comprobar-red)
	[ "$#" -eq 1 ] || uso
	[ "$(id -u)" = 0 ] || die "$1 se ejecuta como root en el servidor"
	configurar "$CONFIG"
	case $1 in
	--servidor) servidor ;;
	--pf-cargar) pf_cargar ;;
	--pf-confirmar) pf_confirmar ;;
	--comprobar-red) comprobar_red ;;
	esac
	;;
"")
	configurar "$CONFIG"
	instalar
	;;
*) uso ;;
esac
