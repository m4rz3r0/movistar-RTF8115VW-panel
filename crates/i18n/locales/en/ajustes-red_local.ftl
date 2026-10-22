# Settings in the Local network section: devices, fixed IPs, address handout and IPv6.

## Connected devices
ajuste-red_local-dispositivos-titulo = Connected devices
ajuste-red_local-dispositivos-desc = The devices the router knows on your home network: their name, their address and how they connect. With "Fix this IP", a device always gets the same address.
ajuste-red_local-dispositivos-ayuda = Looking at this list changes nothing on the router. "Fix this IP" saves the address the device has now as its fixed IP (you'll see it under "Fixed IPs"); before applying it, it shows you the change. You can't block a device or set parental controls from here: the router doesn't give the panel a safe way to do it, and it's being looked into.
ajuste-red_local-dispositivos-col-ip = IP address
ajuste-red_local-dispositivos-col-mac = Physical address (MAC)
ajuste-red_local-dispositivos-col-conexion = Connection
ajuste-red_local-dispositivos-col-ip_fija = Fixed IP
ajuste-red_local-dispositivos-sin-nombre = No name
ajuste-red_local-dispositivos-cable = Cable
ajuste-red_local-dispositivos-wifi24 = 2.4 GHz WiFi
ajuste-red_local-dispositivos-wifi5 = 5 GHz WiFi
ajuste-red_local-dispositivos-invitados = Guest WiFi
ajuste-red_local-dispositivos-desconocida = Unknown
ajuste-red_local-dispositivos-desconectado = Not connected now
ajuste-red_local-dispositivos-fijar = Fix this IP
ajuste-red_local-dispositivos-fijar-de = Fix this IP: { $ip } ({ $equipo })
ajuste-red_local-dispositivos-ya-fija = Already has a fixed IP: { $ip }
ajuste-red_local-dispositivos-no-fijable = This IP can't be fixed
ajuste-red_local-dispositivos-fijar-desconectado = Connect it to fix its IP
ajuste-red_local-dispositivos-sin-ip-fija = No fixed IP
ajuste-red_local-dispositivos-vacia = The router doesn't know any device on your network yet.

## Fixed IPs
ajuste-red_local-reservas-titulo = Fixed IPs
ajuste-red_local-reservas-desc = Devices that always get the same IP address on your network, such as a printer or a console you open ports for.
ajuste-red_local-reservas-ayuda = The router recognises each device by its physical address (MAC) and always gives it the IP you choose. The change applies the next time the device asks for an address: disconnect and reconnect it and it picks it up straight away. Some phones and computers may change their physical address from time to time (the option is usually called "private address", "random address" or "random MAC"). If the device doesn't get its fixed IP, set that option to "fixed" or turn it off in the device's WiFi settings for this network.
ajuste-red_local-reservas-mac = Physical address (MAC)
ajuste-red_local-reservas-mac-desc = The device's identifier: six pairs of digits and letters separated by colons, such as 02:00:00:00:00:01.
ajuste-red_local-reservas-ip = IP address
ajuste-red_local-reservas-ip-desc = The address the device will always have, inside your network (for example 192.168.1.60) and not already used by another device.
ajuste-red_local-reservas-activa = Active
ajuste-red_local-reservas-activa-desc = If you turn it off, the reservation is kept but the device goes back to getting any address.
ajuste-red_local-reservas-activa-recomendado = Recommended if you want the device to have its fixed IP now. Pros: it gets it the next time it connects. Cons: nothing important; turn it off only to keep it for later without using it.
ajuste-red_local-reservas-col-mac = Physical address (MAC)
ajuste-red_local-reservas-col-ip = IP address
ajuste-red_local-reservas-col-activa = Active
ajuste-red_local-reservas-activa-de = Fixed IP of { $equipo } active
ajuste-red_local-reservas-estado-conectado = Connected
ajuste-red_local-reservas-estado-desactivada = Turned off
ajuste-red_local-reservas-anadir = Add fixed IP
ajuste-red_local-reservas-nueva = New fixed IP
ajuste-red_local-reservas-cambiar = Change the fixed IP
ajuste-red_local-reservas-borrar-de = Delete the fixed IP of { $equipo }
ajuste-red_local-reservas-vacia = There are no fixed IPs yet. Add one here or with "Fix this IP" in the device list.
ajuste-red_local-reservas-error-fuera = That IP isn't on your network. It has to be on the same network as the router ({ $router }).
ajuste-red_local-reservas-error-router = That's the router's IP: choose another one.
ajuste-red_local-reservas-error-ip-repetida = That IP is already another device's fixed IP: choose another one.
ajuste-red_local-reservas-error-mac-repetida = That device already has a fixed IP: change it in its row instead of adding another.
ajuste-red_local-reservas-error-difusion = That IP belongs to the whole network (the first and last ones can't be given to a device): choose another one.
ajuste-red_local-reservas-error-cambiada = That fixed IP has changed since you opened the page (maybe from another window). Reload the page and try again.

## Address handout (DHCP)
ajuste-red_local-dhcp-titulo = Address handout (DHCP)
ajuste-red_local-dhcp-desc = The router automatically gives an IP address to each device that joins your network.
ajuste-red_local-dhcp-ayuda = Changing the range or the duration affects every device at home when it renews its address. If you turn it off, new devices will not be able to connect unless you set their address by hand. Fixed IPs must stay inside your network.
ajuste-red_local-dhcp-activo = Automatic handout
ajuste-red_local-dhcp-activo-desc = If you turn it off, the router stops handing out addresses and each device has to be set up by hand.
ajuste-red_local-dhcp-activo-recomendado = Recommended if you want your devices to connect on their own, as in almost every home. Pros: every phone, computer or TV gets its address without you doing anything. Cons: none significant for most people; it is only turned off if another device on your network already hands out addresses.
ajuste-red_local-dhcp-inicio = First address
ajuste-red_local-dhcp-inicio-desc = The first address the router may hand out (for example 192.168.1.100).
ajuste-red_local-dhcp-fin = Last address
ajuste-red_local-dhcp-fin-desc = The last address the router may hand out. Between the first and the last there must be room for all your devices.
ajuste-red_local-dhcp-concesion = Duration of each address
ajuste-red_local-dhcp-concesion-desc = How long each device keeps its address before asking for a renewal.
ajuste-red_local-dhcp-concesion-hora = 1 hour
ajuste-red_local-dhcp-concesion-hora-desc = Addresses are freed quickly, useful if many different devices come and go; in exchange, devices renew them more often.
ajuste-red_local-dhcp-concesion-doce_horas = 12 hours
ajuste-red_local-dhcp-concesion-doce_horas-desc = A middle ground: each device keeps its address for half a day.
ajuste-red_local-dhcp-concesion-dia = 1 day
ajuste-red_local-dhcp-concesion-dia-desc = Devices keep their address all day; those no longer used take a day to be freed.
ajuste-red_local-dhcp-concesion-semana = 1 week
ajuste-red_local-dhcp-concesion-semana-desc = Addresses hardly change, but those of devices that no longer come take a while to be freed.
ajuste-red_local-dhcp-concesion-recomendado = Recommended if mostly the same devices connect at home, plus the odd visitor. Pros: each device keeps its address, and those of devices that stop coming are freed within half a day. Cons: if many different devices come and go, one hour frees addresses sooner.

## IPv6 at home
ajuste-red_local-ipv6-titulo = IPv6 at home
ajuste-red_local-ipv6-desc = Also gives IPv6 addresses, the new version of internet addresses, to the devices on your network.
ajuste-red_local-ipv6-ayuda = It only helps if your internet connection has IPv6. Once on, devices start using IPv6 where they can; if your connection doesn't have it, devices gain nothing. If some service stops working after turning it on, turn it off again.
ajuste-red_local-ipv6-ipv6 = IPv6 on the local network
ajuste-red_local-ipv6-ipv6-desc = If you turn it on, the router gives your devices IPv6 addresses as well as the usual ones.
ajuste-red_local-ipv6-anuncios = Router announcements
ajuste-red_local-ipv6-anuncios-desc = The router tells devices that IPv6 is available so they set themselves up. Without these announcements, hardly any will use IPv6.
ajuste-red_local-ipv6-anuncios-recomendado = Recommended if IPv6 is on in your home network. Pros: devices set up IPv6 on their own and find their way out to the internet over IPv6. Cons: none significant for most people; if you don't use IPv6, they do nothing.
