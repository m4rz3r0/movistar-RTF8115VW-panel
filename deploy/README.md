# Despliegue en servidor-casa

El panel corre en el servidor doméstico (FreeBSD 15.1, `192.0.2.10`) en dos
jails PkgBase, como las demás del servidor:

| Jail | IP | Contenido |
|---|---|---|
| `routerpanel` | 192.0.2.20 | base mínima (`FreeBSD-set-minimal-jail`), el binario y su web; servicio `routerpanel` en `0.0.0.0:3000` |
| `routerpanel-build` | 192.0.2.21 | base completa, `rustup` (stable + `wasm32-unknown-unknown`), `cargo-leptos` 0.3.11, `wasm-bindgen-cli` (la versión del `Cargo.lock`), `binaryen` (`wasm-opt`), `git`; usuario sin privilegios `rpbuild` (uid 3160) con el clon en `/home/rpbuild/panel` |

Se entra por `https://panel.home.arpa` (Caddy, con
`import solo_lan`); unbound del host resuelve el nombre a `192.0.2.11`.

## Ficheros

| Fichero | Se instala en |
|---|---|
| `install.sh` | se ejecuta desde el portátil; ver abajo |
| `jail.conf.snippet` | añadido a `/etc/jail.conf` del host |
| `pf.conf.snippet` | dos bloques en `/etc/pf.conf` del host, entre marcas `# >>> routerpanel` / `# <<< routerpanel` |
| `routerpanel.rc` | `/usr/local/etc/rc.d/routerpanel` en la jail `routerpanel` |
| `actualizar.sh` | `/usr/local/sbin/routerpanel-actualizar` en el **host** |
| `doas.conf.snippet` | nada: explica por qué no hace falta regla de doas |

## Rutas

En el host:

- `/usr/local/routerpanel/releases/<versión>/{panel,site/,COMMIT}` — versiones
  compiladas (root, sólo lectura para los demás). Se monta por nullfs `ro,late`
  (`/etc/fstab`) en `/usr/local/routerpanel/releases` de la jail `routerpanel`.
- `/usr/local/etc/cron.d/routerpanel` — el actualizador cada 15 minutos.
- `/var/log/routerpanel-update.log` (rotado por
  `/usr/local/etc/newsyslog.conf.d/routerpanel.conf`) y
  `/var/log/routerpanel-build.log` (salida de cargo de la última compilación).
- `/var/db/routerpanel-actualizar/fallida` — última etiqueta que falló dos
  veces seguidas (no compiló o no arrancó); no se reintenta hasta que haya
  otra. `intentos` (etiqueta y número) lleva la cuenta del primer fallo. Los
  `--ref` no tocan ninguno de los dos.
- `/root/routerpanel-admin-password.txt` (600) — sólo en instalaciones
  anteriores al asistente: contraseña inicial del usuario `admin`
  (cámbiala y borra el fichero). `install.sh` ya no lo crea.

En la jail `routerpanel`:

- `/usr/local/routerpanel/current` → `releases/<versión>` (enlace relativo; lo
  cambia el actualizador).
- `/var/db/routerpanel/panel.db` — SQLite (directorio 700 del usuario
  `routerpanel`): usuarios, credenciales del router (contraseña cifrada),
  historial, parámetros de la instancia. Mientras el panel no tiene usuarios,
  ahí mismo está `instalar.token` (600), el token del asistente.
- `/usr/local/etc/routerpanel/history.key` (600) — clave maestra; la crea el
  panel al arrancar en ese directorio (700, `routerpanel`). Cifra las
  credenciales del router y los secretos del historial: no se borra.
- `/usr/local/etc/routerpanel/router.secret` (600, `routerpanel`) — sólo en
  instalaciones anteriores al asistente: contraseña del router que el panel
  importa una vez a la base de datos. Después se puede borrar
  ([`docs/operacion.md`](../docs/operacion.md#actualizar-desde-una-versión-anterior-al-asistente-v01x)).
- `/var/log/routerpanel.log` — salida del panel; la escribe `daemon(8)` y la
  rota newsyslog (`/etc/newsyslog.conf.d/routerpanel.conf`, SIGHUP a `daemon -H`).

El servicio arranca con `RP_LISTEN=0.0.0.0:3000`, `RP_DB`, `RP_HISTORY_KEY`,
`RP_TRUSTED_PROXY=192.0.2.11`, `RP_ORIGIN` (obligatoria;
`routerpanel_origin` en `rc.conf`, por defecto `https://panel.home.arpa`),
`RP_LOG=info` y `LEPTOS_OUTPUT_NAME=panel`, `LEPTOS_SITE_ROOT=<versión>/site`,
`LEPTOS_SITE_ADDR=0.0.0.0:3000`, `LEPTOS_ENV=PROD` (ver `routerpanel.rc`).
Sólo si existe `router.secret` (instalaciones anteriores al asistente) añade
`RP_ROUTER_URL`, `RP_ROUTER_USER` y `RP_ROUTER_SECRET` (`routerpanel_router_url`,
`_user`, `_secret`; por defecto `http://192.168.1.1`, `1234` y ese fichero)
para que el panel importe las credenciales.

## Instalar

Desde el portátil, en la raíz del repositorio (con `~/servidor-casa` al lado,
de donde sale el Caddyfile; otra ruta con `CADDYFILE=...`):

```sh
deploy/install.sh
```

Ya no pide la contraseña del router ni crea usuarios: eso lo hace el
asistente de instalación del panel (`/instalar`, ver
[`docs/operacion.md`](../docs/operacion.md#primera-instalación-el-asistente)).

Es idempotente: se puede repetir y sólo hace lo que falta. Pasos:

1. Sube `deploy/`, el Caddyfile y un `git bundle` del repositorio a
   `~/routerpanel-deploy` del servidor.
2. `--servidor` (root): valida los guiones y el Caddyfile (con el binario y el
   entorno del servicio de Caddy: `caddy_command`, `caddy_env`); crea las jails
   (dataset ZFS, `pkg --rootdir … -r FreeBSD-base install FreeBSD-set-…`), el
   usuario `routerpanel` (uid 3159, distinto de cualquiera del host), rutas, nullfs, rc.d, newsyslog
   (lo que va en directorios de un usuario de jail —`~rpbuild/.ssh/config`,
   `known_hosts`, el bundle, los permisos de un `router.secret` de antes— se
   hace como ese usuario desde dentro de la jail, nunca como root desde el
   host);
   paquetes, `rustup`, `cargo-leptos`, clave de despliegue y clon en la jail
   de compilación (si Forgejo aún no acepta la clave, clona del bundle); el
   actualizador y su cron en el host; `panel.home.arpa` en unbound
   (`local-unbound-checkconf` y `service local_unbound reload`); el Caddyfile
   (sólo si cambia; `service caddy reload`). Si el panel ya está desplegado y
   aún no tiene usuarios, dice dónde está el token del asistente
   (`/var/db/routerpanel/instalar.token` en la jail; no lo imprime). Al final
   imprime la clave pública de despliegue.
3. `--pf-cargar`: inserta `pf.conf.snippet` en `/etc/pf.conf`, lo comprueba con
   `pfctl -nf`, guarda la copia en `/etc/pf.conf.bak-routerpanel`, arma un
   interruptor de hombre muerto (`daemon`, 180 s: restaura la copia en el
   fichero y en el núcleo) y carga las reglas.
4. El portátil abre una conexión SSH **nueva** y comprueba los sitios web
   (HTTPS por Caddy) y el SSH de Forgejo (2222), y lanza `--comprobar-red`:
   el panel llega al router :80 pero no a Internet ni a Caddy; la jail de
   compilación llega a crates.io, a `/salud` y al SSH de Forgejo («Permission
   denied (publickey)» cuenta como que llega) pero no al router ni a Caddy.
   Sólo si todo cuadra, `--pf-confirmar` desarma el interruptor. Si no, las
   reglas vuelven solas.

### Clave de despliegue

La clave pública que imprime `install.sh` (también en
`/usr/local/jails/containers/routerpanel-build/home/rpbuild/.ssh/id_ed25519.pub`)
se da de alta en Forgejo como **clave de despliegue de sólo lectura** del
repositorio. Hasta entonces el actualizador registra «sin acceso al
repositorio» y termina sin hacer nada.

La jail de compilación habla directamente con la forja
(`192.0.2.30:2222`, alias `forja` en `~rpbuild/.ssh/config`):
`git.home.arpa` resuelve al proxy inverso, no a
la forja. La clave de host está fijada
(`HostKeyAlias git.home.arpa`, `StrictHostKeyChecking yes`); `install.sh`
comprueba su huella antes de escribirla.

### Primer despliegue

Mientras no exista ninguna etiqueta `v*` el actualizador no tiene nada que
desplegar. La primera vez se compila a mano la rama con el mismo guion:

```sh
ssh admin@192.0.2.10 doas routerpanel-actualizar --ref nucleo
deploy/install.sh            # otra vez: dice dónde está el token del asistente
```

Después, `https://panel.home.arpa/instalar` con ese token: crea el
administrador, guarda las credenciales del router y hace la lectura inicial.

`--ref` acepta una rama o un commit; la versión se llama `<ref>-<sha corto>`.
Si no hay acceso al repositorio, compila lo que ya tenga el clon.

## Actualizador

`routerpanel-actualizar` corre en el host como root (cron cada 15 min, con
`lockf` para no solaparse):

1. `git fetch --tags` en la jail de compilación (como `rpbuild`). Todas las
   órdenes de git cuyo resultado usa el host van con `/usr/local/bin/git` y el
   PATH del sistema (no el de `~rpbuild/.cargo/bin`, que puede escribir
   cualquier `build.rs`).
2. Última etiqueta `v*` por `sort -V`; si es la desplegada (o la que ya falló),
   termina. Lo que sale de la jail no es de fiar: el nombre de la versión
   (la etiqueta, o `<ref>-<sha>`) tiene que ser `[A-Za-z0-9][A-Za-z0-9._-]*`
   sin `..` (como mucho 100 caracteres) y el commit, 7–40 cifras
   hexadecimales; si no, el actualizador registra el error y no hace nada
   (el nombre acaba en rutas que root borra).
3. `cargo leptos build --release`, `cargo test --workspace` y
   `cargo test --workspace --features panel/ssr` (como `rpbuild`; si el
   `Cargo.lock` pide otra versión de `wasm-bindgen`, instala antes
   `wasm-bindgen-cli` de esa versión). Cada `cargo test` tiene una hora como
   mucho (`timeout`): una prueba colgada cuenta como fallo y no bloquea las
   ejecuciones siguientes.
4. Saca `target/release/panel` y `target/site/` como un `tar` generado
   **dentro** de la jail (`jexec -U rpbuild … tar -cf -`) y lo extrae en el
   host con `--no-same-owner --no-same-permissions` en
   `releases/.<versión>.tmp` (con `pipefail`: si falla el `tar` de la jail,
   la versión se descarta). Rechaza todo lo que no sea fichero o directorio
   normal (enlaces simbólicos, dispositivos…) y los enlaces duros; deja todo
   de root con 755/644 (el binario 755) y lo renombra a `releases/<versión>/`.
   El host, que corre como root, nunca abre rutas de la jail de compilación:
   las controla el código que se compila.
5. Para el servicio y copia `panel.db` (y `-wal`/`-shm` si los hay) a
   `/var/db/routerpanel/antes-de-actualizar/`, dentro de la jail y como el
   usuario `routerpanel` (se conserva la última copia).
6. Cambia `current` (rename atómico), arranca y espera a que `/salud` responda
   200 con `"ok":true` **y** a que quien escucha en el 3000 sea el binario de
   la versión nueva (`sockstat` + `ps`): 60 s de reloj como mucho. `/salud`
   se pregunta desde la jail de compilación, la única además de Caddy a la que
   pf deja llegar al 3000.
7. Si no responde: para, restaura la base de datos de antes (una versión nueva
   puede haber aplicado una migración que la anterior no conoce, y sqlx no
   arrancaría), vuelve a la versión anterior, arranca y borra la versión
   fallida. La etiqueta se reintenta en la ejecución siguiente y sólo queda
   marcada como fallida si falla dos veces seguidas (un tropiezo de red o de
   una prueba no la descarta). Sin versión anterior a la
   que volver (primer despliegue, o `current` apunta a una que ya no existe)
   el servicio queda **parado**, se quita `current` (rc.d no arranca nada sin
   él, tampoco al reiniciar el host) y se borra igualmente la fallida: nunca
   queda `current` en una versión que no respondió. Un `--ref` que da la
   misma versión que la desplegada no cambia nada: reinicia el servicio y
   comprueba `/salud`; si no responde, la deja desplegada y en marcha (era la
   que funcionaba; lo normal es que falle otra cosa, p. ej. el router).
8. Conserva las 3 versiones más recientes (nunca borra la desplegada) y borra
   los `releases/.*.tmp` que haya dejado una ejecución cortada.

Los registros se crean con `umask 027` (640).

**El actualizador sólo despliega el panel.** Los ficheros de `deploy/`
(`actualizar.sh`, `routerpanel.rc`, `pf.conf.snippet`, cron, newsyslog,
Caddyfile…) sólo los instala `deploy/install.sh`: si una versión cambia algo
de `deploy/`, hay que repetir `deploy/install.sh` desde el portátil (es
idempotente y vuelve a pasar por el interruptor de pf si cambian las reglas).
Hasta entonces el servidor sigue con los de antes, aunque el panel ya sea el
nuevo.

**Diferencia con el diseño**: el diseño ponía el cron en la jail de compilación
y una regla de doas en el host para reiniciar el servicio. No puede funcionar
(desde una jail no se llega al doas del host), así que el actualizador corre en
el host y sólo la compilación va dentro de la jail, sin privilegios. No hay
reglas de doas nuevas (`doas.conf.snippet`).

## Red (pf)

- Al `192.0.2.20:3000` sólo llegan Caddy (`.11`) y la jail de compilación
  (`.21`); el resto se bloquea, también desde la LAN y la VPN.
- La jail `routerpanel` sólo puede salir hacia la web del router
  (`192.168.1.1`, tcp 80). Hacia
  `em0` el tráfico va traducido por el nat (el filtro ve la IP del host), así
  que el nat de la jail lleva la etiqueta `ROUTERPANEL` y el filtro deja pasar
  sólo lo etiquetado hacia `192.168.1.1:80`; hacia el host y las demás jails (lo0,
  sin nat) se filtra por la dirección de origen. Las respuestas del 3000 casan
  con el estado del `pass` de entrada.
- La jail de compilación, que ejecuta código del repositorio, sale libremente
  a Internet (crates.io, rustup, pkg), pero de la red de casa y de la VPN
  sólo alcanza el SSH de Forgejo (`192.0.2.30:2222`), el DNS del host
  (`192.0.2.10:53`), el panel (`.20:3000`) y a sí misma (las pruebas
  escuchan en 127.0.0.1, que en la jail es su propia IP). Mismo truco: etiqueta
  `ROUTERPANEL_BUILD` en su nat para lo que va por `em0`.
- El interruptor de hombre muerto es
  `sh -c "sleep 180 && : >/var/run/routerpanel-pf-deadman.saltado && cp … && pfctl -f …"`
  bajo `daemon -p` (el pidfile es el del `sh`). Para desarmarlo se mata primero
  el `sh` y luego su `sleep`; y con `&&`, aunque el `sleep` muriera antes, no se
  restauraría nada (con `;` sí). `daemon` borra el pidfile cuando el `sh`
  termina, así que «saltó» se reconoce por la marca `.saltado` (se crea justo
  antes de restaurar y `--pf-cargar` la borra al armar): con ella,
  `--pf-confirmar` falla con «el interruptor ya saltó» (las reglas vigentes son
  las anteriores) y hay que repetir `--pf-cargar`; sin marca ni pidfile, dice
  que no hay interruptor armado.

Por eso la jail `routerpanel` **no** se actualiza con `pkg -j` (no tiene red ni
`pkg`): desde el host,

```sh
doas pkg --rootdir /usr/local/jails/containers/routerpanel upgrade -r FreeBSD-base
doas jexec routerpanel service routerpanel restart
```

La de compilación sí, como las demás: `doas pkg -j routerpanel-build upgrade`.

## Operación

```sh
doas jexec routerpanel service routerpanel status|restart
doas tail /usr/local/jails/containers/routerpanel/var/log/routerpanel.log
doas tail /var/log/routerpanel-update.log
doas jexec -U routerpanel routerpanel env RP_DB=/var/db/routerpanel/panel.db \
    /usr/local/routerpanel/current/panel user passwd <usuario>
```

Volver a mano a otra versión: `ln -sfh releases/<versión>` sobre
`/usr/local/jails/containers/routerpanel/usr/local/routerpanel/current` y
reiniciar el servicio.
