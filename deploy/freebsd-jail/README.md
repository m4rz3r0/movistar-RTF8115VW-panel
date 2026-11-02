# Instalación en jails de FreeBSD

El panel corre en un servidor FreeBSD (probado en 15.x) en una jail PkgBase:

| Jail (`config.local`) | Contenido |
|---|---|
| `JAIL_PANEL` (`JAIL_PANEL_IP`) | base mínima (`FreeBSD-set-minimal-jail`), el binario y su web; servicio `routerpanel` en `PANEL_ESCUCHA` |

El servidor nunca compila: no tiene Rust, ni jail de compilación, ni ejecuta
nada del repositorio. Su actualizador instala versiones ya compiladas para
`x86_64-unknown-freebsd`, de una de estas dos fuentes, en este orden:

- la [release de Forgejo](#el-runner-de-actions-del-pc) de cada etiqueta
  `vX.Y.Z`, que compila y publica un PC con Linux con un runner de Forgejo
  Actions (`.forgejo/workflows/freebsd.yml` y `deploy/cross/`);
- la [release de GitHub](#la-release-de-github), que publica
  `.github/workflows/release.yml` con su atestación de origen.

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

Deja ahí `jail.conf`, `pf.conf`, `rc.conf` (las variables del servicio),
`actualizador.conf`, `known_hosts` si pusiste `REPO_HOST_CLAVE` y, según
`MODULOS`, `Caddyfile.routerpanel` y `unbound.conf`.

## Ficheros

| Fichero | Se instala en |
|---|---|
| `install.sh` | se ejecuta desde el equipo de trabajo |
| `jail.conf.plantilla` | añadido a `/etc/jail.conf` del host |
| `pf.conf.plantilla` | dos bloques en `/etc/pf.conf` del host, entre marcas `# >>> routerpanel` / `# <<< routerpanel` |
| `routerpanel.rc` | `/usr/local/etc/rc.d/routerpanel` en la jail del panel |
| `actualizar.sh` | `/usr/local/sbin/routerpanel-actualizar` en el host |
| `../lib/git.sh`, `../lib/precompiladas.sh` | `/usr/local/libexec/routerpanel-actualizar/` en el host |
| `doas.conf.snippet` | nada: explica por qué no hace falta ninguna regla de doas |

## Rutas

En el host:

- `/usr/local/routerpanel/releases/<versión>/{panel,site/,COMMIT}`: las
  versiones instaladas, de root. Entran por nullfs `ro,late` (`/etc/fstab`)
  en la misma ruta de la jail del panel.
- `/usr/local/etc/routerpanel-actualizar.conf` (600): configuración del
  actualizador.
- `/usr/local/etc/routerpanel-actualizar/` (700): `id_ed25519` (la clave de
  despliegue, de solo lectura en el servidor git), `known_hosts` (la clave
  del servidor git) y `forgejo-token` (600, el token de solo lectura de
  Forgejo; lo pones tú, `install.sh` no lo crea).
- `/usr/local/etc/cron.d/routerpanel`: el actualizador cada 15 minutos.
- `/var/log/routerpanel-update.log`, rotado por
  `/usr/local/etc/newsyslog.conf.d/routerpanel.conf`.
- `/var/db/routerpanel-actualizar/`: `fallida` (la última etiqueta que falló
  dos veces seguidas; no se reintenta hasta que haya otra), `esperando` (la
  etiqueta cuya versión precompilada se espera) y `rechazada-<fuente>` y
  `no-disponible-<fuente>` (la etiqueta de la que ya se registró un rechazo
  o una fuente no disponible, para no repetirlo en cada pasada).

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

1. Sube los guiones de `deploy/` y la configuración ya resuelta a
   `~/routerpanel-deploy` de `SERVIDOR_SSH`.
2. Ejecuta allí `install.sh --servidor` como root con `doas`. Comprueba la
   sintaxis de los guiones y añade la jail a `/etc/jail.conf`. Según
   `MODULOS`:
   - `rc`: dataset ZFS (`ZFS_DATASET/<jail>`) y base de la jail del panel,
     usuario `routerpanel` (uid 3159), nullfs, rc.d con sus variables y
     newsyslog. Si el panel ya está desplegado y no tiene usuarios, dice
     dónde está el token del asistente (no lo imprime).
   - `actualizador`: en el host, `git` (y `gh` y `jq` si pusiste
     `GITHUB_REPO`), la clave de despliegue, la clave del servidor git, el
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
   router por el puerto 80 y no a Internet ni al proxy, y a su propio
   `/salud`; el host tiene que llegar al repositorio («Permission denied
   (publickey)» cuenta como que llega). Solo si todo cuadra,
   `--pf-confirmar` desarma el interruptor. Si no, las reglas anteriores
   vuelven solas a los 180 s.

Si `--pf-confirmar` dice «el interruptor ya saltó», pf tiene otra vez las
reglas anteriores: repite `install.sh`.

### Repositorio y clave de despliegue

`REPO_URL` admite cualquier URL de git: `https://…` o
`ssh://git@equipo:puerto/…`. El actualizador solo lanza
`git ls-remote --tags` contra ella desde el host: no clona, no saca código y
no ejecuta nada del repositorio. Le basta con ver las etiquetas y sus
commits.

Por SSH, da de alta en el servidor git la clave pública que imprime
`install.sh` como clave de despliegue de solo lectura. También está en
`/usr/local/etc/routerpanel-actualizar/id_ed25519.pub`. Hasta entonces el
actualizador registra «sin acceso al repositorio» y no hace nada. La clave
del servidor git queda fijada en
`/usr/local/etc/routerpanel-actualizar/known_hosts`: la de
`REPO_HOST_CLAVE` (una línea de `known_hosts`) o, si está vacía, la primera
que presente cuando `install.sh` se conecta. El actualizador nunca acepta
una clave nueva.

### Primer despliegue

No hay despliegue a mano: crea la primera etiqueta `vX.Y.Z` y, en cuanto su
versión precompilada esté en la release de Forgejo o en la de GitHub, el
actualizador la
instala (como mucho 15 minutos después). Para no esperar al cron:

```sh
ssh SERVIDOR doas routerpanel-actualizar
deploy/freebsd-jail/install.sh   # otra vez: dice dónde está el token del asistente
```

Después abre `PANEL_ORIGEN/instalar` con ese token. El asistente crea el
administrador, guarda las credenciales del router y hace la lectura inicial.

## Actualizador

`routerpanel-actualizar` corre en el host como root, desde cron cada 15
minutos y sin solaparse (`lockf`). Así no hace falta ninguna regla de doas.
No llama nunca a `cargo` ni a `rustc`. `routerpanel-actualizar --ayuda`
muestra el uso; no tiene más opciones. En cada ejecución:

1. Con `git ls-remote --tags REPO_URL`, saca la última etiqueta `vX.Y.Z`
   (por `sort -V`) y su commit. Como lee el remoto en cada pasada, no le
   afecta que reescriba su historial.
2. Si esa etiqueta ya está desplegada, o ya falló, termina.
3. Busca la versión ya compilada, primero en la release de Forgejo (si hay
   `FORGEJO_URL` y `FORGEJO_REPO`) y después en la de GitHub (si hay
   `GITHUB_REPO`), y la instala de la primera que la tenga y pase todas las
   comprobaciones (abajo). La deja en `releases/<versión>/`, de root.
   - Si ninguna la tiene todavía (Forgejo responde 404 porque el PC aún no
     la ha compilado), registra una sola vez «esperando la versión
     precompilada de vX.Y.Z» y termina sin error. La vuelve a buscar en cada
     ejecución, sin límite de tiempo, y esperar no cuenta como fallo.
   - Si alguna la tiene pero algo no cuadra, la rechaza (lo registra con el
     motivo, una vez por etiqueta y fuente) y vuelve a esperar: una release
     rechazada no cuenta como fallo de la etiqueta, para que una buena
     posterior (el runner repite el trabajo, o sale la de GitHub) se pueda
     instalar. Nunca se instala nada sin comprobar.
   - Sin `FORGEJO_URL` o `FORGEJO_REPO`, lo registra una vez y no mira
     Forgejo. Si el token no existe, no es de root o lo pueden leer el grupo
     u otros, Forgejo no está disponible (lo registra una vez por etiqueta)
     y el token no se usa.
4. Para el servicio y copia `panel.db` a
   `/var/db/routerpanel/antes-de-actualizar/`.
5. Cambia `current`, arranca y espera como mucho 60 s a que `/salud`
   (preguntado desde la propia jail del panel) responda con `"ok":true`
   desde el binario nuevo.
6. Si no responde, para el servicio, restaura la base de datos de antes,
   vuelve a la versión anterior y borra la nueva. La etiqueta se reintenta en
   la siguiente ejecución y solo queda marcada como fallida si falla dos
   veces seguidas. Si no hay versión anterior (primer despliegue), el
   servicio queda parado y sin `current`, también tras reiniciar el host.
7. Conserva las 3 versiones más recientes; nunca borra la desplegada.

Las pruebas no corren en ninguna parte del camino al servidor. Pásalas en el
equipo de desarrollo antes de crear la etiqueta (`cargo test --workspace`,
`cargo test --workspace --features panel/ssr` y los e2e): el actualizador
despliega cualquier `vX.Y.Z` cuya versión precompilada pase las
comprobaciones, y solo `/salud` y la vuelta atrás te protegen de una versión
rota.

El actualizador solo despliega el panel. Si una versión cambia algo de
`deploy/` (el actualizador, `routerpanel.rc`, las reglas de pf…), repite
`install.sh` desde el equipo de trabajo; hasta entonces el servidor sigue con
los ficheros de antes.

### Qué se comprueba

Las dos fuentes traen el mismo archivo,
`routerpanel-<versión>-x86_64-unknown-freebsd.tar.gz`
(`deploy/cross/empaquetar-freebsd.sh`), y `SHA256SUMS`. Antes de instalarlo,
sobre una copia propia:

- el sha256 del archivo es el de `SHA256SUMS`;
- trae una sola carpeta, la de la versión, con `panel`, `site/` y `COMMIT`,
  y solo ficheros y carpetas normales;
- `COMMIT` es el commit de la etiqueta que dio `git ls-remote`;
- `panel` es un ejecutable de FreeBSD x86-64.

Además, según la fuente:

- **GitHub**: la atestación de origen del archivo (la de
  `api.github.com/repos/<GITHUB_REPO>/attestations/sha256:<sha256>`, sin
  token) tiene que pasar `gh attestation verify --bundle` con
  `--repo GITHUB_REPO`, `--source-ref refs/tags/vX.Y.Z` y
  `--signer-workflow GITHUB_REPO/.github/workflows/release.yml`: el archivo
  lo compiló el flujo de release de ese repositorio para esa etiqueta.
- **Forgejo**: la release existe
  (`GET FORGEJO_URL/api/v1/repos/FORGEJO_REPO/releases/tags/vX.Y.Z`) y los
  ficheros se descargan con sus nombres de
  `FORGEJO_URL/FORGEJO_REPO/releases/download/vX.Y.Z/`: la URL sale de la
  configuración, nunca de la respuesta, y curl no sigue redirecciones, así
  que el token solo va a `FORGEJO_URL`. curl lee la cabecera con el token de
  su entrada estándar (`-K -`), nunca de sus argumentos. Con
  `PRECOMPILADAS_MINISIGN`, además, la firma de `SHA256SUMS`.

Nada de lo que llega se ejecuta antes de desplegarlo: `jq` y `gh` solo lo
leen.

## La release de GitHub

Con `GITHUB_REPO=propietario/nombre` en `config.local` (y repitiendo
`install.sh`, que instala `gh` y `jq` en el host), el actualizador descarga
la versión de
`https://github.com/<GITHUB_REPO>/releases/download/vX.Y.Z/`. El flujo
`.github/workflows/release.yml` compila la versión para FreeBSD con
[`cross`](https://github.com/cross-rs/cross) al subir cada etiqueta, la mete
en `SHA256SUMS` junto a las de Linux y publica una atestación de origen
(SLSA, firmada con Sigstore) de cada archivo. Tarda un rato en terminar:
hasta entonces el actualizador espera.

Si `REPO_URL` no es GitHub (por ejemplo, un servidor git propio), las
etiquetas tienen que ser las mismas en los dos sitios: el `COMMIT` del
archivo de GitHub tiene que ser el commit de la etiqueta en `REPO_URL`.

Para comprobar a mano una versión publicada:

```sh
gh attestation verify routerpanel-vX.Y.Z-x86_64-unknown-freebsd.tar.gz --repo <GITHUB_REPO>
```

## El runner de Actions del PC

Un PC con Linux compila cada etiqueta para FreeBSD y la publica como la
release de esa etiqueta en Forgejo; el actualizador la instala de ahí. Si el
PC está apagado, el servidor no compila: el trabajo espera en la cola de
Forgejo hasta que el runner vuelve (como mucho `ABANDONED_JOB_TIMEOUT` de la
sección `[actions]` de `app.ini`; con `720h`, 30 días), y el actualizador
espera a la release, o usa antes la de GitHub si sale primero.

### Cómo funciona

1. Al empujar una etiqueta `v*`, Forgejo lanza
   `.forgejo/workflows/freebsd.yml` en un runner con la etiqueta `pc`. Que
   exista `.forgejo/workflows/` hace que Forgejo no ejecute los flujos de
   `.github/workflows/`, que son los de GitHub.
2. El trabajo trae la etiqueta con `git fetch --depth 1` (el token del
   trabajo va en `GIT_CONFIG_*` como `http.extraheader`, nunca en la URL) y
   lanza `deploy/cross/publicar-forgejo.sh`. No usa acciones de terceros.
3. `publicar-forgejo.sh` compila con
   `compilar-freebsd.sh --release vX.Y.Z --repo <Forgejo>/<repo>.git`: el
   código sale del commit de la etiqueta con `git archive` a
   `~/rtf-cross/fuente`, nunca de una copia de trabajo, y la compilación es
   incremental entre etiquetas. Usa la misma versión de `cargo-leptos` que la
   release de GitHub y `cross` para el binario. Empaqueta con
   `empaquetar-freebsd.sh` (el mismo archivo que GitHub) y escribe
   `SHA256SUMS`; con `RP_MINISIGN_SECRETA`, también `SHA256SUMS.minisig`.
4. Trae la release de la etiqueta o, si no existe, la crea con el commit en
   la descripción, y sube el archivo y, el último, `SHA256SUMS`. Si el
   trabajo se repite, sustituye los adjuntos que ya estaban.

### Seguridad

Quien controle el PC (o pueda crear etiquetas en el repositorio) puede
desplegar otro binario en la jail del panel: el commit tiene que ser el de
la etiqueta, pero el binario no se puede comprobar contra el código. Con
minisign solo vale lo firmado con una clave que no sale del PC. Las
versiones de GitHub no tienen ese problema: su atestación dice qué flujo y
qué etiqueta las compilaron.

El servidor entra en Forgejo con un token de solo lectura propio, no con el
del trabajo. El runner corre como tu usuario (ejecutor host), así que solo
debe atender a repositorios tuyos: regístralo en tu usuario o en tu
organización, no en toda la instancia.

### Instalar el runner (una vez)

En el PC hace falta Linux con `rustup`, `git`, `podman`, `cross`, `rsync`,
`file`, `flock`, `curl` y `jq`. Si falta `cargo-leptos` o tiene otra
versión, `compilar-freebsd.sh` lo instala.

1. En Forgejo, activa Actions (`[actions] ENABLED = true` en `app.ini`, con
   `ABANDONED_JOB_TIMEOUT = 720h` para que las etiquetas esperen al PC) y la
   unidad «Actions» en los ajustes del repositorio.
2. Instala `forgejo-runner` en el PC (el paquete de tu distribución o el
   binario publicado por Forgejo) y regístralo con la etiqueta `pc:host`
   (ejecutor host: corre directamente en el PC, no en un contenedor). Saca el
   token de registro de los ajustes de Actions de tu usuario u organización
   («Runners», «Crear nuevo runner»):

   ```sh
   mkdir -p ~/.config/forgejo-runner && cd ~/.config/forgejo-runner
   forgejo-runner register --no-interactive --instance https://forge.example \
       --token <token de registro> --name pc --labels pc:host
   ```

3. Arráncalo como servicio de usuario, con el `PATH` en el que estén
   `cargo`, `cross` y `podman`:

   ```ini
   # ~/.config/systemd/user/forgejo-runner.service
   [Unit]
   Description=Runner de Forgejo Actions
   After=network-online.target

   [Service]
   WorkingDirectory=%h/.config/forgejo-runner
   Environment=PATH=%h/.cargo/bin:/usr/local/bin:/usr/bin:/bin
   ExecStart=/usr/bin/forgejo-runner daemon
   Restart=on-failure

   [Install]
   WantedBy=default.target
   ```

   ```sh
   systemctl --user daemon-reload
   systemctl --user enable --now forgejo-runner
   loginctl enable-linger    # para que corra sin la sesión abierta
   ```

4. En el servidor, como root, el token de solo lectura del actualizador
   (créalo en Forgejo, en «Ajustes», «Aplicaciones», con permiso
   `read:repository`):

   ```sh
   install -m 600 -o root -g wheel /dev/null /usr/local/etc/routerpanel-actualizar/forgejo-token
   vi /usr/local/etc/routerpanel-actualizar/forgejo-token    # el token, en una línea
   ```

   y en `deploy/config.local` del equipo de trabajo `FORGEJO_URL` y
   `FORGEJO_REPO`; repite `deploy/freebsd-jail/install.sh`, que los pasa al
   actualizador e instala `curl` en el host.

- Para firmar: `pkg install minisign` en el host y `PRECOMPILADAS_MINISIGN`
  en `config.local`. En el PC,
  `minisign -G -W -s ~/.config/routerpanel/minisign.key -p ~/.config/routerpanel/minisign.pub`
  y `RP_MINISIGN_SECRETA=~/.config/routerpanel/minisign.key` en el entorno
  del servicio del runner. Con `-W` la clave no lleva contraseña y el PC
  firma solo.

Para probar la compilación sin publicar nada, en el PC:
`deploy/cross/compilar-freebsd.sh --prueba [REF]`.

### Migrar desde el buzón

Antes, el PC entregaba la versión por SSH a un buzón del servidor
(`deploy/cross/lanzar.sh`, `entregar.sh`, el temporizador `rtf-cross` y el
usuario `rpentrega`). Ya no existe. Para pasar al runner:

1. Instala el runner y pon el token del servidor (arriba).
2. En el PC, apaga el temporizador y borra lo que usaba:

   ```sh
   systemctl --user disable --now rtf-cross.timer
   rm ~/.config/systemd/user/rtf-cross.service ~/.config/systemd/user/rtf-cross.timer \
       ~/.config/rtf-cross.env ~/.ssh/routerpanel-buzon ~/.ssh/routerpanel-buzon.pub
   rm -r ~/rtf-cross/bin
   systemctl --user daemon-reload
   ```

   `~/rtf-cross` (el clon y `target/`) se queda: el runner lo reutiliza.
   Quita `PC_COMPILAR_SSH` y `PRECOMPILADAS_BUZON_SSH` de
   `deploy/config.local`; ya no hacen nada.
3. En el servidor, como root, borra el usuario del buzón, su carpeta y el
   receptor:

   ```sh
   pw userdel rpentrega
   pw groupdel rpentrega 2>/dev/null || true
   rm -r /var/db/routerpanel-buzon
   rm -f /usr/local/libexec/routerpanel-actualizar/recibir-precompilada.sh
   sshd -T | grep -Ei '^(allowusers|allowgroups)'
   ```

   Si `rpentrega` estaba en `AllowUsers` o `AllowGroups` de `sshd_config`,
   quítalo y haz `service sshd reload`; si pf dejaba al PC llegar al SSH del
   host solo para el buzón, quita esa regla.
4. Repite `deploy/freebsd-jail/install.sh` y comprueba el registro del
   actualizador tras la próxima etiqueta.

## Migrar una instalación existente

Las instalaciones anteriores tenían una jail de compilación
(`JAIL_COMPILAR`, normalmente `routerpanel-build`) donde el actualizador
clonaba el repositorio y compilaba. Ya no se usa. Para pasar a la
instalación sin compilar:

1. En `deploy/config.local`, pon `FORGEJO_URL` y `FORGEJO_REPO` (con el
   [runner del PC](#el-runner-de-actions-del-pc)) o
   `GITHUB_REPO=propietario/nombre` si vas a usar la release de GitHub.
   Las claves de la jail de compilación (`JAIL_COMPILAR`,
   `JAIL_COMPILAR_IP`, `UID_COMPILAR`, `RED_LAN`, `RED_VPN`, `DNS_IP`,
   `TRABAJOS_COMPILACION`, `PRECOMPILADAS_ESPERA`) ya no hacen nada; puedes
   quitarlas.
2. Repite `deploy/freebsd-jail/install.sh`. Pasa la clave de despliegue de la
   jail de compilación al host
   (`/usr/local/etc/routerpanel-actualizar/id_ed25519`), así que la que ya
   diste de alta en el servidor git sigue valiendo; si no la encuentra, crea
   una nueva e imprime su parte pública para darla de alta. Fija la clave del
   servidor git en el host (`REPO_HOST_CLAVE` o la primera que presente),
   instala el actualizador nuevo, quita de pf las reglas de la jail de
   compilación y deja a la jail del panel llegar a su `/salud`.
3. Comprueba que el actualizador funciona sin la jail:

   ```sh
   ssh SERVIDOR doas routerpanel-actualizar
   ssh SERVIDOR doas tail /var/log/routerpanel-update.log
   ```

   No debe decir «sin acceso al repositorio». Si la última etiqueta ya está
   desplegada, no registra nada; si no, la instala o dice que la espera.
4. Cuando funcione, borra la jail de compilación. Desde el servidor, como
   root:

   ```sh
   service jail stop routerpanel-build
   zfs destroy -r <ZFS_DATASET>/routerpanel-build
   ```

   y quita su bloque `routerpanel-build { … }` de `/etc/jail.conf`. El
   registro de compilación `/var/log/routerpanel-build.log` también sobra.

## Red (pf)

- Al puerto del panel solo llegan los proxies de confianza y la propia
  jail del panel (el actualizador pregunta `/salud` desde ella). El resto se
  bloquea, también desde la red local y la VPN.
- La jail del panel solo puede salir a la web del router (`ROUTER_IP`, tcp
  80).
- El actualizador corre en el host: necesita llegar al repositorio
  (`REPO_URL`), con `FORGEJO_URL` a Forgejo y, con `GITHUB_REPO`, a GitHub
  por HTTPS (las descargas de la release y `api.github.com`). Eso lo deciden las reglas del propio
  host, no estas.

Como la jail del panel no tiene red ni `pkg`, no se actualiza con `pkg -j`.
Hazlo desde el host:

```sh
doas pkg --rootdir <JAILS_RAIZ>/<JAIL_PANEL> upgrade -r FreeBSD-base
doas jexec <JAIL_PANEL> service routerpanel restart
```

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
