# Operación del panel

El día a día con el panel ya instalado. Cómo se instala en cada plataforma:
[`deploy/README.md`](../deploy/README.md).

Las órdenes cambian según cómo se instaló. En esta guía:

- **Linux**: la instalación de [`deploy/linux/`](../deploy/linux/README.md)
  (systemd, `DynamicUser`), como root.
- **Docker**: la de [`deploy/docker/`](../deploy/docker/README.md), desde la
  raíz del repositorio (`docker` o `podman`; `compose` es
  `docker compose -f deploy/docker/compose.yaml`).
- **FreeBSD**: la de [`deploy/freebsd-jail/`](../deploy/freebsd-jail/README.md),
  en el servidor con `doas`. `<JAIL_PANEL>` y `<JAILS_RAIZ>` son los valores
  de tu `deploy/config.local`.

| | Base de datos (`RP_DB`) | Clave maestra (`RP_HISTORY_KEY`) |
|---|---|---|
| Linux | `/var/lib/routerpanel/panel.db` | `/var/lib/routerpanel/history.key` (según `env.example`) |
| Docker | `/data/panel.db` (volumen `datos`) | `/data/history.key` |
| FreeBSD | `/var/db/routerpanel/panel.db` (en la jail) | `/usr/local/etc/routerpanel/history.key` (en la jail) |

Sin `RP_DB` ni `RP_HISTORY_KEY`, el panel usa `/var/lib/routerpanel/panel.db`
y `/etc/routerpanel/history.key` en Linux, y `/var/db/routerpanel/panel.db` y
`/usr/local/etc/routerpanel/history.key` en FreeBSD.

## Primera instalación: el asistente

Un panel sin usuarios solo sirve el asistente de instalación (`/instalar`
bajo `RP_ORIGIN`; cualquier otra página lleva allí):

1. **Token.** Al arrancar sin usuarios, el panel genera un token de un solo
   uso, lo escribe en su registro («token de instalación: …») y en
   `instalar.token` (600), junto a la base de datos:

   ```sh
   # Linux
   cat /var/lib/routerpanel/instalar.token
   # Docker
   compose exec panel cat /data/instalar.token
   # FreeBSD
   doas cat <JAILS_RAIZ>/<JAIL_PANEL>/var/db/routerpanel/instalar.token
   ```

   Cada arranque sin administrador genera uno nuevo: si lo pierdes, reinicia
   el panel. El asistente solo responde desde la red local (o desde las
   redes de `RP_INSTALL_NETS`) y sin el token no hace nada.
2. **Administrador.** Con el token, usuario y contraseña (mínimo 12
   caracteres) del primer usuario del panel. Al crearlo se borra el token y
   queda la sesión abierta. Desde entonces no hay token ni asistente; para
   recuperar el acceso, `panel user passwd` (más abajo).
3. **Router.** Dirección de la web del router (de fábrica,
   `http://192.168.1.1`), su usuario y su contraseña. «Probar» entra, lee
   modelo y firmware y sale sin guardar; «Guardar y seguir» las guarda
   **cifradas** en la base de datos (con la clave maestra) solo si entran.
   Con un firmware que no es uno de los probados el panel queda en **solo
   lectura** hasta confirmarlo en «Configuración del router → Conexión con el
   router».
4. **Lectura inicial.** Lee todas las funciones de lectura y guarda la «foto
   de partida» cifrada. Con eso la instalación termina: `/instalar` responde
   404 y el panel se usa como siempre.

Si el proceso se reinicia a medias, el asistente sigue donde se quedó (con un
token nuevo si aún no había administrador).

## Configuración del router (`/router`)

Sección de la navegación principal con los ajustes del router, cada uno en su
página; `/router` lleva a la primera:

- **Conexión con el router** (`/router/conexion`): credenciales (la
  contraseña guardada nunca se muestra; para cambiar algo se escribe la del
  router y el panel prueba a entrar antes de guardar), firmware y modo solo
  lectura (con «Confirmar el firmware»), y la red detectada del router.

## Ajustes del panel (`/panel`)

En el menú del usuario; solo lo del panel. Secciones:

- **Panel**: idioma por defecto de la instancia («El del navegador» si no se
  fija ninguno) y zona horaria. Al cambiar el idioma por defecto la página se
  recarga para pintarse con el que le toque.
- **Visualización** y **Tema**: el modo de visualización (texto, gráfica o
  mixto) y el tema de los colores; cada usuario elige los suyos.
- **Usuarios**: alta, baja y cambio de contraseña.

Las operaciones delicadas (credenciales y contraseña del router, la conexión
de internet, confirmar el firmware y todo lo de usuarios) piden volver a
escribir la contraseña del panel; durante 5 minutos no se vuelve a pedir en
esa sesión. Con un cambio aplicándose o vigilándose, las credenciales del
router no se pueden cambiar ni esperan en la cola (se pide esperar).

## Idioma

Español e inglés. El selector está en la cabecera (en el menú en el móvil),
en el acceso y en el asistente; la elección se guarda en el navegador (cookie
`rp_lang`, un año) y recarga la página. Sin elección, manda el idioma por
defecto de la instancia («Ajustes del panel» → Panel) si se ha fijado; si no,
el idioma del navegador (`Accept-Language`); si no, español.

## Acceso y usuarios

- Se entra por `RP_ORIGIN`, a través del proxy con HTTPS y solo desde la red
  local ([`proxy-tls.md`](proxy-tls.md)). Un usuario y su contraseña del
  panel; no es la del router.
- La sesión caduca a los 30 minutos sin uso y, como mucho, a las 12 horas.
- Tras varios intentos fallidos el panel hace esperar (hasta 60 s) por IP y por
  usuario; la respuesta no dice si el usuario existe.

Los usuarios se gestionan en «Ajustes del panel» o con el propio binario:
`panel user add <usuario>` (alta) y `panel user passwd <usuario>` (cambio de
contraseña; cierra todas las sesiones de ese usuario). Abren la base de datos
de `RP_DB`, así que se ejecutan como el usuario del servicio, que es su
dueño. Piden la contraseña dos veces, sin eco; mínimo 12 caracteres:

```sh
# Linux (el mismo usuario dinámico y el mismo directorio que el servicio)
systemd-run --pty --wait --quiet -p DynamicUser=yes -p User=routerpanel \
    -p StateDirectory=routerpanel -E RP_DB=/var/lib/routerpanel/panel.db \
    /opt/routerpanel/current/panel user add <usuario>

# Docker
compose exec panel /opt/routerpanel/panel user add <usuario>

# FreeBSD
doas jexec -U routerpanel <JAIL_PANEL> env RP_DB=/var/db/routerpanel/panel.db \
    /usr/local/routerpanel/current/panel user add <usuario>
```

(`user passwd` igual). No hace falta reiniciar el servicio. Borrar usuarios,
solo desde «Ajustes del panel» (nunca el último).

## Cambiar la contraseña del router

Las credenciales del router están en la base de datos del panel, con la
contraseña cifrada con la clave maestra. Para cambiarla:

1. Cambia la contraseña **en el router** (en su web). Hasta el paso
   siguiente el sondeo del panel fallará (`"router":"error"` en `/salud`).
2. En «Configuración del router → Conexión con el router», escribe la nueva
   (y la dirección y el usuario si cambian), «Probar la conexión» y «Guardar
   las credenciales». El panel prueba a entrar antes de guardar y desde ese
   momento usa las nuevas; no hace falta reiniciar nada.
3. Comprueba `/salud` desde el equipo del panel (o desde el proxy):

   ```sh
   curl -s http://127.0.0.1:3000/salud   # la dirección de RP_LISTEN
   ```

   Da `"ok":true` y `"router":"ok"` cuando el último sondeo del router (cada
   10 s) ha ido bien; `"router":"sin_configurar"` si el panel aún no tiene
   credenciales (asistente sin terminar). Si da error, mira el registro del
   panel (abajo).

## La clave maestra y las copias de seguridad

`history.key` es la clave maestra del panel: cifra las credenciales del
router, los valores secretos del historial y la foto de partida. La crea el
panel la primera vez que arranca. No la borres ni la cambies: sin ella el
panel no puede usar las credenciales guardadas (habría que volver a pasar por
el asistente) ni deshacer los cambios ya guardados.

Para una copia de seguridad, guarda **las dos cosas**:

- la base de datos: `panel.db` y, si existen, `panel.db-wal` y
  `panel.db-shm`. Lo más sencillo es parar el servicio, copiar los ficheros
  y volver a arrancarlo;
- la clave maestra, `history.key`, **aparte**: una copia de la base sin la
  clave no sirve de nada, y las dos juntas descifran todo lo guardado.

```sh
# Linux
systemctl stop routerpanel
cp -p /var/lib/routerpanel/panel.db* /ruta/de/la/copia/
systemctl start routerpanel
# Docker (con el contenedor parado también se puede copiar; un fichero
# -wal o -shm que no exista da error y se puede ignorar)
compose stop panel
mkdir -p ./copia
for f in panel.db panel.db-wal panel.db-shm; do compose cp panel:/data/$f ./copia/; done
compose start panel
# FreeBSD
doas jexec <JAIL_PANEL> service routerpanel stop
doas cp -p <JAILS_RAIZ>/<JAIL_PANEL>/var/db/routerpanel/panel.db* /ruta/de/la/copia/
doas jexec <JAIL_PANEL> service routerpanel start
```

`history.key` se copia igual, desde la ruta de la tabla de arriba. Para
restaurar: con el servicio parado, la base y la clave en su sitio, del
usuario del servicio y con permisos 600.

## Registros

El panel escribe sus registros (arranque, errores con el router, cambios
aplicados y revertidos) en la salida de error, con el nivel de `RP_LOG`
(por defecto `info`). Nunca llevan la contraseña del router ni valores de
campos secretos.

```sh
# Linux
journalctl -u routerpanel -f
# Docker
compose logs -f panel
# FreeBSD (lo rota newsyslog)
doas tail -f <JAILS_RAIZ>/<JAIL_PANEL>/var/log/routerpanel.log
doas jexec <JAIL_PANEL> service routerpanel status
```

Los actualizadores tienen su propio registro: en FreeBSD,
`/var/log/routerpanel-update.log` (y la salida de la última compilación en
`/var/log/routerpanel-build.log`); en Linux, la salida de
`routerpanel-actualizar` (o `journalctl -u routerpanel-actualizar` si lo
lanza un temporizador).

## Actualizar

Antes de actualizar conviene una copia de seguridad (arriba). Los
actualizadores ya copian la base de datos antes de cambiar de versión y
vuelven solos a la anterior si la nueva no responde en `/salud`.

- **Linux**: `routerpanel-actualizar` descarga la última release, comprueba su
  sha256 (y su firma de minisign si le das la clave) y la despliega; o a mano,
  con una release nueva en `releases/` y el enlace `current`
  ([`deploy/linux/`](../deploy/linux/README.md#actualizar)).
- **Docker**: la versión nueva del repositorio (`git pull` o la etiqueta que
  quieras) y `compose up -d --build`.
- **FreeBSD**: el actualizador del host compila la última etiqueta `v*` en la
  jail de compilación, cada 15 minutos
  ([`deploy/freebsd-jail/`](../deploy/freebsd-jail/README.md#actualizador)).
  Una compilación tarda bastante: si la lanzas a mano, hazlo en un terminal
  propio y desligada de la sesión
  (`doas daemon -f /usr/local/sbin/routerpanel-actualizar`). Los cambios en
  `deploy/` no los despliega el actualizador: hay que repetir `install.sh`.

Para volver atrás a mano: el enlace `current` a la versión anterior y, si la
nueva cambió el esquema de la base de datos, la copia de antes de actualizar
(en FreeBSD, `/var/db/routerpanel/antes-de-actualizar/` en la jail). Cada guía
de instalación lo explica.

## Actualizar desde una versión anterior al asistente (v0.1.x)

Las instalaciones de antes del asistente guardaban la contraseña del router
en un fichero (`router.secret`) y el servicio la pasaba con `RP_ROUTER_URL`,
`RP_ROUTER_USER` y `RP_ROUTER_SECRET`. Al arrancar la versión nueva:

- Con usuarios en la base de datos, el asistente no aparece nunca.
- Si la base de datos aún no tiene credenciales del router, el panel las
  importa **una vez** de esas variables (cifradas) y sigue sondeando; queda en
  el registro («router: credenciales importadas del entorno…»). Desde
  entonces usa las de la base de datos y ya no lee el fichero.
- Las tres variables van juntas o ninguna: con una o dos, el panel no arranca
  (error claro en el registro).

Una vez importadas (ver el registro, o «Configuración del router → Conexión
con el router»), se pueden quitar las variables y borrar el fichero. En
FreeBSD, el `routerpanel.rc` que instala `install.sh` solo las pasa si existe
el fichero:

```sh
doas jexec -U routerpanel <JAIL_PANEL> rm /usr/local/etc/routerpanel/router.secret
doas jexec <JAIL_PANEL> service routerpanel restart
```
