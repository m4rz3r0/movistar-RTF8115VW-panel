# Settings in the Internet section: internet connection and fibre, DNS relay and dynamic DNS.

## DNS relay
ajuste-internet-dns-titulo = DNS relay
ajuste-internet-dns-desc = DNS turns website names into addresses. With the relay, the router forwards your devices' lookups to your connection's DNS servers.
ajuste-internet-dns-ayuda = If you don't know why you'd change it, leave it as it is. A change here can make devices on your network stop finding websites even though the internet is still connected; if that happens, go back to the previous value.
ajuste-internet-dns-rele = DNS relay on
ajuste-internet-dns-rele-desc = In principle, the router passes your devices' lookups to your connection's DNS servers. With the relay off it already answers them itself; what turning it on changes on this router is still to be checked.

## Fixed name for your connection (dynamic DNS)
ajuste-internet-ddns-titulo = Fixed name for your connection (dynamic DNS)
ajuste-internet-ddns-desc = Gives your connection a name that doesn't change (such as myhome.example) even when your internet address does, so you can reach home from outside.
ajuste-internet-ddns-ayuda = You need an account with one of the dynamic DNS services the router supports. The router tells the service every time your internet address changes. It is only useful if you have something at home to reach from outside, such as a camera with an open port.
ajuste-internet-ddns-activa = Active
ajuste-internet-ddns-activa-desc = If you turn it off, the router stops updating the service and the name may be left with an old address.
ajuste-internet-ddns-servicio = Service
ajuste-internet-ddns-servicio-desc = The dynamic DNS service where you have the account.
ajuste-internet-ddns-servicio-no_ip = No-IP
ajuste-internet-ddns-servicio-no_ip-desc = The No-IP service.
ajuste-internet-ddns-servicio-now_ip = Now-IP
ajuste-internet-ddns-servicio-now_ip-desc = The Now-IP service.
ajuste-internet-ddns-servicio-dyndns = DynDNS
ajuste-internet-ddns-servicio-dyndns-desc = The DynDNS service.
ajuste-internet-ddns-servicio-now_dns = Now-DNS
ajuste-internet-ddns-servicio-now_dns-desc = The Now-DNS service.
ajuste-internet-ddns-nombre = Name
ajuste-internet-ddns-nombre-desc = The full name you registered with the service, for example myhome.example.
ajuste-internet-ddns-cuenta = Account
ajuste-internet-ddns-cuenta-desc = The user name or email of your account with the service.
ajuste-internet-ddns-contrasena = Password
ajuste-internet-ddns-contrasena-desc = The password (or the key the service gives you) for that account.

## Internet connection (read only)
ajuste-internet-conexion-titulo = Internet connection
ajuste-internet-conexion-desc = How the router connects to the internet: the connection type, the address your operator gives you and its details.
ajuste-internet-conexion-ayuda = The router has several connections with the operator. One carries the internet; the others belong to the operator (the phone, its services) and you don't need to touch them. The panel only shows them: changing the connection could leave you without internet.
ajuste-internet-conexion-motivo = The internet connection is not changed from the panel: a mistake would leave you without internet.
ajuste-internet-conexion-tipo = Type
ajuste-internet-conexion-tipo-desc = How the router identifies itself to the operator.
ajuste-internet-conexion-tipo-pppoe = PPPoE
ajuste-internet-conexion-tipo-pppoe-desc = With a user and a password from the operator.
ajuste-internet-conexion-tipo-dhcp = Automatic (DHCP)
ajuste-internet-conexion-tipo-dhcp-desc = The operator hands out the address, with no user or password.
ajuste-internet-conexion-tipo-fija = Fixed address
ajuste-internet-conexion-tipo-fija-desc = An address set by hand that does not change.
ajuste-internet-conexion-estado = Status
ajuste-internet-conexion-estado-desc = Whether the connection is up right now.
ajuste-internet-conexion-estado-true = Up
ajuste-internet-conexion-estado-false = Down
ajuste-internet-conexion-ip = Address
ajuste-internet-conexion-ip-desc = The address it has now; on the internet one, your public address.
ajuste-internet-conexion-puerta = Gateway
ajuste-internet-conexion-puerta-desc = The operator's equipment the router sends traffic to.
ajuste-internet-conexion-dns_1 = Operator DNS
ajuste-internet-conexion-dns_1-desc = The first DNS server the operator gives.
ajuste-internet-conexion-dns_2 = Operator DNS (second)
ajuste-internet-conexion-dns_2-desc = The backup DNS server the operator gives.
ajuste-internet-conexion-vlan = VLAN
ajuste-internet-conexion-vlan-desc = The tag the operator uses to keep this connection apart from the others on the same fibre.
ajuste-internet-conexion-mtu = MTU
ajuste-internet-conexion-mtu-desc = The largest size of each packet, in bytes.
ajuste-internet-conexion-ipv6 = IPv6
ajuste-internet-conexion-ipv6-desc = Whether the connection also has IPv6 addresses up.
ajuste-internet-conexion-ipv6-true = Up
ajuste-internet-conexion-ipv6-false = No IPv6
ajuste-internet-conexion-usuario = User
ajuste-internet-conexion-usuario-desc = The user the router signs in to the operator with.
ajuste-internet-conexion-contrasena = Password
ajuste-internet-conexion-contrasena-desc = That user's password. You need it if you put your own router in its place.
ajuste-internet-conexion-del-operador = From the operator (not your internet)
ajuste-internet-conexion-internet = Your internet connection
ajuste-internet-conexion-monopuesto-titulo = How to switch to bridge mode (monopuesto)
ajuste-internet-conexion-monopuesto-pasos =
    Write down the user and password of your internet connection (above, with the eye).
    Connect your own router by cable to a port on this router.
    In this router's web, set the internet connection to bridge mode (monopuesto) and save. The router restarts.
    On your router, set up the internet connection as PPPoE with that user and password.
    Check that your router has internet and connect your devices to it.
ajuste-internet-conexion-monopuesto-avisos =
    It is for people who have their own router behind this one. If you don't, don't do it.
    While you switch, you are without internet for a few minutes.
    WiFi, your network's addresses and ports move to your router: this one's stop working and the panel will no longer see them.
    The operator's TV and phone may stop working if they depend on this router.
    The panel does not do it for you. To go back, undo the change in the router's web or reset it to factory settings.

## Fibre
ajuste-internet-fibra-titulo = Fibre
ajuste-internet-fibra-desc = The status of the fibre that reaches the router and the details that identify it to the operator.
ajuste-internet-fibra-ayuda = If the fibre is not "Operational", the router has no internet even if everything else is fine. The serial numbers and the password identify your router to the operator: don't share them.
ajuste-internet-fibra-motivo = The fibre can only be viewed: the operator sets its details.
ajuste-internet-fibra-estado = Fibre status
ajuste-internet-fibra-estado-desc = Which step the router is at while connecting to the operator over the fibre.
ajuste-internet-fibra-estado-o1 = Initial (O1)
ajuste-internet-fibra-estado-o1-desc = It has just started or receives no signal over the fibre.
ajuste-internet-fibra-estado-o2 = Standby (O2)
ajuste-internet-fibra-estado-o2-desc = It receives a signal and waits for the operator to talk to it.
ajuste-internet-fibra-estado-o3 = Identifying (O3)
ajuste-internet-fibra-estado-o3-desc = It gives its serial number to the operator.
ajuste-internet-fibra-estado-o4 = Ranging (O4)
ajuste-internet-fibra-estado-o4-desc = The operator measures how far away it is to order the traffic.
ajuste-internet-fibra-estado-o5 = Operational (O5)
ajuste-internet-fibra-estado-o5-desc = Connected and working normally.
ajuste-internet-fibra-estado-o6 = Intermittent signal (O6)
ajuste-internet-fibra-estado-o6-desc = It lost the signal for a moment and is trying to get it back.
ajuste-internet-fibra-estado-o7 = Stopped by the operator (O7)
ajuste-internet-fibra-estado-o7-desc = The operator has cut this router's connection.
ajuste-internet-fibra-serie = Fibre serial number
ajuste-internet-fibra-serie-desc = Identifies the router to the operator on the fibre.
ajuste-internet-fibra-serie_acs = Serial number for the operator
ajuste-internet-fibra-serie_acs-desc = The one the operator uses to manage the router remotely.
ajuste-internet-fibra-contrasena = Fibre password
ajuste-internet-fibra-contrasena-desc = The key the router uses to identify itself on the fibre.
