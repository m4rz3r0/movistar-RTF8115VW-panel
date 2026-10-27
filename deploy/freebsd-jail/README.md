# Instalación en jails de FreeBSD

El panel corre en un servidor FreeBSD (probado en 15.x) en dos jails PkgBase:

| Jail (`config.local`) | Contenido |
|---|---|
| `JAIL_PANEL` (`JAIL_PANEL_IP`) | base mínima (`FreeBSD-set-minimal-jail`), el binario y su web; servicio `routerpanel` en `PANEL_ESCUCHA` |
| `JAIL_COMPILAR` (`JAIL_COMPILAR_IP`) | base completa, `rustup` 1.28.1 o posterior (instala solo, al compilar, la versión de Rust de `rust-toolchain.toml` con `wasm32-unknown-unknown`; la 1.28.0 no lo hace), `cargo-leptos` 0.3.11, `wasm-bindgen-cli` (la versión del `Cargo.lock`), `binaryen` (`wasm-opt`), `git`; usuario sin privilegios `rpbuild` (uid `UID_COMPILAR`) con el clon en `/home/rpbuild/panel` |

Se entra por un proxy inverso con HTTPS (`PROXIES_CONFIANZA`) en
`PANEL_ORIGEN`. Con el módulo `caddyfile`, `install.sh` imprime el sitio para el
Caddyfile; con `unbound`, mete el nombre del panel en el unbound del host.

## Configuración

Todo lo propio de cada instalación está en `deploy/config.local` (git lo
ignora): cópialo de [`deploy/config.example`](../config.example), que explica
cada clave, y cambia los valores. Si falta una clave obligatoria,
`install.sh` se para y dice cuál.

Para revisar lo que se va a instalar sin conectarse a nada:

```sh
deploy/freebsd-jail/install.sh --plantillas /tmp/plantillas
```

genera ahí `jail.conf`, `pf.conf` (los dos bloques de reglas), `rc.conf`
(las variables del servicio), `resolv.conf` de la jail de compilación,
`actualizador.conf`, la configuración SSH del usuario que compila y, según
`MODULOS`, `Caddyfile.routerpanel` y `unbound.conf`.

## Ficheros

| Fichero | Se instala en |
|---|---|
| `install.sh` | se ejecuta desde el equipo de trabajo; ver abajo |
| `jail.conf.plantilla` | añadido a `/etc/jail.conf` del host |
| `pf.conf.plantilla` | dos bloques en `/etc/pf.conf` del host, entre marcas `# >>> routerpanel` / `# <<< routerpanel` |
| `routerpanel.rc` | `/usr/local/etc/rc.d/routerpanel` en la jail del panel |
| `actualizar.sh` | `/usr/local/sbin/routerpanel-actualizar` en el **host** |
| `../lib/git.sh` | `/usr/local/libexec/routerpanel-actualizar/git.sh` en el host |
| `../lib/precompiladas.sh` | `/usr/local/libexec/routerpanel-actualizar/precompiladas.sh` en el host ([versiones precompiladas](#versiones-precompiladas-el-pc-de-compilación)) |
| `recibir-precompilada.sh` | `/usr/local/libexec/routerpanel-actualizar/recibir-precompilada.sh` en el host: la orden forzada del usuario del buzón |
| `doas.conf.snippet` | nada: explica por qué no hace falta regla de doas |

## Rutas

En el host:

- `/usr/local/routerpanel/releases/<versión>/{panel,site/,COMMIT}` — versiones
  compiladas (root, sólo lectura para los demás). Se monta por nullfs `ro,late`
  (`/etc/fstab`) en `/usr/local/routerpanel/releases` de la jail del panel.
- `/usr/local/etc/routerpanel-actualizar.conf` (600) — jails, `REPO_URL` y `TRABAJOS_COMPILACION` del
  actualizador.
- `/usr/local/etc/cron.d/routerpanel` — el actualizador cada 15 minutos.
- `/var/log/routerpanel-update.log` (rotado por
  `/usr/local/etc/newsyslog.conf.d/routerpanel.conf`) y
  `/var/log/routerpanel-build.log` (salida de cargo de la última compilación).
- `/var/db/routerpanel-actualizar/fallida` — última etiqueta que falló dos
  veces seguidas (no compiló o no arrancó); no se reintenta hasta que haya
  otra. `intentos` (etiqueta y número) lleva la cuenta del primer fallo. Los
  `--ref` no tocan ninguno de los dos. `precompilada` (etiqueta y hora) es
  cuándo se vio la etiqueta sin versión precompilada; solo con
  `PRECOMPILADAS_ESPERA`.
- `/var/db/routerpanel-buzon/entrada/<versión>/` — solo con versiones
  precompiladas: lo que deja el PC de compilación (del usuario `rpentrega`).

En la jail del panel:

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
  ([`docs/operacion.md`](../../docs/operacion.md#actualizar-desde-una-versión-anterior-al-asistente-v01x)).
- `/var/log/routerpanel.log` — salida del panel; la escribe `daemon(8)` y la
  rota newsyslog (`/etc/newsyslog.conf.d/routerpanel.conf`, SIGHUP a `daemon -H`).

## El servicio

`routerpanel.rc` lee estas variables de `rc.conf` (en la jail;
`install.sh` pone las cuatro primeras con `sysrc`):

| Variable | Pasa al panel | Por defecto |
|---|---|---|
| `routerpanel_enable` | | `NO` |
| `routerpanel_origin` | `RP_ORIGIN` (`PANEL_ORIGEN`) | ninguno: sin ella el servicio no arranca y dice por qué |
| `routerpanel_listen` | `RP_LISTEN` y `LEPTOS_SITE_ADDR` (`PANEL_ESCUCHA`) | `127.0.0.1:3000` |
| `routerpanel_trusted_proxy` | `RP_TRUSTED_PROXY` (`PROXIES_CONFIANZA`) | `127.0.0.1` |
| `routerpanel_install_nets` | `RP_INSTALL_NETS`, sólo si no está vacía | vacía |
| `routerpanel_router_url`, `_user`, `_secret` | `RP_ROUTER_URL`, `RP_ROUTER_USER`, `RP_ROUTER_SECRET`, sólo si existe el fichero `_secret` (instalaciones anteriores al asistente) | `http://192.168.1.1`, `1234` y `/usr/local/etc/routerpanel/router.secret` |

Además: `RP_DB`, `RP_HISTORY_KEY`, `RP_LOG=info`, `LEPTOS_OUTPUT_NAME=panel`,
`LEPTOS_SITE_ROOT=<versión>/site` y `LEPTOS_ENV=PROD`.

## Instalar

Desde el equipo de trabajo, en la raíz del repositorio, con
`deploy/config.local` preparado:

```sh
deploy/freebsd-jail/install.sh
```

No pide la contraseña del router ni crea usuarios: eso lo hace el asistente
de instalación del panel (`/instalar`, ver
[`docs/operacion.md`](../../docs/operacion.md#primera-instalación-el-asistente)).

Es idempotente: se puede repetir y sólo hace lo que falta. Pasos:

1. Sube `deploy/` (con la configuración ya resuelta) y un `git bundle` del
   repositorio a `~/routerpanel-deploy` de `SERVIDOR_SSH`.
2. `--servidor` (root, con `doas`): comprueba la sintaxis de los guiones,
   genera las plantillas y añade las jails a `/etc/jail.conf`. Según
   `MODULOS`:
   - `rc`: dataset ZFS (`ZFS_DATASET/<jail>`) y base
     (`pkg --rootdir … -r FreeBSD-base install FreeBSD-set-…`), usuario
     `routerpanel` (uid 3159, distinto de cualquiera del host), rutas, nullfs,
     rc.d con sus variables y newsyslog. Si el panel ya está desplegado y aún
     no tiene usuarios, dice dónde está el token del asistente (no lo
     imprime).
   - `actualizador`: la jail de compilación con sus paquetes, `rustup`,
     `cargo-leptos`, clave de despliegue y clon de `REPO_URL` (si el servidor
     git aún no acepta la clave, clona del bundle); el actualizador, su
     configuración y su cron en el host. Al final imprime la clave pública de
     despliegue. Lo que va en directorios de un usuario de jail
     (`~rpbuild/.ssh/…`, el bundle, los permisos de un `router.secret` de
     antes) se hace como ese usuario desde dentro de la jail, nunca como root
     desde el host.
   - `unbound`: `/var/unbound/conf.d/routerpanel.conf` con el nombre de
     `PANEL_ORIGEN` apuntando al primer proxy de `PROXIES_CONFIANZA` (o a la
     jail del panel); `local-unbound-checkconf` y `service local_unbound reload`.
   - `caddyfile`: imprime el sitio para el Caddyfile (no lo instala).
3. `--pf-cargar` (`pf`): inserta el bloque de nat antes de la primera regla
   `nat` de `/etc/pf.conf` y el de filtro antes de `block all`; lo comprueba
   con `pfctl -nf`, guarda la copia en `/etc/pf.conf.bak-routerpanel`, arma
   un interruptor de hombre muerto (`daemon`, 180 s: restaura la copia en el
   fichero y en el núcleo) y carga las reglas.
4. El equipo de trabajo abre una conexión SSH **nueva**, comprueba las webs
   de `COMPROBAR_URLS` y lanza `--comprobar-red`: el panel llega al router
   :80 pero no a Internet ni al proxy; la jail de compilación llega a
   crates.io, a `/salud` y al repositorio («Permission denied (publickey)»
   cuenta como que llega) pero no al router ni al proxy. Sólo si todo cuadra,
   `--pf-confirmar` desarma el interruptor. Si no, las reglas vuelven solas.

### Repositorio y clave de despliegue

`REPO_URL` es cualquier URL de git: `https://…` o `ssh://git@equipo:puerto/…`.
Si el servidor git está en la red propia, pon su IP en la URL: pf solo abre
a la jail de compilación esa IP y ese puerto.

Por SSH, la clave pública que imprime `install.sh` (también en
`<JAILS_RAIZ>/<JAIL_COMPILAR>/home/rpbuild/.ssh/id_ed25519.pub`) se da de
alta en el servidor git como **clave de despliegue de sólo lectura**. Hasta
entonces el actualizador registra «sin acceso al repositorio» y termina sin
hacer nada. La clave del servidor git queda fijada: la de `REPO_HOST_CLAVE`
(una línea de `known_hosts`) o, si está vacía, la primera que presente.

### Primer despliegue

Mientras no exista ninguna etiqueta `v*` el actualizador no tiene nada que
desplegar. La primera vez se compila a mano una rama con el mismo guion:

```sh
ssh SERVIDOR doas routerpanel-actualizar --ref RAMA
deploy/freebsd-jail/install.sh   # otra vez: dice dónde está el token del asistente
```

Después, `PANEL_ORIGEN/instalar` con ese token: crea el administrador, guarda
las credenciales del router y hace la lectura inicial.

`--ref` acepta una rama o un commit; la versión se llama `<ref>-<sha corto>`.
Si no hay acceso al repositorio, compila lo que ya tenga el clon.

## Actualizador

`routerpanel-actualizar` corre en el host como root (cron cada 15 min, con
`lockf` para no solaparse):

1. En la jail de compilación (como `rpbuild`) pone `REPO_URL` como `origin` y
   ejecuta `traer_ultima_etiqueta` (`deploy/lib/git.sh`):
   `git fetch --force --prune --prune-tags --tags`, la última etiqueta `v*`
   por `sort -V` y `git checkout --detach`. Nunca hace merge, así que tolera
   que el remoto reescriba su historial. Todas las órdenes de git cuyo
   resultado usa el host van con `/usr/local/bin/git` y el PATH del sistema
   (no el de `~rpbuild/.cargo/bin`, que puede escribir cualquier `build.rs`).
2. Si la etiqueta es la desplegada (o la que ya falló), termina. Lo que sale
   de la jail no es de fiar: el nombre de la versión (la etiqueta, o
   `<ref>-<sha>`) tiene que ser `[A-Za-z0-9][A-Za-z0-9._-]*` sin `..` (como
   mucho 100 caracteres) y el commit, 7–40 cifras hexadecimales; si no, el
   actualizador registra el error y no hace nada (el nombre acaba en rutas
   que root borra).
3. Solo con `PRECOMPILADAS_ESPERA` y para una etiqueta: antes de compilar
   mira el buzón; ver [versiones precompiladas](#versiones-precompiladas-el-pc-de-compilación).
   Si instala la precompilada, sigue en el paso 5; si no, compila:
   `cargo leptos build --release --precompress` (como `rpbuild`; la web
   sale también en `.br` y `.gz`, que el panel sirve ya comprimida; si el
   `Cargo.lock` pide otra versión de `wasm-bindgen`, instala antes
   `wasm-bindgen-cli` de esa versión), con prioridad baja y, si se fijó
   `TRABAJOS_COMPILACION` en la configuración, ese número de trabajos de
   cargo a la vez (menos memoria). **El servidor solo compila**: no ejecuta
   las pruebas (el servidor es lento y las pruebas, además, necesitan
   memoria). Las pruebas completas (`cargo test --workspace` y
   `cargo test --workspace --features panel/ssr`) y los e2e se pasan en el
   equipo de desarrollo **antes de crear la etiqueta**, y es obligatorio: el
   actualizador despliega cualquier `v*` que compile, y solo `/salud` y la
   vuelta atrás protegen de una versión rota. La compilación tiene una hora
   como mucho (`timeout`): un compilador colgado cuenta como fallo y no
   bloquea las ejecuciones siguientes.
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
   200 con `"ok":true` **y** a que quien escucha en el puerto del panel sea el
   binario de la versión nueva (`sockstat` + `ps`): 60 s de reloj como mucho.
   La URL sale de `routerpanel_listen` (si es `0.0.0.0` o `127.0.0.1`, con la
   IP de la jail del panel) y se pregunta desde la jail de compilación, la
   única además de los proxies a la que pf deja llegar al panel.
7. Si no responde: para, restaura la base de datos de antes (una versión nueva
   puede haber aplicado una migración que la anterior no conoce, y sqlx no
   arrancaría), vuelve a la versión anterior, arranca y borra la versión
   fallida. La etiqueta se reintenta en la ejecución siguiente y sólo queda
   marcada como fallida si falla dos veces seguidas (un tropiezo de red o de
   memoria no la descarta). Sin versión anterior a la
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
(`actualizar.sh`, `git.sh`, `routerpanel.rc`, las reglas de pf, cron,
newsyslog…) sólo los instala `install.sh`: si una versión cambia algo de
`deploy/`, hay que repetirlo desde el equipo de trabajo (es idempotente y
vuelve a pasar por el interruptor de pf si cambian las reglas). Hasta
entonces el servidor sigue con los de antes, aunque el panel ya sea el nuevo.

**Por qué el actualizador corre en el host**: con el cron en la jail de
compilación haría falta una regla de doas en el host para reiniciar el
servicio, y desde una jail no se llega al doas del host. Por eso el
actualizador corre en el host y sólo la compilación va dentro de la jail, sin
privilegios. No hay
reglas de doas nuevas (`doas.conf.snippet`).

## Versiones precompiladas (el PC de compilación)

Opcional. Compilar el panel en el servidor lleva sus minutos de CPU y
bastante memoria. Si hay un PC con Linux más potente (el **PC de
compilación**), puede compilar la versión para FreeBSD cuando está encendido
y dejarla en un buzón del servidor; el actualizador la instala en lugar de
compilarla. Si el PC está apagado (o en otro sistema), el servidor compila
como siempre pasado un rato. **Sin `PRECOMPILADAS_ESPERA` en la
configuración, nada de esto se usa** y el actualizador hace exactamente lo
mismo que antes (lo comprueba `deploy/pruebas/precompiladas.sh`).

### Cómo funciona

1. Tras crear y subir la etiqueta, desde el equipo de trabajo:

   ```sh
   deploy/cross/lanzar.sh vX.Y.Z
   ```

   Comprueba que es una etiqueta `v*` y que la copia de trabajo no tiene
   cambios sin confirmar; manda al PC (`PC_COMPILAR_SSH`) `deploy/cross/`,
   `deploy/lib/` y un `git bundle` de la etiqueta, y allí
   `compilar-freebsd.sh --release vX.Y.Z` compila y `entregar.sh` lo sube al
   buzón (`PRECOMPILADAS_BUZON_SSH`). Si el PC no responde en 5 s, lo dice y
   sale con 3: el servidor compilará él mismo. `--prueba [REF]` compila
   cualquier referencia sin entregarla (la versión se llama
   `prueba-<sha>` y el buzón no la admite).
2. En el PC, `compilar-freebsd.sh` saca el código del commit de la etiqueta
   con `git archive` (nunca de una copia de trabajo) a `~/rtf-cross/fuente`
   (con `target/` dentro: la siguiente compilación es incremental) y ejecuta
   `cargo leptos build --release --precompress` como el servidor, con la
   misma versión de `cargo-leptos` (la de `install.sh`, que es la que fija
   `wasm-bindgen`) y el binario para `x86_64-unknown-freebsd` compilado con
   [`cross`](https://github.com/cross-rs/cross) (un contenedor de podman con
   el enlazador y la libc de FreeBSD; SQLite y el resto de C van enlazados
   dentro del binario). El binario pide las mismas bibliotecas que el que
   compila el servidor (`libthr`, `libgcc_s`, `libc` y `libm`, todas de la
   base). La web no depende de la plataforma. Empaqueta, como las releases de
   Linux, `routerpanel-<versión>-x86_64-unknown-freebsd.tar.gz` (una carpeta
   con `panel`, `site/`, `COMMIT`, `LICENSE` y `AVISO-LEGAL.md`) y
   `SHA256SUMS`; con `RP_MINISIGN_SECRETA`, también `SHA256SUMS.minisig`.
3. `entregar.sh` sube el archivo, la firma y `SHA256SUMS` (el último) por SSH
   como el usuario `rpentrega` del servidor. Ese usuario solo puede ejecutar
   `recibir-precompilada.sh`, que guarda cada fichero en
   `/var/db/routerpanel-buzon/entrada/<versión>/` (con un rename: nunca a
   medias), solo con los tres nombres admitidos, hasta 300 MB por fichero y
   las 3 versiones más recientes.
4. El actualizador (cron, cada 15 min), con una etiqueta nueva:
   - si en el buzón están el archivo y `SHA256SUMS` (y la firma, con
     `PRECOMPILADAS_MINISIGN`), lo copia a una carpeta propia y comprueba,
     sobre la copia: la firma (con clave), el sha256, que el archivo trae
     solo ficheros y carpetas normales, sin enlaces, en una única carpeta
     con el nombre de la versión; que `COMMIT` es el commit de la etiqueta
     en el clon de la jail de compilación y que `panel` es un ejecutable de
     FreeBSD x86-64. Si todo cuadra, lo deja en `releases/<versión>/`
     (`panel`, `site/`, `COMMIT`, de root, 755/644) y lo despliega como las
     suyas (copia de la base de datos, `/salud`, vuelta atrás). Si algo no
     cuadra, lo registra y compila aquí en la misma ejecución. En los dos
     casos vacía la carpeta de esa versión del buzón;
   - si no ha llegado, espera: apunta cuándo vio la etiqueta y termina sin
     hacer nada, hasta `PRECOMPILADAS_ESPERA` minutos después; a partir de
     ahí compila aquí como siempre. Con el cron cada 15 minutos, la espera
     real se redondea hacia arriba (20 min son, como mucho, 30). Con
     `PRECOMPILADAS_ESPERA=0` no espera nunca, pero usa la precompilada si ya
     está.

   Si la precompilada no arranca (`/salud`), la vuelta atrás es la de
   siempre y el siguiente intento ya no la tiene (se borró del buzón):
   compila aquí. Los `--ref` nunca miran el buzón.

### Por qué así (el PC empuja a una cuenta limitada)

- El PC nunca entra en el servidor con una shell: su clave solo vale para
  `rpentrega` con `restrict` (sin terminal, reenvíos ni agente),
  `command=` (la única orden es `recibir-precompilada.sh`) y `from=` (solo
  desde la IP del PC). No hace falta tocar `sshd_config` (un `sftp` con
  `chroot` sí lo necesita) y el receptor solo escribe ficheros con nombre
  conocido, de tamaño limitado, en su carpeta.
- La alternativa, que el servidor fuera a buscarla al PC, obligaría al
  actualizador, que corre como root, a guardar una clave para entrar en un
  equipo de escritorio y a esperar conexiones que fallan cada vez que el PC
  está apagado.
- Nada de lo que llega se ejecuta en el host. root lo lee de una copia
  propia (un `cp` con tiempo máximo, por si alguien pusiera una FIFO) y lo
  comprueba todo antes de instalarlo, igual que con los artefactos que salen
  de la jail de compilación.

Confianza: quien controle el PC de compilación (o su clave del buzón) puede
hacer que se despliegue otro binario en la jail del panel, igual que hoy
quien pueda crear etiquetas en el repositorio. El commit tiene que ser el de
la etiqueta, pero el binario no se puede comprobar contra el código. La firma
de minisign (opcional) añade que solo vale lo firmado con una clave que no
sale del PC. La jail del panel sigue igual de cerrada (pf: solo el router).

### Instalar (a mano, una vez)

En `deploy/config.local` del equipo de trabajo:

```sh
PRECOMPILADAS_ESPERA=20
PC_COMPILAR_SSH=usuario@pc-de-compilacion
PRECOMPILADAS_BUZON_SSH=rpentrega@<IP del servidor vista desde el PC>
# Opcional: PRECOMPILADAS_MINISIGN=<la línea de la clave pública>
```

En el PC de compilación (Linux con `rustup`, `podman`, `cross`, `rsync`,
`file`; `cargo-leptos` lo instala el guion si falta o es otra versión):

```sh
ssh-keygen -t ed25519 -N '' -C routerpanel-buzon -f ~/.ssh/routerpanel-buzon
cat ~/.ssh/routerpanel-buzon.pub     # para el authorized_keys de abajo
```

En el servidor, como root (`doas`); `IP_DEL_PC` y la clave, los del PC:

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

- El `authorized_keys` es de root: `rpentrega` no puede quitarse las
  restricciones. `-w no`: sin contraseña. Necesita una shell de verdad
  (`/bin/sh`) porque sshd ejecuta la orden forzada con ella.
- Si `sshd -T` muestra `AllowUsers` o `AllowGroups`, añade `rpentrega` en
  `sshd_config` y `service sshd reload`. Si pf limita quién llega al SSH del
  host, deja pasar al PC.
- Con firma: `pkg install minisign` en el host y `PRECOMPILADAS_MINISIGN` en
  `config.local`. En el PC, `minisign -G -W -s ~/.config/routerpanel/minisign.key -p ~/.config/routerpanel/minisign.pub`
  y `RP_MINISIGN_SECRETA=~/.config/routerpanel/minisign.key` en el entorno
  de `lanzar.sh`/`compilar-freebsd.sh` (sin contraseña, `-W`: el PC firma
  solo).

Después, desde el equipo de trabajo, `deploy/freebsd-jail/install.sh`
(instala `precompiladas.sh` y `recibir-precompilada.sh` y pasa
`PRECOMPILADAS_ESPERA` y `PRECOMPILADAS_MINISIGN` a la configuración del
actualizador). Por último, en el PC, fija la clave del servidor y prueba el
buzón (`recibir` sin argumentos responde con el uso y no guarda nada):

```sh
ssh-keyscan -t ed25519 <servidor> >>~/.ssh/known_hosts   # compárala con ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub del servidor
ssh -i ~/.ssh/routerpanel-buzon -o IdentitiesOnly=yes rpentrega@<servidor> recibir
```

y una compilación de prueba desde el equipo de trabajo:
`deploy/cross/lanzar.sh --prueba`.

### Temporizador en el PC (opcional)

En vez de lanzarlo a mano, el PC puede mirar cada 15 minutos, mientras está
encendido, si hay una etiqueta nueva en el repositorio y compilarla y
entregarla (`compilar-freebsd.sh --ultima`, que no repite lo ya entregado).
Necesita llegar al repositorio (`REPO_URL`) desde el PC y los guiones en
`~/rtf-cross/bin` (los copia `lanzar.sh`). No viene activado:

```sh
cp deploy/cross/rtf-cross.service deploy/cross/rtf-cross.timer ~/.config/systemd/user/
printf 'RP_REPO_URL=%s\nRP_BUZON_SSH=%s\n' '<REPO_URL>' 'rpentrega@<servidor>' >~/.config/rtf-cross.env
systemctl --user daemon-reload
systemctl --user enable --now rtf-cross.timer
```

Las unidades de usuario solo corren con la sesión abierta (o con
`loginctl enable-linger`).

## Red (pf)

- Al puerto del panel sólo llegan los proxies de confianza y la jail de
  compilación; el resto se bloquea, también desde la red local y la VPN.
- La jail del panel sólo puede salir hacia la web del router (`ROUTER_IP`,
  tcp 80). Hacia la interfaz externa (`INTERFAZ_EXT`) el tráfico va traducido
  por el nat (el filtro ve la IP del host), así que el nat de la jail lleva la
  etiqueta `ROUTERPANEL` y el filtro deja pasar sólo lo etiquetado hacia el
  router; hacia el host y las demás jails (lo0, sin nat) se filtra por la
  dirección de origen. Las respuestas del panel casan con el estado del
  `pass` de entrada.
- La jail de compilación, que ejecuta código del repositorio, sale libremente
  a Internet (crates.io, rustup, pkg), pero de `RED_LAN` y `RED_VPN` sólo
  alcanza el repositorio git (si `REPO_URL` lleva una IP), el DNS
  (`DNS_IP`:53), el panel y a sí misma (127.0.0.1 en la jail es su propia IP:
  lo que la compilación escuche en local). Mismo truco: etiqueta `ROUTERPANEL_BUILD` en su
  nat para lo que va por la interfaz externa.
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

Por eso la jail del panel **no** se actualiza con `pkg -j` (no tiene red ni
`pkg`): desde el host,

```sh
doas pkg --rootdir <JAILS_RAIZ>/<JAIL_PANEL> upgrade -r FreeBSD-base
doas jexec <JAIL_PANEL> service routerpanel restart
```

La de compilación sí, como las demás: `doas pkg -j <JAIL_COMPILAR> upgrade`.

## Operación

```sh
doas jexec <JAIL_PANEL> service routerpanel status|restart
doas tail <JAILS_RAIZ>/<JAIL_PANEL>/var/log/routerpanel.log
doas tail /var/log/routerpanel-update.log
doas jexec -U routerpanel <JAIL_PANEL> env RP_DB=/var/db/routerpanel/panel.db \
    /usr/local/routerpanel/current/panel user passwd <usuario>
```

Volver a mano a otra versión: `ln -sfh releases/<versión>` sobre
`<JAILS_RAIZ>/<JAIL_PANEL>/usr/local/routerpanel/current` y reiniciar el
servicio.
