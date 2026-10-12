# Ajustes del tema Puertos: reglas de apertura, UPnP y ayudantes de protocolos.

## Puertos abiertos
ajuste-puertos-reglas-titulo = Puertos abiertos
ajuste-puertos-reglas-desc = Deja que desde internet se llegue a un aparato de tu casa, como una consola, una cámara o un servidor de juegos.
ajuste-puertos-reglas-ayuda = Cada regla deja pasar por un puerto lo que llega de internet hacia un aparato concreto de tu red. Ese aparato queda expuesto: abre solo lo que te pida su manual y borra las reglas que ya no uses. Dale antes una IP fija al aparato; si no, la regla puede acabar apuntando a otro.
ajuste-puertos-reglas-nombre = Nombre
ajuste-puertos-reglas-nombre-desc = Un nombre para reconocer la regla, por ejemplo «Consola» o «Cámara del patio».
ajuste-puertos-reglas-protocolo = Protocolo
ajuste-puertos-reglas-protocolo-desc = El tipo de tráfico que se deja pasar. El manual del aparato o del juego dice cuál necesita. Si pide TCP y UDP, crea dos reglas iguales, una con cada uno.
ajuste-puertos-reglas-protocolo-tcp = TCP
ajuste-puertos-reglas-protocolo-tcp-desc = El más habitual: páginas web, servidores y la mayoría de los programas.
ajuste-puertos-reglas-protocolo-udp = UDP
ajuste-puertos-reglas-protocolo-udp-desc = Lo usan sobre todo los juegos en línea, las llamadas y el vídeo en directo.
ajuste-puertos-reglas-externo_desde = Puerto de internet (desde)
ajuste-puertos-reglas-externo_desde-desc = El puerto por el que se llega desde internet, de 1 a 65535. Para un solo puerto, pon el mismo número en «desde» y «hasta».
ajuste-puertos-reglas-externo_hasta = Puerto de internet (hasta)
ajuste-puertos-reglas-externo_hasta-desc = El último puerto del rango que se abre. Tiene que ser igual o mayor que el primero.
ajuste-puertos-reglas-destino = Aparato de destino (IP)
ajuste-puertos-reglas-destino-desc = La dirección IP del aparato de tu red que recibe lo que entra, por ejemplo 192.168.1.60.
ajuste-puertos-reglas-interno_desde = Puerto del aparato (desde)
ajuste-puertos-reglas-interno_desde-desc = El puerto en el que escucha el aparato. Casi siempre es el mismo que el de internet.
ajuste-puertos-reglas-interno_hasta = Puerto del aparato (hasta)
ajuste-puertos-reglas-interno_hasta-desc = El último puerto del rango en el aparato. El rango debe tener el mismo tamaño que el de internet.
ajuste-puertos-reglas-activa = Activa
ajuste-puertos-reglas-activa-desc = Si la desactivas, la regla se guarda pero el puerto queda cerrado.

## Apertura automática de puertos (UPnP)
ajuste-puertos-upnp-titulo = Apertura automática de puertos (UPnP)
ajuste-puertos-upnp-desc = Permite que los aparatos de tu red abran puertos por su cuenta, sin que tengas que crear reglas.
ajuste-puertos-upnp-ayuda = Algunas consolas y programas de descargas entre usuarios (P2P) lo usan para jugar en línea o conectar mejor; si lo apagas, puede que avisen de una conexión restringida y tengas que abrir sus puertos a mano. A cambio, con UPnP cualquier programa de tu red, también uno malicioso, puede abrir puertos sin preguntar.
ajuste-puertos-upnp-activo = UPnP activado
ajuste-puertos-upnp-activo-desc = Si lo activas, los aparatos de tu red pueden abrir puertos hacia internet sin pedir permiso.
ajuste-puertos-upnp-activo-recomendado = Recomendado si prefieres decidir tú qué puertos se abren. A favor: ningún programa de tu red, tampoco uno malicioso, puede abrir puertos sin que lo sepas. En contra: algunas consolas y programas de descargas pueden avisar de una conexión restringida y tendrás que abrir sus puertos a mano.

## Ayudantes de protocolos (ALG)
ajuste-puertos-alg-titulo = Ayudantes de protocolos (ALG)
ajuste-puertos-alg-desc = Ayudas del router para que algunos programas antiguos funcionen a través de él: transferencia de archivos, llamadas por internet, redes privadas y videoconferencia.
ajuste-puertos-alg-ayuda = Cada ayudante retoca el tráfico de un tipo de programa para que pase bien por el router. Si funciona todo, no los toques. Apagar uno puede impedir que ese tipo de programa funcione desde tu red; encender el de llamadas a veces da problemas con los teléfonos o aplicaciones de llamadas de tu red.
ajuste-puertos-alg-ftp = Transferencia de archivos (FTP)
ajuste-puertos-alg-ftp-desc = Hace que funcionen los programas de transferencia de archivos que usan el modo activo de FTP.
ajuste-puertos-alg-ftp-recomendado = Recomendado si usas programas de transferencia de archivos o no lo sabes. A favor: funcionan también los que usan el modo activo de FTP, el más antiguo. En contra: poco para la mayoría, aunque estos ayudantes se han usado alguna vez en ataques para abrir puertos hacia la red de casa.
ajuste-puertos-alg-sip = Llamadas por internet (SIP)
ajuste-puertos-alg-sip-desc = Retoca las llamadas por internet de los aparatos de tu red. Los teléfonos y aplicaciones de llamadas actuales ya saben atravesar el router por sí solos.
ajuste-puertos-alg-sip-recomendado = Recomendado si usas llamadas por internet en el móvil o el ordenador, o no lo sabes. A favor: evita las llamadas sin sonido o que se cortan, un fallo habitual de este ayudante, y cierra una vía usada en ataques para abrir puertos. En contra: algún teléfono por internet antiguo puede necesitarlo para funcionar.
ajuste-puertos-alg-pptp = Redes privadas (PPTP)
ajuste-puertos-alg-pptp-desc = Deja que los ordenadores de tu red se conecten a redes privadas (VPN) antiguas de tipo PPTP.
ajuste-puertos-alg-h323 = Videoconferencia (H.323)
ajuste-puertos-alg-h323-desc = Hace que funcionen los equipos de videoconferencia antiguos que usan H.323.
