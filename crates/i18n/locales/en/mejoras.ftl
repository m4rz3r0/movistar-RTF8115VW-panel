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
mejoras-consejo-reenvios = Check that no open port points to the router itself.
mejoras-consejo-sin_api = The router does not let you block ping from the internet or turn off file sharing, the media server or USB.

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
mejoras-paso-espejo-desc = Recommended if you are not running a diagnosis of the connection. Pros: no socket at home receives a copy of all the internet traffic. Cons: none; it is turned on only to diagnose.
mejoras-paso-hora_neutra = Take the time from a neutral server
mejoras-paso-hora_neutra-desc = Recommended if you want to depend as little as possible on the operator's servers. Pros: the time comes from the Spanish Navy's Royal Observatory. Cons: the privacy gain is minimal.
