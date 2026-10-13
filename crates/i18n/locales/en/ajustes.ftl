# Router settings page: texts shared by every section, errors when reading
# or writing a value, levels and risks. The texts of each setting live in
# `ajustes-<section>.ftl`.

## Page
ajustes-titulo = Router settings
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
ajustes-nivel-oculto = Hidden by the operator
ajustes-nivel-oculto-desc = Under "More options". The router's web page does not show it, but the router accepts it.

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
ajustes-error-corto = Too short: it must have at least { $min } characters (accented letters count double).
ajustes-error-largo = Too long: at most { $max } characters (accented letters count double).
ajustes-error-fuera-de-rango = That number is not within the accepted values.
ajustes-error-ip = Not a valid IP address (four numbers from 0 to 255 separated by dots).
ajustes-error-mac = Not a valid MAC address (six pairs of digits and letters separated by colons).
ajustes-error-opcion = Choose one of the options.
ajustes-error-control = It contains characters that cannot be typed, such as line breaks.
ajustes-error-ascii = Only letters without accents, digits and common symbols are accepted.
ajustes-error-solo-lectura = This value can only be viewed.
