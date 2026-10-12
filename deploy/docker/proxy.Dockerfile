# Proxy inverso de ejemplo de compose.yaml: la imagen oficial con Caddyfile.example.
FROM docker.io/library/caddy:2.11.7-alpine
COPY Caddyfile.example /etc/caddy/Caddyfile
