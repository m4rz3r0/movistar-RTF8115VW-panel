# Settings in the Local network section: devices, fixed IPs, address handout and IPv6.

## Connected devices
ajuste-red_local-dispositivos-titulo = Connected devices
ajuste-red_local-dispositivos-desc = The devices the router knows on your home network, with their address and whether they are connected now.
ajuste-red_local-dispositivos-ayuda = This list is only for reading: it changes nothing on the router. It tells you what is connected and lets you give a device a fixed IP.

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
