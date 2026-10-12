# First-install setup wizard (/instalar).
instalar-titulo = Set up the panel
instalar-pasos = Setup steps
instalar-paso-admin = Administrator
instalar-paso-router = Router
instalar-paso-lectura = Initial read
instalar-paso-fin = Done

## Administrator
instalar-admin-texto = Create the user you will sign in to the panel with.
instalar-token = Setup token
instalar-token-ayuda = It is in the service log and in the instalar.token file, next to the panel database. It only works once.
instalar-usuario = Panel username
instalar-contrasena = Password (at least { $n } characters)
instalar-repetir = Repeat the password
instalar-no-coinciden = The passwords do not match
instalar-crear-admin = Create the administrator

## Router
instalar-router-texto = The address of the router's web interface and its configuration username and password. The panel tries to sign in before saving them.
instalar-router-url = Router address
instalar-router-url-ejemplo = http://192.168.1.1
instalar-router-usuario = Router username
instalar-router-contrasena = Router password
instalar-router-usuario-ejemplo = 1234
instalar-router-url-sugerida = Suggested from this computer's network: check it.
instalar-probar = Test
instalar-guardar = Save and continue
instalar-detectado = { $modelo } with firmware { $firmware }
instalar-firmware-probado = Firmware tested with the panel
instalar-firmware-no-probado = Untested firmware: the panel stays read-only until you confirm it in Router settings → Router connection

## Initial read
instalar-lectura-texto = The panel now reads the router's whole configuration and stores it encrypted as the starting snapshot, so you can compare against it and go back to it later. It may take a minute.
instalar-leer = Read the configuration
instalar-leyendo = Reading…
instalar-leidas = { $n ->
    [one] Read { $n } router function.
   *[other] Read { $n } router functions.
}
instalar-fallidas = { $n ->
    [one] { $n } function could not be read: the service log says which.
   *[other] { $n } functions could not be read: the service log says which.
}

## Done
instalar-fin-texto = The panel is set up.
instalar-red = Router network detected:
instalar-red-sin-detectar = The router network could not be detected: you can try again in Router settings → Router connection.
instalar-ir-estado = Go to Status

## Errors
instalar-error-token = Invalid setup token
instalar-error-terminada = Setup is already done
instalar-error-red = The setup wizard only answers from the local network
instalar-error-usuario = Invalid username: 1 to 64 characters, no spaces
instalar-error-contrasena-corta = The password must be at least { $n } characters long
instalar-error-contrasena-larga = The password is too long: at most { $n } bytes
instalar-error-url = Invalid address: type something like http://192.168.1.1
instalar-error-faltan = The router username or password is missing
instalar-error-router = Cannot sign in to the router: { $error }
instalar-error-sin-router = Set up the router first
