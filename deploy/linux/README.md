# Instalación en Linux con systemd

El panel es un binario (`panel`) y su web (`site/`). Las releases para Linux
(`routerpanel-<versión>-<objetivo>.tar.gz`, con `x86_64-unknown-linux-gnu` o
`aarch64-unknown-linux-gnu`) los traen ya compilados junto a esta carpeta.
Se compilan en Ubuntu 22.04 y piden glibc 2.35 o posterior: valen para
Debian 12, Ubuntu 22.04 y Raspberry Pi OS de 64 bits (bookworm) en adelante.

## Ficheros

| Fichero | Se instala en |
|---|---|
| `routerpanel.service` | `/etc/systemd/system/routerpanel.service` |
| `env.example` | `/etc/routerpanel/env` (cópialo y ajústalo) |
| `actualizar-release.sh` | `/usr/local/sbin/routerpanel-actualizar` (opcional) |
| `nftables-routerpanel.nft` | `/etc/nftables.d/routerpanel.nft` (opcional, ejemplo) |

## Rutas

- `/opt/routerpanel/releases/<versión>/{panel,site/,…}` — versiones (root).
- `/opt/routerpanel/current` → `releases/<versión>` — la desplegada.
- `/etc/routerpanel/env` — configuración (`RP_*`; ver `env.example`).
- `/var/lib/routerpanel/` (en realidad `/var/lib/private/routerpanel`, por
  `DynamicUser`) — `panel.db`, `history.key` y, mientras no hay
  administrador, `instalar.token`. Es lo único donde el servicio escribe.

`history.key` cifra las credenciales del router y los secretos del
historial. Haz copia de ella aparte de `panel.db`: una copia de la base de
datos sin la clave no sirve, y las dos juntas lo descifran todo.

## Instalación

Como root, con la release desempaquetada en la carpeta actual:

```sh
v=v1.2.3   # la versión que instalas
install -d /opt/routerpanel/releases
cp -r . /opt/routerpanel/releases/$v
ln -sfn releases/$v /opt/routerpanel/current
install -m 644 deploy/linux/routerpanel.service /etc/systemd/system/
install -d /etc/routerpanel
install -m 600 deploy/linux/env.example /etc/routerpanel/env
$EDITOR /etc/routerpanel/env          # al menos RP_ORIGIN
systemctl edit routerpanel            # si hace falta: IPAddressAllow (abajo)
systemctl daemon-reload
systemctl enable --now routerpanel
```

Para compilarlo tú (`cargo leptos build --release`), lo que se instala es
`target/release/panel` y `target/site/`, con la misma estructura.

### Primer arranque: el asistente

Sin usuarios, el panel solo sirve `/instalar` y pide un token de un solo uso
que genera al arrancar. Está en el registro y en un fichero:

```sh
journalctl -u routerpanel | grep 'token de instalación'
cat /var/lib/routerpanel/instalar.token
```

El asistente solo responde desde la red local (o desde `RP_INSTALL_NETS`) y
siempre con el token; al crear el administrador, el token se borra.

## Red

El panel solo necesita hablar con el router y con quien le hace peticiones.
La unidad lo limita con `IPAddressDeny=any` e `IPAddressAllow=localhost
192.168.1.1/32`: **ajusta a la IP del router y del proxy**. `IPAddressAllow`
vale para la entrada y la salida, así que si el proxy inverso está en otro
equipo, añade su IP, y si tu router no está en `192.168.1.1`, cambia esa. Sin
tocar el fichero instalado:

```sh
systemctl edit routerpanel
# [Service]
# IPAddressAllow=
# IPAddressAllow=localhost 192.168.1.1/32 192.0.2.10/32
```

(la línea vacía borra la lista de la unidad). Pon la IP del proxy también en
`RP_TRUSTED_PROXY` y, si escucha fuera de este equipo, cambia `RP_LISTEN`.

`nftables-routerpanel.nft` es un ejemplo del mismo límite en el cortafuegos
(útil si no te fías solo de systemd o para los contenedores): solo deja al
servicio abrir conexiones hacia el router.

### Proxy inverso

Cualquiera que haga HTTPS y pase `X-Forwarded-For`. Por ejemplo, un
Caddyfile en el mismo equipo:

```
panel.example {
	reverse_proxy 127.0.0.1:3000
}
```

y `RP_ORIGIN=https://panel.example`. `deploy/docker/Caddyfile.example` tiene
un ejemplo con CA interna y solo para la red local.

### Instalaciones anteriores al asistente

`RP_ROUTER_URL`, `RP_ROUTER_USER` y `RP_ROUTER_SECRET` solo sirven para
importar una vez las credenciales del router. Con `DynamicUser` el servicio no
puede leer un fichero de root: pásalo como credencial (`systemctl edit
routerpanel`):

```
[Service]
LoadCredential=router.secret:/etc/routerpanel/router.secret
Environment=RP_ROUTER_SECRET=%d/router.secret
```

## Actualizar

### A mano

Igual que la instalación: la versión nueva en `releases/<versión>`, el enlace
`current` hacia ella y `systemctl restart routerpanel`. Para volver atrás,
el enlace a la anterior y otro reinicio (si la nueva migró la base de datos,
también la copia de antes).

### `actualizar-release.sh` (opcional)

Descarga la última release, comprueba su sha256 (y la firma de minisign si
le das la clave), la despliega y espera a que `/salud` responda; si no, vuelve
a la anterior con la base de datos de antes. Conserva 3 versiones. Necesita
`curl`, `sha256sum` y `tar` (y `minisign` si hay clave).

```sh
install -m 755 deploy/linux/actualizar-release.sh /usr/local/sbin/routerpanel-actualizar
cat >/etc/routerpanel/actualizar.env <<'EOF'
RP_RELEASES_URL=https://github.com/<usuario>/<repo>/releases
RP_OBJETIVO=x86_64-unknown-linux-gnu
#RP_MINISIGN_CLAVE=RW...
EOF
chmod 600 /etc/routerpanel/actualizar.env
routerpanel-actualizar --comprobar   # solo dice si hay una versión nueva
routerpanel-actualizar               # actualiza
```

La última versión sale de `<base>/latest/download/SHA256SUMS` y los ficheros
de `<base>/download/<versión>/`, como en GitHub. Otros alojamientos valen si
sirven esas mismas rutas.

No se instala ningún temporizador. Si quieres que se actualice solo, por
ejemplo una vez al día:

```ini
# /etc/systemd/system/routerpanel-actualizar.service
[Unit]
Description=Actualizar el panel del router
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=/usr/local/sbin/routerpanel-actualizar
```

```ini
# /etc/systemd/system/routerpanel-actualizar.timer
[Unit]
Description=Actualizar el panel del router cada día

[Timer]
OnCalendar=daily
RandomizedDelaySec=1h
Persistent=true

[Install]
WantedBy=timers.target
```

```sh
systemctl daemon-reload
systemctl enable --now routerpanel-actualizar.timer
```

El actualizador es lo único que sale a internet; el panel no.
