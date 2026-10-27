# Ajustes del tema Seguridad: gestión remota del operador, contraseña del
# router y usuarios de la web del router.

## Gestión remota del operador (TR-069)
ajuste-seguridad-gestion_remota-titulo = Gestión remota del operador
ajuste-seguridad-gestion_remota-desc = Permite que tu operador vea y cambie la configuración del router a distancia, sin pasar por tu casa.
ajuste-seguridad-gestion_remota-ayuda = Con ella encendida, el operador puede actualizar el router, revisarlo cuando llamas a soporte y cambiar cualquier ajuste, incluidas las contraseñas de la WiFi, sin pedirte permiso. Por ella llegan también las actualizaciones que corrigen fallos de seguridad. La otra cara es que el router queda esperando llamadas del operador desde internet: esa vía ha sido la entrada de ataques conocidos contra routers de otros operadores, aunque solo es un peligro si el router tiene un fallo sin corregir. Si la apagas, nadie de fuera puede cambiar el router, pero deja de recibir actualizaciones, el soporte técnico no puede revisarlo a distancia, la aplicación del operador para gestionar el router puede dejar de funcionar y, si el operador cambia algo de tu línea, el cambio no llegará al router. Lo que ya funciona (internet, teléfono y televisión) sigue igual. La recomendación de apagarla es para quien prioriza su privacidad y seguridad; si prefieres que el operador lo mantenga todo al día, déjala encendida.
ajuste-seguridad-gestion_remota-activa = Gestión remota
ajuste-seguridad-gestion_remota-activa-desc = Si el operador puede entrar en el router a distancia.
ajuste-seguridad-gestion_remota-activa-encendida = Encendida
ajuste-seguridad-gestion_remota-activa-encendida-desc = Recibes las actualizaciones y el soporte a distancia del operador; a cambio, puede cambiar el router sin preguntarte.
ajuste-seguridad-gestion_remota-activa-apagada = Apagada
ajuste-seguridad-gestion_remota-activa-apagada-desc = Nadie de fuera puede cambiar el router; a cambio, dejas de recibir las actualizaciones y el soporte a distancia del operador.
ajuste-seguridad-gestion_remota-activa-recomendado = Recomendado si priorizas tu privacidad y seguridad. A favor: el operador deja de poder cambiar la configuración a distancia y se cierra una vía de ataque conocida. En contra: dejarás de recibir las actualizaciones automáticas del operador, su soporte no podrá revisar el router a distancia y los cambios que haga en tu línea, también en el teléfono, no llegarán al router.

## Gestión remota: aviso periódico
ajuste-seguridad-gestion_remota_avanzado-titulo = Aviso periódico al operador
ajuste-seguridad-gestion_remota_avanzado-desc = Cada cuánto tiempo el router se pone en contacto con el operador por su cuenta.
ajuste-seguridad-gestion_remota_avanzado-ayuda = Además de cuando el operador lo llama, el router se comunica con él cada cierto tiempo para decirle que sigue ahí y preguntarle si tiene cambios o actualizaciones pendientes. Lo deja puesto el operador: si no tienes un motivo, no lo cambies. Solo cuenta con la gestión remota encendida.
ajuste-seguridad-gestion_remota_avanzado-aviso = Aviso periódico
ajuste-seguridad-gestion_remota_avanzado-aviso-desc = Si lo apagas, el router habla con el operador sobre todo cuando se enciende, cuando cambian algunos de sus ajustes o cuando el operador lo llama.
ajuste-seguridad-gestion_remota_avanzado-intervalo = Cada cuánto
ajuste-seguridad-gestion_remota_avanzado-intervalo-desc = El tiempo entre un aviso y el siguiente.
ajuste-seguridad-gestion_remota_avanzado-intervalo-hora = Cada hora
ajuste-seguridad-gestion_remota_avanzado-intervalo-hora-desc = El operador sabe enseguida si el router deja de responder.
ajuste-seguridad-gestion_remota_avanzado-intervalo-dia = Cada día
ajuste-seguridad-gestion_remota_avanzado-intervalo-dia-desc = Un aviso al día.
ajuste-seguridad-gestion_remota_avanzado-intervalo-semana = Cada semana
ajuste-seguridad-gestion_remota_avanzado-intervalo-semana-desc = Un aviso a la semana: el router habla menos con el operador.

## Gestión remota: servidor y claves
ajuste-seguridad-gestion_remota_oculto-titulo = Servidor y claves de la gestión remota
ajuste-seguridad-gestion_remota_oculto-desc = La dirección del servidor del operador y las claves con las que el router y el operador se reconocen.
ajuste-seguridad-gestion_remota_oculto-ayuda = Estos datos los pone el operador; las contraseñas salen censuradas, con un ojo para verlas. Si escribes uno mal, el operador deja de poder gestionar el router hasta que deshagas el cambio. Cámbialos solo si el operador te lo pide.
ajuste-seguridad-gestion_remota_oculto-servidor = Dirección del servidor del operador
ajuste-seguridad-gestion_remota_oculto-servidor-desc = La dirección web con la que el router se pone en contacto, por ejemplo https://gestion.example. Sin espacios, tildes ni ñ.
ajuste-seguridad-gestion_remota_oculto-usuario = Usuario ante el operador
ajuste-seguridad-gestion_remota_oculto-usuario-desc = El nombre con el que el router se presenta al servidor del operador.
ajuste-seguridad-gestion_remota_oculto-contrasena = Contraseña ante el operador
ajuste-seguridad-gestion_remota_oculto-contrasena-desc = La contraseña con la que el router se presenta al servidor del operador.
ajuste-seguridad-gestion_remota_oculto-usuario_entrada = Usuario para las llamadas del operador
ajuste-seguridad-gestion_remota_oculto-usuario_entrada-desc = El nombre que tiene que dar el operador cuando es él quien llama al router.
ajuste-seguridad-gestion_remota_oculto-contrasena_entrada = Contraseña para las llamadas del operador
ajuste-seguridad-gestion_remota_oculto-contrasena_entrada-desc = La contraseña que tiene que dar el operador cuando es él quien llama al router.

## Contraseña del router
ajuste-seguridad-contrasena_router-titulo = Contraseña del router
ajuste-seguridad-contrasena_router-desc = La contraseña con la que se entra en la web del router y con la que entra este panel.
ajuste-seguridad-contrasena_router-ayuda = Si sigue siendo la que venía en la pegatina del router, cualquiera que la vea puede entrar en él: cámbiala por una tuya. El panel prueba la nueva antes de quedarse con ella y, si algo falla, vuelve a poner la anterior.

## Usuarios del router
ajuste-seguridad-usuarios_router-titulo = Cuentas de la web del router
ajuste-seguridad-usuarios_router-desc = Las cuentas de la web del router y lo que puede hacer cada una.
ajuste-seguridad-usuarios_router-ayuda = En este router, la web solo deja entrar a la cuenta 1234: las demás cuentas existen, pero no pueden entrar en la web. Hay además una cuenta del operador que la web del router no enseña: es la que usa su soporte técnico para entrar en el router. Por eso aquí puede aparecer una cuenta más de las que ves allí. Ni esa cuenta ni la que usa este panel para entrar se pueden borrar o cambiar desde aquí.
ajuste-seguridad-usuarios_router-motivo = Se cambia con los botones de cada cuenta, no aquí.
ajuste-seguridad-usuarios_router-nombre = Nombre de la cuenta
ajuste-seguridad-usuarios_router-nombre-desc = El nombre de la cuenta en la web del router.
ajuste-seguridad-usuarios_router-papel = Tipo de cuenta
ajuste-seguridad-usuarios_router-papel-desc = Lo que puede hacer la cuenta en la web del router.
ajuste-seguridad-usuarios_router-papel-operador = Operador
ajuste-seguridad-usuarios_router-papel-operador-desc = La cuenta del soporte técnico del operador: puede tocarlo todo.
ajuste-seguridad-usuarios_router-papel-administrador = Administrador
ajuste-seguridad-usuarios_router-papel-administrador-desc = Puede cambiar toda la configuración que enseña la web del router.
ajuste-seguridad-usuarios_router-papel-usuario = Usuario
ajuste-seguridad-usuarios_router-papel-usuario-desc = Una cuenta con menos permisos que la de administrador.
ajuste-seguridad-usuarios_router-crear = Crear la cuenta en el router
ajuste-seguridad-usuarios_router-crear-titulo = Cuenta nueva en el router
ajuste-seguridad-usuarios_router-nuevo-nombre = Nombre
ajuste-seguridad-usuarios_router-nuevo-nombre-ayuda = De 4 a 20 letras sin tilde, números o «_», sin espacios.
ajuste-seguridad-usuarios_router-contrasena = Contraseña
ajuste-seguridad-usuarios_router-contrasena-ayuda = De { $min } a { $max } caracteres: letras sin tilde, números, espacios y signos del teclado inglés, sin «`». Sin espacio al principio ni al final, ni dos seguidos. Pulsa el ojo para comprobar lo que has escrito.
ajuste-seguridad-usuarios_router-cambiar = Cambiar contraseña o grupo
ajuste-seguridad-usuarios_router-cambiar-de = Cambiar contraseña o grupo de { $nombre }
ajuste-seguridad-usuarios_router-cambiar-boton = Guardar los cambios
ajuste-seguridad-usuarios_router-cambiar-nota = La web del router pide una contraseña nueva también para cambiar solo el grupo. El panel comprueba que la cuenta entra con ella; la anterior no se puede recuperar, así que este cambio no se deshace.
ajuste-seguridad-usuarios_router-cambiado = Hecho: { $nombre } ya entra con la contraseña nueva.
ajuste-seguridad-usuarios_router-borrar = Borrar
ajuste-seguridad-usuarios_router-borrar-de = Borrar { $nombre }
ajuste-seguridad-usuarios_router-vacia = El router no tiene ningún usuario en su web.

## Cortafuegos (fase 3): las reglas, solo de lectura, y la regla propia del
## panel para no responder al ping desde internet.

ajuste-seguridad-cortafuegos-titulo = Cortafuegos
ajuste-seguridad-cortafuegos-desc = Las reglas que deciden qué puede entrar en el router desde internet.
ajuste-seguridad-cortafuegos-ayuda = El router trae unas reglas del operador que deciden qué deja entrar desde internet y desde dónde. Aquí se ven por filtros, en el orden en que el router las mira; cambiar una puede dejarte sin acceso desde fuera, así que revisa cada cambio antes de aplicarlo y deshazlo si algo deja de funcionar.
ajuste-seguridad-cortafuegos-motivo = Las reglas se cambian desde el editor del cortafuegos, filtro a filtro.
ajuste-seguridad-cortafuegos-filtro = Filtro
ajuste-seguridad-cortafuegos-filtro-desc = El grupo de reglas al que pertenece, según la conexión y el tipo de dirección.
ajuste-seguridad-cortafuegos-version = Direcciones
ajuste-seguridad-cortafuegos-version-desc = Si la regla vale para las direcciones IPv4 o para las IPv6.
ajuste-seguridad-cortafuegos-version-ipv4 = IPv4
ajuste-seguridad-cortafuegos-version-ipv4-desc = Las direcciones de siempre, como 192.0.2.1.
ajuste-seguridad-cortafuegos-version-ipv6 = IPv6
ajuste-seguridad-cortafuegos-version-ipv6-desc = Las direcciones nuevas, más largas.
ajuste-seguridad-cortafuegos-regla_activa = Activa
ajuste-seguridad-cortafuegos-regla_activa-desc = Si el router aplica la regla.
ajuste-seguridad-cortafuegos-protocolo = Tráfico
ajuste-seguridad-cortafuegos-protocolo-desc = El tipo de tráfico al que se aplica la regla.
ajuste-seguridad-cortafuegos-protocolo-icmp = ICMP
ajuste-seguridad-cortafuegos-protocolo-icmp-desc = Mensajes de control, como el ping.
ajuste-seguridad-cortafuegos-protocolo-tcp = TCP
ajuste-seguridad-cortafuegos-protocolo-tcp-desc = Conexiones como las de la web o SSH.
ajuste-seguridad-cortafuegos-protocolo-udp = UDP
ajuste-seguridad-cortafuegos-protocolo-udp-desc = Mensajes sueltos, como los de DNS.
ajuste-seguridad-cortafuegos-accion = Qué hace
ajuste-seguridad-cortafuegos-accion-desc = Lo que hace el router con el tráfico que encaja con la regla.
ajuste-seguridad-cortafuegos-accion-descartar = Lo descarta
ajuste-seguridad-cortafuegos-accion-descartar-desc = No contesta: para quien llama, el router no está.
ajuste-seguridad-cortafuegos-accion-rechazar = Lo rechaza
ajuste-seguridad-cortafuegos-accion-rechazar-desc = Contesta que no.
ajuste-seguridad-cortafuegos-accion-permitir = Lo deja pasar
ajuste-seguridad-cortafuegos-accion-permitir-desc = Lo deja entrar.
ajuste-seguridad-cortafuegos-icmp = Mensaje de control
ajuste-seguridad-cortafuegos-icmp-desc = Qué mensajes de control abarca la regla.
ajuste-seguridad-cortafuegos-icmp-ninguno = —
ajuste-seguridad-cortafuegos-icmp-ninguno-desc = La regla no es de mensajes de control.
ajuste-seguridad-cortafuegos-icmp-cualquiera = Cualquiera
ajuste-seguridad-cortafuegos-icmp-cualquiera-desc = Todos los mensajes de control, también el ping.
ajuste-seguridad-cortafuegos-icmp-eco = Ping (IPv4)
ajuste-seguridad-cortafuegos-icmp-eco-desc = La petición de ping de IPv4.
ajuste-seguridad-cortafuegos-icmp-eco_v6 = Ping (IPv6)
ajuste-seguridad-cortafuegos-icmp-eco_v6-desc = La petición de ping de IPv6.
ajuste-seguridad-cortafuegos-origen = Desde
ajuste-seguridad-cortafuegos-origen-desc = De dónde puede venir el tráfico; 0.0.0.0 es cualquier sitio.
ajuste-seguridad-cortafuegos-origen_mascara = Máscara del origen
ajuste-seguridad-cortafuegos-origen_mascara-desc = Cuántas direcciones abarca el origen.
ajuste-seguridad-cortafuegos-puerto_desde = Puerto desde
ajuste-seguridad-cortafuegos-puerto_desde-desc = El primer puerto del router al que va el tráfico; 0 es cualquiera.
ajuste-seguridad-cortafuegos-puerto_hasta = Puerto hasta
ajuste-seguridad-cortafuegos-puerto_hasta-desc = El último puerto del router al que va el tráfico.
ajuste-seguridad-cortafuegos-sin-ping = No responder al ping desde internet
ajuste-seguridad-cortafuegos-sin-ping-ayuda = Añade una regla del panel que descarta el ping que llega de internet por IPv4 a la conexión del router; el ping por IPv6 no lo toca. Las reglas del operador no se tocan, y «Deshacer» quita solo esta.
ajuste-seguridad-cortafuegos-desactivada = La regla del panel está en el router pero desactivada: no descarta nada. El panel no la activa; actívala o bórrala desde la web del router.
ajuste-seguridad-cortafuegos-ya-puesta = Regla del panel ya puesta. Es solo para IPv4: el ping por IPv6 no lo toca.
ajuste-seguridad-cortafuegos-puesta-sin-efecto = Regla del panel puesta; el router sigue respondiendo al ping por otra regla del operador.
ajuste-seguridad-cortafuegos-sin-leer = No se pueden leer las reglas del cortafuegos: sin ellas, el panel no añade la suya.
ajuste-seguridad-cortafuegos-sin-filtro = El cortafuegos no tiene el grupo de reglas de entrada de la conexión a internet: el panel no sabe dónde poner la suya.
ajuste-seguridad-cortafuegos-sin-ping-recomendado = Recomendado si quieres que tu router pase más desapercibido en internet. A favor: la regla descarta el ping que llega por IPv4, y a quien recorre internet buscando equipos le cuesta más ver que el tuyo está encendido. En contra: no podrás comprobar con un ping desde fuera que tu conexión funciona, y algunas pruebas del operador podrían fallar.
ajuste-seguridad-cortafuegos-activar-recomendado = Recomendado si no necesitas que el router conteste al ping desde internet: la regla del panel ya está en el router, pero desactivada, y el panel no la activa; actívala desde la web del router. A favor: descartaría el ping que llega por IPv4, y a quien recorre internet buscando equipos le costaría más ver que el tuyo está encendido. En contra: no podrás comprobar con un ping desde fuera que tu conexión funciona, y algunas pruebas del operador podrían fallar.
ajuste-seguridad-cortafuegos-revisar-recomendado = Recomendado si no entras en el router desde fuera de casa: el panel no cambia esas reglas, pero puedes pedir al operador que cierre lo que no uses. A favor: menos puertas abiertas desde internet. En contra: el operador puede necesitarlas para darte soporte o poner al día el router.
ajuste-seguridad-cortafuegos-servicio-ping = el ping
ajuste-seguridad-cortafuegos-servicio-web = la web del router
ajuste-seguridad-cortafuegos-servicio-ssh = el SSH
ajuste-seguridad-cortafuegos-servicio-gestion = la gestión remota del operador
ajuste-seguridad-cortafuegos-servicio-ping-ipv4 = el ping por IPv4
ajuste-seguridad-cortafuegos-servicio-ping-ipv6 = el ping por IPv6
ajuste-seguridad-cortafuegos-y = y
