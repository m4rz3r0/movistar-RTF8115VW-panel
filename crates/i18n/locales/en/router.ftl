# Router: router settings. Its first page is the connection.
router-conexion = Router connection
router-conexion-sub = Address of the router's web interface, the username and password the panel logs in with, and its firmware; the router's own password is also changed here. They are stored in the panel's database.

## Credentials
router-credenciales = Password the panel uses to log in to the router
router-texto = The address, username and password the panel uses to log in to the router. Saving them here changes nothing on the router: it is for when the password was changed from the router's own web interface. The panel tries to log in before saving.
router-sin-configurar = The router is not configured yet.
router-url = Router address
router-usuario = Router username
router-contrasena = Router password
router-contrasena-ayuda = The one the panel has saved. Press the eye to see it; if you leave it alone, the saved one stays.
router-probar = Test the connection
router-guardar = Save the credentials
router-guardadas = Credentials saved: the panel now uses the new ones.

## Firmware
router-firmware-titulo = Firmware
router-firmware = Firmware: { $firmware }
router-firmware-desconocido = Firmware: not read yet
router-modo-solo-lectura = Read-only: the firmware has not been tested with the panel
router-modo-escritura = Changes are allowed
router-confirmar-texto = If you know this firmware behaves like the tested one, confirm it to make changes. The change manager still verifies and reverts whatever fails.
router-confirmar = Confirm the firmware

## Router network
router-red-titulo = Router network
router-red-texto = What the panel has detected about the router. The internet connection is the one the change safety net watches: if it goes down while a change is being watched, the change is undone.
router-red-sin-detectar = The router network has not been detected yet.
router-red-internet = Internet: { $conexion }
router-red-internet-tipo = Internet: { $conexion } ({ $tipo })
router-red-internet-sin-detectar = Internet: not detected
router-red-lan = LAN { $subred }
router-red-lan-sin-detectar = LAN: not detected
router-red-telefonia-si = Phone: yes
router-red-telefonia-no = Phone: no
router-red-television-si = TV: yes
router-red-television-no = TV: no
router-red-tipo-pppoe = PPPoE
router-red-tipo-dhcp = DHCP
router-red-tipo-estatica = static IP
router-red-tipo-otro = other type
router-red-conexion = Internet connection
router-red-conexion-ayuda = Usually the detected one. Pick another only if the panel gets it wrong.
router-red-automatica = Automatic: the detected one ({ $conexion })
router-red-automatica-sin-detectar = Automatic: the detected one (none yet)
router-red-manual-no-existe = The chosen connection, { $conexion }, does not exist on the router: the detected one is used meanwhile.
router-red-guardar = Save the connection
router-red-guardada = Internet connection saved.
router-red-redetectar = Detect again
router-red-redetectada = Router network detected again.
router-error-cambio-en-curso = A change is in progress on the router: wait for it to finish before changing the internet connection or detecting it again.
router-error-conexion-no-existe = The router has no connection named { $nombre }.

## Change the router password
contrasena-router-titulo = Change the router password
contrasena-router-texto = Changes the password of the router's web interface. From then on, the panel logs in with the new one and you will need it too to log in to the router. The panel tries the new one before keeping it and, if anything fails, puts the previous one back.
contrasena-router-nueva = New password
contrasena-router-nueva-ayuda = Between { $min } and { $max } characters; accented letters and ñ count as two. Press the eye to check what you have typed.
contrasena-router-cambiar = Change
contrasena-router-cambiando = Changing…
contrasena-router-cambiada = Router password changed: the panel now logs in with the new one. Use the new one too to log in to the router's web interface yourself.
contrasena-router-confirmar-titulo = Change the router password?
contrasena-router-confirmar-texto = The router will have the new password. From then on, the panel will use it to log in, and the router's web interface will take it too.
contrasena-router-confirmar-nota = If the panel cannot log in with the new one, it puts the previous one back. It takes a few seconds: do not close the page.
contrasena-router-confirmar-boton = Change the password
contrasena-router-error-corta = The new password is too short: it needs at least { $n } characters.
contrasena-router-error-larga = The new password is too long: { $n } characters at most (accented letters and ñ count as two).
contrasena-router-error-control = The new password cannot contain line breaks, tabs or other invisible characters.
contrasena-router-error-igual = The new password is the same one the router already has.
contrasena-router-error-en-curso = Another change is in progress on the router: wait for it to finish before changing the password.
contrasena-router-error-sin-router = The panel does not know how to log in to the router yet: save its address, username and password first.
contrasena-router-error-sondeo = The panel could not pause its router readings in time and has changed nothing. Try again in a moment.
contrasena-router-error-rechazada = The router did not accept the new password: it keeps the previous one, and so does the panel.
contrasena-router-error-restaurada = The panel could not check that the new password works, so it has put the previous one back on the router. Nothing has changed.
contrasena-router-error-no-guardada = The router accepted the new password, but the panel could not save it, so it has put the previous one back. Nothing has changed.
contrasena-router-error-grave = The panel cannot log in to the router with either the new password or the previous one. To get access back, log in to the router's web interface (try the new one and, if that fails, the previous one) and set by hand the password the panel has saved: press the eye in "Password the panel uses to log in to the router", on this same page, to see it.
contrasena-router-historial-nota = This change cannot be undone from here: to set another password, change it again in Router settings.
contrasena-router-enlace = Change the router password
contrasena-router-enlace-texto = It is changed in Router settings, next to the password the panel logs in with.
