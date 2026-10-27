# RTF8115VW router panel

*[Español](README.md)*

A web panel for the Askey RTF8115VW router that Movistar and O2 install in
Spain. It runs on a machine on your network (a server, a Raspberry Pi or a
container) and talks to the router's web interface.

- **Status**: internet connection, uptime, traffic, load, RAM and clients,
  with 1 h, 24 h and 7-day graphs, plus warnings such as TR-069 remote
  management or WPS being on.
- **Settings**: the router's settings, grouped by topic (Internet, WiFi,
  Local network, Ports, Security, Phone and System) and explained in plain
  language. Each one says what it does, what happens when you change it and
  which value is recommended.
- **Improvements**: two bundles, "Recommended security" and "Maximum
  privacy", reviewed and applied in one go.
- **Library**: every function of the router's API, with its description.
- **History** of everything changed, with undo.
- Its own users, Spanish and English, light or dark theme, and comfortable
  on a phone.

This is an independent project, not affiliated with Telefónica, Movistar, O2
or Askey. Read the [legal notice](AVISO-LEGAL.md) (in Spanish) before using
it.

## Screenshots

| Status | On a phone | Router connection |
|---|---|---|
| ![Router status on the desktop](docs/capturas/estado-escritorio.png) | ![Router status on a phone](docs/capturas/estado-movil.png) | ![Router connection settings](docs/capturas/configuracion-escritorio.png) |

| Settings: WiFi | Settings: Ports, light theme | Improvements |
|---|---|---|
| ![WiFi settings as cards](docs/capturas/ajustes-wifi-escritorio.png) | ![Port rules in a list with columns](docs/capturas/ajustes-puertos-claro.png) | ![The Recommended security and Maximum privacy bundles, with their steps](docs/capturas/mejoras-escritorio.png) |

They were made with the simulated router and made-up data.

## Tested router and firmware

| Model | Firmware |
|---|---|
| Askey RTF8115VW | `ES_g14.17_RTF_TEF001_V9.17_V034` |

With any other firmware, the panel starts **read-only**: it shows the status
and reads the settings, but changes nothing. If everything looks right,
confirm it in "Router settings → Router connection" and open an issue with
the model, the firmware and what you tested, so it can be added to the list.

## How it changes things

The panel only writes what has been checked on a real router. Everything
else is shown, with its value and recommendation, but without a switch. The
list is in [funciones-del-panel.md](docs/funciones-del-panel.md) (in
Spanish).

Before saving, a summary shows what changes ("before → after"). Changes that
can cut access to the router or to the internet are **watched**: if the
router stops answering or the internet goes down, they undo themselves. Any
change can be undone later from its notice or from the History, and "As it
was at install time" compares the router with the snapshot the setup wizard
took.

Passwords are masked, with an eye icon to reveal them.

There are two things this router's web interface does not let the panel do
properly:

- **Changing another account's password.** Only the `1234` account can log
  in to the web interface, so the panel cannot verify the new password.
  Instead it deletes the account and creates it again.
- **Stopping replies to ping from the internet.** Another operator rule keeps
  allowing it.

### Improvements

- **Recommended security.**
  - Turns off the operator's remote management (TR-069), WPS, UPnP, traffic
    mirroring and sending the log to another machine.
  - Sets every WiFi network to WPA2-AES and isolates the devices on the guest
    network.
- **Maximum privacy.**
  - Turns off remote management, mirroring and sending the log.
  - Hands out neutral DNS servers from the provider you choose, and can also
    take the time from a neutral server.

Each bundle says who it is for, what you gain and what you lose. It is
applied as a single change, watched and with a single undo. It only contains
steps the panel already writes; steps that touch something unchecked stay out
until they are checked.

Neutral DNS servers only change who answers. Queries still travel
unencrypted, so the operator can still see them. To hide them you need
encrypted DNS (DoH or DoT) on each device, or a VPN.

## Installation

There are three ways, each with its guide (in Spanish) in
[`deploy/`](deploy/README.md):

- **Linux with systemd** ([`deploy/linux/`](deploy/linux/README.md)). Uses
  the prebuilt releases for x86_64 and aarch64 (Debian 12, Ubuntu 22.04,
  64-bit Raspberry Pi OS or newer). Every release ships `SHA256SUMS`, and the
  optional updater checks it.
- **Podman or Docker with Compose** ([`deploy/docker/`](deploy/docker/README.md)).
  Includes the panel and an example Caddy proxy.
- **FreeBSD jails** ([`deploy/freebsd-jail/`](deploy/freebsd-jail/README.md)).
  Builds from source and updates itself on every tag.

The panel does not speak HTTPS. It needs an **HTTPS reverse proxy** in front
(Caddy, nginx…), limited to the local network, that passes the client's IP in
`X-Forwarded-For`; without HTTPS, the browser will not keep the session. See
[`docs/proxy-tls.md`](docs/proxy-tls.md).

The first run is finished in the browser, with the setup wizard. Day-to-day
operation (users, backups, logs and updates) is in
[`docs/operacion.md`](docs/operacion.md).

### Trying it without a router

With Podman or Docker, the `prueba` profile adds a simulated router
(`http://router-sim:8081`, user and password `sim`):

```sh
export RP_ORIGIN=https://panel.home.arpa PANEL_DOMINIO=panel.home.arpa
podman compose -f deploy/docker/compose.yaml --profile prueba up -d --build
```

## Configuration (`RP_*`)

The panel's environment variables (`crates/panel/src/config.rs`):

| Variable | Default | What it is |
|---|---|---|
| `RP_ORIGIN` | none: **required** | the panel's address as the browser writes it (`https://panel.home.arpa`); several, separated by commas |
| `RP_LISTEN` | `127.0.0.1:3000` | address and port to listen on (HTTP, behind the proxy) |
| `RP_TRUSTED_PROXY` | `127.0.0.1` | IPs or networks of the proxies trusted for `X-Forwarded-For`, separated by commas |
| `RP_DB` | Linux: `/var/lib/routerpanel/panel.db`; FreeBSD: `/var/db/routerpanel/panel.db` | SQLite database (users, encrypted credentials, history) |
| `RP_HISTORY_KEY` | Linux: `/etc/routerpanel/history.key`; FreeBSD: `/usr/local/etc/routerpanel/history.key` | the key that encrypts credentials and secrets; the panel creates it |
| `RP_INSTALL_NETS` | empty | networks, besides the local one, the setup wizard answers to |
| `RP_LOG` | `info` | log level (`tracing`'s `EnvFilter` syntax) |
| `RP_ROUTER_URL`, `RP_ROUTER_USER`, `RP_ROUTER_SECRET` | empty | only for installs older than the wizard: imported once into the database. All three or none |

If you start the binary by hand, Leptos also needs `LEPTOS_SITE_ROOT` (the
`site/` folder), `LEPTOS_SITE_PKG_DIR=pkg`, `LEPTOS_OUTPUT_NAME=panel` and
`LEPTOS_SITE_ADDR` (same as `RP_LISTEN`). The installation guides already
set them.

## Building

- Rust, at the version in [`rust-toolchain.toml`](rust-toolchain.toml).
  `rustup` installs it on its own, with the `wasm32-unknown-unknown` target.
- `cargo-leptos` 0.3.11: `cargo install --locked cargo-leptos --version 0.3.11`.
- `wasm-bindgen-cli`, at **the same version** as `wasm-bindgen` in
  `Cargo.lock`: `cargo install --locked wasm-bindgen-cli --version <version>`.
- `binaryen` (`wasm-opt`), from your system's package manager.

```sh
cargo leptos build --release   # target/release/panel and target/site/
```

## Development without a router

`router-sim` is a simulated router built from captured, sanitised responses
(`crates/hgu/fixtures`).

```sh
cargo run -p router-sim      # on 127.0.0.1:8081, user and password "sim"
mkdir -p /tmp/panel
RP_LISTEN=127.0.0.1:3000 RP_DB=/tmp/panel/panel.db RP_HISTORY_KEY=/tmp/panel/history.key \
RP_TRUSTED_PROXY=127.0.0.1 RP_ORIGIN=http://localhost:3000 cargo leptos watch
```

Open `http://localhost:3000`, not `127.0.0.1`. The session cookie is
`Secure`, and browsers only accept it without HTTPS on `localhost`. With no
users, the panel opens the setup wizard (`/instalar`). The token is in
`/tmp/panel/instalar.token` and the router is `http://127.0.0.1:8081`.

## Layout

| Path | What it is |
|---|---|
| `crates/hgu` | router client: session, reads and writes (one at a time, 150 ms apart) and network detection; sanitised captures in `fixtures/` |
| `crates/ajustes` | Settings catalogue (`ajustes/<topic>.toml`): fields, items and the router functions each setting reads and writes |
| `crates/catalogo` | reviewed function catalogue (`funciones.toml`) and masked secrets (`secretos.toml`) |
| `crates/panel` | server and web app (SSR and hydration), setup wizard, authentication, polling, watched changes, history and settings; the `panel user add` and `panel user passwd` commands |
| `crates/i18n` | interface strings in Spanish and English (Fluent) |
| `crates/router-sim` | simulated router for tests and development |
| `tools/gen-catalogo` | generates the catalogue draft and [`docs/api.md`](docs/api.md) |
| `tools/comprobar-publicacion` | checks that no personal or home-network data gets published |
| `e2e/` | browser tests (Playwright) against the simulator |
| `deploy/` | the installation methods |
| `docs/` | the router API, the panel's functions, day-to-day operation and the proxy |

## Checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p panel --all-targets --features ssr -- -D warnings
cargo clippy -p panel --lib --target wasm32-unknown-unknown --no-default-features --features hydrate -- -D warnings
cargo nextest run --workspace --features panel/ssr   # or cargo test, slower
cargo test --doc --workspace --features panel/ssr    # nextest does not run doctests
cargo audit && cargo deny check
cargo run -q -p comprobar-publicacion -- --arbol . --sin-locales
```

The browser tests use the debug binary from `cargo leptos build`. Since
`cargo test` with `--features panel/ssr` leaves a different binary in the
same place, `cargo leptos build` goes last:

```sh
cargo build -p router-sim
cargo build -p panel --features ssr --example e2e_grupo
cargo leptos build
cd e2e && npm ci && npx playwright install chromium && npx playwright test
```

`E2E_WORKERS` sets how many run in parallel (by default 1 or 2, depending on
the CPU). Each one starts its own panels and simulators on ports from 3195
and 8195 upwards. `npx playwright test capturas-readme` regenerates the
screenshots above. Review them by hand afterwards and put their new sha256 in
`tools/publicacion-permitidos.txt`.

## Contributing

Before your first `git push`, turn on the hook that checks what you push:

```sh
git config core.hooksPath tools/hooks
git config routerpanel.sinListas true   # unless you are the maintainer
```

The hook runs `comprobar-publicacion` on everything that goes out. It looks
for MACs, IPs and domains that are not documentation ones and, in the
maintainer's clone, for their local lists of personal data, which are not in
the repository. The second line turns those lists off in your clone.

Use documentation addresses in examples and tests (`192.0.2.x`,
`*.example`, `*.home.arpa`). Router captures are sanitised with
`tools/sanear-fixtures.sh`.

## Licence

[MIT](LICENSE). See also the [legal notice](AVISO-LEGAL.md).
