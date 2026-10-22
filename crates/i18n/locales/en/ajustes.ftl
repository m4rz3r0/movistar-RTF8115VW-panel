# Router settings page: texts shared by every section, errors when reading
# or writing a value, levels and risks. The texts of each setting live in
# `ajustes-<section>.ftl`.

## Page
ajustes-titulo = Settings
ajustes-desc = Change what you need on the router in plain language. Every change is watched and, if the router stops responding, it is undone on its own.
ajustes-mas-opciones = More options
ajustes-mas-opciones-desc = Settings you will hardly ever need to touch.
ajustes-detalles-tecnicos = Technical details
ajustes-detalles-tecnicos-desc = For anyone who wants to know which router functions are used underneath.
ajustes-detalles-lectura = Read
ajustes-detalles-escrituras = Writes
ajustes-detalles-parametros = Parameters
ajustes-recomendado = Recommended
ajustes-recomendado-desc = The option we suggest for most homes.
ajustes-solo-lectura = Settings can only be viewed for now: the router firmware has not been tested with the panel. You can confirm it in Router configuration.
ajustes-valor-desconocido = Unrecognised value ({ $valor })
ajustes-valor-desconocido-desc = The router has a value the panel does not know. It is kept as it is until you choose another one.
ajustes-efecto = When applied
ajustes-temas = Settings topics
ajustes-tema-elegir = Topic
ajustes-actualizando = Updating…
ajustes-volver = Go to the Internet settings
ajustes-solo-lectura-aviso = Settings can only be viewed for now: the router firmware has not been tested with the panel yet.
ajustes-solo-lectura-enlace = Confirm it in Router settings
ajustes-que-hace = What does this do?
ajustes-editar = Edit
ajustes-guardar = Save
ajustes-cancelar = Cancel
ajustes-recomendado-valor = Recommended: { $valor }
ajustes-recomendado-pros = Pros and cons
ajustes-recomendado-marca = Pros:
ajustes-secreto-tapado-pedir = The router won't let this password be read: type it to save the changes.
ajustes-secreto-sin-leer-desc = The router won't let its current value be read; if you type one, the new one is saved.
ajustes-hueco = This setting will have its own screen here; it is not ready yet.
ajustes-lista-vacia = None set up.
ajustes-lista-sin-editar = For now this list can only be viewed.

## Sections
ajustes-tema-internet = Internet
ajustes-tema-internet-desc = How your home connects to the internet and which name servers it uses.
ajustes-tema-wifi = WiFi
ajustes-tema-wifi-desc = Your wireless networks: name, password, security and radios.
ajustes-tema-red_local = Home network
ajustes-tema-red_local-desc = The devices at home, their addresses and fixed IPs.
ajustes-tema-puertos = Ports
ajustes-tema-puertos-desc = What can reach the devices in your home from the internet.
ajustes-tema-seguridad = Security
ajustes-tema-seguridad-desc = Who can get into the router and the operator's remote management.
ajustes-tema-telefono = Phone
ajustes-tema-telefono-desc = The router's landline.
ajustes-tema-sistema = System
ajustes-tema-sistema-desc = The router clock and the traffic copy for diagnosis.

## Levels
ajustes-nivel-basico = Essentials
ajustes-nivel-basico-desc = Always in view.
ajustes-nivel-avanzado = Advanced
ajustes-nivel-avanzado-desc = Under "More options": for those who know what they are looking for.
ajustes-nivel-oculto = Hidden
ajustes-nivel-oculto-desc = Not shown in the operator's web interface.

## Risks
ajustes-riesgo-bajo = Low risk
ajustes-riesgo-bajo-desc = Does not cut the connection. It is watched for a few seconds after applying it.
ajustes-riesgo-medio = Medium risk
ajustes-riesgo-medio-desc = May briefly cut the WiFi or some service. It is watched for a couple of minutes and undone if something fails.
ajustes-riesgo-alto = High risk
ajustes-riesgo-alto-desc = May leave you without internet or without access to the router. It is watched for several minutes and undone if something fails.

## Errors
ajustes-error-tema = That settings section does not exist.
ajustes-error-lectura = This setting could not be read from the router. Try again in a moment.
ajustes-error-vacio = This field cannot be left empty.
ajustes-error-corto = Too short: it must take up at least as much as { $min } unaccented letters. Each accented letter takes up about two, "€" about three and an emoji about four.
ajustes-error-largo = Too long: there is room for { $max } unaccented letters. Each accented letter takes up about two, "€" about three and an emoji about four.
ajustes-error-fuera-de-rango = It must be a number between { $min } and { $max }.
ajustes-error-fuera-de-rango-desde = It must be a number equal to or greater than { $min }.
ajustes-error-fuera-de-rango-hasta = It must be a number equal to or less than { $max }.
ajustes-error-numero = It must be a whole number, without decimals.
ajustes-error-ip = Not a valid IP address (four numbers from 0 to 255 separated by dots).
ajustes-error-mac = Not a valid MAC address (six pairs of digits and letters separated by colons).
ajustes-error-opcion = Choose one of the options.
ajustes-error-control = It contains characters that cannot be typed, such as line breaks.
ajustes-error-ascii = Only letters without accents, digits and common symbols are accepted.
ajustes-error-solo-lectura = This value can only be viewed.
ajustes-error-sin-cambios = It's already like that: the router has those values and there is nothing to change.
ajustes-error-bloqueada = The panel cannot safely change this setting yet.
ajustes-error-router = The router could not make the change. Try again in a moment.
ajustes-error-invalida = The panel could not prepare this change. Reload the page and try again.
ajustes-error-ajuste = That setting does not exist.
ajustes-error-campo = That setting has no such value. Reload the page and try again.
cambio-error-secreto-tapado = This can't be saved: the same write carries a password that the router won't let be read or shows masked, and sending it back could erase it. Type it to save the changes.
cambio-sin-deshacer-tapado = This change can't be put back: the router won't let the previous value of “{ $campos }” be read. If needed, type it again in Settings.
cambio-aviso-tapado = “{ $campos }” stays as it was typed: the router won't let the previous value be read.
ajustes-error-cambio = That change was not made from Settings. You can undo it from the History.
ajustes-error-no-deshacer = That change can no longer be undone: only a change that is already applied (once the watch is over) and has not been undone before can be undone.
ajustes-error-requiere = You need to choose this for “{ $campo }”.

## Saving a setting: summary and confirmation
ajustes-grupo-revisar = Review the change
ajustes-grupo-deshacer-titulo = Put it back as it was
ajustes-grupo-intro = This is what will change on the router.
ajustes-grupo-ajuste-detalle = { $ajuste }: { $detalle }
ajustes-grupo-paso = Step { $n }
ajustes-grupo-pasos-desc = Untick a step if you don't want to apply it now.
ajustes-grupo-ya-esta = Already like that
ajustes-grupo-ya-esta-desc = The router already has these values: this step is not written.
ajustes-grupo-col-dato = Value
ajustes-grupo-col-antes = Now
ajustes-grupo-col-despues = After
ajustes-grupo-vacio = (empty)
ajustes-grupo-vigilancia = After applying it, the panel watches the router for { $plazo } and undoes it on its own if the router stops responding or the internet goes down.
ajustes-grupo-ninguno = Tick at least one step to apply the change.
ajustes-grupo-aplicar = Apply
ajustes-grupo-aplicando = Applying…
ajustes-grupo-cancelar = Cancel
ajustes-grupo-cerrar = Close
ajustes-grupo-deshacer = Undo
ajustes-grupo-preparando = Preparing…
ajustes-grupo-estado-aplicando = Applying the change and checking that the router has it…
ajustes-grupo-estado-vigilando = Applied. Watching the router: { $resto } left.
ajustes-grupo-estado-aplicado = Done: the router now has the change.
ajustes-grupo-estado-revertido = The router stopped responding properly and the change was undone automatically.
ajustes-grupo-estado-fallido = The change could not be applied. The router is as it was.
ajustes-grupo-estado-parcial = The change could not be applied and part of it did not go back to how it was. Check it in the History.

## How it was at installation (starting snapshot)
ajustes-foto-titulo = How it was at installation
ajustes-foto-desc = What has changed on the router since you installed the panel. If you don't like something, you can go back to the value it had then.
ajustes-foto-enlace = See what has changed since you installed the panel
ajustes-foto-tomada = Compared with how the router was on { $fecha }.
ajustes-foto-sin-foto = There is nothing to compare with: this panel was installed before the setup assistant saved how the router was. Whatever you change from the panel is kept in the History.
ajustes-foto-sin-diferencias = Everything is as it was when the panel was installed.
ajustes-foto-ir-historial = See the History
ajustes-foto-ir-ajustes = Back to the settings
ajustes-foto-ver-tema = See in { $tema }
ajustes-foto-al-instalar = At installation
ajustes-foto-ahora = Now
ajustes-foto-volver = Go back to this value
ajustes-foto-alta = Added after installation
ajustes-foto-baja = It was there at installation and is gone now
ajustes-foto-lista-nota = Changes to lists are only shown here: to undo them, go to their section.
ajustes-foto-sin-dato = Not saved at installation
ajustes-foto-sin-dato-desc = How these settings were was not saved when the panel was installed, so they cannot be compared. This is normal if the panel was installed with an earlier version, which saved less; it does not affect the router or your changes.
ajustes-foto-sin-leer = These settings could not be read from the router just now; try again in a moment:
ajustes-foto-error-leer = The snapshot of how the router was could not be read. Try again in a moment.
ajustes-foto-error-cifrado = How the router was at installation cannot be read: the panel no longer has the key it was saved with. This happens if the key the panel uses to protect its data has been changed.
ajustes-foto-error-sin-foto = There is no snapshot of how the router was when the panel was installed.
ajustes-foto-error-sin-dato = How this setting was at installation was not saved.
ajustes-foto-error-lista = Lists cannot be put back as they were from here: change them in their section.
ajustes-foto-error-desconocido = At installation, the router had a value here that the panel does not recognise, so it cannot put it back.
ajustes-foto-error-secreto = At installation, the router did not let this key be read, so the panel cannot put it back.

## Whether each network broadcasts: badge on a network's card and note on a radio's
ajustes-wifi-red-encendida = On
ajustes-wifi-red-apagada = Off
ajustes-wifi-red-sin-radio = Not broadcasting: the radio is off
ajustes-wifi-radio-sin-redes = The radio is on, but none of its networks is broadcasting: there is no WiFi on this band.
ajustes-wifi-red-no-emite = On, but not broadcasting
ajustes-wifi-radio-ocultas = { $n ->
    [one] The radio broadcasts one network that the panel does not show.
   *[other] The radio broadcasts { $n } networks that the panel does not show.
}
ajustes-wifi-radio-ocultas-enlace = View in the Library
ajustes-wifi-radio-apagada = Off
ajustes-wifi-radio-sin-redes-corto = On, no networks
ajustes-wifi-radio-redes-sin-leer-corto = On, networks not read
ajustes-wifi-radio-redes-sin-leer = The radio is on, but some of its networks could not be read: it is not known whether it broadcasts any.

## Elements of each tab: state, role, data, recommendation and actions
ajustes-estado-sin-leer = Not read
ajustes-estado-desconocido = No data
ajustes-papel-principal = Main
ajustes-papel-invitados = Guests
ajustes-papel-radio = Radio
ajustes-dato-canal_y_ancho = Channel and width
ajustes-dato-canal-auto = Automatic channel
ajustes-dato-canal-numero = Channel { $canal }
ajustes-dato-canal-y-ancho = { $canal } · { $ancho } MHz
ajustes-dato-conectados = Connected devices
ajustes-recomendacion-aplicar = Apply the recommendation
ajustes-accion-encender = Turn on
ajustes-accion-apagar = Turn off
ajustes-accion-quitar_wps = Turn off WPS
ajustes-accion-ocultar_nombre = Hide the name
ajustes-accion-mostrar_nombre = Show the name
ajustes-accion-aislar = Isolate the devices
ajuste-accion-encender-titulo = WiFi networks, turn on
ajuste-accion-apagar-titulo = WiFi networks, turn off
ajuste-accion-quitar_wps-titulo = WiFi networks, turn off WPS
ajuste-accion-ocultar_nombre-titulo = WiFi networks, hide the name
ajuste-accion-mostrar_nombre-titulo = WiFi networks, show the name
ajuste-accion-aislar-titulo = WiFi networks, isolate the devices
ajustes-accion-activar_regla = Turn on
ajustes-accion-desactivar_regla = Turn off
ajustes-accion-borrar_regla = Delete
ajuste-accion-activar_regla-titulo = Open ports, turn rules on
ajuste-accion-desactivar_regla-titulo = Open ports, turn rules off
ajuste-accion-borrar_regla-titulo = Open ports, delete rules
ajustes-accion-activar_reserva = Turn on
ajustes-accion-desactivar_reserva = Turn off
ajustes-accion-borrar_reserva = Delete
ajuste-accion-activar_reserva-titulo = Fixed IPs, turn on
ajuste-accion-desactivar_reserva-titulo = Fixed IPs, turn off
ajuste-accion-borrar_reserva-titulo = Fixed IPs, delete
ajustes-accion-demasiados = At most { $max } at a time: unmark some.
ajustes-accion-no-aplica = That action can't be applied to everything marked.
ajustes-accion-ya-estaba = All the marked ones were already like that: there is nothing to change.
ajustes-accion-tapado = This action does not work for “{ $ajuste }”: saving it means typing a password the router won't let be read. Change it from its details.
ajustes-recomendacion-ahora = Now: { $campo }, { $valor }.
ajustes-problema-wifi-wps-true = WPS is on: anyone who gets to the router can connect by pressing its button, without the password.
ajustes-problema-wifi-activa-false = The network is off: no device can connect to it.
ajustes-problema-wifi-activa-true = The network is on: if you have no visitors right now, it is one more network visible from outside.
ajustes-problema-wifi-aislamiento-false = Devices on this network can see each other: visitors would see yours.
ajustes-problema-wifi-aislamiento-true = Devices on this network can't see each other: you won't be able to, for example, cast to the TV.
ajustes-problema-wifi-visible-false = The network name is hidden: it has to be typed by hand to connect.
ajustes-problema-wifi-seguridad-abierta = The network is open: anyone nearby can connect without a password.
ajustes-leido-hace = { $s ->
    [0] Read just now
    [one] Read 1 s ago
   *[other] Read { $s } s ago
}
ajustes-leido-hace-min = Read { $min } min ago
ajustes-apagada-desde = Off since { $fecha }
ajustes-elemento-no-existe = There is nothing called “{ $id }” in this tab. Pick one of its cards.

## Grid of elements: mark, bar, card, details and change notice
ajustes-resumen = Summary of the section
ajustes-marcar = Mark { $nombre }
ajustes-barra = Actions for the marked items
ajustes-marcadas = { $n ->
    [one] 1 marked
   *[other] { $n } marked
}
ajustes-desmarcar-todo = Unmark all
ajustes-barra-sin-leer = Left out because they could not be read: { $nombres }.
ajustes-barra-ninguna = No action works for everything marked.
ajustes-insignia-aplicando = Applying…
ajustes-insignia-comprobando = Checking…
ajustes-interruptor-si = { $nombre } on
ajustes-interruptor-no = { $nombre } off
ajustes-abrir = Open
ajustes-abrir-nombre = Open { $nombre }
ajustes-volver-a-leer = Read again
ajustes-detalle-cerrar = Close the details
ajustes-detalle-volver = ‹ { $tema }
ajustes-detalle-basico = The essentials
ajustes-descartar = Discard
ajustes-guardar-cambios = Save changes
ajustes-sin-guardar-titulo = You have unsaved changes
ajustes-sin-guardar-texto = If you go on, the changes to “{ $nombre }” are lost.
ajustes-sin-guardar-seguir = Keep editing
ajustes-sin-guardar-descartar = Discard the changes
ajustes-aviso-cambio = { $titulo }: { $campo }, { $antes } → { $despues }
ajustes-aviso-varios = { $titulo }: { $n ->
    [one] one change
   *[other] { $n } changes
}
ajustes-aviso-cerrar = Close the notice
ajustes-aviso-conectados = { $n ->
    [one] The connected device loses its WiFi.
   *[other] The { $n } connected devices lose their WiFi.
}

## WiFi summary strip
ajustes-resumen-de = { $n } of { $total }
ajustes-resumen-sin-dato = —
ajustes-resumen-wifi-redes = Networks on
ajustes-resumen-wifi-redes-detalle = The ones broadcasting right now.
ajustes-resumen-wifi-radios = Radios on
ajustes-resumen-wifi-radios-detalle = The 2.4 and 5 GHz bands.
ajustes-resumen-wifi-conectados = Connected devices
ajustes-resumen-wifi-conectados-detalle = Across all the networks in the panel.
ajustes-resumen-recomendaciones = Pending recommendations
ajustes-resumen-recomendaciones-detalle = Each card shows the most important one.

## Internet, Security, Phone and System in a grid: card states
ajustes-estado-como-se-recomienda = As recommended
ajustes-estado-con-recomendacion = Has a recommendation
ajustes-estado-dns-si = On
ajustes-estado-dns-no = Off
ajustes-estado-ddns-si = Active
ajustes-estado-ddns-no = Off
ajustes-estado-ddns-sin-usar = Not in use
ajustes-estado-gestion-remota-si = On
ajustes-estado-gestion-remota-no = Off
ajustes-estado-linea-si = Active
ajustes-estado-linea-no = Off
ajustes-estado-voz-si = On
ajustes-estado-voz-no = Off
ajustes-estado-paso-directo-si = On
ajustes-estado-paso-directo-no = Off
ajustes-estado-hora-si = Automatic
ajustes-estado-hora-no = Not corrected
ajustes-estado-diagnostico-si = Copying traffic
ajustes-estado-diagnostico-no = Off
ajustes-dato-nombres_ddns = Names
ajustes-dato-nombres_ddns-ninguno = None
ajustes-dato-cuentas = Accounts

## Security and System actions
ajustes-accion-apagar_gestion_remota = Turn off remote management
ajustes-accion-usar_hora_neutra = Use a neutral time server
ajuste-accion-apagar_gestion_remota-titulo = Remote management, turn it off
ajuste-accion-usar_hora_neutra-titulo = Router time, use a neutral server

## What is happening now (the card's recommendation)
ajustes-problema-seguridad-activa-encendida = Remote management is on: the operator can get into the router from the internet and change any setting without asking you.
ajustes-problema-sistema-activa-false = The router does not set its clock over the internet: it drifts, and after a power cut it can end up with a very different time.
ajustes-problema-sistema-servidor_1 = The router sets its clock from { $valor }: it depends on whoever runs that server.
ajustes-problema-sistema-espejo-true = The traffic copy is on: anyone who plugs into that socket can see everything your home does on the internet.
ajustes-problema-telefono-tonos-en_la_voz = The keys you press during a call are mixed into the voice: if the call sounds bad, a phone menu may not recognise them.
ajustes-problema-telefono-tonos-mensajes = The keys you press during a call go as separate messages: if the operator does not support it, phone menus do not get them.
ajustes-problema-telefono-activo-false = Internet phone is off: the landline cannot make or receive calls.

## Summary strip of Internet, Security, Phone and System
ajustes-resumen-internet-conexion = Internet connection
ajustes-resumen-internet-conectada = Connected
ajustes-resumen-internet-sin-conexion = Not connected
ajustes-resumen-internet-conexion-sin-leer = The connection could not be read.
ajustes-resumen-internet-tipo-pppoe = Over PPPoE, with the operator's user name and password.
ajustes-resumen-internet-tipo-dhcp = With an address the operator gives on its own (DHCP).
ajustes-resumen-internet-tipo-fija = With a fixed address.
ajustes-resumen-internet-tipo-otro = Of a type the panel does not recognise.
ajustes-resumen-internet-rele = DNS relay
ajustes-resumen-internet-rele-detalle = Whether the router forwards name lookups.
ajustes-resumen-internet-ddns = Fixed names
ajustes-resumen-internet-ddns-detalle = Registered with dynamic DNS.
ajustes-resumen-seguridad-remota = Remote management
ajustes-resumen-seguridad-remota-detalle = Whether the operator can change the router remotely.
ajustes-resumen-seguridad-aviso = Check-in with the operator
ajustes-resumen-seguridad-aviso-detalle = How often the router contacts it on its own.
ajustes-resumen-seguridad-cuentas = Router accounts
ajustes-resumen-seguridad-cuentas-detalle = The ones that can log in to its web, the operator's too.
ajustes-resumen-telefono-linea = Line
ajustes-resumen-telefono-linea-detalle = The landline plugged into the router.
ajustes-resumen-telefono-voz = Internet phone
ajustes-resumen-telefono-voz-detalle = The router's connection to the operator for calls.
ajustes-resumen-telefono-fax = Fax over the internet
ajustes-resumen-telefono-fax-detalle = Faxes are sent in a way designed for the internet.
ajustes-resumen-sistema-hora = Time over the internet
ajustes-resumen-sistema-hora-detalle = The router corrects its clock on its own.
ajustes-resumen-sistema-servidores = Time servers
ajustes-resumen-sistema-servidores-detalle = The ones the router asks.
ajustes-resumen-sistema-servidores-primero = The first one: { $servidor }.
ajustes-resumen-sistema-servidores-ninguno = None is set.
ajustes-resumen-sistema-copia = Traffic copy
ajustes-resumen-sistema-copia-detalle = For diagnosis; when off, nobody gets it.

## Switches with their own name, single detail column and Internet and Phone actions
ajustes-interruptor-dns-si = DNS relay on
ajustes-interruptor-dns-no = DNS relay off
ajustes-interruptor-hora-si = Automatic time on
ajustes-interruptor-hora-no = Automatic time off
ajustes-interruptor-servidores-si = Internet phone on
ajustes-interruptor-servidores-no = Internet phone off
ajustes-interruptor-fax-y-modem-si = Fax pass-through on
ajustes-interruptor-fax-y-modem-no = Fax pass-through off
ajustes-detalle-opciones = Options
ajustes-accion-activar_rele = Turn on the relay
ajustes-accion-desactivar_rele = Turn off the relay
ajustes-accion-encender_voz = Turn on internet phone
ajustes-accion-apagar_voz = Turn off internet phone
ajuste-accion-activar_rele-titulo = DNS relay, turn it on
ajuste-accion-desactivar_rele-titulo = DNS relay, turn it off
ajuste-accion-encender_voz-titulo = Internet phone, turn it on
ajuste-accion-apagar_voz-titulo = Internet phone, turn it off
ajustes-problema-sistema-servidor_2 = The backup server is { $valor }: it depends on whoever runs that server.
