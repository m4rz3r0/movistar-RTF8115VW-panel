# Operación del panel

Guía del día a día con el panel ya instalado en servidor-casa
(`192.0.2.10`). Cómo se instala, las jails y pf: [`deploy/README.md`](../deploy/README.md).

Todas las órdenes se lanzan en el servidor (`ssh admin@192.0.2.10`) con
`doas`. La jail del panel se llama `routerpanel`; desde el host su raíz está en
`/usr/local/jails/containers/routerpanel`.

> **Órdenes largas en un terminal propio.** Una compilación (`routerpanel-actualizar`)
> tarda bastante más de 15 minutos. Lánzala en un terminal tuyo con SSH, no
> desde una herramienta con límite de tiempo, y mejor desligada de la sesión
> para que un corte de SSH no la mate:
>
> ```sh
> doas daemon -f /usr/local/sbin/routerpanel-actualizar --ref nucleo
> doas tail -f /var/log/routerpanel-update.log
> ```

## Primera instalación: el asistente

Un panel sin usuarios sólo sirve el asistente de instalación
(`https://panel.home.arpa/instalar`; cualquier otra página lleva allí):

1. **Token.** Al arrancar sin usuarios, el panel genera un token de un solo
   uso, lo escribe en su registro («token de instalación: …») y en
   `/var/db/routerpanel/instalar.token` (600, junto a la base de datos). En el
   host:

   ```sh
   doas cat /usr/local/jails/containers/routerpanel/var/db/routerpanel/instalar.token
   ```

   `deploy/install.sh` dice dónde está si el panel aún no tiene usuarios. El
   asistente sólo responde desde la red local y sin el token no hace nada.
2. **Administrador.** Con el token, usuario y contraseña (mínimo 12
   caracteres) del primer usuario del panel. Al crearlo se borra el token y
   queda la sesión abierta.
3. **Router.** Dirección (`http://192.168.1.1`), usuario (`1234`) y
   contraseña de la web del router. «Probar» entra, lee modelo y firmware y
   sale sin guardar; «Guardar y seguir» las guarda **cifradas** en la base de
   datos (con `history.key`) sólo si entran. Con un firmware que no es uno de
   los probados el panel queda en **solo lectura** hasta confirmarlo en
   «Router → Conexión con el router».
4. **Lectura inicial.** Lee todas las funciones de lectura y guarda la «foto
   de partida» cifrada. Con eso la instalación termina: `/instalar` responde
   404 y el panel se usa como siempre.

Si el proceso se reinicia a medias, el asistente sigue donde se quedó (con un
token nuevo si aún no había administrador).

## Router (`/router`)

Sección de la navegación principal con los ajustes del router, cada uno en su
página; `/router` lleva a la primera:

- **Conexión con el router** (`/router/conexion`): credenciales (la
  contraseña guardada nunca se muestra; para cambiar algo se escribe la del
  router y el panel prueba a entrar antes de guardar), firmware y modo solo
  lectura (con «Confirmar el firmware»).

## Ajustes del panel (`/panel`)

En el menú del usuario; sólo lo del panel. Secciones:

- **Panel**: idioma por defecto de la instancia («El del navegador» si no se
  fija ninguno) y zona horaria. Al cambiar el idioma por defecto la página se
  recarga para pintarse con el que le toque.
- **Privacidad e instancia**: DNS preferidos, dispositivos de confianza,
  ventana de mantenimiento y servidor propio (opcionales).
- **Usuarios**: alta, baja y cambio de contraseña.

Las operaciones delicadas (credenciales del router, confirmar el firmware y
todo lo de usuarios) piden volver a escribir la contraseña del panel; durante
5 minutos no se vuelve a pedir en esa sesión. «Revelar» un secreto en el
Explorador usa la misma ventana. Con un cambio aplicándose o vigilándose, las
credenciales del router no se pueden cambiar (se pide esperar).

## Idioma

Español e inglés. El selector está en la cabecera (en el menú en el móvil),
en el acceso y en el asistente; la elección se guarda en el navegador (cookie
`rp_lang`, un año) y recarga la página. Sin elección, manda el idioma por
defecto de la instancia («Ajustes del panel» → Panel) si se ha fijado; si no,
el idioma del navegador (`Accept-Language`); si no, español.

## Acceso y usuarios

- Se entra por `https://panel.home.arpa`, sólo desde la LAN (Caddy
  `solo_lan`). Un usuario y su contraseña del panel; no es la del router.
- La sesión caduca a los 30 minutos sin uso y, como mucho, a las 12 horas.
- Tras varios intentos fallidos el panel hace esperar (hasta 60 s) por IP y por
  usuario; la respuesta no dice si el usuario existe.
- Si queda de antes `/root/routerpanel-admin-password.txt` (la contraseña
  inicial que creaba `install.sh` antes del asistente), cambia esa contraseña
  y borra el fichero.

Los usuarios se gestionan en «Ajustes del panel» o con el propio binario,
dentro de la jail y como el usuario del servicio (la base de datos es suya).
Pide la contraseña dos veces, sin eco; mínimo 12 caracteres:

```sh
# Alta
doas jexec -U routerpanel routerpanel env RP_DB=/var/db/routerpanel/panel.db \
    /usr/local/routerpanel/current/panel user add <usuario>

# Cambio de contraseña (cierra todas las sesiones de ese usuario)
doas jexec -U routerpanel routerpanel env RP_DB=/var/db/routerpanel/panel.db \
    /usr/local/routerpanel/current/panel user passwd <usuario>
```

No hace falta reiniciar el servicio. Borrar usuarios, sólo desde «Ajustes del
panel» (nunca el último).

## Cambiar la contraseña del router

Las credenciales del router están en la base de datos del panel, con la
contraseña cifrada con `history.key`. Para cambiarla:

1. Cambia la contraseña **en el router** (su web, `http://192.168.1.1`).
   Hasta el paso siguiente el sondeo del panel fallará («router: error» en
   `/salud`).
2. En «Router → Conexión con el router», escribe la nueva (y la dirección y el
   usuario si cambian), «Probar la conexión» y «Guardar las credenciales». El
   panel prueba a entrar antes de guardar y desde ese momento usa las nuevas;
   no hace falta reiniciar nada.
3. Comprueba `/salud`:

   ```sh
   doas jexec routerpanel-build fetch -qo - http://192.0.2.20:3000/salud
   ```

   Da `"ok":true` y `"router":"ok"` cuando el último sondeo del router (cada
   10 s) ha ido bien; `"router":"sin_configurar"` si el panel aún no tiene
   credenciales (asistente sin terminar). Si da error, mira el registro del
   panel (abajo).

`history.key` (`/usr/local/etc/routerpanel/`, dentro de la jail) es la clave
maestra del panel: cifra las credenciales del router, los valores secretos
del historial y la foto de partida. No la borres ni la cambies: sin ella el
panel no arranca con las credenciales guardadas (habría que volver a pasar por
el asistente) y no se podrán deshacer los cambios ya guardados.

## Actualizar desde una versión anterior al asistente (v0.1.x)

Las instalaciones de antes guardaban la contraseña del router en
`/usr/local/etc/routerpanel/router.secret` y el servicio la pasaba con
`RP_ROUTER_URL`, `RP_ROUTER_USER` y `RP_ROUTER_SECRET`. Al arrancar la versión
nueva:

- Con usuarios en la base de datos, el asistente no aparece nunca.
- Si la base de datos aún no tiene credenciales del router, el panel las
  importa **una vez** de esas variables (cifradas) y sigue sondeando; queda en
  el registro («router: credenciales importadas del entorno…»). Desde
  entonces usa las de la base de datos y ya no lee el fichero.
- Las tres variables van juntas o ninguna: con una o dos, el panel no arranca
  (error claro en el registro).

El actualizador sólo despliega el panel, así que el servicio sigue con su
`routerpanel.rc` de antes (que pasa las variables) hasta repetir
`deploy/install.sh`; el nuevo sólo las pasa si existe `router.secret`. Una vez
importadas (ver el registro, o «Router → Conexión con el router»), **se pueden
quitar**:

```sh
doas jexec -U routerpanel routerpanel rm /usr/local/etc/routerpanel/router.secret
doas jexec routerpanel service routerpanel restart
```

Con el `routerpanel.rc` de antes las variables se siguen pasando, pero con
credenciales en la base de datos el panel ni las mira ni lee el fichero;
`deploy/install.sh` instala el nuevo, que ya no las pasa.

## Registros

| Fichero (en el host) | Qué hay |
|---|---|
| `/usr/local/jails/containers/routerpanel/var/log/routerpanel.log` | el panel: arranque, errores con el router, cambios aplicados y revertidos (nivel `RP_LOG=info`, ver `routerpanel.rc`) |
| `/var/log/routerpanel-update.log` | el actualizador: etiquetas nuevas, compilaciones, despliegues, vueltas atrás |
| `/var/log/routerpanel-build.log` | salida de cargo de la **última** compilación |

```sh
doas tail -f /usr/local/jails/containers/routerpanel/var/log/routerpanel.log
doas tail -n 50 /var/log/routerpanel-update.log
doas jexec routerpanel service routerpanel status
```

Los rota newsyslog. Los registros nunca llevan la contraseña del router ni
valores de campos secretos.

## Actualizaciones

- Se publican con una etiqueta `v*` (p. ej. `v0.1.1`) en Forgejo. El
  actualizador (`/usr/local/sbin/routerpanel-actualizar`, cron cada 15 minutos
  en el host) toma la más reciente (`sort -V`), la compila y prueba en la jail
  `routerpanel-build`, copia la base de datos, cambia la versión y espera a
  `/salud`. Si no responde en 60 s, vuelve a la anterior con la base de datos
  de antes. Una etiqueta que falla (al compilar, en las pruebas o al
  arrancar) se reintenta una vez en la ejecución siguiente; si vuelve a
  fallar queda marcada como fallida y no se reintenta hasta que haya otra
  (`/var/db/routerpanel-actualizar/fallida`; ver `routerpanel-update.log`).
- **Cambios en `deploy/`** (el propio actualizador, `routerpanel.rc`, pf,
  cron, Caddy…): el actualizador sólo despliega el panel. Tras publicar una
  versión que toque `deploy/`, repite `deploy/install.sh` desde el portátil
  (en la raíz del repositorio, con esa versión sacada). Es idempotente; si
  cambian las reglas de pf vuelve a pasar por el interruptor de hombre muerto.
- Para desplegar una rama o un commit a mano (en un terminal propio, ver
  arriba):

  ```sh
  doas daemon -f /usr/local/sbin/routerpanel-actualizar --ref <rama|commit>
  ```

  La versión se llama `<ref>-<sha corto>`. Si ya es la desplegada, sólo
  reinicia el servicio y comprueba `/salud`.
- Ojo: mientras haya una etiqueta `v*` más reciente que lo desplegado, el cron
  la vuelve a desplegar en 15 minutos. Un `--ref` sirve para probar, no para
  quedarse en una rama.

## Volver atrás a mano

El actualizador ya vuelve atrás solo si la versión nueva no responde. Si la
nueva arranca pero se porta mal:

1. Versiones disponibles (se guardan las 3 últimas) y la actual:

   ```sh
   ls -lt /usr/local/routerpanel/releases/
   readlink /usr/local/jails/containers/routerpanel/usr/local/routerpanel/current
   ```

2. Para el servicio y apunta `current` a la versión buena (enlace relativo):

   ```sh
   doas jexec routerpanel service routerpanel stop
   doas ln -sfh releases/<versión> \
       /usr/local/jails/containers/routerpanel/usr/local/routerpanel/current
   ```

3. Si la versión mala cambió el esquema de la base de datos (una migración
   nueva), la vieja no arrancará con ella. Recupera la copia de antes del último
   despliegue (se pierde lo registrado desde entonces):

   ```sh
   doas jexec -U routerpanel routerpanel /bin/sh -c '
       cd /var/db/routerpanel && rm -f panel.db panel.db-wal panel.db-shm &&
       cp -p antes-de-actualizar/* .'
   ```

4. Arranca y comprueba `/salud` (como arriba).
5. Para que el cron no la vuelva a desplegar, marca la etiqueta mala como
   fallida (hasta que se publique otra):

   ```sh
   echo <etiqueta mala, p. ej. v0.2.0> | doas tee /var/db/routerpanel-actualizar/fallida
   ```

## pf y el interruptor de hombre muerto

Las reglas de pf del panel (`deploy/pf.conf.snippet`) sólo se tocan con
`deploy/install.sh`, que las carga con un interruptor de hombre muerto: tras
cargarlas, si en **180 s** nadie las confirma, el servidor vuelve solo a las
reglas anteriores (fichero y núcleo; queda en syslog: `pf: reglas anteriores
restauradas (hombre muerto)`). `install.sh` comprueba desde una conexión SSH
nueva que los sitios web y el SSH de Forgejo siguen respondiendo y que las
jails alcanzan sólo lo que deben; sólo entonces confirma.

- Si una comprobación falla, no confirma: espera 3 minutos y todo vuelve a
  estar como antes. No hace falta hacer nada.
- Si `--pf-confirmar` dice «el interruptor ya saltó», las comprobaciones
  tardaron más de 180 s y las reglas vigentes son las anteriores (queda la
  marca `/var/run/routerpanel-pf-deadman.saltado` y la línea en syslog).
  Repite `deploy/install.sh`.
- Si dice «no hay interruptor armado», no se llegó a cargar nada con
  `--pf-cargar` o alguien lo paró a mano: mira `doas pfctl -sr` antes de
  seguir.
- Para ver qué hay cargado: `doas pfctl -sr | grep -c ROUTERPANEL` (más de 0:
  reglas del panel cargadas). Copia de antes del último cambio:
  `/etc/pf.conf.bak-routerpanel`.
- Nunca edites a mano los bloques entre `# >>> routerpanel` y `# <<< routerpanel`
  de `/etc/pf.conf`: los regenera `install.sh`.
