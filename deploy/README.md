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
- `/var/db/routerpanel-actualizar/fallida` — última versión que no compiló o
  no arrancó; no se reintenta hasta que haya otra etiqueta.
- `/root/routerpanel-admin-password.txt` (600) — contraseña inicial del usuario
  `admin` del panel.

En la jail `routerpanel`:

- `/usr/local/routerpanel/current` → `releases/<versión>` (enlace relativo; lo
  cambia el actualizador).
- `/var/db/routerpanel/panel.db` — SQLite (directorio 700 del usuario
  `routerpanel`).
- `/usr/local/etc/routerpanel/router.secret` (600, `routerpanel`) — contraseña
  del router, sin salto de línea; `history.key` lo crea el panel al arrancar
  (600) en el mismo directorio (700, `routerpanel`).
- `/var/log/routerpanel.log` — salida del panel; la escribe `daemon(8)` y la
  rota newsyslog (`/etc/newsyslog.conf.d/routerpanel.conf`, SIGHUP a `daemon -H`).

El servicio arranca con `RP_LISTEN=0.0.0.0:3000`, `RP_DB`, `RP_ROUTER_URL=http://192.168.1.1`,
`RP_ROUTER_USER=1234`, `RP_ROUTER_SECRET`, `RP_HISTORY_KEY`,
`RP_TRUSTED_PROXY=192.0.2.11`, `RP_ORIGIN=https://panel.home.arpa`,
`RP_LOG=info` y `LEPTOS_OUTPUT_NAME=panel`, `LEPTOS_SITE_ROOT=<versión>/site`,
`LEPTOS_SITE_ADDR=0.0.0.0:3000`, `LEPTOS_ENV=PROD` (ver `routerpanel.rc`).

## Instalar

Desde el portátil, en la raíz del repositorio (con `~/servidor-casa` al lado,
de donde sale el Caddyfile; otra ruta con `CADDYFILE=...`):

```sh
deploy/install.sh            # pide la contraseña del router si aún no está
```

Es idempotente: se puede repetir y sólo hace lo que falta. Pasos:

1. Sube `deploy/`, el Caddyfile y un `git bundle` del repositorio a
   `~/routerpanel-deploy` del servidor.
2. `--servidor` (root): valida los guiones y el Caddyfile (con el binario y el
   entorno del servicio de Caddy: `caddy_command`, `caddy_env`); crea las jails
   (dataset ZFS, `pkg --rootdir … -r FreeBSD-base install FreeBSD-set-…`), el
   usuario `routerpanel` (uid 3159, distinto de cualquiera del host), rutas, nullfs, `router.secret`, rc.d, newsyslog;
   paquetes, `rustup`, `cargo-leptos`, clave de despliegue y clon en la jail
   de compilación (si Forgejo aún no acepta la clave, clona del bundle); el
   actualizador y su cron en el host; `panel.home.arpa` en unbound
   (`local-unbound-checkconf` y `service local_unbound reload`); el Caddyfile
   (sólo si cambia; `service caddy reload`); y, si ya hay versión desplegada,
   el usuario `admin` del panel. Al final imprime la clave pública de
   despliegue.
3. `--pf-cargar`: inserta `pf.conf.snippet` en `/etc/pf.conf`, lo comprueba con
   `pfctl -nf`, guarda la copia en `/etc/pf.conf.bak-routerpanel`, arma un
   interruptor de hombre muerto (`daemon`, 180 s: restaura la copia en el
   fichero y en el núcleo) y carga las reglas.
4. El portátil abre una conexión SSH **nueva** y comprueba los sitios web
   (HTTPS por Caddy) y el SSH de Forgejo (2222). Sólo si todo responde,
   `--pf-confirmar` desarma el interruptor. Si no, las reglas vuelven solas.

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
deploy/install.sh            # otra vez: ahora crea el usuario del panel
```

`--ref` acepta una rama o un commit; la versión se llama `<ref>-<sha corto>`.
Si no hay acceso al repositorio, compila lo que ya tenga el clon.

## Actualizador

`routerpanel-actualizar` corre en el host como root (cron cada 15 min, con
`lockf` para no solaparse):

1. `git fetch --tags` en la jail de compilación (como `rpbuild`).
2. Última etiqueta `v*` por `sort -V`; si es la desplegada (o la que ya falló),
   termina.
3. `cargo leptos build --release`, `cargo test --workspace` y
   `cargo test --workspace --features panel/ssr` (como `rpbuild`; si el
   `Cargo.lock` pide otra versión de `wasm-bindgen`, instala antes
   `wasm-bindgen-cli` de esa versión).
4. Copia `target/release/panel` y `target/site/` a `releases/<etiqueta>/`.
5. Cambia `current` (rename atómico), `jexec routerpanel service routerpanel restart`
   y espera a que `/salud` responda 200 con `"ok":true` (60 s como mucho; se
   pregunta desde la jail de compilación, la única además de Caddy a la que pf
   deja llegar al 3000).
6. Si no responde: vuelve a la versión anterior y la reinicia; la etiqueta
   queda marcada como fallida.
7. Conserva las 3 versiones más recientes (nunca borra la desplegada).

**Diferencia con el diseño**: el diseño ponía el cron en la jail de compilación
y una regla de doas en el host para reiniciar el servicio. No puede funcionar
(desde una jail no se llega al doas del host), así que el actualizador corre en
el host y sólo la compilación va dentro de la jail, sin privilegios. No hay
reglas de doas nuevas (`doas.conf.snippet`).

## Red (pf)

- Al `192.0.2.20:3000` sólo llegan Caddy (`.11`) y la jail de compilación
  (`.21`); el resto se bloquea, también desde la LAN y la VPN.
- La jail `routerpanel` sólo puede salir hacia el router (`192.168.1.1`). Hacia
  `em0` el tráfico va traducido por el nat (el filtro ve la IP del host), así
  que el nat de la jail lleva la etiqueta `ROUTERPANEL` y el filtro deja pasar
  sólo lo etiquetado hacia `192.168.1.1`; hacia el host y las demás jails (lo0,
  sin nat) se filtra por la dirección de origen. Las respuestas del 3000 casan
  con el estado del `pass` de entrada.
- La jail de compilación sale libremente (crates.io, rustup, pkg, Forgejo).

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
    /usr/local/routerpanel/current/panel user passwd admin
```

Volver a mano a otra versión: `ln -sfh releases/<versión>` sobre
`/usr/local/jails/containers/routerpanel/usr/local/routerpanel/current` y
reiniciar el servicio.
