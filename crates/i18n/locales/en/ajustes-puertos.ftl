# Settings in the Ports section: forwarding rules, UPnP and protocol helpers.

## Open ports
ajuste-puertos-reglas-titulo = Open ports
ajuste-puertos-reglas-desc = Lets something on the internet reach a device in your home, such as a console, a camera or a game server.
ajuste-puertos-reglas-ayuda = Each rule lets what arrives from the internet on a port through to a specific device on your network. That device becomes exposed: only open what its manual asks for and delete rules you no longer use. Give the device a fixed IP first; otherwise the rule may end up pointing at another one.
ajuste-puertos-reglas-nombre = Name
ajuste-puertos-reglas-nombre-desc = A name to recognise the rule, for example "Console" or "Garden camera".
ajuste-puertos-reglas-protocolo = Protocol
ajuste-puertos-reglas-protocolo-desc = The kind of traffic let through. The manual of the device or game says which one it needs.
ajuste-puertos-reglas-protocolo-tcp = TCP
ajuste-puertos-reglas-protocolo-tcp-desc = The most common: web pages, servers and most programs.
ajuste-puertos-reglas-protocolo-udp = UDP
ajuste-puertos-reglas-protocolo-udp-desc = Mostly used by online games, calls and live video.
ajuste-puertos-reglas-externo_desde = Internet port (from)
ajuste-puertos-reglas-externo_desde-desc = The port reached from the internet, from 1 to 65535. For a single port, enter the same number in "from" and "to".
ajuste-puertos-reglas-externo_hasta = Internet port (to)
ajuste-puertos-reglas-externo_hasta-desc = The last port of the range that is opened. It must be equal to or greater than the first.
ajuste-puertos-reglas-destino = Destination device (IP)
ajuste-puertos-reglas-destino-desc = The IP address of the device on your network that receives what comes in, for example 192.168.1.60.
ajuste-puertos-reglas-interno_desde = Device port (from)
ajuste-puertos-reglas-interno_desde-desc = The port the device listens on. It is almost always the same as the internet one.
ajuste-puertos-reglas-interno_hasta = Device port (to)
ajuste-puertos-reglas-interno_hasta-desc = The last port of the range on the device. The range must be the same size as the internet one.
ajuste-puertos-reglas-activa = Active
ajuste-puertos-reglas-activa-desc = If you turn it off, the rule is kept but the port stays closed.

## Automatic port opening (UPnP)
ajuste-puertos-upnp-titulo = Automatic port opening (UPnP)
ajuste-puertos-upnp-desc = Lets the devices on your network open ports by themselves, without you creating rules.
ajuste-puertos-upnp-ayuda = Some consoles and peer-to-peer (P2P) download programs use it to play online or connect better; if you turn it off, they may report a restricted connection and you may have to open their ports by hand. On the other hand, with UPnP any program on your network, including a malicious one, can open ports without asking.
ajuste-puertos-upnp-activo = UPnP on
ajuste-puertos-upnp-activo-desc = If you turn it on, the devices on your network can open ports to the internet without asking.
ajuste-puertos-upnp-activo-recomendado = It is safer for you to decide which ports are opened. Only turn it on if a console or program really needs it.

## Protocol helpers (ALG)
ajuste-puertos-alg-titulo = Protocol helpers (ALG)
ajuste-puertos-alg-desc = Router helpers that let some older programs work through it: file transfer, internet calls, private networks and video conferencing.
ajuste-puertos-alg-ayuda = Each helper adjusts the traffic of one kind of program so it gets through the router properly. If everything works, leave them alone. Turning one off may stop that kind of program from working from your network; turning on the calls helper sometimes causes trouble with the phones or calling apps on your network.
ajuste-puertos-alg-ftp = File transfer (FTP)
ajuste-puertos-alg-ftp-desc = Makes file transfer programs that use FTP active mode work.
ajuste-puertos-alg-ftp-recomendado = It doesn't get in the way of anything and avoids failures with older file transfer programs.
ajuste-puertos-alg-sip = Internet calls (SIP)
ajuste-puertos-alg-sip-desc = Adjusts the internet calls of devices on your network. Current phones and calling apps already know how to get through the router on their own.
ajuste-puertos-alg-sip-recomendado = This helper often causes calls with no sound or that drop; many internet calling services ask for it to be off.
ajuste-puertos-alg-pptp = Private networks (PPTP)
ajuste-puertos-alg-pptp-desc = Lets the computers on your network connect to older PPTP private networks (VPN).
ajuste-puertos-alg-h323 = Video conferencing (H.323)
ajuste-puertos-alg-h323-desc = Makes older video conferencing equipment that uses H.323 work.
