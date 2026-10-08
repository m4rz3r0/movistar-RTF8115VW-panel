#!/bin/sh
# Regenera crates/hgu/fixtures a partir de las respuestas XML capturadas del
# router real, sustituyendo secretos, MAC, IP públicas, SSID y nombres de equipo.
#
# Uso: tools/sanear-fixtures.sh <dir_xml_bruto> [fichero_secretos]
#
# Cada etiqueta XML (y cada clave de parámetro en los nombres de fichero) debe
# figurar en crates/router-sim/fixtures-tags.txt con su clase; si aparece una
# nueva, la herramienta falla sin tocar las fixtures hasta que se decida.
#
# Si se indica fichero_secretos, se añaden a él (modo 600, una por línea,
# conservando las existentes) los valores reales retirados, para comprobar con
#   RTF_SECRETS_FILE=<fichero> cargo test -p router-sim fixtures_have_no_secrets
# Conviene añadirle a mano los secretos conocidos por otra vía que no salen en
# las capturas (contraseña del router, claves WiFi…). Ese fichero contiene
# secretos: no debe quedar dentro del repositorio.
set -eu

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
    echo "uso: $0 <dir_xml_bruto> [fichero_secretos]" >&2
    exit 2
fi

raiz=$(cd "$(dirname "$0")/.." && pwd)
bruto=$1

if [ $# -eq 2 ]; then
    umask 077
    exec cargo run --quiet --manifest-path "$raiz/Cargo.toml" -p router-sim --bin sanear-fixtures -- \
        "$bruto" "$raiz/crates/hgu/fixtures" --secrets "$2"
fi
exec cargo run --quiet --manifest-path "$raiz/Cargo.toml" -p router-sim --bin sanear-fixtures -- \
    "$bruto" "$raiz/crates/hgu/fixtures"
