# Panel del router RTF8115VW

*[English](README.en.md)*

Panel web para el router Askey RTF8115VW que Movistar y O2 instalan en España.
Corre en un equipo de tu red (un servidor, una Raspberry Pi, un contenedor) y
habla con la web del router:

- **Estado**: conexión a internet, tiempo encendido, tráfico, carga, RAM y
  clientes, con gráficas de 1 h, 24 h y 7 días y avisos (gestión remota
  TR-069, WPS…).
- **Ajustes**: los ajustes del router agrupados por temas (Internet, WiFi,
  Red local, Puertos, Seguridad, Teléfono y Sistema) y explicados en lenguaje
  claro: qué hace cada uno, qué pasa al cambiarlo, su riesgo y el valor
  recomendado, con para quién es, a favor y en contra. De un vistazo se ve
  qué está encendido, qué pide atención y qué recomienda el panel; se
  enciende o se apaga con un interruptor y se puede actuar sobre varios
  elementos a la vez, con un solo deshacer.
- **Mejoras**: dos lotes de ajustes, «Seguridad recomendada» y «Privacidad
  máxima», que se revisan y se aplican de una vez, con un solo deshacer; la
  barra lateral cuenta lo que falta.
- **Biblioteca**: todas las funciones de la API del router, con su
  descripción. Los cambios se **vigilan**: si el router deja de responder o
  se pierde internet, se deshacen solos.
- **Historial** de todo lo cambiado, con deshacer.
- Usuarios propios con contraseña, en español e inglés, tema claro u oscuro
  y usable en el móvil.

Proyecto independiente, sin relación con Telefónica, Movistar, O2 ni Askey:
lee el [aviso legal](AVISO-LEGAL.md) antes de usarlo.

## Capturas

| Estado | En el móvil | Conexión con el router |
|---|---|---|
| ![Estado del router en el escritorio](docs/capturas/estado-escritorio.png) | ![Estado del router en el móvil](docs/capturas/estado-movil.png) | ![Configuración de la conexión con el router](docs/capturas/configuracion-escritorio.png) |

| Ajustes: WiFi | Ajustes: Puertos, en el tema claro | Mejoras |
|---|---|---|
| ![Ajustes de la WiFi en tarjetas](docs/capturas/ajustes-wifi-escritorio.png) | ![Reglas de puertos en una lista con columnas](docs/capturas/ajustes-puertos-claro.png) | ![Los lotes Seguridad recomendada y Privacidad máxima, con sus pasos](docs/capturas/mejoras-escritorio.png) |

Hechas con el router simulado y datos inventados. Se regeneran con
`npx playwright test capturas-readme` (en `e2e/`, ver
[Comprobaciones](#comprobaciones)).

## Router y firmware probados

| Modelo | Firmware |
|---|---|
| Askey RTF8115VW | `ES_g14.17_RTF_TEF001_V9.17_V034` |

Con otro firmware el panel arranca en **solo lectura**: ve el estado y lee,
pero no cambia nada. Si todo cuadra, confírmalo en «Configuración del router
→ Conexión con el router» y cuéntalo en una incidencia del repositorio
(modelo, firmware y qué has probado) para añadirlo a la lista.

## Qué detecta y qué configuras

Al conectar, el panel **detecta** del router: modelo y firmware; qué conexión
lleva internet y de qué tipo es; la red local y la dirección del router en
ella; si hay telefonía y televisión; y qué radios WiFi tiene.

Tú **configuras**:

- en el asistente de instalación, el primer usuario y las credenciales del
  router (se guardan cifradas);
- en «Configuración del router → Conexión con el router», las credenciales,
  la confirmación del firmware y, si la detección se equivoca, qué conexión
  es la de internet;
- en «Ajustes del panel», el idioma y la zona horaria por defecto, tu modo
  de visualización, tu tema (automático, como el del sistema, oscuro o
  claro) y los usuarios.

Los ajustes del router se cambian en «Ajustes» (`/ajustes/<tema>`), sin
nombres de la API a la vista: cambiar la contraseña WiFi, apagar el WPS,
fijar la IP de un dispositivo, abrir un puerto o apagar la gestión remota del
operador (TR-069). Cada tema dice cuándo leyó el router, con «Volver a
leer»:

- **WiFi, Internet, Seguridad, Teléfono y Sistema** empiezan con una franja
  de resumen (las recomendaciones pendientes, en ámbar) y van en una rejilla
  de tarjetas: cada una con su estado (verde si está encendida y emite, gris
  si está apagada), su interruptor, sus datos principales y su
  recomendación. Al abrirla, sus campos se despliegan debajo, a todo el
  ancho.
- **Puertos** (las reglas) y **Red local** (las IP fijas y los
  dispositivos) van en listas con columnas, que se ordenan por columna y,
  con más de ocho filas, se filtran; al abrir una fila, su detalle entra por
  la derecha. Los ajustes que no son listas, como UPnP o el DHCP, van en
  tarjetas encima.
- Se marcan varias tarjetas o filas (un círculo que aparece al pasar el
  ratón) y una barra ofrece lo que se puede hacer con todas: un solo cambio
  con un solo deshacer. Cada elemento abierto tiene su dirección
  (`/ajustes/<tema>/<elemento>`), y se recorren con el teclado.
- Apagar algo que corta (una red WiFi con dispositivos conectados) pregunta
  antes; lo demás se aplica al momento, con «Deshacer» en el aviso.

Antes de guardar unos campos, un resumen enseña lo que cambia («antes →
después»); después, el cambio se vigila y se puede deshacer desde el propio
resumen o desde el Historial. «Cómo estaba al instalar» compara el router con
la foto que tomó el asistente y deja volver a cualquier ajuste. Las
contraseñas van censuradas, con un ojo para verlas; lo demás, en claro. Lo
que no está en Ajustes se cambia desde la Biblioteca, también con vigilancia.

### Mejoras

«Mejoras» (`/mejoras`) junta en dos lotes los ajustes que conviene cambiar
y dice, en cada uno, para quién es, qué se gana y qué se pierde:

- **Seguridad recomendada**: apaga la gestión remota del operador (TR-069)
  y su aviso periódico, el WPS de cada red WiFi, la apertura automática de
  puertos (UPnP), el espejo del tráfico y el envío del registro del sistema
  a otro equipo; deja cada red WiFi en WPA2-AES y, si la red de invitados
  está encendida, aísla entre sí sus dispositivos. Lleva además unos
  consejos sin botón, para lo que el router no deja cambiar desde su web.
- **Privacidad máxima**: apaga la gestión remota, su aviso, el espejo y el
  envío del registro; reparte a la red de casa unos DNS neutros, con el
  proveedor a elegir (Quad9, Quad9 sin filtro, DNS4EU, Cloudflare, AdGuard o
  Cloudflare familiar, que filtra webs para adultos) o, si ya usas unos DNS
  propios, los mantiene; y, si lo marcas, toma la hora de un servidor
  neutro.

Cada tarjeta dice qué pasos están hechos, cuáles faltan y cuáles no se
pudieron leer, y la barra lateral cuenta los pendientes. «Revisar y
aplicar» abre la revisión con los pasos por pestaña, cada uno con su «antes
→ después» y su casilla; se aplica como un solo cambio, vigilado y con un
solo deshacer. En Ajustes, una recomendación que forma parte de un lote lo
dice y enlaza a Mejoras.

Los DNS neutros solo cambian quién responde: el router los reparte y cada
dispositivo les pregunta directamente, pero sin cifrar, así que el operador
sigue viendo las consultas por el camino.
Para ocultarlas hace falta DNS cifrado (DoH o DoT) en cada dispositivo o
una VPN.

Los lotes se han probado contra el router simulado (`router-sim`); la
comprobación en el router real va aparte (la televisión y el teléfono con
TR-069 apagado, qué pool DHCP usa el descodificador, el registro…). Hasta
entonces, mejor aplicarlos sabiendo que el «Deshacer» los devuelve.

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
| `crates/ajustes` | catálogo de Ajustes (`ajustes/<tema>.toml`): cada ajuste con sus campos, los elementos de cada tema y las funciones del router que lee y escribe |
| `crates/catalogo` | catálogo revisado de funciones (`funciones.toml`: riesgo, bloqueadas, lecturas pareadas) y secretos que se enmascaran (`secretos.toml`) |
| `crates/panel` | servidor y web (SSR + hidratación), asistente de instalación, autenticación, sondeo, cambios vigilados, historial, ajustes; CLI `panel user add` y `panel user passwd` |
| `crates/i18n` | textos de la interfaz en español e inglés (Fluent, `locales/{es,en}`) |
| `crates/router-sim` | router simulado para pruebas y desarrollo; `sanear-fixtures` |
| `tools/gen-catalogo` | genera el borrador del catálogo y [`docs/api.md`](docs/api.md) |
| `tools/comprobar-publicacion` | revisa que no se publiquen datos personales ni de la red propia |
| `e2e/` | pruebas de navegador (Playwright) contra el simulador |
| `deploy/` | las tres vías de instalación ([`deploy/README.md`](deploy/README.md)) |
| `docs/` | [`api.md`](docs/api.md) (API del router), [`funciones-del-panel.md`](docs/funciones-del-panel.md) (funciones de Ajustes, el Historial y la contraseña del router), [`operacion.md`](docs/operacion.md) (día a día), [`proxy-tls.md`](docs/proxy-tls.md) (HTTPS y proxy) |

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

# Navegador: asistente, idiomas, Ajustes, sin desbordamiento horizontal a
# 360 px; capturas en e2e/capturas/. `cargo leptos build`, lo último (ver
# abajo).
cargo build -p router-sim
cargo build -p panel --features ssr --example e2e_grupo
cargo leptos build
cd e2e && npm ci && npx playwright test
npx playwright test capturas-readme   # solo si cambian las capturas del README
```

El e2e reparte los ficheros entre `E2E_WORKERS` workers (por defecto, uno por
cada 3 hilos de la CPU, entre 1 y 2; en una máquina dedicada, 4; como mucho,
10; `E2E_WORKERS=1`, en serie). Arranca un panel instalado por worker, cuatro
recién instalados para el asistente, dos para el recorrido de Ajustes (uno
por idioma), dos para la rejilla de la WiFi, uno para las demás pestañas en
rejilla y uno para Mejoras, cada uno con su simulador (`e2e/instancias.ts`),
en puertos consecutivos a partir de `E2E_PORT` y `E2E_SIM_PORT` (3195 y
8195; con 4 workers, 3195–3208 y 8195–8208). Cada
cambio se vigila 10 s en vez de 30 s o 2 min (`RP_E2E_VIGILANCIA_S`, que solo
atiende el binario de depuración). Usa el Chromium de Playwright
(`npx playwright install chromium` si no está; o `E2E_CHROMIUM=<ruta>`).

El e2e usa el `target/debug/panel` de `cargo leptos build`, que se compila
como el wasm (con `--cfg erase_components`). `cargo test` y `cargo nextest`
con `--features panel/ssr` dejan otro binario en el mismo sitio, cuyo HTML
el wasm no hidrata: si lo encuentra, el e2e para con «ejecuta `cargo leptos
build`». Por eso `cargo leptos build` va lo último, después de las pruebas.

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
