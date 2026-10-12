# Panel settings.
panel-solo-lectura = The panel is read-only: the router's firmware has not been tested with the panel. Confirm it in Router settings → Router connection to make changes.

## Page
panel-titulo = Panel settings
panel-sub = Language, time zone and panel users. They are stored in the panel's database.
panel-sub-router = The connection to the router is under
panel-sub-router-enlace = Router settings
panel-sub-preferencias = and the parameters that presets and recommendations use, under
panel-sub-preferencias-enlace = Recommendations → Your preferences
panel-guardar = Save
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

## Display
panel-vista = Display
panel-vista-ayuda = How data with charts is shown, such as on Status. It's yours: each panel user chooses their own. You can also change it in the Status header.
panel-vista-modo = Display mode
panel-vista-texto = Text
panel-vista-texto-ayuda = No charts: indicators as a list of names and values, and the history as tables with the latest value, minimum, average and maximum. Handy with a screen reader or on small screens.
panel-vista-grafica = Charts
panel-vista-grafica-ayuda = History charts first and full width; indicators compact, with just their value.
panel-vista-mixto = Mixed
panel-vista-mixto-ayuda = Indicators with their last-hour sparkline and, below, the history charts. This is the default.

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
