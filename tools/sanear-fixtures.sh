#!/bin/sh
# Regenera crates/hgu/fixtures a partir de las respuestas XML capturadas del
# router real, sustituyendo secretos, MAC, IP públicas, SSID y nombres de equipo.
#
# Uso: tools/sanear-fixtures.sh <dir_xml_bruto> [fichero_secretos]
#
# Si se indica fichero_secretos, se escriben en él (modo 600) los valores reales
# retirados, uno por línea, para comprobar las fixtures con
#   RTF_SECRETS_FILE=<fichero> cargo test -p router-sim fixtures_have_no_secrets
# Ese fichero contiene secretos: no debe quedar dentro del repositorio.
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
