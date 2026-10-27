# Instalación

Tres formas de instalar el panel. En todas, el panel escucha en HTTP sin
cifrar y se entra a través de un **proxy inverso con HTTPS** limitado a la
red local ([`docs/proxy-tls.md`](../docs/proxy-tls.md)), y la primera vez se
termina en el navegador con el asistente de instalación
([`docs/operacion.md`](../docs/operacion.md#primera-instalación-el-asistente)).

| Vía | Para quién | Guía |
|---|---|---|
| Linux con systemd | Debian 12, Ubuntu 22.04, Raspberry Pi OS de 64 bits o posterior (x86_64 y aarch64, glibc 2.35 o más). Usa las releases ya compiladas, con un actualizador opcional | [`linux/`](linux/README.md) |
| Docker o Podman (Compose) | cualquier equipo con contenedores; incluye un proxy Caddy de ejemplo y un perfil de prueba con el router simulado | [`docker/`](docker/README.md) |
| Jails de FreeBSD | un servidor FreeBSD con jails; compila desde el código en una jail aparte y se actualiza solo con cada etiqueta | [`freebsd-jail/`](freebsd-jail/README.md) |

Otros ficheros:

- [`config.example`](config.example): la configuración de la vía de FreeBSD
  (se copia a `config.local`, que git ignora).
- `lib/`: funciones de sh compartidas por los instaladores y actualizadores.
- `cross/`: compilar la versión de FreeBSD en otro equipo con Linux (el PC
  de compilación) y entregarla al buzón del servidor, que así no compila
  ([versiones precompiladas](freebsd-jail/README.md#versiones-precompiladas-el-pc-de-compilación)).
- `pruebas/`: pruebas de cada vía que no tocan ningún servidor
  (`plantillas.sh`, `linux.sh`, `docker.sh`, `release-local.sh`,
  `historial-reescrito.sh`, `precompiladas.sh`).
