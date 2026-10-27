# Operación del panel

El día a día con el panel ya instalado. La instalación está en
[`deploy/README.md`](../deploy/README.md). Las órdenes de esta guía son para:

- **Linux**: [`deploy/linux/`](../deploy/linux/README.md) (systemd,
  `DynamicUser`), como root.
- **Podman o Docker**: [`deploy/docker/`](../deploy/docker/README.md), desde
  la raíz del repositorio; `compose` es
  `podman compose -f deploy/docker/compose.yaml` (o lo mismo con `docker`).
- **FreeBSD**: [`deploy/freebsd-jail/`](../deploy/freebsd-jail/README.md), en
  el servidor con `doas`; `<JAIL_PANEL>` y `<JAILS_RAIZ>` salen de tu
  `deploy/config.local`.

| | Base de datos (`RP_DB`) | Clave maestra (`RP_HISTORY_KEY`) |
|---|---|---|
| Linux | `/var/lib/routerpanel/panel.db` | `/var/lib/routerpanel/history.key` (según `env.example`) |
| Podman o Docker | `/data/panel.db` (volumen `datos`) | `/data/history.key` |
| FreeBSD | `/var/db/routerpanel/panel.db` (en la jail) | `/usr/local/etc/routerpanel/history.key` (en la jail) |

Sin esas variables, el panel usa en Linux `/var/lib/routerpanel/panel.db` y
`/etc/routerpanel/history.key`; en FreeBSD, las de la tabla.

## Primera instalación: el asistente

Un panel sin usuarios solo sirve el asistente (`/instalar` bajo `RP_ORIGIN`;
cualquier otra página lleva allí):

1. **Token.** Al arrancar sin usuarios, el panel genera un token de un solo
   uso y lo escribe en `instalar.token` (permisos 600), junto a la base de
   datos. El registro solo dice dónde está:

   ```sh
   # Linux
   cat /var/lib/routerpanel/instalar.token
   # Podman o Docker
   compose exec panel cat /data/instalar.token
   # FreeBSD
   doas cat <JAILS_RAIZ>/<JAIL_PANEL>/var/db/routerpanel/instalar.token
   ```

   Si no puede escribir el fichero, el token sale en el registro. Si lo
   pierdes, reinicia el panel y tendrás uno nuevo. El asistente solo responde
   desde la red local o desde las redes de `RP_INSTALL_NETS`.
2. **Administrador.** Con el token, el usuario y la contraseña (mínimo 12
   caracteres) del primer usuario. Al crearlo se borra el token y queda la
   sesión abierta. Si más adelante pierdes el acceso, usa `panel user passwd`.
3. **Router.** Dirección de la web del router (de fábrica,
   `http://192.168.1.1`), usuario y contraseña. «Probar» entra, lee modelo y
   firmware y sale sin guardar. «Guardar y seguir» las guarda cifradas con la
   clave maestra, solo si entran. Con un firmware que no es de los probados,
   el panel queda en solo lectura hasta que lo confirmes en «Configuración del
   router → Conexión con el router».
4. **Lectura inicial.** Lee todas las funciones de lectura y guarda cifrada
   la «foto de partida». Con eso termina: `/instalar` responde 404.

Si el proceso se reinicia a medias, el asistente sigue donde se quedó (con un
token nuevo si aún no había administrador).

## Configuración del router (`/router`)

En la navegación principal. **Conexión con el router** (`/router/conexion`)
tiene las credenciales (la
contraseña guardada nunca se muestra; al cambiarlas, el panel prueba a entrar
antes de guardar), el cambio de la contraseña del router, el firmware y el
modo solo lectura («Confirmar el firmware»), y la red detectada del router.

## Ajustes del panel (`/panel`)

En el menú del usuario. Solo afecta al panel:

- **Panel**: idioma por defecto («El del navegador» si no se fija ninguno) y
  zona horaria.
- **Visualización** y **Tema**: modo de visualización (texto, gráfica o
  mixto) y colores. Cada usuario elige los suyos.
- **Usuarios**: alta, baja y cambio de contraseña.

Las operaciones delicadas (credenciales y contraseña del router, conexión a
internet, confirmar el firmware y todo lo de usuarios) piden otra vez la
contraseña del panel. Durante 5 minutos no se vuelve a pedir en esa sesión.
Mientras un cambio se aplica o se vigila, las credenciales del router no se
pueden cambiar ni quedan en la cola.

## Idioma

Español e inglés. El selector está en la cabecera (en el móvil, en el menú),
en el acceso y en el asistente, y guarda la elección en la cookie `rp_lang`
durante un año. Sin elección, manda el idioma por defecto de «Ajustes del
panel», luego el del navegador (`Accept-Language`) y, si no, español.

## Acceso y usuarios

- Se entra por `RP_ORIGIN`, a través del proxy con HTTPS y solo desde la red
  local ([`proxy-tls.md`](proxy-tls.md)), con un usuario del panel. Su
  contraseña no es la del router.
- La sesión caduca tras 30 minutos sin uso y, como mucho, a las 12 horas.
- Tras cada intento fallido, el panel hace esperar más (hasta 60 s) al
  dispositivo que falla: su IP, o su red /64 en IPv6. El freno es por
  dispositivo, no por usuario, así que nadie de la red puede dejar a otro sin
  entrar. La respuesta no dice si el usuario existe.

Los usuarios se gestionan en «Ajustes del panel» o con el binario:
`panel user add <usuario>` y `panel user passwd <usuario>` (este cierra todas
sus sesiones). Se ejecutan como el usuario del servicio, porque abren la base
de `RP_DB`, y piden la contraseña dos veces, sin eco (mínimo 12 caracteres):

```sh
# Linux (el mismo usuario dinámico y el mismo directorio que el servicio)
systemd-run --pty --wait --quiet -p DynamicUser=yes -p User=routerpanel \
    -p StateDirectory=routerpanel -E RP_DB=/var/lib/routerpanel/panel.db \
    /opt/routerpanel/current/panel user add <usuario>

# Podman o Docker
compose exec panel /opt/routerpanel/panel user add <usuario>

# FreeBSD
doas jexec -U routerpanel <JAIL_PANEL> env RP_DB=/var/db/routerpanel/panel.db \
    /usr/local/routerpanel/current/panel user add <usuario>
```

`user passwd` va igual. No hace falta reiniciar el servicio. Los usuarios
solo se borran desde «Ajustes del panel», y nunca el último.

## Cambiar la contraseña del router

Lo más sencillo es la tarjeta «Cambiar la contraseña del router» de
«Configuración del router → Conexión con el router». El panel escribe la
nueva, prueba a entrar con ella y desde ese momento la usa. Si no consigue
entrar, intenta volver a la anterior y avisa.

Si la cambias desde la web del router:

1. Hasta el paso siguiente, el panel no podrá entrar (`"router":"error"` en
   `/salud`).
2. En «Conexión con el router», escribe la nueva (y la dirección y el usuario
   si cambian), «Probar la conexión» y «Guardar las credenciales». No hace
   falta reiniciar nada.
3. Comprueba `/salud` desde el equipo del panel o desde el proxy:

   ```sh
   curl -s http://127.0.0.1:3000/salud   # la dirección de RP_LISTEN
   ```

`/salud` responde `{"ok":true,"router":…}` sin preguntar al router: dice cómo
acabó la última comunicación del sondeo, que va cada 10 s. `router` es `ok` si
respondió hace menos de 2 minutos; `error` si no respondió, rechazó las
credenciales o hace más de eso; `pendiente` justo al arrancar, y
`sin_configurar` si aún no hay credenciales. Si da error, el registro del
panel tiene la causa.

## La clave maestra y las copias de seguridad

`history.key` cifra las credenciales del router, los valores secretos del
historial y la foto de partida. La crea el panel al arrancar por primera vez.
No la borres ni la cambies: sin ella, el panel no puede usar las credenciales
guardadas ni deshacer los cambios ya hechos.

Una copia de seguridad lleva la base de datos (`panel.db` y, si existen,
`panel.db-wal` y `panel.db-shm`, con el servicio parado) y, **aparte**,
`history.key`. La base sin la clave no sirve, y las dos juntas descifran todo.

```sh
# Linux
systemctl stop routerpanel
cp -p /var/lib/routerpanel/panel.db* /ruta/de/la/copia/
systemctl start routerpanel
# Podman o Docker (si falta el -wal o el -shm, el error se puede ignorar)
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
restaurar, con el servicio parado, deja la base y la clave en su sitio, del
usuario del servicio y con permisos 600.

## Registros

El panel escribe sus registros (arranque, errores con el router, cambios
aplicados y revertidos) en la salida de error, con el nivel de `RP_LOG` (por
defecto `info`). Nunca llevan la contraseña del router ni valores de campos
secretos.

```sh
# Linux
journalctl -u routerpanel -f
# Podman o Docker
compose logs -f panel
# FreeBSD (lo rota newsyslog)
doas tail -f <JAILS_RAIZ>/<JAIL_PANEL>/var/log/routerpanel.log
doas jexec <JAIL_PANEL> service routerpanel status
```

Los actualizadores tienen su propio registro: en FreeBSD,
`/var/log/routerpanel-update.log` (y `/var/log/routerpanel-build.log` para la
última compilación); en Linux, la salida de `routerpanel-actualizar` o
`journalctl -u routerpanel-actualizar` si lo lanza un temporizador.

## Actualizar

Haz antes una copia de seguridad. Los actualizadores copian la base de datos y
vuelven solos a la versión anterior si la nueva no responde en `/salud`.

- **Linux**: `routerpanel-actualizar` descarga la última release, comprueba su
  sha256 (y su firma de minisign si le das la clave) y la despliega. También
  se puede hacer a mano, con la release nueva en `releases/` y el enlace
  `current` ([`deploy/linux/`](../deploy/linux/README.md#actualizar)).
- **Podman o Docker**: trae la versión nueva del repositorio (`git pull` o la etiqueta
  que quieras) y ejecuta `compose up -d --build`.
- **FreeBSD**: el actualizador del host compila la última etiqueta `v*` en la
  jail de compilación cada 15 minutos
  ([`deploy/freebsd-jail/`](../deploy/freebsd-jail/README.md#actualizador)).
  Una compilación tarda bastante: si la lanzas a mano, hazlo desligada de la
  sesión (`doas daemon -f /usr/local/sbin/routerpanel-actualizar`). Los cambios
  en `deploy/` no los despliega el actualizador: hay que repetir `install.sh`.

Para volver atrás a mano, apunta `current` a la versión anterior y, si la
nueva cambió el esquema de la base de datos, recupera la copia de antes de
actualizar (en FreeBSD, `/var/db/routerpanel/antes-de-actualizar/` en la
jail).

## Actualizar desde una versión anterior al asistente (v0.1.x)

Las instalaciones de antes del asistente guardaban la contraseña del router en
un fichero (`router.secret`) y el servicio la pasaba con `RP_ROUTER_URL`,
`RP_ROUTER_USER` y `RP_ROUTER_SECRET`. Al arrancar la versión nueva:

- Con usuarios en la base de datos, el asistente no aparece.
- Si la base aún no tiene credenciales del router, el panel las importa
  **una vez** de esas variables y lo anota en el registro («router:
  credenciales importadas del entorno…»).
- Las tres variables van juntas: con una o dos, el panel no arranca.

Una vez importadas, puedes quitar las variables y borrar el fichero. En
FreeBSD, el `routerpanel.rc` que instala `install.sh` solo las pasa si existe
el fichero:

```sh
doas jexec -U routerpanel <JAIL_PANEL> rm /usr/local/etc/routerpanel/router.secret
doas jexec <JAIL_PANEL> service routerpanel restart
```
