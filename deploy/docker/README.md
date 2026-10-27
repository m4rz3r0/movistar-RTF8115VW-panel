# Panel con Podman o Docker (Compose)

| Fichero | Para qué |
|---|---|
| `Dockerfile` | la imagen del panel: lo compila y lo ejecuta en `debian:bookworm-slim` con un usuario sin privilegios |
| `compose.yaml` | el panel, sin puertos publicados, con un proxy inverso delante; el perfil `prueba` añade un router simulado |
| `Caddyfile.example`, `proxy.Dockerfile` | el proxy de ejemplo: HTTPS con una CA propia y solo para la red local |
| `../pruebas/docker.sh` | prueba de humo de todo lo anterior |

Los ejemplos usan `podman compose`. Con Docker, cambia `podman` por
`docker`: las órdenes son las mismas.

## Arrancar

Desde la raíz del repositorio:

```sh
export RP_ORIGIN=https://panel.home.arpa PANEL_DOMINIO=panel.home.arpa
podman compose -f deploy/docker/compose.yaml up -d --build
```

`RP_ORIGIN` es obligatoria. Es la dirección del panel tal como la escribe el
navegador, con el puerto si no es el 443. Las demás variables están
explicadas al principio de `compose.yaml` y se pueden dejar en un `.env` junto
a él. `PANEL_DOMINIO` tiene que resolver a la IP de este equipo, en el DNS de
tu red o en el `hosts` de cada equipo.

Para probarlo sin router, añade `--profile prueba`. El router simulado está
en `http://router-sim:8081`, con usuario y contraseña `sim`.

Con Podman sin root hay que cambiar dos cosas más; lo explica
[Podman sin root](#podman-sin-root).

El proxy usa su propia CA (`tls internal`), así que el navegador avisará
hasta que instales su certificado raíz:

```sh
podman compose -f deploy/docker/compose.yaml exec proxy find /data -name root.crt
```

El Caddyfile va dentro de la imagen del proxy. Si lo cambias, levántalo otra
vez con `--build`.

## Primer arranque: el token del asistente

Sin usuarios, el panel solo sirve `/instalar` y pide un token de un solo uso
que crea al arrancar. El registro solo dice dónde está, para que no lo vea
quien pueda leer los registros del contenedor:

```sh
podman compose -f deploy/docker/compose.yaml exec panel cat /data/instalar.token
```

El token se borra al crear el administrador.

## Datos

Todo lo del panel está en el volumen `datos` (`/data` en el contenedor):

- `panel.db`: usuarios, credenciales del router (cifradas) e historial;
- `history.key`: la clave que las cifra. Sin ella no se lee nada de
  `panel.db`, y con las dos juntas se descifra todo, así que guarda las
  copias con cuidado;
- `instalar.token`, mientras no haya administrador.

`down` conserva los volúmenes; `down -v` los **borra**.

## Quién llega al panel

1. **El proxy solo deja pasar la red local.** Corta las conexiones que no
   vienen de una IP privada (o de `PROXY_REDES_EXTRA`). Mira la IP de la
   conexión, nunca `X-Forwarded-For`, y se la pasa al panel en esa cabecera.
2. **El panel solo se fía del proxy.** No publica puertos y
   `RP_TRUSTED_PROXY` es la IP fija del proxy (`RP_IP_PROXY`), no la subred.
   En la subred también está la puerta de enlace de la red, por la que llega
   cualquier conexión directa desde este equipo.
3. **El asistente pide red local y token.** Sin el token de
   `/data/instalar.token` no crea el administrador, venga de donde venga la
   petición.
4. **Después, usuario y contraseña**, con el freno de intentos del panel.

La red de Compose usa por defecto una subred de documentación
(`198.51.100.0/24`), que no es privada. Así, ni la puerta de enlace ni otro
contenedor conectado a la red cuentan como red local. Si cambias
`RP_RED_INTERNA`, usa otra subred **no privada**, por ejemplo
`203.0.113.0/24`, con `RP_IP_PROXY` dentro de ella. Con una subred privada,
cualquier conexión directa desde este equipo o desde otro contenedor llegaría
al asistente, y solo la pararía el token.

## Podman sin root

Sin root, el reenvío de puertos de Podman no conserva la IP del cliente. El
proxy ve todas las conexiones como si vinieran de su propia IP, así que corta
todo y el asistente dice que no es la red local. Para usarlo así, hay que
aceptarlo a sabiendas:

```sh
export PROXY_REDES_EXTRA=198.51.100.10/32   # la IP del proxy
export RP_INSTALL_NETS=198.51.100.10        # solo mientras usas el asistente
export RP_PUERTO_HTTPS=8443                 # sin root no se puede usar el 443
export RP_ORIGIN=https://panel.home.arpa:8443
```

Con eso, cualquiera que llegue al puerto publicado pasa las dos primeras
capas, y el freno de intentos ve a todos como un solo cliente. Limita quién
llega con el cortafuegos del equipo o publicando el puerto solo en la IP de
tu red (`RP_PUERTO_HTTPS=192.0.2.10:8443`). Quita `RP_INSTALL_NETS` al
terminar la instalación.

Con Podman con root (`sudo podman compose …`) o con Docker no pasa nada de
esto: la IP del cliente llega tal cual.

## Salida a internet

El panel solo habla con el router, pero la red de Compose sale a internet.
[`deploy/linux/nftables-routerpanel.nft`](../linux/nftables-routerpanel.nft)
trae un ejemplo que limita las conexiones nuevas de esa red al router, para
Podman con root o Docker.

## Tu propio proxy

Quita el servicio `proxy`, conecta tu proxy a la red de Compose con una IP
fija y pon esa IP en `RP_TRUSTED_PROXY`, en `compose.yaml`. El panel escucha
en `panel:3000`.

## Prueba de humo

```sh
deploy/pruebas/docker.sh
```

Levanta el proyecto `routerpanel-prueba` con el perfil `prueba` y el proxy en
el puerto 8500, y comprueba `/salud`, el token y las capas de arriba. Al
terminar lo borra con `down -v`, solo ese proyecto. Usa `podman compose`,
`docker compose` o `podman-compose`, el que encuentre; si no hay ninguno, se
salta con un aviso.
