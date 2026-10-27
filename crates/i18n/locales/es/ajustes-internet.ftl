# Ajustes del tema Internet: conexión a internet y fibra, relé DNS y DNS dinámico.

## Relé DNS
ajuste-internet-dns-titulo = Relé DNS
ajuste-internet-dns-desc = El DNS traduce los nombres de las páginas web a direcciones. Con el relé, el router reenvía esas consultas de tus dispositivos a los servidores DNS de tu conexión.
ajuste-internet-dns-ayuda = Si no sabes para qué lo quieres cambiar, déjalo como está. Un cambio aquí puede hacer que los dispositivos de tu red dejen de encontrar las páginas web aunque internet siga conectado; si pasa, vuelve al valor anterior.
ajuste-internet-dns-rele = Relé DNS activado
ajuste-internet-dns-rele-desc = En principio, el router pasa las consultas de tus dispositivos a los servidores DNS de tu conexión. Con el relé apagado ya las contesta él; qué cambia al encenderlo está aún por comprobar con este router.

## Nombre fijo para tu conexión (DNS dinámico)
ajuste-internet-ddns-titulo = Nombre fijo para tu conexión (DNS dinámico)
ajuste-internet-ddns-desc = Da a tu conexión un nombre que no cambia (como micasa.example) aunque cambie tu dirección de internet, para llegar a casa desde fuera.
ajuste-internet-ddns-ayuda = Necesitas una cuenta en uno de los servicios de DNS dinámico que admite el router. El router avisa al servicio cada vez que cambia tu dirección de internet. Solo es útil si tienes algo en casa a lo que entrar desde fuera, como una cámara con un puerto abierto.
ajuste-internet-ddns-activa = Activa
ajuste-internet-ddns-activa-desc = Si la desactivas, el router deja de avisar al servicio y el nombre puede quedarse con una dirección antigua.
ajuste-internet-ddns-servicio = Servicio
ajuste-internet-ddns-servicio-desc = El servicio de DNS dinámico en el que tienes la cuenta.
ajuste-internet-ddns-servicio-no_ip = No-IP
ajuste-internet-ddns-servicio-no_ip-desc = El servicio No-IP.
ajuste-internet-ddns-servicio-now_ip = Now-IP
ajuste-internet-ddns-servicio-now_ip-desc = El servicio Now-IP.
ajuste-internet-ddns-servicio-dyndns = DynDNS
ajuste-internet-ddns-servicio-dyndns-desc = El servicio DynDNS.
ajuste-internet-ddns-servicio-now_dns = Now-DNS
ajuste-internet-ddns-servicio-now_dns-desc = El servicio Now-DNS.
ajuste-internet-ddns-nombre = Nombre
ajuste-internet-ddns-nombre-desc = El nombre completo que diste de alta en el servicio, por ejemplo micasa.example.
ajuste-internet-ddns-cuenta = Cuenta
ajuste-internet-ddns-cuenta-desc = El usuario o el correo de tu cuenta en el servicio.
ajuste-internet-ddns-contrasena = Contraseña
ajuste-internet-ddns-contrasena-desc = La contraseña (o la clave que te dé el servicio) de esa cuenta.

## Conexión a internet (solo de lectura)
ajuste-internet-conexion-titulo = Conexión a internet
ajuste-internet-conexion-desc = Cómo se conecta el router a internet: el tipo de conexión, la dirección que te da el operador y sus datos.
ajuste-internet-conexion-ayuda = El router tiene varias conexiones con el operador. Una lleva internet; las demás son del operador (el teléfono, sus servicios) y no hace falta tocarlas. El panel solo las muestra: cambiar la conexión puede dejarte sin internet.
ajuste-internet-conexion-motivo = La conexión a internet no se cambia desde el panel: un error te dejaría sin internet.
ajuste-internet-conexion-tipo = Tipo
ajuste-internet-conexion-tipo-desc = Cómo se identifica el router ante el operador.
ajuste-internet-conexion-tipo-pppoe = PPPoE
ajuste-internet-conexion-tipo-pppoe-desc = Con un usuario y una contraseña del operador.
ajuste-internet-conexion-tipo-dhcp = Automática (DHCP)
ajuste-internet-conexion-tipo-dhcp-desc = El operador da la dirección sola, sin usuario ni contraseña.
ajuste-internet-conexion-tipo-fija = Dirección fija
ajuste-internet-conexion-tipo-fija-desc = Una dirección puesta a mano que no cambia.
ajuste-internet-conexion-estado = Estado
ajuste-internet-conexion-estado-desc = Si la conexión está en marcha ahora.
ajuste-internet-conexion-estado-true = En marcha
ajuste-internet-conexion-estado-false = Parada
ajuste-internet-conexion-ip = Dirección
ajuste-internet-conexion-ip-desc = La dirección que tiene ahora; en la de internet, tu dirección pública.
ajuste-internet-conexion-puerta = Puerta de enlace
ajuste-internet-conexion-puerta-desc = El equipo del operador al que el router manda el tráfico.
ajuste-internet-conexion-dns_1 = DNS del operador
ajuste-internet-conexion-dns_1-desc = El primer servidor DNS que da el operador.
ajuste-internet-conexion-dns_2 = DNS del operador (segundo)
ajuste-internet-conexion-dns_2-desc = El servidor DNS de reserva que da el operador.
ajuste-internet-conexion-vlan = VLAN
ajuste-internet-conexion-vlan-desc = La etiqueta con la que el operador separa esta conexión de las demás por la misma fibra.
ajuste-internet-conexion-mtu = MTU
ajuste-internet-conexion-mtu-desc = El tamaño máximo de cada paquete, en bytes.
ajuste-internet-conexion-ipv6 = IPv6
ajuste-internet-conexion-ipv6-desc = Si la conexión tiene también direcciones IPv6 en marcha.
ajuste-internet-conexion-ipv6-true = En marcha
ajuste-internet-conexion-ipv6-false = Sin IPv6
ajuste-internet-conexion-usuario = Usuario
ajuste-internet-conexion-usuario-desc = El usuario con el que el router entra en el operador.
ajuste-internet-conexion-contrasena = Contraseña
ajuste-internet-conexion-contrasena-desc = La contraseña de ese usuario. Hace falta si pones un router tuyo en su lugar.
ajuste-internet-conexion-del-operador = Del operador (no es tu internet)
ajuste-internet-conexion-internet = Tu conexión a internet
ajuste-internet-conexion-sin-identificar = Conexión { $id }: no se sabe si es la de internet
ajuste-internet-conexion-monopuesto-titulo = Cómo pasar a monopuesto
ajuste-internet-conexion-monopuesto-pasos =
    Apunta el usuario y la contraseña de tu conexión a internet (arriba, con el ojo).
    Pon la conexión a internet de este router en monopuesto (puente). Cómo se hace depende del modelo y del operador: si no lo tienes claro, pregunta a tu operador.
    Conecta tu router propio por cable a este router.
    En tu router, configura la conexión a internet por PPPoE con ese usuario y esa contraseña. Va en su propia VLAN: pon la que use tu operador para internet (en Movistar suele ser la 6).
    Comprueba que tu router tiene internet y conecta tus dispositivos a él.
ajuste-internet-conexion-monopuesto-avisos =
    Es para quien tiene un router propio detrás de este. Si no lo tienes, no lo hagas.
    Mientras lo cambias, te quedas sin internet hasta que tu router conecte.
    La WiFi, las direcciones de tu red y los puertos pasan a tu router: los de este dejan de servir y puede que el panel ya no pueda entrar en él.
    La televisión y el teléfono del operador pueden dejar de funcionar si dependen de este router.
    El panel no lo hace por ti ni sabe deshacerlo. Si algo sale mal y no hay otra forma de volver atrás, el último recurso es restablecer este router de fábrica y configurarlo de nuevo.

## Fibra
ajuste-internet-fibra-titulo = Fibra
ajuste-internet-fibra-desc = El estado de la fibra que llega al router y sus datos de identificación ante el operador.
ajuste-internet-fibra-ayuda = Si la fibra no está «Operativa», el router no tiene internet aunque todo lo demás esté bien. Los números de serie y la contraseña identifican tu router ante el operador: no los compartas.
ajuste-internet-fibra-motivo = La fibra solo se consulta: sus datos los pone el operador.
ajuste-internet-fibra-estado = Estado de la fibra
ajuste-internet-fibra-estado-desc = En qué punto está el router al conectarse por la fibra con el operador.
ajuste-internet-fibra-estado-o1 = Inicial (O1)
ajuste-internet-fibra-estado-o1-desc = Acaba de encenderse o no recibe señal por la fibra.
ajuste-internet-fibra-estado-o2 = En espera (O2)
ajuste-internet-fibra-estado-o2-desc = Recibe señal y espera a que el operador le hable.
ajuste-internet-fibra-estado-o3 = Identificándose (O3)
ajuste-internet-fibra-estado-o3-desc = Da su número de serie al operador.
ajuste-internet-fibra-estado-o4 = Ajustando la distancia (O4)
ajuste-internet-fibra-estado-o4-desc = El operador mide lo lejos que está para ordenar el tráfico.
ajuste-internet-fibra-estado-o5 = Operativa (O5)
ajuste-internet-fibra-estado-o5-desc = Conectada y funcionando con normalidad.
ajuste-internet-fibra-estado-o6 = Señal intermitente (O6)
ajuste-internet-fibra-estado-o6-desc = Ha perdido la señal un momento y trata de recuperarla.
ajuste-internet-fibra-estado-o7 = Parada por el operador (O7)
ajuste-internet-fibra-estado-o7-desc = El operador ha cortado la conexión de este router.
ajuste-internet-fibra-serie = Número de serie de la fibra
ajuste-internet-fibra-serie-desc = Identifica el router ante el operador en la fibra.
ajuste-internet-fibra-serie_acs = Número de serie para el operador
ajuste-internet-fibra-serie_acs-desc = El que usa el operador para gestionar el router a distancia.
ajuste-internet-fibra-contrasena = Contraseña de la fibra
ajuste-internet-fibra-contrasena-desc = La clave con la que el router se identifica en la fibra.
