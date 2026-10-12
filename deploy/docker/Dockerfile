# Imagen del panel. Se construye desde la raíz del repositorio (el contexto
# es el repositorio entero; .dockerignore deja fuera target/, git y lo local):
#
#   docker build -f deploy/docker/Dockerfile -t routerpanel .
#
# o con deploy/docker/compose.yaml. Etapas:
#   herramientas  Rust, cargo-leptos, wasm-bindgen-cli (la versión de
#                 Cargo.lock) y binaryen (wasm-opt);
#   compilacion   cargo leptos build --release;
#   router-sim    el router simulado (solo para el perfil «prueba» de Compose);
#   panel         la imagen final (la de por defecto): debian:bookworm-slim,
#                 usuario sin privilegios y los datos en el volumen /data.

ARG RUST_VERSION=1.99.0

# --- herramientas -------------------------------------------------------------
FROM docker.io/library/rust:${RUST_VERSION}-bookworm AS herramientas
ARG RUST_VERSION
ARG CARGO_LEPTOS_VERSION=0.3.11
ARG BINARYEN_VERSION=123
# El rust-toolchain.toml del repositorio pide «stable», que pediría descargar
# otra; se usa la de la imagen, con el objetivo wasm.
ENV RUSTUP_TOOLCHAIN=${RUST_VERSION}
RUN rustup target add wasm32-unknown-unknown
RUN cargo install --locked cargo-leptos --version "${CARGO_LEPTOS_VERSION}"
# wasm-bindgen-cli tiene que ser exactamente la versión de wasm-bindgen de
# Cargo.lock (si no, cargo-leptos descargaría otra al compilar).
COPY Cargo.lock /tmp/Cargo.lock
RUN v=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/version = |"/, ""); print; exit }' /tmp/Cargo.lock) \
	&& test -n "$v" \
	&& cargo install --locked wasm-bindgen-cli --version "$v" \
	&& rm -rf /usr/local/cargo/registry
# binaryen de sus releases (el de Debian es demasiado antiguo para el wasm de
# Rust actual), comprobado con la suma que publica junto al archivo.
RUN set -eu; \
	case "$(uname -m)" in \
	x86_64) arq=x86_64 ;; \
	aarch64) arq=aarch64 ;; \
	*) echo "binaryen: arquitectura no soportada: $(uname -m)" >&2; exit 1 ;; \
	esac; \
	f="binaryen-version_${BINARYEN_VERSION}-${arq}-linux.tar.gz"; \
	url="https://github.com/WebAssembly/binaryen/releases/download/version_${BINARYEN_VERSION}"; \
	cd /tmp; \
	curl -fsSLO "$url/$f"; \
	curl -fsSLO "$url/$f.sha256"; \
	sha256sum -c "$f.sha256"; \
	mkdir -p /opt/binaryen; \
	tar -xzf "$f" -C /opt/binaryen --strip-components=1; \
	ln -s /opt/binaryen/bin/wasm-opt /usr/local/bin/wasm-opt; \
	rm -f "$f" "$f.sha256"; \
	wasm-opt --version

# --- compilacion --------------------------------------------------------------
FROM herramientas AS compilacion
WORKDIR /src
COPY . .
# Cachés de cargo entre compilaciones (BuildKit o buildah); lo compilado se
# copia fuera de la caché para la etapa siguiente.
RUN --mount=type=cache,id=routerpanel-cargo,target=/usr/local/cargo/registry \
	--mount=type=cache,id=routerpanel-target,target=/src/target \
	cargo leptos build --release \
	&& mkdir -p /out \
	&& cp target/release/panel /out/panel \
	&& cp -r target/site /out/site

FROM herramientas AS compilacion-sim
WORKDIR /src
COPY . .
RUN --mount=type=cache,id=routerpanel-cargo,target=/usr/local/cargo/registry \
	--mount=type=cache,id=routerpanel-target-sim,target=/src/target \
	cargo build --release -p router-sim \
	&& mkdir -p /out \
	&& cp target/release/router-sim /out/router-sim \
	&& cp -r crates/hgu/fixtures /out/fixtures

# --- router-sim (perfil «prueba») ---------------------------------------------
FROM docker.io/library/debian:bookworm-slim AS router-sim
RUN useradd --system --no-create-home --uid 10002 routersim
COPY --from=compilacion-sim /out/ /opt/router-sim/
USER routersim
EXPOSE 8081
ENTRYPOINT ["/opt/router-sim/router-sim", "--listen", "0.0.0.0:8081", "--fixtures", "/opt/router-sim/fixtures"]

# --- panel (imagen final) -----------------------------------------------------
FROM docker.io/library/debian:bookworm-slim AS panel
RUN useradd --system --no-create-home --home-dir /data --uid 10001 routerpanel \
	&& mkdir -m 700 /data \
	&& chown routerpanel:routerpanel /data
COPY --from=compilacion /out/ /opt/routerpanel/
# Datos: panel.db, history.key (la clave que cifra las credenciales del
# router y los secretos del historial) y, hasta crear el administrador,
# instalar.token. RP_LISTEN escucha en todas las interfaces del contenedor,
# es decir, en la red de Compose: no se publica ningún puerto del panel.
ENV RP_DB=/data/panel.db \
	RP_HISTORY_KEY=/data/history.key \
	RP_LISTEN=0.0.0.0:3000 \
	RP_LOG=info \
	LEPTOS_OUTPUT_NAME=panel \
	LEPTOS_SITE_ROOT=/opt/routerpanel/site \
	LEPTOS_SITE_PKG_DIR=pkg \
	LEPTOS_ENV=PROD
VOLUME /data
WORKDIR /data
USER routerpanel
EXPOSE 3000
ENTRYPOINT ["/opt/routerpanel/panel"]
