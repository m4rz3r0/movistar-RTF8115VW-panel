# HTTPS y proxy inverso

El panel escucha en HTTP sin cifrar (`RP_LISTEN`, por defecto
`127.0.0.1:3000`) y espera tener delante un proxy inverso que:

1. **sirva HTTPS.** La cookie de sesión (`__Host-rp_session`) es `Secure`: el
   navegador solo la guarda por HTTPS (o por `http://localhost`, para
   desarrollar). El panel manda además HSTS (`max-age` de un año, sin
   `includeSubDomains`): tras entrar una vez por HTTPS, el navegador ya no usa
   HTTP con ese nombre;
2. **deje entrar solo a la red local** (o a la VPN). El panel cambia la
   configuración del router: no lo publiques en internet;
3. **pase la IP del cliente en `X-Forwarded-For`**, poniendo la IP de la
   conexión y no la que traiga el cliente. El panel la usa para frenar los
   intentos de acceso de cada dispositivo y para que el asistente de
   instalación solo responda desde la red local.

En el panel:

- `RP_ORIGIN`: la dirección del panel tal como la escribe el navegador
  (`https://panel.home.arpa`, con `:puerto` si no es el 443). Varias,
  separadas por comas, si se entra por más de un nombre. El panel rechaza
  las peticiones que cambian algo y no vienen de su propia web: manda
  `Sec-Fetch-Site` si el navegador lo envía y, si no, `Origin` tiene que ser
  uno de ellos.
- `RP_TRUSTED_PROXY`: las IPs o redes de los proxies (separadas por comas;
  por defecto `127.0.0.1`). Solo se lee `X-Forwarded-For` cuando la conexión
  viene de una de ellas; si no, cuenta la IP de la conexión. Pon solo el
  proxy, nunca la red entera: cualquiera de ella podría decir ser otra IP.

Si el proxy y el panel están en equipos distintos, el panel tiene que
escuchar en una dirección a la que llegue el proxy (`RP_LISTEN=0.0.0.0:3000`
o la IP concreta), y conviene que el cortafuegos del equipo del panel solo
deje llegar al puerto desde el proxy (ver `deploy/linux/nftables-routerpanel.nft`
o las reglas de pf de `deploy/freebsd-jail/`).

En los ejemplos, el panel es `panel.home.arpa` y escucha en `127.0.0.1:3000`,
en el mismo equipo que el proxy. Haz que el nombre resuelva a la IP del
proxy en tu red (en el DNS de tu red o en el `hosts` de cada equipo).

## Caddy

```caddyfile
panel.home.arpa {
	# Una CA propia: el navegador avisará hasta que instales su certificado
	# raíz. Con un dominio público y DNS hacia fuera, quita esta línea y
	# Caddy pedirá un certificado de verdad.
	tls internal

	# Solo la red local: se mira la IP de la conexión, no X-Forwarded-For.
	@fuera not remote_ip private_ranges
	abort @fuera

	# Caddy pone X-Forwarded-For con la IP de la conexión (y no se fía del
	# que trae el cliente salvo que venga de sus trusted_proxies).
	reverse_proxy 127.0.0.1:3000
}
```

`deploy/docker/Caddyfile.example` es el mismo ejemplo, para Compose.

## nginx

```nginx
server {
    listen 443 ssl;
    http2 on;
    server_name panel.home.arpa;

    # Un certificado de tu CA o de una pública.
    ssl_certificate     /etc/nginx/tls/panel.home.arpa.crt;
    ssl_certificate_key /etc/nginx/tls/panel.home.arpa.key;

    # Solo la red local.
    allow 10.0.0.0/8;
    allow 172.16.0.0/12;
    allow 192.168.0.0/16;
    allow 127.0.0.1;
    deny all;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        # La IP de la conexión, sustituyendo la cabecera que traiga el
        # cliente (no $proxy_add_x_forwarded_for, que la conserva).
        proxy_set_header X-Forwarded-For $remote_addr;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## En el panel

Con cualquiera de los dos, en la misma máquina:

```sh
RP_LISTEN=127.0.0.1:3000
RP_ORIGIN=https://panel.home.arpa
RP_TRUSTED_PROXY=127.0.0.1
```

Con el proxy en otro equipo (p. ej. `192.0.2.20`):

```sh
RP_LISTEN=0.0.0.0:3000
RP_ORIGIN=https://panel.home.arpa
RP_TRUSTED_PROXY=192.0.2.20
```

Para comprobarlo, entra por `RP_ORIGIN`:

- si tras entrar vuelves a la página de acceso, el navegador no guarda la
  cookie: falta HTTPS;
- si cambiar algo da «Origen no permitido», el navegador no manda
  `Sec-Fetch-Site` y el nombre o el puerto no coinciden con `RP_ORIGIN`;
- si el asistente de instalación responde 403 desde la red local, el panel
  no ve la IP del cliente: revisa `RP_TRUSTED_PROXY` y `X-Forwarded-For`.
