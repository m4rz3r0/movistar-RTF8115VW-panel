# Instalación en jails de FreeBSD

El panel corre en un servidor FreeBSD (probado en 15.x) en una jail PkgBase:

| Jail (`config.local`) | Contenido |
|---|---|
| `JAIL_PANEL` (`JAIL_PANEL_IP`) | base mínima (`FreeBSD-set-minimal-jail`), el binario y su web; servicio `routerpanel` en `PANEL_ESCUCHA` |

El servidor nunca compila: no tiene Rust, ni jail de compilación, ni ejecuta
nada del repositorio. Su actualizador instala versiones ya compiladas para
`x86_64-unknown-freebsd`, de una de estas dos fuentes:

- la [release de GitHub](#la-release-de-github) de cada etiqueta `vX.Y.Z`,
  que publica `.github/workflows/release.yml` con su atestación de origen;
- el [buzón](#el-pc-de-compilación-buzón) del servidor, donde las deja un PC
  con Linux (`deploy/cross/`).

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
| `recibir-precompilada.sh` | `/usr/local/libexec/routerpanel-actualizar/` en el host ([buzón](#el-pc-de-compilación-buzón)) |
| `doas.conf.snippet` | nada: explica por qué no hace falta ninguna regla de doas |

## Rutas

En el host:

- `/usr/local/routerpanel/releases/<versión>/{panel,site/,COMMIT}`: las
  versiones instaladas, de root. Entran por nullfs `ro,late` (`/etc/fstab`)
  en la misma ruta de la jail del panel.
- `/usr/local/etc/routerpanel-actualizar.conf` (600): configuración del
  actualizador.
- `/usr/local/etc/routerpanel-actualizar/` (700): `id_ed25519` (la clave de
  despliegue, de solo lectura en el servidor git) y `known_hosts` (la clave
  del servidor git).
- `/usr/local/etc/cron.d/routerpanel`: el actualizador cada 15 minutos.
- `/var/log/routerpanel-update.log`, rotado por
  `/usr/local/etc/newsyslog.conf.d/routerpanel.conf`.
- `/var/db/routerpanel-actualizar/`: `fallida` (la última etiqueta que falló
  dos veces seguidas; no se reintenta hasta que haya otra) y `esperando` (la
  etiqueta cuya versión precompilada se espera).
- `/var/db/routerpanel-buzon/entrada/<versión>/`: lo que deja el PC de
  compilación, si lo usas.

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
versión precompilada esté en GitHub o en el buzón, el actualizador la
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
3. Busca la versión ya compilada, primero en el buzón y después en GitHub
   (si hay `GITHUB_REPO`), y la instala de la primera que la tenga y pase
   todas las comprobaciones (abajo). La deja en `releases/<versión>/`, de
   root.
   - Si ninguna la tiene todavía, registra una sola vez «esperando la
     versión precompilada de vX.Y.Z» y termina sin error. La vuelve a buscar
     en cada ejecución, sin límite de tiempo, y esperar no cuenta como fallo.
   - Si alguna la tiene pero algo no cuadra, la rechaza (lo registra con el
     motivo y vacía esa carpeta del buzón) y cuenta como un fallo de la
     etiqueta. Nunca se instala nada sin comprobar.
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
- **Buzón**: con `PRECOMPILADAS_MINISIGN`, la firma de `SHA256SUMS`.

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

## El PC de compilación (buzón)

Es opcional. Un PC con Linux compila la versión para FreeBSD y la deja en un
buzón del servidor; el actualizador la instala de ahí. Sirve sin GitHub, o
para tener la versión antes de que termine la release. Si el PC está
apagado, el servidor no compila: espera a GitHub o a que el PC la entregue.

### Cómo funciona

1. Después de crear y subir la etiqueta, desde el equipo de trabajo:

   ```sh
   deploy/cross/lanzar.sh vX.Y.Z
   ```

   Comprueba que es una etiqueta `vX.Y.Z` y que no hay cambios sin
   confirmar, y manda al PC (`PC_COMPILAR_SSH`) los guiones y un `git bundle`
   de la etiqueta. Allí `compilar-freebsd.sh` compila y `entregar.sh` lo sube
   al buzón (`PRECOMPILADAS_BUZON_SSH`). Si el PC no responde en 5 s, lo dice
   y sale con 3. `--prueba [REF]` compila cualquier referencia sin
   entregarla.
2. El PC saca el código del commit de la etiqueta con `git archive` a
   `~/rtf-cross/fuente`, nunca de una copia de trabajo. Compila con la misma
   versión de `cargo-leptos` que la release de GitHub, y el binario para
   `x86_64-unknown-freebsd` con `cross`. Empaqueta con
   `empaquetar-freebsd.sh` (el mismo archivo que GitHub) y escribe
   `SHA256SUMS`; con `RP_MINISIGN_SECRETA`, también `SHA256SUMS.minisig`.
3. `entregar.sh` sube los ficheros por SSH como el usuario `rpentrega` del
   servidor, que solo puede ejecutar `recibir-precompilada.sh`. Este guarda
   cada fichero en `/var/db/routerpanel-buzon/entrada/<versión>/`, solo con
   los nombres admitidos, hasta 300 MB por fichero y las 3 versiones más
   recientes.
4. Cuando el actualizador ve la etiqueta, mira el buzón antes que GitHub y,
   si la versión está y pasa las comprobaciones, la despliega. En los dos
   casos (instalada o rechazada) vacía esa carpeta del buzón.

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
una clave que no sale del PC. Las versiones de GitHub no tienen ese
problema: su atestación dice qué flujo y qué etiqueta las compilaron.

### Instalar (a mano, una vez)

En `deploy/config.local` del equipo de trabajo:

```sh
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
que pasa `PRECOMPILADAS_MINISIGN` al actualizador. Por último, en el PC, fija
la clave del servidor y prueba el buzón (`recibir` sin argumentos responde
con el uso y no guarda nada):

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

## Migrar una instalación existente

Las instalaciones anteriores tenían una jail de compilación
(`JAIL_COMPILAR`, normalmente `routerpanel-build`) donde el actualizador
clonaba el repositorio y compilaba. Ya no se usa. Para pasar a la
instalación sin compilar:

1. En `deploy/config.local`, pon `GITHUB_REPO=propietario/nombre` si vas a
   usar la release de GitHub (o prepara el [buzón](#el-pc-de-compilación-buzón)).
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
  (`REPO_URL`) y, con `GITHUB_REPO`, a GitHub por HTTPS (las descargas de
  la release y `api.github.com`). Eso lo deciden las reglas del propio
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
