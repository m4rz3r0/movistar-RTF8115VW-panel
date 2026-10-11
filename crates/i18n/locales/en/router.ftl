# Router: router settings. Its first page is the connection.
router-conexion = Router connection
router-conexion-sub = Address of the router's web interface, the username and password the panel logs in with, and its firmware. They are stored in the panel's database.

## Credentials
router-credenciales = Router credentials
router-texto = Address of the router's web interface and its configuration user. The saved password is never shown: to change anything, type the router password; the panel tries to log in before saving.
router-sin-configurar = The router is not configured yet.
router-url = Router address
router-usuario = Router username
router-contrasena = Router password
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
