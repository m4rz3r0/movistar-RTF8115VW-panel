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
ajustes-secreto = Hidden
ajustes-secreto-desc = For security, the panel does not show it. Only type a new one if you want to change it.
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
ajustes-secreto-placeholder = Leave it empty to keep it
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
ajustes-tema-sistema-desc = Time, log and other general router settings.

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
