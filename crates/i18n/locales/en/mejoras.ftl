# Improvements: bundles that apply several settings in one click with a single undo.

## Recommended security
mejoras-lote-seguridad = Recommended security
mejoras-lote-seguridad-desc = Recommended if you want to close the doors you do not use. Pros: it closes the operator's remote management channel, pairing by code (WPS) and automatic port opening (UPnP), and leaves the WiFi protected with WPA2-AES. Cons: router updates stop arriving, since the operator only sends them through remote management; its app and the support team's remote diagnosis stop working; some consoles will need you to open their ports by hand, and very old WiFi devices (from before about 2006) will not connect.
mejoras-lote-seguridad-titulo = Improvements, recommended security

## Maximum privacy
mejoras-lote-privacidad = Maximum privacy
mejoras-lote-privacidad-desc = Recommended if you want the router to tell the operator and third parties as little as possible. Pros: the router stops checking in with the operator and stops sending out its log or a copy of the traffic, and your website name lookups (DNS) are no longer answered by the operator. Cons: the same as turning off remote management; on top of that, those lookups still travel unencrypted, so the operator sees them on the way, along with the name of every website you visit: hiding them takes encrypted DNS on each device or a VPN.
mejoras-lote-privacidad-titulo = Improvements, maximum privacy

## Shared notice
mejoras-aviso-tr069 = A factory reset or a technician can turn remote management back on. If you call support, they may ask you to turn it on. It is worth turning it on now and then to get the router's updates.

## Tips without a button
mejoras-consejo-wpa3 = Use WPA3 and management frame protection if the router ever allows it: this one does not support it.
mejoras-consejo-nombre_y_mac = Do not hide the network name or filter by MAC: they do not protect you, and a hidden name makes phones announce it wherever they go.
mejoras-consejo-contrasena_admin = Change the admin password printed on the router's label (coming to the panel later).
mejoras-consejo-reenvios = Check that no port forwarding rule points to the router itself.
mejoras-consejo-sin_api = This panel cannot block ping from the internet or turn off file sharing, the media server or USB.

## Steps
mejoras-paso-gestion_remota = Turn off the operator's remote management
mejoras-paso-gestion_remota-desc = Recommended if you want to close the channel through which the operator changes the router remotely and you accept handling updates yourself. Pros: the most powerful remote management channel disappears and the router stops sending its status to the operator. Cons: router updates no longer arrive, since they only come this way; the operator's app and the support team's remote diagnosis stop working, and a factory reset or a technician can turn it back on.
mejoras-paso-aviso_operador = Turn off the regular check-in with the operator
mejoras-paso-aviso_operador-desc = Recommended if you turn off remote management: it reinforces it. Pros: the router stops contacting the operator at regular intervals. Cons: the same as turning off remote management.
mejoras-paso-wps = Remove WPS
mejoras-paso-wps-desc = Recommended if you do not need to connect devices by pressing the router's button, which is almost always. Pros: it closes pairing by code, which can be guessed in about 11,000 tries. Cons: new devices connect by typing the password.
mejoras-paso-upnp = Turn off automatic port opening (UPnP)
mejoras-paso-upnp-desc = Recommended if you have no consoles or programs that need to open ports on their own. Pros: no device or malicious program opens ports without your permission. Cons: some consoles will switch to a restricted connection and you will have to open their ports by hand.
mejoras-paso-wpa2 = Encrypt the WiFi with WPA2-AES
mejoras-paso-wpa2-desc = Recommended if you have no WiFi devices from before about 2006. Pros: the old TKIP cipher and the mixed mode, which are weaker, are removed. Cons: very old devices will not connect.
mejoras-paso-aislar_invitados = Isolate the guest network
mejoras-paso-aislar_invitados-desc = Recommended if you use the guest network for visitors or for connected home devices. Pros: devices on that network cannot see each other. Cons: they cannot cast to a TV or print to each other.
mejoras-paso-espejo = Turn off the internet traffic mirror
mejoras-paso-espejo-desc = Recommended if you are not running a diagnosis of the connection. Pros: no network port at home receives a copy of all the internet traffic. Cons: none; it is turned on only to diagnose.
mejoras-paso-registro = Do not send the system log to another computer
mejoras-paso-registro-desc = Recommended if you do not have your own server to collect logs, which is normal at home. Pros: the log, which tells which devices in your home connect and when, does not leave the router unencrypted towards another computer. Cons: if you use your own server for logs, it stops receiving the router's.
mejoras-paso-dns_casa = Use neutral DNS on the home network
mejoras-paso-dns_casa-desc = Recommended if you do not want the operator to answer or filter your website name lookups and you accept that they still travel unencrypted. Pros: the operator stops answering them, and some services block malicious domains. Cons: the operator still sees them on the way, you start trusting another company, and you may lose some operator feature that depends on its DNS.
mejoras-paso-hora_neutra = Take the time from a neutral server
mejoras-paso-hora_neutra-desc = Recommended if you want to depend as little as possible on the operator's servers. Pros: the time comes from the Spanish Navy's Royal Observatory. Cons: the privacy gain is minimal.

## DNS providers (the note for each one, in the review's picker)
mejoras-dns-quad9 = Quad9
mejoras-dns-quad9-desc = Blocks malicious domains. Based in Switzerland, it does not keep your IP address.
mejoras-dns-quad9_sin_filtro = Quad9 without filtering
mejoras-dns-quad9_sin_filtro-desc = The same as Quad9, but without blocking anything.
mejoras-dns-dns4eu = DNS4EU
mejoras-dns-dns4eu-desc = European Union infrastructure and law. It applies the blocks ordered by the courts.
mejoras-dns-cloudflare = Cloudflare
mejoras-dns-cloudflare-desc = Very fast. It is a United States company.
mejoras-dns-adguard = AdGuard
mejoras-dns-adguard-desc = Blocks ads and trackers. It may break some websites.
mejoras-dns-elegir = DNS service for the home network

## State of the bundles and their review
mejoras-motivo-sin-leer = The { $pestana } tab could not be read.
mejoras-motivo-sin-dato = The router does not give this value.
mejoras-revision-sin-leer = { $n ->
    [one] One step could not be read and is not in the review.
   *[other] { $n } steps could not be read and are not in the review.
}
mejoras-error-lote = That improvement bundle does not exist.
mejoras-error-ya-aplicado = This bundle is already applied: there is nothing to change.
mejoras-error-proveedor = That DNS service is not in the list.
mejoras-motivo-pasos-sin-leer = { $n ->
    [one] One step could not be read; the rest is already done.
   *[other] { $n } steps could not be read; the rest is already done.
}
mejoras-error-lectura = None of the steps of this bundle could be read from the router. Try again in a moment.
mejoras-error-sin-leer = { $n ->
    [one] What could be read is already done, but one step could not be read. Read again and retry.
   *[other] What could be read is already done, but { $n } steps could not be read. Read again and retry.
}

## Improvements page
mejoras-titulo = Recommended improvements for your router
mejoras-desc = Groups of settings you review and apply at once, with a single undo.
mejoras-estado-aplicado = Applied
mejoras-estado-pendientes = { $pendientes } of { $total } pending
mejoras-estado-sin-leer = Not read
mejoras-paso-estado-hecho = Done
mejoras-paso-estado-pendiente = Pending
mejoras-paso-estado-sin-leer = Not read
mejoras-paso-estado-no-aplica = Does not apply
mejoras-opcional = Optional
mejoras-revisar = Review and apply
mejoras-volver-a-leer = Read again
mejoras-mas-consejos = More tips
mejoras-pasos = Steps
