# Panel del router RTF8115VW

*[English](README.en.md)*

Panel web para el router Askey RTF8115VW que Movistar y O2 instalan en España.
Funciona en un equipo de tu red (un servidor, una Raspberry Pi o un
contenedor) y habla con la web del router.

- **Estado**: conexión a internet, tiempo encendido, tráfico, carga, RAM y
  clientes, con gráficas de 1 h, 24 h y 7 días, y avisos como la gestión
  remota TR-069 o el WPS encendidos.
- **Ajustes**: los del router, agrupados por temas (Internet, WiFi, Red
  local, Puertos, Seguridad, Teléfono y Sistema) y explicados en lenguaje
  claro. Cada uno dice qué hace, qué pasa al cambiarlo y qué valor se
  recomienda.
- **Mejoras**: dos lotes, «Seguridad recomendada» y «Privacidad máxima», que
  se revisan y se aplican de una vez.
- **Biblioteca**: todas las funciones de la API del router, con su
  descripción.
- **Historial** de todo lo cambiado, con deshacer.
- Usuarios propios, español e inglés, tema claro u oscuro y uso cómodo en el
  móvil.

Es un proyecto independiente, sin relación con Telefónica, Movistar, O2 ni
Askey. Lee el [aviso legal](AVISO-LEGAL.md) antes de usarlo.

## Capturas

| Estado | En el móvil | Conexión con el router |
|---|---|---|
| ![Estado del router en el escritorio](docs/capturas/estado-escritorio.png) | ![Estado del router en el móvil](docs/capturas/estado-movil.png) | ![Configuración de la conexión con el router](docs/capturas/configuracion-escritorio.png) |

| Ajustes: WiFi | Ajustes: Puertos, en el tema claro | Mejoras |
|---|---|---|
| ![Ajustes de la WiFi en tarjetas](docs/capturas/ajustes-wifi-escritorio.png) | ![Reglas de puertos en una lista con columnas](docs/capturas/ajustes-puertos-claro.png) | ![Los lotes Seguridad recomendada y Privacidad máxima, con sus pasos](docs/capturas/mejoras-escritorio.png) |

Están hechas con el router simulado y datos inventados.

## Router y firmware probados

| Modelo | Firmware |
|---|---|
| Askey RTF8115VW | `ES_g14.17_RTF_TEF001_V9.17_V034` |

Con otro firmware, el panel arranca en **solo lectura**: muestra el estado y
lee los ajustes, pero no cambia nada. Si todo cuadra, confírmalo en
«Configuración del router → Conexión con el router» y abre una incidencia con
el modelo, el firmware y lo que has probado para añadirlo a la lista.

## Cómo cambia las cosas

El panel solo escribe lo que se ha comprobado en un router real. El resto se
muestra, con su valor y su recomendación, pero sin interruptor. La lista está
en [funciones-del-panel.md](docs/funciones-del-panel.md).

Antes de guardar, un resumen enseña qué cambia («antes → después»). Los
cambios que pueden cortar el acceso al router o a internet se **vigilan**: si
el router deja de responder o se pierde internet, se deshacen solos. Todo
cambio se puede deshacer después desde el propio aviso o desde el Historial, y
«Cómo estaba al instalar» compara el router con la foto que tomó el asistente.

Las contraseñas aparecen censuradas, con un ojo para verlas.

En este router hay dos cosas que su web no deja hacer bien:

- **Cambiar la contraseña de otra cuenta.** Solo la cuenta `1234` entra en la
  web, así que el panel no puede comprobar la nueva. Lo que hace es borrar la
  cuenta y crearla de nuevo.
- **Dejar de responder al ping desde internet.** Otra regla del operador lo
  sigue permitiendo.

### Mejoras

- **Seguridad recomendada.**
  - Apaga la gestión remota del operador (TR-069), el WPS, el UPnP, el espejo
    del tráfico y el envío del registro a otro equipo.
  - Deja cada red WiFi en WPA2-AES y aísla a los dispositivos de la red de
    invitados.
- **Privacidad máxima.**
  - Apaga la gestión remota, el espejo y el envío del registro.
  - Reparte unos DNS neutros, del proveedor que elijas, y opcionalmente toma
    la hora de un servidor neutro.

Cada lote dice para quién es, qué se gana y qué se pierde. Se aplica como un
solo cambio, vigilado y con un solo deshacer. Solo lleva los pasos que el
panel ya escribe; los que tocan algo sin comprobar no aparecen hasta que se
comprueben.

Los DNS neutros solo cambian quién responde. Las consultas siguen viajando
sin cifrar y el operador puede verlas. Para ocultarlas hace falta DNS cifrado
(DoH o DoT) en cada dispositivo, o una VPN.

## Instalación

Hay tres formas; cada una tiene su guía en [`deploy/`](deploy/README.md):

- **Linux con systemd** ([`deploy/linux/`](deploy/linux/README.md)). Usa las
  releases ya compiladas para x86_64 y aarch64 (Debian 12, Ubuntu 22.04,
  Raspberry Pi OS de 64 bits o más nuevos). Cada release trae `SHA256SUMS`,
  y el actualizador opcional lo comprueba.
- **Podman o Docker con Compose** ([`deploy/docker/`](deploy/docker/README.md)).
  Incluye el panel y un proxy Caddy de ejemplo.
- **Jails de FreeBSD** ([`deploy/freebsd-jail/`](deploy/freebsd-jail/README.md)).
  Compila desde el código y se actualiza solo con cada etiqueta.

El panel no habla HTTPS. Necesita delante un **proxy inverso con HTTPS**
(Caddy, nginx…) limitado a la red local, que le pase la IP del cliente en
`X-Forwarded-For`; sin HTTPS, el navegador no guarda la sesión. Lo explica
[`docs/proxy-tls.md`](docs/proxy-tls.md).

La primera vez se termina en el navegador, con el asistente de instalación.
El día a día (usuarios, copias de seguridad, registros y actualizaciones) está
en [`docs/operacion.md`](docs/operacion.md).

### Probarlo sin router

Con Podman o Docker, el perfil `prueba` añade un router simulado
(`http://router-sim:8081`, usuario y contraseña `sim`):

```sh
export RP_ORIGIN=https://panel.home.arpa PANEL_DOMINIO=panel.home.arpa
podman compose -f deploy/docker/compose.yaml --profile prueba up -d --build
```

## Configuración (`RP_*`)

Variables de entorno del panel (`crates/panel/src/config.rs`):

| Variable | Por defecto | Qué es |
|---|---|---|
| `RP_ORIGIN` | ninguno: **obligatoria** | la dirección del panel tal como la escribe el navegador (`https://panel.home.arpa`); si hay varias, separadas por comas |
| `RP_LISTEN` | `127.0.0.1:3000` | dirección y puerto donde escucha (HTTP, detrás del proxy) |
| `RP_TRUSTED_PROXY` | `127.0.0.1` | IP o redes de los proxies de confianza para `X-Forwarded-For`, separadas por comas |
| `RP_DB` | Linux: `/var/lib/routerpanel/panel.db`; FreeBSD: `/var/db/routerpanel/panel.db` | base de datos SQLite (usuarios, credenciales cifradas, historial) |
| `RP_HISTORY_KEY` | Linux: `/etc/routerpanel/history.key`; FreeBSD: `/usr/local/etc/routerpanel/history.key` | la clave que cifra las credenciales y los secretos; la crea el panel |
| `RP_INSTALL_NETS` | vacía | redes, además de la local, desde las que responde el asistente de instalación |
| `RP_LOG` | `info` | nivel de los registros (sintaxis de `EnvFilter` de `tracing`) |
| `RP_ROUTER_URL`, `RP_ROUTER_USER`, `RP_ROUTER_SECRET` | vacías | solo para instalaciones anteriores al asistente: se importan una vez a la base de datos. Van las tres o ninguna |

Si lanzas el binario a mano, hacen falta además las de Leptos:
`LEPTOS_SITE_ROOT` (la carpeta `site/`), `LEPTOS_SITE_PKG_DIR=pkg`,
`LEPTOS_OUTPUT_NAME=panel` y `LEPTOS_SITE_ADDR` (igual que `RP_LISTEN`). Las
guías de instalación ya las ponen.

## Compilar

- Rust, en la versión de [`rust-toolchain.toml`](rust-toolchain.toml).
  `rustup` la instala sola, con el objetivo `wasm32-unknown-unknown`.
- `cargo-leptos` 0.3.11: `cargo install --locked cargo-leptos --version 0.3.11`.
- `wasm-bindgen-cli`, en **la misma versión** que `wasm-bindgen` en
  `Cargo.lock`: `cargo install --locked wasm-bindgen-cli --version <versión>`.
- `binaryen` (`wasm-opt`), del gestor de paquetes del sistema.

```sh
cargo leptos build --release   # target/release/panel y target/site/
```

## Desarrollo sin router

`router-sim` es un router simulado a partir de respuestas capturadas y
saneadas (`crates/hgu/fixtures`).

```sh
cargo run -p router-sim      # en 127.0.0.1:8081, usuario y contraseña «sim»
mkdir -p /tmp/panel
RP_LISTEN=127.0.0.1:3000 RP_DB=/tmp/panel/panel.db RP_HISTORY_KEY=/tmp/panel/history.key \
RP_TRUSTED_PROXY=127.0.0.1 RP_ORIGIN=http://localhost:3000 cargo leptos watch
```

Entra por `http://localhost:3000`, no por `127.0.0.1`. La cookie de sesión
es `Secure`, y el navegador solo la acepta sin HTTPS en `localhost`. Sin
usuarios, el panel abre el asistente (`/instalar`). El token está en
`/tmp/panel/instalar.token` y el router es `http://127.0.0.1:8081`.

## Estructura

| Ruta | Qué es |
|---|---|
| `crates/hgu` | cliente del router: sesión, lecturas y escrituras (de una en una, con 150 ms entre peticiones) y detección de la red; capturas saneadas en `fixtures/` |
| `crates/ajustes` | catálogo de Ajustes (`ajustes/<tema>.toml`): campos, elementos y funciones del router que lee y escribe cada ajuste |
| `crates/catalogo` | catálogo revisado de funciones (`funciones.toml`) y secretos que se censuran (`secretos.toml`) |
| `crates/panel` | servidor y web (SSR e hidratación), asistente, autenticación, sondeo, cambios vigilados, historial y ajustes; las órdenes `panel user add` y `panel user passwd` |
| `crates/i18n` | textos de la interfaz en español e inglés (Fluent) |
| `crates/router-sim` | router simulado para pruebas y desarrollo |
| `tools/gen-catalogo` | genera el borrador del catálogo y [`docs/api.md`](docs/api.md) |
| `tools/comprobar-publicacion` | revisa que no se publiquen datos personales ni de la red propia |
| `e2e/` | pruebas de navegador (Playwright) contra el simulador |
| `deploy/` | las formas de instalación |
| `docs/` | la API del router, las funciones del panel, el día a día y el proxy |

## Comprobaciones

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p panel --all-targets --features ssr -- -D warnings
cargo clippy -p panel --lib --target wasm32-unknown-unknown --no-default-features --features hydrate -- -D warnings
cargo nextest run --workspace --features panel/ssr   # o cargo test, más despacio
cargo test --doc --workspace --features panel/ssr    # nextest no pasa los doctests
cargo audit && cargo deny check
cargo run -q -p comprobar-publicacion -- --arbol . --sin-locales
```

Las pruebas de navegador usan el binario de depuración de `cargo leptos
build`. Como `cargo test` con `--features panel/ssr` deja otro binario en el
mismo sitio, `cargo leptos build` va al final:

```sh
cargo build -p router-sim
cargo build -p panel --features ssr --example e2e_grupo
cargo leptos build
cd e2e && npm ci && npx playwright install chromium && npx playwright test
```

`E2E_WORKERS` fija cuántos procesos en paralelo (por defecto, 1 o 2 según la
CPU). Cada uno arranca sus paneles y simuladores en puertos a partir de 3195
y 8195. `npx playwright test capturas-readme` regenera las capturas de arriba.
Después hay que revisarlas a mano y poner su nuevo sha256 en
`tools/publicacion-permitidos.txt`.

## Contribuir

Antes del primer `git push`, activa el gancho que revisa lo que se sube:

```sh
git config core.hooksPath tools/hooks
git config routerpanel.sinListas true   # si no eres el mantenedor
```

El gancho pasa `comprobar-publicacion` por todo lo que sale. Busca MAC, IP y
dominios que no sean de documentación y, en el clon del mantenedor, también
sus listas locales de datos propios, que no están en el repositorio. La
segunda línea desactiva esas listas en tu clon.

En ejemplos y pruebas usa direcciones de documentación (`192.0.2.x`,
`*.example`, `*.home.arpa`). Las capturas del router se sanean con
`tools/sanear-fixtures.sh`.

## Licencia

[MIT](LICENSE). Ver también el [aviso legal](AVISO-LEGAL.md).
