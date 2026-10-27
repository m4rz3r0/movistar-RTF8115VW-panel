# Ajustes del tema Puertos: reglas de apertura, UPnP y ayudantes de protocolos.

## Puertos abiertos
ajuste-puertos-reglas-titulo = Puertos abiertos
ajuste-puertos-reglas-desc = Deja que desde internet se llegue a un dispositivo de tu casa, como una consola, una cámara o un servidor de juegos.
ajuste-puertos-reglas-ayuda = Cada regla deja pasar por un puerto lo que llega de internet hacia un dispositivo concreto de tu red. Ese dispositivo queda expuesto: abre solo lo que te pida su manual y borra las reglas que ya no uses. Dale antes una IP fija al dispositivo; si no, la regla puede acabar apuntando a otro.
ajuste-puertos-reglas-nombre = Nombre
ajuste-puertos-reglas-nombre-desc = Un nombre para reconocer la regla, por ejemplo «Consola» o «Cámara del patio».
ajuste-puertos-reglas-protocolo = Protocolo
ajuste-puertos-reglas-protocolo-desc = El tipo de tráfico que se deja pasar. El manual del dispositivo o del juego dice cuál necesita. Si pide los dos, elige «TCP y UDP».
ajuste-puertos-reglas-protocolo-tcp = TCP
ajuste-puertos-reglas-protocolo-tcp-desc = El más habitual: páginas web, servidores y la mayoría de los programas.
ajuste-puertos-reglas-protocolo-udp = UDP
ajuste-puertos-reglas-protocolo-udp-desc = Lo usan sobre todo los juegos en línea, las llamadas y el vídeo en directo.
ajuste-puertos-reglas-externo_desde = Puerto de internet (desde)
ajuste-puertos-reglas-externo_desde-desc = El puerto por el que se llega desde internet, de 1 a 65535. Para un solo puerto, pon el mismo número en «desde» y «hasta».
ajuste-puertos-reglas-externo_hasta = Puerto de internet (hasta)
ajuste-puertos-reglas-externo_hasta-desc = El último puerto del rango que se abre. Tiene que ser igual o mayor que el primero.
ajuste-puertos-reglas-destino = Dispositivo de destino (IP)
ajuste-puertos-reglas-destino-desc = La dirección IP del dispositivo de tu red que recibe lo que entra, por ejemplo 192.168.1.60.
ajuste-puertos-reglas-interno_desde = Puerto del dispositivo (desde)
ajuste-puertos-reglas-interno_desde-desc = El puerto en el que escucha el dispositivo. Casi siempre es el mismo que el de internet.
ajuste-puertos-reglas-interno_hasta = Puerto del dispositivo (hasta)
ajuste-puertos-reglas-interno_hasta-desc = El último puerto del rango en el dispositivo. El rango debe tener el mismo tamaño que el de internet.
ajuste-puertos-reglas-activa = Activa
ajuste-puertos-reglas-activa-desc = Si la desactivas, la regla se guarda pero el puerto queda cerrado.
ajuste-puertos-reglas-activa-recomendado = Recomendado si quieres que el puerto se abra en cuanto guardes la regla. A favor: el juego o el dispositivo funciona enseguida. En contra: desde ese momento el dispositivo queda expuesto a internet por ese puerto; desactiva o borra la regla cuando ya no la uses.
ajuste-puertos-reglas-protocolo-ambos = TCP y UDP
ajuste-puertos-reglas-protocolo-ambos-desc = Para lo que pide los dos, como muchas consolas. Se guardan dos reglas, una con cada protocolo, en un solo cambio.
ajuste-puertos-reglas-paso-igual = Igual que la regla TCP, pero con el protocolo UDP.
ajuste-puertos-reglas-paso-igual-sin-el-primero = Con los mismos datos que la regla TCP de arriba, que no se aplicará, pero con el protocolo UDP.
ajuste-puertos-reglas-abrir = Abrir un puerto
ajuste-puertos-reglas-nueva = Abrir un puerto
ajuste-puertos-reglas-cambiar = Cambiar la regla
ajuste-puertos-reglas-puerto = Puerto de internet
ajuste-puertos-reglas-puerto-desc = Un número, como 8080, o un rango con un guion, como 27015-27030. Lo dice el manual del dispositivo o del juego. Abre solo los que te pida.
ajuste-puertos-reglas-dispositivo = Dispositivo
ajuste-puertos-reglas-dispositivo-desc = El dispositivo de tu casa que recibe lo que entra por ese puerto. Queda expuesto a internet: elige solo el que lo necesita y, mejor, uno con IP fija.
ajuste-puertos-reglas-dispositivo-elegir = Elige un dispositivo
ajuste-puertos-reglas-otra-ip = Otra IP…
ajuste-puertos-reglas-puerto-distinto = El dispositivo usa otro puerto
ajuste-puertos-reglas-interno = Puerto en el dispositivo
ajuste-puertos-reglas-interno-desc = Casi siempre es el mismo que el de internet: déjalo vacío. Cámbialo solo si el manual del dispositivo dice que escucha en otro (con un rango, el primero).
ajuste-puertos-reglas-col-protocolo = Protocolo
ajuste-puertos-reglas-col-externo = Puerto externo
ajuste-puertos-reglas-col-destino = Dispositivo
ajuste-puertos-reglas-col-activa = Activa
ajuste-puertos-reglas-hacia = { $puerto } hacia { $ip }
ajuste-puertos-reglas-estado-abierta = Abierta
ajuste-puertos-reglas-estado-cerrada = Cerrada
ajuste-puertos-reglas-estado-sin-destino = Sin dispositivo
ajuste-puertos-reglas-a-ip = { $puerto } en el dispositivo
ajuste-puertos-reglas-destino-perdido-desc = El router no conoce ningún dispositivo ni ninguna IP fija con esa dirección. Si el dispositivo ya no existe, borra la regla; si cambió de IP, edítala (y dale una IP fija para que no vuelva a pasar).
ajuste-puertos-reglas-borrar-de = Borrar la regla { $nombre }
ajuste-puertos-reglas-activa-de = Regla { $nombre } activa
ajuste-puertos-reglas-vacia = No hay ningún puerto abierto. Ábrelo solo si un juego o un dispositivo te lo pide.
ajuste-puertos-reglas-error-puerto = Escribe un puerto de 1 a 65535, como 8080, o un rango con un guion, como 27015-27030.
ajuste-puertos-reglas-error-dispositivo = Elige un dispositivo o escribe su IP.
ajuste-puertos-reglas-error-invertido = El primer puerto del rango tiene que ser menor que el último, como 27015-27030.
ajuste-puertos-reglas-error-distintos = El rango del dispositivo tiene que tener los mismos puertos que el de internet.
ajuste-puertos-reglas-error-solapa = Ese puerto ya está abierto en otra regla con el mismo protocolo: elige otro puerto o cambia esa regla.
ajuste-puertos-reglas-error-fuera = Esa IP no es la de un dispositivo de tu red. Tiene que ser de la misma red que el router ({ $router }), pero no la del propio router.
ajuste-puertos-reglas-error-cero = El puerto 0 no existe: los puertos van de 1 a 65535.
ajuste-puertos-reglas-error-cambiada = Esa regla ha cambiado desde que abriste la página (quizá desde otra ventana). Recarga la página y prueba de nuevo.

## Apertura automática de puertos (UPnP)
ajuste-puertos-upnp-titulo = Apertura automática de puertos (UPnP)
ajuste-puertos-upnp-desc = Permite que los dispositivos de tu red abran puertos por su cuenta, sin que tengas que crear reglas.
ajuste-puertos-upnp-ayuda = Algunas consolas y programas de descargas entre usuarios (P2P) lo usan para jugar en línea o conectar mejor; si lo apagas, puede que avisen de una conexión restringida y tengas que abrir sus puertos a mano. A cambio, con UPnP cualquier programa de tu red, también uno malicioso, puede abrir puertos sin preguntar.
ajuste-puertos-upnp-activo = UPnP activado
ajuste-puertos-upnp-activo-desc = Si lo activas, los dispositivos de tu red pueden abrir puertos hacia internet sin pedir permiso.
ajuste-puertos-upnp-activo-recomendado = Recomendado si prefieres decidir tú qué puertos se abren. A favor: ningún programa de tu red, tampoco uno malicioso, puede abrir puertos sin que lo sepas. En contra: algunas consolas y programas de descargas pueden avisar de una conexión restringida y quizá tengas que abrir sus puertos a mano.

## Ayudantes de protocolos (ALG)
ajuste-puertos-alg-titulo = Ayudantes de protocolos (ALG)
ajuste-puertos-alg-desc = Ayudas del router para que algunos programas antiguos funcionen a través de él: transferencia de archivos, llamadas por internet, redes privadas y videoconferencia.
ajuste-puertos-alg-ayuda = Cada ayudante retoca el tráfico de un tipo de programa para que pase bien por el router. Aquí ves cuáles tiene encendidos tu router y puedes cambiarlos. Con uno apagado, ese tipo de programa puede no funcionar desde tu red; con el de llamadas encendido, a veces fallan los teléfonos o las aplicaciones de llamadas de tu red.
ajuste-puertos-alg-ftp = Transferencia de archivos (FTP)
ajuste-puertos-alg-ftp-desc = Hace que funcionen los programas de transferencia de archivos que usan el modo activo de FTP.
ajuste-puertos-alg-sip = Llamadas por internet (SIP)
ajuste-puertos-alg-sip-desc = Retoca las llamadas por internet de los dispositivos de tu red. Los teléfonos y aplicaciones de llamadas actuales ya saben atravesar el router por sí solos.
ajuste-puertos-alg-pptp = Redes privadas (PPTP)
ajuste-puertos-alg-pptp-desc = Deja que los ordenadores de tu red se conecten a redes privadas (VPN) antiguas de tipo PPTP.
ajuste-puertos-alg-h323 = Videoconferencia (H.323)
ajuste-puertos-alg-h323-desc = Hace que funcionen los equipos de videoconferencia antiguos que usan H.323.

## Dispositivo expuesto (DMZ), solo de lectura
ajuste-puertos-dmz-titulo = Dispositivo expuesto (DMZ)
ajuste-puertos-dmz-desc = Un dispositivo de tu casa que recibe todo lo que llega de internet sin una regla de puertos.
ajuste-puertos-dmz-ayuda = Con la DMZ activa, el router manda a ese dispositivo todo lo que llega de internet y no tiene una regla propia: queda expuesto como si no hubiera router delante. A veces se usa con una consola o con un router propio detrás de este. Aquí ves si está activa y hacia qué dispositivo.
ajuste-puertos-dmz-motivo = El panel no puede cambiar la DMZ: hazlo desde la web del router.
ajuste-puertos-dmz-equipo = Dispositivo (IP)
ajuste-puertos-dmz-equipo-desc = La dirección IP del dispositivo que recibe todo lo que llega de internet.
ajuste-puertos-dmz-activa = Activa
ajuste-puertos-dmz-activa-desc = Si está activa, ese dispositivo recibe todo lo que llega de internet sin una regla.

## NAT loopback
ajuste-puertos-loopback-titulo = Entrar a tus servicios desde casa (NAT loopback)
ajuste-puertos-loopback-desc = Deja que los dispositivos de tu casa lleguen a un servicio de tu red usando tu dirección de internet o tu dominio, igual que desde fuera.
ajuste-puertos-loopback-ayuda = Sirve si tienes un servidor en casa (una web, una nube, una cámara) con un puerto abierto y entras a él con tu dominio: con esto apagado, desde casa no carga y fuera sí. Si no tienes nada así, da igual cómo esté.
ajuste-puertos-loopback-activo = NAT loopback activado
ajuste-puertos-loopback-activo-desc = Si lo apagas, desde tu casa no podrás entrar a tus servicios con tu dirección de internet; desde fuera seguirán funcionando.
