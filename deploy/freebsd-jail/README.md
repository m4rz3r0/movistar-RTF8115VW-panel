# Instalación en jails de FreeBSD

El panel corre en un servidor FreeBSD (probado en 15.x) con dos jails PkgBase:

| Jail (`config.local`) | Contenido |
|---|---|
| `JAIL_PANEL` (`JAIL_PANEL_IP`) | base mínima (`FreeBSD-set-minimal-jail`), el binario y su web; servicio `routerpanel` en `PANEL_ESCUCHA` |
| `JAIL_COMPILAR` (`JAIL_COMPILAR_IP`) | base completa, `git`, `binaryen` (`wasm-opt`), `rustup` 1.28.1 o posterior y `cargo-leptos` 0.3.11; usuario sin privilegios `rpbuild` (uid `UID_COMPILAR`) con el clon en `/home/rpbuild/panel` |

`rustup` instala al compilar la versión de Rust de `rust-toolchain.toml` con
`wasm32-unknown-unknown` (la 1.28.0 no lo hace). El actualizador instala
`wasm-bindgen-cli` en la versión que pida el `Cargo.lock`.

Se entra por un proxy inverso con HTTPS (`PROXIES_CONFIANZA`) en
`PANEL_ORIGEN`. Con el módulo `caddyfile`, `install.sh` imprime el sitio para
el Caddyfile; con `unbound`, mete el nombre del panel en el unbound del host.

## Configuración

Todo lo propio de cada instalación va en `deploy/config.local`, que git
ignora. Cópialo de [`deploy/config.example`](../config.example), que explica
cada clave, y cambia los valores. Si falta una clave obligatoria,
`install.sh` se para y dice cuál. Con `--config FICHERO` usa otro fichero.

Para ver lo que se va a instalar sin conectarte a nada:

```sh
deploy/freebsd-jail/install.sh --plantillas /tmp/plantillas
```

Deja ahí `jail.conf`, `pf.conf`, `rc.conf` (las variables del servicio), el
`resolv.conf` de la jail de compilación, `actualizador.conf`, la
configuración SSH del usuario que compila y, según `MODULOS`,
`Caddyfile.routerpanel` y `unbound.conf`.

## Ficheros

| Fichero | Se instala en |
|---|---|
| `install.sh` | se ejecuta desde el equipo de trabajo |
| `jail.conf.plantilla` | añadido a `/etc/jail.conf` del host |
| `pf.conf.plantilla` | dos bloques en `/etc/pf.conf` del host, entre marcas `# >>> routerpanel` / `# <<< routerpanel` |
| `routerpanel.rc` | `/usr/local/etc/rc.d/routerpanel` en la jail del panel |
| `actualizar.sh` | `/usr/local/sbin/routerpanel-actualizar` en el host |
| `../lib/git.sh`, `../lib/precompiladas.sh` | `/usr/local/libexec/routerpanel-actualizar/` en el host |
| `recibir-precompilada.sh` | `/usr/local/libexec/routerpanel-actualizar/` en el host ([versiones precompiladas](#versiones-precompiladas-el-pc-de-compilación)) |
| `doas.conf.snippet` | nada: explica por qué no hace falta ninguna regla de doas |

## Rutas

En el host:

- `/usr/local/routerpanel/releases/<versión>/{panel,site/,COMMIT}`: las
  versiones compiladas, de root. Entran por nullfs `ro,late` (`/etc/fstab`)
  en la misma ruta de la jail del panel.
- `/usr/local/etc/routerpanel-actualizar.conf` (600): configuración del
  actualizador.
- `/usr/local/etc/cron.d/routerpanel`: el actualizador cada 15 minutos.
- `/var/log/routerpanel-update.log`, rotado por
  `/usr/local/etc/newsyslog.conf.d/routerpanel.conf`, y
  `/var/log/routerpanel-build.log`, con la salida de la última compilación.
- `/var/db/routerpanel-actualizar/fallida`: la última etiqueta que falló dos
  veces seguidas. No se reintenta hasta que haya otra.
- `/var/db/routerpanel-buzon/entrada/<versión>/`: solo con versiones
  precompiladas, lo que deja el PC de compilación.

En la jail del panel:

- `/usr/local/routerpanel/current` → `releases/<versión>`: enlace relativo que
  cambia el actualizador.
- `/var/db/routerpanel/panel.db`: la base de datos SQLite (usuarios,
  credenciales del router cifradas, historial, ajustes). Mientras el panel no
  tiene usuarios, en ese directorio está `instalar.token`, el token del
  asistente. La copia previa a cada actualización queda en
  `/var/db/routerpanel/antes-de-actualizar/`.
- `/usr/local/etc/routerpanel/history.key` (600): la clave maestra. La crea el
  panel al arrancar y cifra las credenciales del router y los secretos del
  historial. No la borres.
- `/usr/local/etc/routerpanel/router.secret`: solo en instalaciones anteriores
  al asistente. El panel importa la contraseña una vez y después puedes
  borrarlo
  ([`docs/operacion.md`](../../docs/operacion.md#actualizar-desde-una-versión-anterior-al-asistente-v01x)).
- `/var/log/routerpanel.log`: la salida del panel, rotada por newsyslog.

## El servicio

`routerpanel.rc` lee estas variables del `rc.conf` de la jail. `install.sh`
pone las cuatro primeras con `sysrc`.

| Variable | Pasa al panel | Por defecto |
|---|---|---|
| `routerpanel_enable` | | `NO` |
| `routerpanel_origin` | `RP_ORIGIN` (`PANEL_ORIGEN`) | ninguno: sin ella el servicio no arranca y dice por qué |
| `routerpanel_listen` | `RP_LISTEN` y `LEPTOS_SITE_ADDR` (`PANEL_ESCUCHA`) | `127.0.0.1:3000` |
| `routerpanel_trusted_proxy` | `RP_TRUSTED_PROXY` (`PROXIES_CONFIANZA`) | `127.0.0.1` |
| `routerpanel_install_nets` | `RP_INSTALL_NETS`, solo si no está vacía | vacía |
| `routerpanel_router_url`, `_user`, `_secret` | `RP_ROUTER_URL`, `RP_ROUTER_USER`, `RP_ROUTER_SECRET`, solo si existe el fichero `_secret` (instalaciones anteriores al asistente) | `http://192.168.1.1`, `1234` y `/usr/local/etc/routerpanel/router.secret` |

## Instalar

Desde el equipo de trabajo, en la raíz del repositorio y con
`deploy/config.local` preparado:

```sh
deploy/freebsd-jail/install.sh
```

No pide la contraseña del router ni crea usuarios: eso lo hace el asistente
del panel (`/instalar`, ver
[`docs/operacion.md`](../../docs/operacion.md#primera-instalación-el-asistente)).
Puedes repetirlo cuantas veces quieras; solo hace lo que falta. Pasos:

1. Sube los guiones de `deploy/`, la configuración ya resuelta y un
   `git bundle` del repositorio a `~/routerpanel-deploy` de `SERVIDOR_SSH`.
2. Ejecuta allí `install.sh --servidor` como root con `doas`. Comprueba la
   sintaxis de los guiones y añade las jails a `/etc/jail.conf`. Según
   `MODULOS`:
   - `rc`: dataset ZFS (`ZFS_DATASET/<jail>`) y base de la jail del panel,
     usuario `routerpanel` (uid 3159), nullfs, rc.d con sus variables y
     newsyslog. Si el panel ya está desplegado y no tiene usuarios, dice
     dónde está el token del asistente (no lo imprime).
   - `actualizador`: la jail de compilación con sus paquetes, `rustup`,
     `cargo-leptos`, la clave de despliegue y el clon de `REPO_URL` (si el
     servidor git aún no acepta la clave, clona del bundle). En el host, el
     actualizador, su configuración y su cron. Al final imprime la clave
     pública de despliegue.
   - `unbound`: `/var/unbound/conf.d/routerpanel.conf` con el nombre de
     `PANEL_ORIGEN` apuntando al primer proxy de `PROXIES_CONFIANZA` (o a la
     jail del panel). Lo comprueba con `local-unbound-checkconf` y recarga
     `local_unbound`.
   - `caddyfile`: imprime el sitio para el Caddyfile; no lo instala.
3. Con el módulo `pf`, `--pf-cargar` mete el bloque de nat antes de la
   primera regla `nat` de `/etc/pf.conf` y el de filtro antes de `block all`.
   Lo comprueba con `pfctl -nf`, guarda una copia en
   `/etc/pf.conf.bak-routerpanel`, arma un interruptor de hombre muerto de
   180 s y carga las reglas.
4. El equipo de trabajo abre una conexión SSH nueva, comprueba las webs de
   `COMPROBAR_URLS` y lanza `--comprobar-red`. El panel tiene que llegar al
   router por el puerto 80 y no a Internet ni al proxy. La jail de
   compilación tiene que llegar a crates.io, a `/salud` y al repositorio
   («Permission denied (publickey)» cuenta como que llega), pero no al
   router ni al proxy. Solo si todo cuadra, `--pf-confirmar` desarma el
   interruptor. Si no, las reglas anteriores vuelven solas a los 180 s.

Si `--pf-confirmar` dice «el interruptor ya saltó», pf tiene otra vez las
reglas anteriores: repite `install.sh`.

### Repositorio y clave de despliegue

`REPO_URL` admite cualquier URL de git: `https://…` o
`ssh://git@equipo:puerto/…`. Si el servidor git está en tu red, pon su IP en
la URL: pf solo abre a la jail de compilación esa IP y ese puerto.

Por SSH, da de alta en el servidor git la clave pública que imprime
`install.sh` como clave de despliegue de solo lectura. También está en
`<JAILS_RAIZ>/<JAIL_COMPILAR>/home/rpbuild/.ssh/id_ed25519.pub`. Hasta
entonces el actualizador registra «sin acceso al repositorio» y no hace nada.
La clave del servidor git queda fijada: la de `REPO_HOST_CLAVE` (una línea de
`known_hosts`) o, si está vacía, la primera que presente.

### Primer despliegue

Mientras no exista ninguna etiqueta `v*`, el actualizador no tiene nada que
desplegar. La primera vez compila una rama a mano:

```sh
ssh SERVIDOR doas routerpanel-actualizar --ref RAMA
deploy/freebsd-jail/install.sh   # otra vez: dice dónde está el token del asistente
```

Después abre `PANEL_ORIGEN/instalar` con ese token. El asistente crea el
administrador, guarda las credenciales del router y hace la lectura inicial.

`--ref` acepta una rama o un commit y la versión se llama
`<ref>-<sha corto>`. Si no hay acceso al repositorio, compila lo que ya tenga
el clon.

## Actualizador

`routerpanel-actualizar` corre en el host como root, desde cron cada 15
minutos y sin solaparse (`lockf`). Así no hace falta ninguna regla de doas.
En cada ejecución:

1. En la jail de compilación, como `rpbuild`, trae el repositorio y saca la
   última etiqueta `v*` (por `sort -V`). Nunca hace merge, así que no le
   afecta que el remoto reescriba su historial.
2. Si esa etiqueta ya está desplegada, o ya falló, termina. Si el nombre de
   la versión o el commit tienen una forma rara, registra el error y no hace
   nada.
3. Con `PRECOMPILADAS_ESPERA`, mira antes el buzón (ver
   [versiones precompiladas](#versiones-precompiladas-el-pc-de-compilación)).
   Si instala la precompilada, salta al paso 5.
4. Compila con `cargo leptos build --release --precompress`, con prioridad
   baja, una hora como mucho y, si fijaste `TRABAJOS_COMPILACION`, ese número
   de trabajos a la vez (gasta menos memoria). Copia el binario y la web a
   `releases/<versión>/`, de root.
5. Para el servicio y copia `panel.db` a
   `/var/db/routerpanel/antes-de-actualizar/`.
6. Cambia `current`, arranca y espera como mucho 60 s a que `/salud` responda
   con `"ok":true` desde el binario nuevo.
7. Si no responde, para el servicio, restaura la base de datos de antes,
   vuelve a la versión anterior y borra la nueva. La etiqueta se reintenta en
   la siguiente ejecución y solo queda marcada como fallida si falla dos
   veces seguidas. Si no hay versión anterior (primer despliegue), el
   servicio queda parado y sin `current`, también tras reiniciar el host.
8. Conserva las 3 versiones más recientes; nunca borra la desplegada.

Un `--ref` que da la misma versión que la desplegada solo reinicia el
servicio y comprueba `/salud`. Si no responde, la deja como estaba.

El servidor solo compila, no ejecuta las pruebas. Pásalas en el equipo de
desarrollo antes de crear la etiqueta (`cargo test --workspace`,
`cargo test --workspace --features panel/ssr` y los e2e): el actualizador
despliega cualquier `v*` que compile, y solo `/salud` y la vuelta atrás te
protegen de una versión rota.

El actualizador solo despliega el panel. Si una versión cambia algo de
`deploy/` (el actualizador, `routerpanel.rc`, las reglas de pf…), repite
`install.sh` desde el equipo de trabajo; hasta entonces el servidor sigue con
los ficheros de antes.

## Versiones precompiladas (el PC de compilación)

Es opcional. Un PC con Linux más potente compila la versión para FreeBSD y
la deja en un buzón del servidor, y el actualizador la instala en lugar de
compilarla. Si el PC está apagado, el servidor compila él mismo pasado un
rato. Sin `PRECOMPILADAS_ESPERA` no se usa nada de esto.

### Cómo funciona

1. Después de crear y subir la etiqueta, desde el equipo de trabajo:

   ```sh
   deploy/cross/lanzar.sh vX.Y.Z
   ```

   Comprueba que es una etiqueta `vX.Y.Z` y que no hay cambios sin
   confirmar, y manda al PC (`PC_COMPILAR_SSH`) los guiones y un `git bundle`
   de la etiqueta. Allí `compilar-freebsd.sh` compila y `entregar.sh` lo sube
   al buzón (`PRECOMPILADAS_BUZON_SSH`). Si el PC no
   responde en 5 s, lo dice y sale con 3: el servidor compilará él mismo.
   `--prueba [REF]` compila cualquier referencia sin entregarla.
2. El PC saca el código del commit de la etiqueta con `git archive` a
   `~/rtf-cross/fuente`, nunca de una copia de trabajo. Compila como el
   servidor, con la misma versión de `cargo-leptos`, y el binario para
   `x86_64-unknown-freebsd` con [`cross`](https://github.com/cross-rs/cross).
   Empaqueta `routerpanel-<versión>-x86_64-unknown-freebsd.tar.gz` y
   `SHA256SUMS`; con `RP_MINISIGN_SECRETA`, también `SHA256SUMS.minisig`.
3. `entregar.sh` sube los ficheros por SSH como el usuario `rpentrega` del
   servidor, que solo puede ejecutar `recibir-precompilada.sh`. Este guarda
   cada fichero en `/var/db/routerpanel-buzon/entrada/<versión>/`, solo con
   los nombres admitidos, hasta 300 MB por fichero y las 3 versiones más
   recientes.
4. Cuando el actualizador ve una etiqueta nueva:
   - Si ya está en el buzón, comprueba la firma (con
     `PRECOMPILADAS_MINISIGN`), el sha256, que `COMMIT` es el de la etiqueta
     y que `panel` es un ejecutable de FreeBSD x86-64. Si todo cuadra, la
     despliega como una versión propia, con copia de la base de datos,
     `/salud` y vuelta atrás. Si algo falla, lo registra y compila en la
     misma ejecución. En los dos casos vacía esa carpeta del buzón.
   - Si no ha llegado, espera hasta `PRECOMPILADAS_ESPERA` minutos desde que
     vio la etiqueta y después compila él mismo. Como el cron va cada 15
     minutos, la espera se redondea hacia arriba: 20 minutos pueden ser 30.
     Con `PRECOMPILADAS_ESPERA=0` no espera, pero usa la precompilada si ya
     está.

   Si la precompilada no arranca, la vuelta atrás es la de siempre y el
   siguiente intento compila en el servidor. Los `--ref` nunca miran el
   buzón.

### Seguridad del buzón

La clave del PC solo vale para `rpentrega`, con `restrict` (sin terminal,
reenvíos ni agente), `command=` (la única orden es
`recibir-precompilada.sh`) y `from=` (solo desde la IP del PC). No hay que
tocar `sshd_config`. Nada de lo que llega se ejecuta en el host: root lo
comprueba todo sobre una copia propia antes de instalarlo.

Quien controle el PC de compilación o su clave del buzón puede desplegar
otro binario en la jail del panel, igual que quien pueda crear etiquetas en
el repositorio: el commit tiene que ser el de la etiqueta, pero el binario no
se puede comprobar contra el código. Con minisign solo vale lo firmado con
una clave que no sale del PC.

### Instalar (a mano, una vez)

En `deploy/config.local` del equipo de trabajo:

```sh
PRECOMPILADAS_ESPERA=20
PC_COMPILAR_SSH=usuario@pc-de-compilacion
PRECOMPILADAS_BUZON_SSH=rpentrega@<IP del servidor vista desde el PC>
# Opcional: PRECOMPILADAS_MINISIGN=<la línea de la clave pública>
```

En el PC de compilación hace falta Linux con `rustup`, `git`, `podman`,
`cross`, `rsync`, `file` y `flock`. Si falta `cargo-leptos` o tiene otra
versión, el guion lo instala. Crea la clave del buzón:

```sh
ssh-keygen -t ed25519 -N '' -C routerpanel-buzon -f ~/.ssh/routerpanel-buzon
cat ~/.ssh/routerpanel-buzon.pub     # para el authorized_keys de abajo
```

En el servidor, como root (`doas`), con la IP y la clave del PC:

```sh
pw groupadd rpentrega -g 3161
pw useradd rpentrega -u 3161 -g rpentrega -d /var/db/routerpanel-buzon \
    -s /bin/sh -w no -c "Buzón de versiones precompiladas del panel"
install -d -m 755 -o root -g wheel /var/db/routerpanel-buzon /var/db/routerpanel-buzon/.ssh
install -d -m 750 -o rpentrega -g rpentrega /var/db/routerpanel-buzon/entrada
printf '%s\n' 'restrict,from="IP_DEL_PC",command="/usr/local/libexec/routerpanel-actualizar/recibir-precompilada.sh" <el contenido de routerpanel-buzon.pub>' \
    >/var/db/routerpanel-buzon/.ssh/authorized_keys
chown root:wheel /var/db/routerpanel-buzon/.ssh/authorized_keys
chmod 644 /var/db/routerpanel-buzon/.ssh/authorized_keys
sshd -T | grep -Ei '^(allowusers|allowgroups|denyusers|denygroups)'
```

- El `authorized_keys` es de root para que `rpentrega` no pueda quitarse las
  restricciones. `-w no` lo deja sin contraseña. Necesita `/bin/sh` porque
  sshd ejecuta la orden forzada con su shell.
- Si `sshd -T` muestra `AllowUsers` o `AllowGroups`, añade `rpentrega` en
  `sshd_config` y haz `service sshd reload`. Si pf limita quién llega al SSH
  del host, deja pasar al PC.
- Para firmar: `pkg install minisign` en el host y `PRECOMPILADAS_MINISIGN`
  en `config.local`. En el PC,
  `minisign -G -W -s ~/.config/routerpanel/minisign.key -p ~/.config/routerpanel/minisign.pub`
  y `RP_MINISIGN_SECRETA=~/.config/routerpanel/minisign.key` en el entorno de
  `lanzar.sh` o `compilar-freebsd.sh`. Con `-W` la clave no lleva contraseña
  y el PC firma solo.

Después repite `deploy/freebsd-jail/install.sh` desde el equipo de trabajo,
que pasa `PRECOMPILADAS_ESPERA` y `PRECOMPILADAS_MINISIGN` al actualizador.
Por último, en el PC, fija la clave del servidor y prueba el buzón
(`recibir` sin argumentos responde con el uso y no guarda nada):

```sh
ssh-keyscan -t ed25519 <servidor> >>~/.ssh/known_hosts   # compárala con ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub del servidor
ssh -i ~/.ssh/routerpanel-buzon -o IdentitiesOnly=yes rpentrega@<servidor> recibir
```

Y una compilación de prueba desde el equipo de trabajo:
`deploy/cross/lanzar.sh --prueba`.

### Temporizador en el PC (opcional)

En vez de lanzarlo a mano, el PC puede mirar cada 15 minutos, mientras está
encendido, si hay una etiqueta nueva, compilarla y entregarla. No repite lo
ya entregado. Necesita llegar al repositorio (`REPO_URL`) y tener los guiones
en `~/rtf-cross/bin`, que copia `lanzar.sh`. No viene activado:

```sh
cp deploy/cross/rtf-cross.service deploy/cross/rtf-cross.timer ~/.config/systemd/user/
printf 'RP_REPO_URL=%s\nRP_BUZON_SSH=%s\n' '<REPO_URL>' 'rpentrega@<servidor>' >~/.config/rtf-cross.env
systemctl --user daemon-reload
systemctl --user enable --now rtf-cross.timer
```

Las unidades de usuario solo corren con la sesión abierta, salvo que
actives `loginctl enable-linger`.

## Red (pf)

- Al puerto del panel solo llegan los proxies de confianza y la jail de
  compilación. El resto se bloquea, también desde la red local y la VPN.
- La jail del panel solo puede salir a la web del router (`ROUTER_IP`, tcp
  80).
- La jail de compilación ejecuta código del repositorio. Sale libremente a
  Internet (crates.io, rustup, pkg), pero de `RED_LAN` y `RED_VPN` solo
  alcanza el repositorio git (si `REPO_URL` lleva una IP), el DNS
  (`DNS_IP`:53) y el panel.

Como la jail del panel no tiene red ni `pkg`, no se actualiza con `pkg -j`.
Hazlo desde el host:

```sh
doas pkg --rootdir <JAILS_RAIZ>/<JAIL_PANEL> upgrade -r FreeBSD-base
doas jexec <JAIL_PANEL> service routerpanel restart
```

La de compilación sí, como cualquier otra: `doas pkg -j <JAIL_COMPILAR> upgrade`.

## Operación

```sh
doas jexec <JAIL_PANEL> service routerpanel status|restart
doas tail <JAILS_RAIZ>/<JAIL_PANEL>/var/log/routerpanel.log
doas tail /var/log/routerpanel-update.log
doas jexec -U routerpanel <JAIL_PANEL> env RP_DB=/var/db/routerpanel/panel.db \
    /usr/local/routerpanel/current/panel user passwd <usuario>
```

Para volver a mano a otra versión, apunta
`<JAILS_RAIZ>/<JAIL_PANEL>/usr/local/routerpanel/current` a ella con
`ln -sfh releases/<versión>` y reinicia el servicio. Si la versión que
quitas migró la base de datos, la anterior puede no arrancar: con el
servicio parado, vuelve a poner antes los ficheros de
`/var/db/routerpanel/antes-de-actualizar/`, que son la copia previa al último
despliegue.
