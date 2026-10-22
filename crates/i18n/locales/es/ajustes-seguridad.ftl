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
ajuste-seguridad-usuarios_router-titulo = Usuarios de la web del router
ajuste-seguridad-usuarios_router-desc = Las cuentas que pueden entrar en la web del router y lo que puede hacer cada una.
ajuste-seguridad-usuarios_router-ayuda = Hay una cuenta del operador que la web del router no enseña: es la que usa su soporte técnico para entrar en el router. Por eso aquí puede aparecer una cuenta más de las que ves allí. Los nombres de las cuentas no se muestran, como el resto de las claves.
ajuste-seguridad-usuarios_router-motivo = De momento solo se puede ver. Cambiar la contraseña de otra cuenta podría cambiar la que usa este panel y dejarlo fuera; llegará cuando se haya comprobado con el router de verdad.
ajuste-seguridad-usuarios_router-nombre = Nombre de la cuenta
ajuste-seguridad-usuarios_router-nombre-desc = El nombre con el que se entra; no se muestra.
ajuste-seguridad-usuarios_router-papel = Tipo de cuenta
ajuste-seguridad-usuarios_router-papel-desc = Lo que puede hacer la cuenta en la web del router.
ajuste-seguridad-usuarios_router-papel-operador = Operador
ajuste-seguridad-usuarios_router-papel-operador-desc = La cuenta del soporte técnico del operador: puede tocarlo todo.
ajuste-seguridad-usuarios_router-papel-administrador = Administrador
ajuste-seguridad-usuarios_router-papel-administrador-desc = Puede cambiar toda la configuración que enseña la web del router.
ajuste-seguridad-usuarios_router-papel-usuario = Usuario
ajuste-seguridad-usuarios_router-papel-usuario-desc = Una cuenta con menos permisos que la de administrador.
