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
ajuste-seguridad-gestion_remota-activa-recomendado = Recommended if your privacy and security come first. Pros: the operator can no longer change the settings remotely and a known way of attack is closed. Cons: you will stop getting the operator's automatic updates, its support will not be able to check the router remotely, and changes it makes to your line, including the phone, will not reach the router.

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
ajuste-seguridad-gestion_remota_oculto-ayuda = The operator sets these; the passwords are hidden, with an eye button to show them. If you type one wrong, the operator can no longer manage the router until you undo the change. Only change them if the operator asks you to.
ajuste-seguridad-gestion_remota_oculto-servidor = Operator's server address
ajuste-seguridad-gestion_remota_oculto-servidor-desc = The web address the router contacts, for example https://gestion.example. No spaces or accented letters.
ajuste-seguridad-gestion_remota_oculto-usuario = User name with the operator
ajuste-seguridad-gestion_remota_oculto-usuario-desc = The name the router gives to the operator's server.
ajuste-seguridad-gestion_remota_oculto-contrasena = Password with the operator
ajuste-seguridad-gestion_remota_oculto-contrasena-desc = The password the router gives to the operator's server.
ajuste-seguridad-gestion_remota_oculto-usuario_entrada = User name for the operator's calls
ajuste-seguridad-gestion_remota_oculto-usuario_entrada-desc = The name the operator has to give when it is the one calling the router.
ajuste-seguridad-gestion_remota_oculto-contrasena_entrada = Password for the operator's calls
ajuste-seguridad-gestion_remota_oculto-contrasena_entrada-desc = The password the operator has to give when it is the one calling the router.

## Router password
ajuste-seguridad-contrasena_router-titulo = Router password
ajuste-seguridad-contrasena_router-desc = The password used to get into the router's web pages, and the one this panel uses.
ajuste-seguridad-contrasena_router-ayuda = If it is still the one on the router's sticker, anyone who sees it can get in: change it to one of your own. The panel tries the new one before keeping it and, if something goes wrong, puts the old one back.

## Router users
ajuste-seguridad-usuarios_router-titulo = Router web accounts
ajuste-seguridad-usuarios_router-desc = The router's web accounts and what each one can do.
ajuste-seguridad-usuarios_router-ayuda = On this router, the web pages only let the 1234 account in: the other accounts exist, but they cannot log in to the web. There is also an operator account that the router's web pages do not show: its technical support uses it to get into the router. That is why there may be one more account here than you see there. Neither that account nor the one this panel uses to get in can be deleted or changed from here.
ajuste-seguridad-usuarios_router-motivo = It is changed with each account's buttons, not here.
ajuste-seguridad-usuarios_router-nombre = Account name
ajuste-seguridad-usuarios_router-nombre-desc = The account's name in the router's web pages.
ajuste-seguridad-usuarios_router-papel = Account type
ajuste-seguridad-usuarios_router-papel-desc = What the account can do in the router's web pages.
ajuste-seguridad-usuarios_router-papel-operador = Operator
ajuste-seguridad-usuarios_router-papel-operador-desc = The operator's technical support account: it can change everything.
ajuste-seguridad-usuarios_router-papel-administrador = Administrator
ajuste-seguridad-usuarios_router-papel-administrador-desc = Can change all the settings the router's web pages show.
ajuste-seguridad-usuarios_router-papel-usuario = User
ajuste-seguridad-usuarios_router-papel-usuario-desc = An account with fewer permissions than the administrator one.
ajuste-seguridad-usuarios_router-crear = Create the account in the router
ajuste-seguridad-usuarios_router-crear-titulo = New account in the router
ajuste-seguridad-usuarios_router-nuevo-nombre = Name
ajuste-seguridad-usuarios_router-nuevo-nombre-ayuda = 4 to 20 unaccented letters, digits or "_", with no spaces.
ajuste-seguridad-usuarios_router-contrasena = Password
ajuste-seguridad-usuarios_router-contrasena-ayuda = { $min } to { $max } characters: unaccented letters, digits, spaces and the symbols on an English keyboard, except "`". No space at the start or the end, and never two in a row. Press the eye to check what you have typed.
ajuste-seguridad-usuarios_router-cambiar = Change password or group
ajuste-seguridad-usuarios_router-cambiar-de = Change password or group of { $nombre }
ajuste-seguridad-usuarios_router-cambiar-boton = Save the changes
ajuste-seguridad-usuarios_router-cambiar-nota = The router's web pages ask for a new password even to change only the group. The panel checks that the account gets in with it; the previous one cannot be recovered, so this change cannot be undone.
ajuste-seguridad-usuarios_router-cambiado = Done: { $nombre } now gets in with the new password.
ajuste-seguridad-usuarios_router-borrar = Delete
ajuste-seguridad-usuarios_router-borrar-de = Delete { $nombre }
ajuste-seguridad-usuarios_router-vacia = The router has no users in its web pages.

## Firewall (phase 3): the rules, read only, and the panel's own rule so
## the router does not answer ping from the internet.

ajuste-seguridad-cortafuegos-titulo = Firewall
ajuste-seguridad-cortafuegos-desc = The rules that decide what can reach the router from the internet.
ajuste-seguridad-cortafuegos-ayuda = The router comes with the operator's rules, which decide what gets in from the internet and from where; the list of rules shows them. The panel does not change them.
ajuste-seguridad-cortafuegos-motivo = The operator's rules are not changed from the panel.
ajuste-seguridad-cortafuegos-filtro = Filter
ajuste-seguridad-cortafuegos-filtro-desc = The group of rules it belongs to, by connection and address type.
ajuste-seguridad-cortafuegos-version = Addresses
ajuste-seguridad-cortafuegos-version-desc = Whether the rule applies to IPv4 or to IPv6 addresses.
ajuste-seguridad-cortafuegos-version-ipv4 = IPv4
ajuste-seguridad-cortafuegos-version-ipv4-desc = The usual addresses, such as 192.0.2.1.
ajuste-seguridad-cortafuegos-version-ipv6 = IPv6
ajuste-seguridad-cortafuegos-version-ipv6-desc = The newer, longer addresses.
ajuste-seguridad-cortafuegos-regla_activa = Active
ajuste-seguridad-cortafuegos-regla_activa-desc = Whether the router applies the rule.
ajuste-seguridad-cortafuegos-protocolo = Traffic
ajuste-seguridad-cortafuegos-protocolo-desc = The kind of traffic the rule applies to.
ajuste-seguridad-cortafuegos-protocolo-icmp = ICMP
ajuste-seguridad-cortafuegos-protocolo-icmp-desc = Control messages, such as ping.
ajuste-seguridad-cortafuegos-protocolo-tcp = TCP
ajuste-seguridad-cortafuegos-protocolo-tcp-desc = Connections such as web or SSH ones.
ajuste-seguridad-cortafuegos-protocolo-udp = UDP
ajuste-seguridad-cortafuegos-protocolo-udp-desc = Single messages, such as DNS ones.
ajuste-seguridad-cortafuegos-accion = What it does
ajuste-seguridad-cortafuegos-accion-desc = What the router does with traffic that matches the rule.
ajuste-seguridad-cortafuegos-accion-descartar = Drops it
ajuste-seguridad-cortafuegos-accion-descartar-desc = No answer: to the caller, the router is not there.
ajuste-seguridad-cortafuegos-accion-rechazar = Rejects it
ajuste-seguridad-cortafuegos-accion-rechazar-desc = It answers no.
ajuste-seguridad-cortafuegos-accion-permitir = Lets it through
ajuste-seguridad-cortafuegos-accion-permitir-desc = It lets it in.
ajuste-seguridad-cortafuegos-icmp = Control message
ajuste-seguridad-cortafuegos-icmp-desc = Which control messages the rule covers.
ajuste-seguridad-cortafuegos-icmp-ninguno = —
ajuste-seguridad-cortafuegos-icmp-ninguno-desc = The rule is not about control messages.
ajuste-seguridad-cortafuegos-icmp-cualquiera = Any
ajuste-seguridad-cortafuegos-icmp-cualquiera-desc = Every control message, ping included.
ajuste-seguridad-cortafuegos-icmp-eco = Ping (IPv4)
ajuste-seguridad-cortafuegos-icmp-eco-desc = The IPv4 ping request.
ajuste-seguridad-cortafuegos-icmp-eco_v6 = Ping (IPv6)
ajuste-seguridad-cortafuegos-icmp-eco_v6-desc = The IPv6 ping request.
ajuste-seguridad-cortafuegos-origen = From
ajuste-seguridad-cortafuegos-origen-desc = Where the traffic may come from; 0.0.0.0 means anywhere.
ajuste-seguridad-cortafuegos-origen_mascara = Source mask
ajuste-seguridad-cortafuegos-origen_mascara-desc = How many addresses the source covers.
ajuste-seguridad-cortafuegos-puerto_desde = Port from
ajuste-seguridad-cortafuegos-puerto_desde-desc = The first router port the traffic goes to; 0 means any.
ajuste-seguridad-cortafuegos-puerto_hasta = Port to
ajuste-seguridad-cortafuegos-puerto_hasta-desc = The last router port the traffic goes to.
ajuste-seguridad-cortafuegos-sin-ping = Do not answer ping from the internet
ajuste-seguridad-cortafuegos-sin-ping-ayuda = Adds a panel rule that drops ping coming from the internet over IPv4 to the router's connection; ping over IPv6 is not affected. The operator's rules are not touched, and "Undo" removes only this one.
ajuste-seguridad-cortafuegos-desactivada = The panel's rule is on the router but switched off: it drops nothing. The panel does not switch it on; switch it on or delete it from the router's web interface.
ajuste-seguridad-cortafuegos-ya-puesta = The panel's rule is already in place. It is for IPv4 only: ping over IPv6 is not affected.
ajuste-seguridad-cortafuegos-sin-leer = The firewall rules cannot be read: without them, the panel does not add its own.
ajuste-seguridad-cortafuegos-sin-filtro = The firewall has no incoming group of rules for the internet connection: the panel does not know where to put its own.
ajuste-seguridad-cortafuegos-sin-ping-recomendado = Recommended if you want your router to be less noticeable on the internet. Pros: the rule drops ping coming over IPv4, so whoever scans the internet looking for devices has a harder time seeing that yours is on. Cons: you will not be able to check with a ping from outside that your connection works, and some operator tests might fail.
ajuste-seguridad-cortafuegos-activar-recomendado = Recommended if you do not need the router to answer ping from the internet: the panel's rule is already on the router, but switched off, and the panel does not switch it on; switch it on from the router's web interface. Pros: it would drop ping coming over IPv4, so whoever scans the internet looking for devices would have a harder time seeing that yours is on. Cons: you will not be able to check with a ping from outside that your connection works, and some operator tests might fail.
ajuste-seguridad-cortafuegos-revisar-recomendado = Recommended if you never reach the router from outside your home: the panel does not change those rules, but you can ask the operator to close what you do not use. Pros: fewer doors open from the internet. Cons: the operator may need them to give you support or keep the router up to date.
ajuste-seguridad-cortafuegos-servicio-ping = ping
ajuste-seguridad-cortafuegos-servicio-web = the router's web pages
ajuste-seguridad-cortafuegos-servicio-ssh = SSH
ajuste-seguridad-cortafuegos-servicio-gestion = the operator's remote management
ajuste-seguridad-cortafuegos-servicio-ping-ipv4 = ping over IPv4
ajuste-seguridad-cortafuegos-servicio-ping-ipv6 = ping over IPv6
ajuste-seguridad-cortafuegos-y = and
