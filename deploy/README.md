# Instalación

Hay tres formas de instalar el panel. En todas, el panel escucha por HTTP sin
cifrar y se entra a través de un **proxy inverso con HTTPS** limitado a la
red local ([`docs/proxy-tls.md`](../docs/proxy-tls.md)). La primera vez se
termina en el navegador, con el asistente de instalación
([`docs/operacion.md`](../docs/operacion.md#primera-instalación-el-asistente)).

| Forma | Para quién | Guía |
|---|---|---|
| Linux con systemd | Debian 12, Ubuntu 22.04, Raspberry Pi OS de 64 bits o posteriores (x86_64 y aarch64, glibc 2.35 o más). Usa las releases ya compiladas y tiene un actualizador opcional | [`linux/`](linux/README.md) |
| Podman o Docker (Compose) | cualquier equipo con contenedores. Incluye un proxy Caddy de ejemplo y un perfil de prueba con el router simulado | [`docker/`](docker/README.md) |
| Jails de FreeBSD | un servidor FreeBSD con jails. No compila: se actualiza solo con cada etiqueta, con la versión ya compilada de la release de GitHub o de un PC de compilación | [`freebsd-jail/`](freebsd-jail/README.md) |

Otros ficheros:

- [`config.example`](config.example): la configuración de la instalación en
  FreeBSD. Se copia a `config.local`, que git ignora.
- `lib/`: funciones de sh que comparten los instaladores y los actualizadores.
- `cross/`: compila la versión de FreeBSD en otro equipo con Linux y la
  entrega al buzón del servidor, que nunca compila
  ([el PC de compilación](freebsd-jail/README.md#el-pc-de-compilación-buzón)).
  `empaquetar-freebsd.sh` es también el que usa la release de GitHub.
- `pruebas/`: pruebas de cada forma de instalación que no tocan ningún
  servidor.
