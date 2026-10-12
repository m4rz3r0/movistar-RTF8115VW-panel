# Panel settings.
panel-solo-lectura = The panel is read-only: the router's firmware has not been tested with the panel. Confirm it in Router → Router connection to make changes.

## Page
panel-titulo = Panel settings
panel-sub = Language and time zone, privacy and instance, and panel users. They are stored in the panel's database.
panel-sub-router = The connection to the router is under
panel-sub-router-enlace = Router → Router connection
panel-guardado = Saved.

## Panel
panel-panel = Panel
panel-idioma = Default language
panel-idioma-navegador = The browser's
panel-idioma-ayuda = For anyone who hasn't picked a language with the menu selector, which is where you change yours.
panel-zona = Time zone
panel-zona-ayuda = Used for the panel's dates and charts. Peninsular Spain and the Canary Islands follow daylight saving time on their own; UTC and “Other” are a fixed offset.
panel-zona-peninsular = Peninsular Spain (Madrid time)
panel-zona-canarias = Canary Islands
panel-zona-utc = UTC
panel-zona-otra = Other (UTC±HH:MM)
panel-zona-desfase = Offset from UTC

## Privacy and instance
panel-privacidad = Privacy and instance
panel-privacidad-texto = Settings used by actions and recommendations. All of them are optional.
panel-dns = Preferred DNS
panel-dns-ayuda = Up to { $n } IPv4 or IPv6 addresses, separated by commas or spaces. Empty: the ISP's.
panel-dns-sugerencias = Public suggestions
panel-dns-usar = Use { $nombre }
panel-dns-politica = { $nombre } privacy policy
panel-dns-quad9 = Quad9
panel-dns-quad9-ips = 9.9.9.9
panel-dns-quad9-politica = https://quad9.net/privacy/policy/
panel-dns-mullvad = Mullvad
panel-dns-mullvad-ips = 194.242.2.2
panel-dns-mullvad-politica = https://mullvad.net/help/privacy-policy
panel-dns-cloudflare = Cloudflare
panel-dns-cloudflare-ips = 1.1.1.1
panel-dns-cloudflare-politica = https://developers.cloudflare.com/1.1.1.1/privacy/public-dns-resolver/
panel-confianza = Trusted devices
panel-confianza-ayuda = MAC addresses, separated by commas or spaces.
panel-confianza-equipos = Devices the router sees
panel-confianza-anadir = Add { $nombre }
panel-equipo-sin-nombre = (no name)
panel-mantenimiento = Maintenance window
panel-mantenimiento-ayuda = Like 03:00-05:00. Empty: no window.
panel-servidor = Own server
panel-servidor-ayuda = IP or name of a server on the network, if you have one. Empty: none.
panel-guardar = Save

## Users
panel-usuarios = Panel users
panel-usuarios-texto = They can log in to the panel with full permissions. They can also be managed with the panel user command.
panel-usuario-tu = (you)
panel-usuario-borrar = Delete
panel-usuario-borrar-de = Delete { $usuario }
panel-usuario-contrasena = Change password
panel-usuario-contrasena-de = Change { $usuario }'s password
panel-usuario-nuevo = New user
panel-usuario-nombre = Name
panel-usuario-clave = Password (at least { $n } characters)
panel-usuario-crear = Create the user
panel-usuario-clave-nueva = New password (at least { $n } characters)
panel-usuario-cambiar = Change
panel-cancelar = Cancel

## Reauthentication
panel-reautenticar = Confirm it's you
panel-reautenticar-texto = This needs the panel password. It won't be asked again for 5 minutes.
panel-reautenticar-contrasena = Panel password
panel-reautenticar-seguir = Continue

## Errors
panel-error-contrasena = Wrong panel password.
panel-error-cambio-en-curso = A change is in progress on the router: wait for it to finish before changing its credentials.
panel-error-dns = Not an IPv4 or IPv6 address: { $valor }
panel-error-demasiados-dns = At most { $n } DNS servers.
panel-error-mac = Not a MAC address: { $valor }
panel-error-mantenimiento = The maintenance window looks like 03:00-05:00.
panel-error-zona = Invalid time zone: pick one from the list.
panel-error-servidor = The own server is an IP or a name, without http:// or paths.
panel-error-usuario-existe = That user already exists.
panel-error-usuario-no-existe = That user does not exist.
panel-error-ultimo-usuario = The last panel user cannot be deleted.
panel-error-usuario-propio = You cannot delete your own user.
