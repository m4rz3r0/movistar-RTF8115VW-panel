# Panel con Docker o Podman (Compose)

| Fichero | Para qué |
|---|---|
| `Dockerfile` | imagen del panel: compila con `cargo-leptos` 0.3.11, `wasm-bindgen-cli` (la versión de `Cargo.lock`) y `binaryen`, y la ejecuta en `debian:bookworm-slim` con un usuario sin privilegios |
| `compose.yaml` | el panel, sin puertos publicados, y un proxy inverso delante; perfil `prueba` con un router simulado |
| `Caddyfile.example`, `proxy.Dockerfile` | el proxy inverso de ejemplo: HTTPS con una CA interna y solo para la red local |
| `../pruebas/docker.sh` | prueba de humo de todo lo anterior |

## Arrancar

Desde la raíz del repositorio:

```sh
export RP_ORIGIN=https://panel.home.arpa PANEL_DOMINIO=panel.home.arpa
docker compose -f deploy/docker/compose.yaml up -d --build
```

(o `podman compose`). `RP_ORIGIN` es obligatoria: la dirección del panel tal
como la escribe el navegador, con el puerto si no es el 443. Las demás
variables están explicadas al principio de `compose.yaml`; puedes dejarlas en
un `.env` junto a él. Haz que `PANEL_DOMINIO` resuelva en tu red a la IP de
este equipo (en el DNS de tu red o en el `hosts` de cada equipo).

El proxy usa su propia CA (`tls internal`): el navegador avisará hasta que
instales su certificado raíz, que está en el volumen `proxy-datos`:

```sh
docker compose -f deploy/docker/compose.yaml exec proxy find /data -name root.crt
```

El Caddyfile va dentro de la imagen del proxy: si lo cambias, vuelve a
levantarlo con `--build`.

### Datos

Todo lo del panel está en el volumen `datos` (`/data` en el contenedor):

- `panel.db` — usuarios, credenciales del router (cifradas), historial;
- `history.key` — la clave que las cifra; sin ella no se lee nada de
  `panel.db`, y las dos juntas lo descifran todo: guárdalas con cuidado;
- `instalar.token` — mientras no hay administrador.

`docker compose down` conserva los volúmenes; `down -v` los **borra**.

## Primer arranque: el token del asistente

Sin usuarios, el panel solo sirve `/instalar` y pide un token de un solo uso
que genera al arrancar. Está en el registro y en `/data/instalar.token`:

```sh
docker compose -f deploy/docker/compose.yaml logs panel | grep 'token de instalación'
docker compose -f deploy/docker/compose.yaml exec panel cat /data/instalar.token
```

Al crear el administrador, el token se borra.

## Quién llega al panel (y qué protege cada capa)

1. **El proxy, solo red local.** Corta cualquier conexión que no venga de una IP
   privada (o de `PROXY_REDES_EXTRA`). Mira la IP de la conexión, nunca
   `X-Forwarded-For`, y al panel le pasa esa IP en `X-Forwarded-For`.
2. **El panel solo se fía del proxy.** No publica puertos: solo se le llega
   desde la red de Compose. `RP_TRUSTED_PROXY` es la IP **fija** del proxy
   (`RP_IP_PROXY`), no la subred: en la subred también está la puerta de
   enlace de la red, por la que llega cualquier conexión directa desde este
   equipo, y cualquier otro contenedor que conectes a ella. Si una de esas
   le dijera en `X-Forwarded-For` que viene de la red local, el panel no la
   creería: para él es la IP de la puerta de enlace, que con la subred por
   defecto (de documentación) ni siquiera es privada.
3. **El asistente pide red local y token.** Solo responde a clientes de la red
   local o de `RP_INSTALL_NETS`, y aun así sin el token de `/data/instalar.token`
   no crea el administrador. El token es lo que de verdad lo protege: las dos
   capas de antes solo reducen quién puede intentarlo.
4. **Después, usuario y contraseña**, con la sesión, la reautenticación y el
   freno de intentos del panel.

La prueba de humo comprueba la 2 y la 3: una petición directa desde la puerta
de enlace, diciendo venir de la red local, y otra a través del proxy sin
token reciben las dos un 403.

### Podman sin root

Con podman sin root, el reenvío de puertos no conserva la IP del cliente:
el proxy ve todas las conexiones como si vinieran de su propia IP
(`RP_IP_PROXY`), que no es privada con la subred por defecto. Así que la capa
1 corta todo, y el asistente (capa 3) dice que no es la red local. Para usarlo
así tienes que aceptarlo a sabiendas:

```sh
export PROXY_REDES_EXTRA=198.51.100.10/32   # la IP del proxy
export RP_INSTALL_NETS=198.51.100.10        # solo mientras usas el asistente
```

Entonces cualquiera que llegue al puerto publicado pasa esas dos capas:
limita quién llega con el cortafuegos del equipo o publicando el puerto solo
en la IP de tu red (`RP_PUERTO_HTTPS=192.0.2.10:443`, con la IP de este
equipo), y quita `RP_INSTALL_NETS` al terminar la instalación. El freno de
intentos del panel también verá a todos como un solo cliente. Con Docker o
podman con root no pasa: la IP del cliente llega tal cual.

Podman sin root tampoco puede publicar el 443 sin cambiar
`net.ipv4.ip_unprivileged_port_start`; usa otro puerto
(`RP_PUERTO_HTTPS=8443`, y ese puerto en `RP_ORIGIN`).

## Salida a internet

El panel solo habla con el router. La red de Compose sale a internet por
defecto; `deploy/linux/nftables-routerpanel.nft` trae un ejemplo que limita las
conexiones nuevas de esa red al router (con Docker o podman con root).

## Tu propio proxy

Quita el servicio `proxy`, conecta el tuyo a la red de Compose con una IP
fija y ponla en `RP_TRUSTED_PROXY` (en `compose.yaml`) en lugar de la del de
ejemplo. El panel escucha en `panel:3000`.

## Prueba de humo

```sh
deploy/pruebas/docker.sh
```

Levanta el proyecto `routerpanel-prueba` con el perfil `prueba` y el proxy en el
puerto 8500 (`RP_PRUEBA_PROYECTO`, `RP_PRUEBA_PUERTO`), comprueba `/salud` a
través del proxy, el token y las capas de arriba, y lo borra con `down -v`
(solo ese proyecto). Las imágenes compiladas se quedan para la próxima vez.
Sin `docker` ni `podman` con Compose, se salta con un aviso.
