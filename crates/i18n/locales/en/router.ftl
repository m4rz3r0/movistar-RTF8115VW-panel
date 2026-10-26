# Router: router settings. Its first page is the connection.
router-conexion = Router connection
router-conexion-sub = Address of the router's web interface, the username and password the panel logs in with, and its firmware; the router's own password is also changed here. They are stored in the panel's database.

## Credentials
router-credenciales = Password the panel uses to log in to the router
router-texto = The address, username and password the panel uses to log in to the router. Saving them here changes nothing on the router: it is for when the password was changed from the router's own web interface. The panel tries to log in before saving.
router-sin-configurar = The router is not configured yet.
router-url = Router address
router-usuario = Router username
router-contrasena = Password the panel uses
router-contrasena-ayuda = The one the panel has saved. Press the eye to see it; if you leave it alone, the saved one stays.
router-probar = Test the connection
router-guardar = Save the credentials
router-error-escribe-contrasena = With another address or username, type the password too: the saved one is only used with the saved details.
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
contrasena-router-nueva-ayuda = { $min } to { $max } characters: unaccented letters, digits, spaces and the symbols on an English keyboard, except "`". No space at the start or the end, and never two in a row. Press the eye to check what you have typed.
contrasena-router-solo-lectura = It cannot be changed while the panel is read-only: first confirm the firmware in the "Firmware" card.
contrasena-router-cambiar = Change the password
contrasena-router-cambiando = Changing…
contrasena-router-cambiada = Router password changed: the panel now logs in with the new one. Use the new one too to log in to the router's web interface yourself.
contrasena-router-confirmar-titulo = Change the router password?
contrasena-router-confirmar-texto = The router will have the new password. From then on, the new one is used to log in to the router (also from its own web interface); the panel does so by itself.
contrasena-router-confirmar-nota = If the panel cannot log in with the new one, it puts the previous one back. It takes a few seconds: do not close the page.
contrasena-router-confirmar-boton = Change the password
# The keys contrasena-router-error-* and contrasena-router-historial-nota
# (and panel-solo-lectura and comun-error-interno, in their files) are
# stored in each installation's database, in the History: never rename or
# delete them (only change their text).
contrasena-router-error-corta = The new password is too short: it needs at least { $n } characters.
contrasena-router-error-larga = The new password is too long: { $n } characters at most.
contrasena-router-error-control = The new password cannot contain line breaks, tabs or other invisible characters.
contrasena-router-error-caracter = The new password can only contain unaccented letters, digits, spaces and the symbols on an English keyboard, except "`": no accented letters or other symbols.
contrasena-router-error-espacios = The new password cannot start or end with a space, or have two spaces in a row.
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

## Router web users (Settings → Security → Users)
# The keys usuarios-router-error-* and usuarios-router-historial-nota are
# stored in the History: never rename or delete them.
usuarios-router-contrasena = Password
usuarios-router-protegido-panel = It is the user this panel logs in with: it cannot be deleted or changed from here.
usuarios-router-protegido-sistema = It is the operator's account: it cannot be deleted or changed from here.
usuarios-router-sin-credenciales = The panel does not know which user it logs in to the router with: no account can be changed from here.
usuarios-router-aviso-borrar = If you undo the deletion later, the user comes back with a new password: the current one cannot be recovered. The new one will only be visible with the eye, in the change's details in the History. If the panel brings it back by itself (because the change fails), that password is not stored: the History will tell you to change it.
usuarios-router-aviso-deshacer = The user comes back with a new password: the previous one cannot be recovered. You will see it with the eye in this change's details, in the History.
usuarios-router-error-protegido = This user is protected: it is the one the panel uses to log in to the router, or the operator's account. It cannot be deleted or changed from here.
usuarios-router-error-existe = The router already has a user with that name.
usuarios-router-error-nombre = The name must have 4 to 20 unaccented letters, digits or "_", with no spaces.
usuarios-router-error-no-existe = That user is no longer on the router: read the list again.
usuarios-router-error-en-curso = Another change is in progress on the router: wait for it to finish.
usuarios-router-error-rechazado = The router did not accept the change: the user stays as it was.
usuarios-router-error-no-entra = The router answered, but the user cannot log in with the new password. Try again or change it from the router's web interface.
usuarios-router-error-grupo = The user now logs in with the new password, but the router does not show the account type asked for. Read the list again.
usuarios-router-historial-nota = The new password is not stored in the panel: this change cannot be undone from here.
usuarios-router-error-uno-cada-vez = Users are deleted one at a time.
usuarios-router-error-nombre-baja = That name cannot be deleted from the panel: it contains ":" or control characters.
usuarios-router-error-grupo-sistema = That account type is the operator's: choose Administrator or User.
usuarios-router-error-grupo-no-valido = Choose an account type: Administrator or User.
usuarios-router-nota-revertido = The deleted user was created again with a new password that the panel does not store: change it in Settings → Security → Router web accounts to be able to use it.

## Features still to be checked against a real router
validacion-real-pendiente-usuarios = Creating and deleting users has not been checked against a real router yet: it is not available for now.
validacion-real-pendiente-contrasena-otro = Changing another user's password has not been checked against a real router yet: it is not available for now.
validacion-real-pendiente-sin-ping = Not answering ping has not been checked against a real router yet: it is not available for now.
usuarios-router-no-panel = These are not this panel's users: those are in
usuarios-router-no-panel-enlace = Panel settings → Who can log in to this panel
