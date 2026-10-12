# Settings in the Security topic: the operator's remote management, the
# router password and the router's web users.

## Operator's remote management (TR-069)
ajuste-seguridad-gestion_remota-titulo = Operator's remote management
ajuste-seguridad-gestion_remota-desc = Lets your operator see and change the router's settings from a distance, without coming to your home.
ajuste-seguridad-gestion_remota-ayuda = While it is on, the operator can update the router, check it when you call support and change any setting, including the WiFi passwords, without asking you. The updates that fix security flaws also arrive through it. The other side is that the router keeps waiting for calls from the operator over the internet: that way in has been the entry point for known attacks against other operators' routers, although it is only a danger if the router has an unfixed flaw. If you turn it off, nobody outside can change the router, but it stops getting updates, support cannot check it remotely, the operator's app for managing the router may stop working and, if the operator changes something on your line, the change will not reach the router. What already works (internet, phone and TV) stays the same. The recommendation to turn it off is for people who put their privacy and security first; if you would rather the operator kept everything up to date, leave it on.
ajuste-seguridad-gestion_remota-activa = Remote management
ajuste-seguridad-gestion_remota-activa-desc = Whether the operator can get into the router remotely.
ajuste-seguridad-gestion_remota-activa-encendida = On
ajuste-seguridad-gestion_remota-activa-encendida-desc = You get the operator's updates and remote support; in exchange, it can change the router without asking you.
ajuste-seguridad-gestion_remota-activa-apagada = Off
ajuste-seguridad-gestion_remota-activa-apagada-desc = Nobody outside can change the router; in exchange, you stop getting the operator's updates and remote support.
ajuste-seguridad-gestion_remota-activa-recomendado = Recommended if your privacy and security come first. For: the operator can no longer change the settings remotely and a known way of attack is closed. Against: you will stop getting the operator's automatic updates, its support will not be able to check the router remotely, and changes it makes to your line, including the phone, will not reach the router.

## Remote management: regular check-in
ajuste-seguridad-gestion_remota_avanzado-titulo = Regular check-in with the operator
ajuste-seguridad-gestion_remota_avanzado-desc = How often the router contacts the operator on its own.
ajuste-seguridad-gestion_remota_avanzado-ayuda = Besides when the operator calls it, the router contacts the operator every so often to say it is still there and to ask for pending changes or updates. The operator sets this: unless you have a reason, leave it alone. It only matters while remote management is on.
ajuste-seguridad-gestion_remota_avanzado-aviso = Regular check-in
ajuste-seguridad-gestion_remota_avanzado-aviso-desc = If you turn it off, the router talks to the operator mainly when it starts up, when some of its settings change or when the operator calls it.
ajuste-seguridad-gestion_remota_avanzado-intervalo = How often
ajuste-seguridad-gestion_remota_avanzado-intervalo-desc = The time between one check-in and the next.
ajuste-seguridad-gestion_remota_avanzado-intervalo-hora = Every hour
ajuste-seguridad-gestion_remota_avanzado-intervalo-hora-desc = The operator soon knows if the router stops answering.
ajuste-seguridad-gestion_remota_avanzado-intervalo-dia = Every day
ajuste-seguridad-gestion_remota_avanzado-intervalo-dia-desc = One check-in a day.
ajuste-seguridad-gestion_remota_avanzado-intervalo-semana = Every week
ajuste-seguridad-gestion_remota_avanzado-intervalo-semana-desc = One check-in a week: the router talks to the operator less.

## Remote management: server and keys
ajuste-seguridad-gestion_remota_oculto-titulo = Remote management server and keys
ajuste-seguridad-gestion_remota_oculto-desc = The address of the operator's server and the keys the router and the operator use to recognise each other.
ajuste-seguridad-gestion_remota_oculto-ayuda = The operator sets these and they are never shown. If you type one wrong, the operator can no longer manage the router until you undo the change. Only change them if the operator asks you to.
ajuste-seguridad-gestion_remota_oculto-servidor = Operator's server address
ajuste-seguridad-gestion_remota_oculto-servidor-desc = The web address the router contacts, for example https://gestion.example. No spaces or accented letters.
ajuste-seguridad-gestion_remota_oculto-usuario = Username with the operator
ajuste-seguridad-gestion_remota_oculto-usuario-desc = The name the router gives to the operator's server.
ajuste-seguridad-gestion_remota_oculto-contrasena = Password with the operator
ajuste-seguridad-gestion_remota_oculto-contrasena-desc = The password the router gives to the operator's server.
ajuste-seguridad-gestion_remota_oculto-usuario_entrada = Username for the operator's calls
ajuste-seguridad-gestion_remota_oculto-usuario_entrada-desc = The name the operator has to give when it is the one calling the router.
ajuste-seguridad-gestion_remota_oculto-contrasena_entrada = Password for the operator's calls
ajuste-seguridad-gestion_remota_oculto-contrasena_entrada-desc = The password the operator has to give when it is the one calling the router.

## Router password
ajuste-seguridad-contrasena_router-titulo = Router password
ajuste-seguridad-contrasena_router-desc = The password used to get into the router's web pages, and the one this panel uses.
ajuste-seguridad-contrasena_router-ayuda = If it is still the one on the router's sticker, anyone who sees it can get in: change it to one of your own. The panel tries the new one before keeping it and, if something goes wrong, puts the old one back.

## Router users
ajuste-seguridad-usuarios_router-titulo = Router web users
ajuste-seguridad-usuarios_router-desc = The accounts that can get into the router's web pages and what each one can do.
ajuste-seguridad-usuarios_router-ayuda = There is an operator account that the router's web pages do not show: its technical support uses it to get into the router. That is why there may be one more account here than you see there. Account names are not shown, like the rest of the keys.
ajuste-seguridad-usuarios_router-motivo = For now you can only look at it. Changing another account's password could change the one this panel uses and lock it out; it will come once it has been checked against a real router.
ajuste-seguridad-usuarios_router-nombre = Account name
ajuste-seguridad-usuarios_router-nombre-desc = The name used to sign in; it is not shown.
ajuste-seguridad-usuarios_router-papel = Account type
ajuste-seguridad-usuarios_router-papel-desc = What the account can do in the router's web pages.
ajuste-seguridad-usuarios_router-papel-operador = Operator
ajuste-seguridad-usuarios_router-papel-operador-desc = The operator's technical support account: it can change everything.
ajuste-seguridad-usuarios_router-papel-administrador = Administrator
ajuste-seguridad-usuarios_router-papel-administrador-desc = Can change all the settings the router's web pages show.
ajuste-seguridad-usuarios_router-papel-usuario = User
ajuste-seguridad-usuarios_router-papel-usuario-desc = An account with fewer permissions than the administrator one.
