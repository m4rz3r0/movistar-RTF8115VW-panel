# Panel del router RTF8115VW

*[English](README.en.md)*

Panel web para el router Askey RTF8115VW que Movistar y O2 instalan en España.
Corre en un equipo de tu red (un servidor, una Raspberry Pi, un contenedor) y
habla con la web del router:

- **Estado**: conexión a internet, tiempo encendido, tráfico, carga, RAM y
  clientes, con gráficas de 1 h, 24 h y 7 días y avisos (gestión remota
  TR-069, WPS…).
- **Explorador**: todas las funciones de la API del router, con su
  descripción. Los cambios se **vigilan**: si el router deja de responder o
  se pierde internet, se deshacen solos.
- **Historial** de todo lo cambiado, con deshacer.
- Usuarios propios con contraseña, en español e inglés, y usable en el móvil.

Proyecto independiente, sin relación con Telefónica, Movistar, O2 ni Askey:
lee el [aviso legal](AVISO-LEGAL.md) antes de usarlo.

## Capturas

| Estado | En el móvil | Conexión con el router |
|---|---|---|
| ![Estado del router en el escritorio](docs/capturas/estado-escritorio.png) | ![Estado del router en el móvil](docs/capturas/estado-movil.png) | ![Configuración de la conexión con el router](docs/capturas/configuracion-escritorio.png) |

Hechas con el router simulado y datos inventados. Se regeneran con
`npx playwright test capturas-readme` (en `e2e/`, ver
[Comprobaciones](#comprobaciones)).

## Router y firmware probados

| Modelo | Firmware |
|---|---|
| Askey RTF8115VW | `ES_g14.17_RTF_TEF001_V9.17_V034` |

Con otro firmware el panel arranca en **solo lectura**: ve el estado y lee,
pero no cambia nada. Si todo cuadra, confírmalo en «Router → Conexión con el
router» y cuéntalo en una incidencia del repositorio (modelo, firmware y qué
has probado) para añadirlo a la lista.

## Qué detecta y qué configuras

Al conectar, el panel **detecta** del router: modelo y firmware; qué conexión
lleva internet y de qué tipo es; la red local y la dirección del router en
ella; si hay telefonía y televisión; y qué radios WiFi tiene.

Tú **configuras**:

- en el asistente de instalación, el primer usuario y las credenciales del
  router (se guardan cifradas);
- en «Router → Conexión con el router», las credenciales, la confirmación del
  firmware y, si la detección se equivoca, qué conexión es la de internet;
- en «Ajustes del panel», el idioma y la zona horaria por defecto, DNS
  preferidos, dispositivos de confianza, ventana de mantenimiento, servidor
  propio y usuarios.

El resto de ajustes del router se cambian desde el Explorador, con
vigilancia.

## Instalación

Tres vías, cada una con su guía ([`deploy/README.md`](deploy/README.md)):

- **Linux con systemd** ([`deploy/linux/`](deploy/linux/README.md)): releases
  ya compiladas para x86_64 y aarch64 que piden glibc 2.35 o posterior
  (Debian 12, Ubuntu 22.04, Raspberry Pi OS de 64 bits o más nuevos). Cada
  release trae `SHA256SUMS` y, opcionalmente, su firma de minisign; el
  actualizador opcional comprueba las dos.
- **Docker o Podman** ([`deploy/docker/`](deploy/docker/README.md)): Compose
  con el panel y un proxy Caddy de ejemplo.
- **Jails de FreeBSD** ([`deploy/freebsd-jail/`](deploy/freebsd-jail/README.md)):
  compila desde el código (no hay releases para FreeBSD) en una jail aparte y
  se actualiza solo con cada etiqueta.

La primera vez se termina en el navegador con el asistente de instalación; el
día a día (usuarios, copias de seguridad, registros, actualizar) está en
[`docs/operacion.md`](docs/operacion.md).

### HTTPS y proxy inverso

El panel no habla HTTPS por sí mismo y **necesita un proxy inverso con
HTTPS** delante (Caddy, nginx…), limitado a la red local, que le pase la IP
del cliente en `X-Forwarded-For`. Sin HTTPS el navegador no guarda la sesión.
Ejemplos y detalles: [`docs/proxy-tls.md`](docs/proxy-tls.md).

## Configuración (`RP_*`)

Variables de entorno del panel (`crates/panel/src/config.rs`):

| Variable | Por defecto | Qué es |
|---|---|---|
| `RP_ORIGIN` | ninguno: **obligatoria** | dirección del panel tal como la escribe el navegador (`https://panel.home.arpa`); varias, separadas por comas |
| `RP_LISTEN` | `127.0.0.1:3000` | dirección y puerto donde escucha (HTTP, detrás del proxy) |
| `RP_TRUSTED_PROXY` | `127.0.0.1` | IPs o redes de los proxies de los que se fía para `X-Forwarded-For`, separadas por comas |
| `RP_DB` | Linux: `/var/lib/routerpanel/panel.db`; FreeBSD: `/var/db/routerpanel/panel.db` | base de datos SQLite (usuarios, credenciales cifradas, historial) |
| `RP_HISTORY_KEY` | Linux: `/etc/routerpanel/history.key`; FreeBSD: `/usr/local/etc/routerpanel/history.key` | clave maestra que cifra las credenciales y los secretos; la crea el panel |
| `RP_INSTALL_NETS` | vacía | redes desde las que, además de la red local, responde el asistente de instalación |
| `RP_LOG` | `info` | nivel de los registros (sintaxis de `EnvFilter` de `tracing`) |
| `RP_ROUTER_URL`, `RP_ROUTER_USER`, `RP_ROUTER_SECRET` | vacías | solo para instalaciones anteriores al asistente: dirección, usuario y fichero (600) con la contraseña del router, que se importan una vez a la base de datos. Las tres o ninguna |

Además, al lanzar el binario a mano, las de Leptos para la web:
`LEPTOS_SITE_ROOT` (la carpeta `site/`), `LEPTOS_SITE_PKG_DIR=pkg`,
`LEPTOS_OUTPUT_NAME=panel` y `LEPTOS_SITE_ADDR` (igual que `RP_LISTEN`). Las
guías de instalación ya las ponen.

## Compilar

- Rust: la versión fijada en [`rust-toolchain.toml`](rust-toolchain.toml)
  (con `rustup`, se instala sola con su objetivo `wasm32-unknown-unknown`).
- `cargo-leptos` 0.3.11: `cargo install --locked cargo-leptos --version 0.3.11`.
- `wasm-bindgen-cli`, **la misma versión** que `wasm-bindgen` en
  `Cargo.lock`:
  `cargo install --locked wasm-bindgen-cli --version <versión de Cargo.lock>`.
- `binaryen` (`wasm-opt`), del gestor de paquetes del sistema.

```sh
cargo leptos build --release   # target/release/panel y target/site/
```

## Desarrollo sin router

`router-sim` es un router simulado a partir de respuestas capturadas y
saneadas (`crates/hgu/fixtures`): sirve para probar el panel sin tocar un
router de verdad.

```sh
cargo run -p router-sim                     # router simulado en 127.0.0.1:8081 (usuario y contraseña: sim)
mkdir -p /tmp/panel
RP_LISTEN=127.0.0.1:3000 RP_DB=/tmp/panel/panel.db RP_HISTORY_KEY=/tmp/panel/history.key \
RP_TRUSTED_PROXY=127.0.0.1 RP_ORIGIN=http://localhost:3000 cargo leptos watch
```

Entra por `http://localhost:3000` (no `127.0.0.1`: la cookie
`__Host-rp_session` es `Secure` y el navegador solo la acepta por http en
`localhost`). Sin usuarios, el panel abre el asistente de instalación
(`/instalar`): el token está en el registro y en `/tmp/panel/instalar.token`;
el router es `http://127.0.0.1:8081`, usuario y contraseña `sim`.

## Estructura

| Ruta | Qué es |
|---|---|
| `crates/hgu` | cliente del router: sesión, lecturas y escrituras, un único mutex y 150 ms entre peticiones; detección de la red; capturas saneadas en `fixtures/` |
| `crates/catalogo` | catálogo revisado de funciones (`funciones.toml`: riesgo, bloqueadas, lecturas pareadas) y secretos que se enmascaran (`secretos.toml`) |
| `crates/panel` | servidor y web (SSR + hidratación), asistente de instalación, autenticación, sondeo, cambios vigilados, historial, ajustes; CLI `panel user add` y `panel user passwd` |
| `crates/i18n` | textos de la interfaz en español e inglés (Fluent, `locales/{es,en}`) |
| `crates/router-sim` | router simulado para pruebas y desarrollo; `sanear-fixtures` |
| `tools/gen-catalogo` | genera el borrador del catálogo y [`docs/api.md`](docs/api.md) |
| `tools/comprobar-publicacion` | revisa que no se publiquen datos personales ni de la red propia |
| `e2e/` | pruebas de navegador (Playwright) contra el simulador |
| `deploy/` | las tres vías de instalación ([`deploy/README.md`](deploy/README.md)) |
| `docs/` | [`api.md`](docs/api.md) (API del router), [`operacion.md`](docs/operacion.md) (día a día), [`proxy-tls.md`](docs/proxy-tls.md) (HTTPS y proxy) |

## Comprobaciones

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p panel --all-targets --features ssr -- -D warnings
cargo clippy -p panel --lib --target wasm32-unknown-unknown --no-default-features --features hydrate -- -D warnings
# Pruebas con cargo-nextest (cargo install --locked cargo-nextest), que
# reparte cada test en su proceso; perfiles en .config/nextest.toml. Sin él,
# cargo test --workspace [--features panel/ssr] prueba lo mismo, más despacio.
cargo nextest run --workspace
cargo nextest run --workspace --features panel/ssr
cargo test --doc --workspace --features panel/ssr   # nextest no ejecuta los doctests
cargo audit          # excepciones justificadas en .cargo/audit.toml
cargo deny check     # deny.toml
cargo run -q -p comprobar-publicacion -- --arbol . --sin-locales

# Navegador: asistente, idiomas, sin desbordamiento horizontal a 360 px;
# capturas en e2e/capturas/
cargo leptos build && cargo build -p router-sim
cargo build -p panel --features ssr --example e2e_grupo
cd e2e && npm ci && npx playwright test
npx playwright test capturas-readme   # solo si cambian las capturas del README
```

El e2e arranca cinco paneles, cada uno con su simulador (`e2e/instancias.ts`),
en los puertos 3195–3199 y 8195–8199 (a partir de `E2E_PORT` y
`E2E_SIM_PORT`). Usa el Chromium de Playwright (`npx playwright install
chromium` si no está; o `E2E_CHROMIUM=<ruta>`).

Las capturas del README se guardan en `docs/capturas/` sin metadatos. Tras
regenerarlas, revísalas a mano (que no salga nada real) y pon su nuevo sha256
en `tools/publicacion-permitidos.txt`.

## Contribuir

Antes del primer `git push`, activa el gancho que revisa lo que se sube:

```sh
git config core.hooksPath tools/hooks
```

`pre-push` pasa `comprobar-publicacion` por todo lo que sale: MACs, IPs y
dominios que no son de documentación y, además, las listas locales del
mantenedor con sus propios datos (que no están en el repositorio). Por
defecto el gancho exige esas listas y no sube nada sin ellas; si no eres el
mantenedor, desactívalas en tu clon (solo vale en su `.git/config`, y el
gancho avisa en cada push):

```sh
git config routerpanel.sinListas true
```

Las comprobaciones genéricas siguen activas. Usa direcciones de
documentación en ejemplos y pruebas (`192.0.2.x`, `*.example`,
`*.home.arpa`) y sanea las capturas del router con
`tools/sanear-fixtures.sh`.

## Licencia

[MIT](LICENSE). Ver también el [aviso legal](AVISO-LEGAL.md).
