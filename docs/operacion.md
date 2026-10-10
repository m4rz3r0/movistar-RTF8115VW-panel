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

## Acceso y usuarios

- Se entra por `https://panel.home.arpa`, sólo desde la LAN (Caddy
  `solo_lan`). Un usuario y su contraseña del panel; no es la del router.
- La sesión caduca a los 30 minutos sin uso y, como mucho, a las 12 horas.
- Tras varios intentos fallidos el panel hace esperar (hasta 60 s) por IP y por
  usuario; la respuesta no dice si el usuario existe.
- La contraseña inicial de `admin` está en `/root/routerpanel-admin-password.txt`
  (600). Cámbiala la primera vez y borra el fichero.

Los usuarios se gestionan con el propio binario, dentro de la jail y como el
usuario del servicio (la base de datos es suya). Pide la contraseña dos veces,
sin eco; mínimo 12 caracteres:

```sh
# Alta
doas jexec -U routerpanel routerpanel env RP_DB=/var/db/routerpanel/panel.db \
    /usr/local/routerpanel/current/panel user add <usuario>

# Cambio de contraseña (cierra todas las sesiones de ese usuario)
doas jexec -U routerpanel routerpanel env RP_DB=/var/db/routerpanel/panel.db \
    /usr/local/routerpanel/current/panel user passwd <usuario>
```

No hace falta reiniciar el servicio. En esta versión no hay orden para borrar
usuarios.

## Cambiar la contraseña del router

El panel entra en la web del router como el usuario `1234` con la contraseña de
`/usr/local/etc/routerpanel/router.secret` (dentro de la jail; 600, del usuario
`routerpanel`, sin salto de línea al final). Para cambiarla:

1. Para el panel, para que no siga probando la contraseña vieja contra el
   router mientras tanto:

   ```sh
   doas jexec routerpanel service routerpanel stop
   ```

2. Cambia la contraseña **en el router** (su web, `http://192.168.1.1`).
3. Escribe la nueva en `router.secret`, como el usuario `routerpanel` y sin eco
   (escríbela y pulsa Intro):

   ```sh
   doas jexec -U routerpanel routerpanel /bin/sh -c '
       umask 077
       stty -echo; IFS= read -r p; stty echo
       printf %s "$p" >/usr/local/etc/routerpanel/router.secret.nuevo &&
       mv /usr/local/etc/routerpanel/router.secret.nuevo /usr/local/etc/routerpanel/router.secret'
   doas ls -l /usr/local/jails/containers/routerpanel/usr/local/etc/routerpanel/
   ```

   Debe quedar `-rw-------  routerpanel  routerpanel  router.secret`.

4. Arranca y comprueba:

   ```sh
   doas jexec routerpanel service routerpanel start
   doas jexec routerpanel-build fetch -qo - http://192.0.2.20:3000/salud
   ```

   `/salud` da `"ok":true` cuando el último sondeo del router (cada 10 s) ha
   ido bien. Si da error, mira el registro del panel (abajo).

`history.key` (misma carpeta) es la clave con la que el historial guarda los
valores anteriores de los campos secretos. No tiene que ver con la contraseña
del router: no la borres, o no se podrán deshacer los cambios ya guardados.

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
  de antes y marca la etiqueta como fallida (no se reintenta hasta que haya
  otra).
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
