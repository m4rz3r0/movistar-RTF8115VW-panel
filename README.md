# Panel del router RTF8115VW

Panel web para el router Askey RTF8115VW (Movistar/O2) de casa: estado y
gráficas (WAN, disponibilidad, carga, RAM, clientes), explorador de todas las
funciones de la API del router con cambios vigilados (se revierten solos si el
router deja de responder o se pierde internet) e historial con deshacer.
Rust: Leptos 0.8 + Axum, SQLite. Corre en servidor-casa detrás de Caddy en
`https://panel.home.arpa`, sólo desde la LAN.

## Estructura

| Ruta | Qué es |
|---|---|
| `crates/hgu` | cliente del router: sesión, lecturas y escrituras, un único mutex y 150 ms entre peticiones; capturas saneadas en `fixtures/` |
| `crates/catalogo` | catálogo revisado de funciones (`funciones.toml`: riesgo, bloqueadas, lecturas pareadas) y secretos que se enmascaran (`secretos.toml`) |
| `crates/panel` | servidor y web (SSR + hidratación), autenticación, sondeo, cambios vigilados, historial; CLI `panel user add|passwd` |
| `crates/router-sim` | router simulado a partir de las capturas, para pruebas y desarrollo; `sanear-fixtures` |
| `tools/gen-catalogo` | genera el borrador del catálogo y `docs/api.md` |
| `e2e/` | pruebas de navegador (Playwright) contra el simulador |
| `deploy/` | jails, servicio, pf y actualizador del servidor ([`deploy/README.md`](deploy/README.md)) |
| `docs/` | [`api.md`](docs/api.md) (API del router), [`operacion.md`](docs/operacion.md) (día a día), diseño en `superpowers/specs/` |

## Desarrollo

Rust estable con `wasm32-unknown-unknown` (`rust-toolchain.toml`) y
`cargo-leptos`.

```sh
cargo run -p router-sim                     # router simulado en 127.0.0.1:8081 (usuario/contraseña sim)
printf sim > /tmp/router.secret && chmod 600 /tmp/router.secret
printf 'contraseña-larga\n' | RP_DB=/tmp/panel.db cargo run -p panel --features ssr -- user add yo
RP_LISTEN=127.0.0.1:3000 RP_DB=/tmp/panel.db RP_ROUTER_URL=http://127.0.0.1:8081 \
RP_ROUTER_USER=sim RP_ROUTER_SECRET=/tmp/router.secret RP_HISTORY_KEY=/tmp/history.key \
RP_TRUSTED_PROXY=127.0.0.1 RP_ORIGIN=http://localhost:3000 cargo leptos watch
```

Entra por `http://localhost:3000` (no `127.0.0.1`: la cookie
`__Host-rp_session` es `Secure` y el navegador sólo la acepta por http en
`localhost`). Las variables `RP_*` están en `crates/panel/src/config.rs`.

## Comprobaciones

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p panel --all-targets --features ssr -- -D warnings
cargo clippy -p panel --lib --target wasm32-unknown-unknown --no-default-features --features hydrate -- -D warnings
cargo test --workspace
cargo test --workspace --features panel/ssr
cargo audit          # excepciones justificadas en .cargo/audit.toml
cargo deny check     # deny.toml

# Navegador: sin desbordamiento horizontal a 360 px, capturas en e2e/capturas/
cargo leptos build && cargo build -p router-sim
cd e2e && npm ci && npx playwright test
```

El e2e arranca su propio simulador y panel en los puertos 8195 y 3195
(`E2E_SIM_PORT`, `E2E_PORT`) y usa el Chromium de Playwright
(`npx playwright install chromium` si no está; o `E2E_CHROMIUM=<ruta>`).

Las capturas del router se sanean con `tools/sanear-fixtures.sh`; ningún valor
real (MAC, IP pública, claves, nombres) debe acabar en el repositorio.

## Despliegue y operación

- Instalación en el servidor: [`deploy/README.md`](deploy/README.md).
- Versiones: una etiqueta `v*` en Forgejo; el actualizador del servidor la
  compila, prueba y despliega en menos de 15 minutos, y vuelve atrás solo si
  no responde.
- Usuarios, contraseña del router, registros, vuelta atrás a mano y pf:
  [`docs/operacion.md`](docs/operacion.md).
