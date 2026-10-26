# Router: ajustes del router. Su primera página es la conexión.
router-conexion = Conexión con el router
router-conexion-sub = Dirección de la web del router y el usuario y la contraseña con los que entra el panel, y el firmware que tiene; aquí también se cambia la contraseña del propio router. Se guardan en la base de datos del panel.

## Credenciales
router-credenciales = Contraseña con la que el panel entra en el router
router-texto = La dirección, el usuario y la contraseña que usa el panel para entrar en el router. Guardarlos aquí no cambia nada en el router: sirve si la contraseña se ha cambiado desde la propia web del router. El panel prueba a entrar antes de guardar.
router-sin-configurar = El router aún no está configurado.
router-url = Dirección del router
router-usuario = Usuario del router
router-contrasena = Contraseña que usa el panel
router-contrasena-ayuda = La que tiene guardada el panel. Pulsa el ojo para verla; si no la tocas, se queda la guardada.
router-probar = Probar la conexión
router-guardar = Guardar las credenciales
router-error-escribe-contrasena = Con otra dirección u otro usuario, escribe también la contraseña: la guardada solo se usa con los datos guardados.
router-guardadas = Credenciales guardadas: el panel ya usa las nuevas.

## Firmware
router-firmware-titulo = Firmware
router-firmware = Firmware: { $firmware }
router-firmware-desconocido = Firmware: aún sin leer
router-modo-solo-lectura = Solo lectura: el firmware no se ha probado con el panel
router-modo-escritura = Se pueden hacer cambios
router-confirmar-texto = Si sabes que este firmware se comporta como el probado, confírmalo para poder hacer cambios. El gestor de cambios sigue verificando y deshaciendo lo que falle.
router-confirmar = Confirmar el firmware

## Red del router
router-red-titulo = Red del router
router-red-texto = Lo que el panel ha detectado del router. La conexión a internet es la que vigila la red de seguridad de los cambios: si se cae mientras se vigila un cambio, el cambio se deshace.
router-red-sin-detectar = Aún no se ha detectado la red del router.
router-red-internet = Internet: { $conexion }
router-red-internet-tipo = Internet: { $conexion } ({ $tipo })
router-red-internet-sin-detectar = Internet: sin detectar
router-red-lan = LAN { $subred }
router-red-lan-sin-detectar = LAN: sin detectar
router-red-telefonia-si = Telefonía: sí
router-red-telefonia-no = Telefonía: no
router-red-television-si = Televisión: sí
router-red-television-no = Televisión: no
router-red-tipo-pppoe = PPPoE
router-red-tipo-dhcp = DHCP
router-red-tipo-estatica = IP fija
router-red-tipo-otro = otro tipo
router-red-conexion = Conexión a internet
router-red-conexion-ayuda = Normalmente, la detectada. Elige otra solo si el panel se equivoca.
router-red-automatica = Automática: la detectada ({ $conexion })
router-red-automatica-sin-detectar = Automática: la detectada (ninguna por ahora)
router-red-manual-no-existe = La conexión elegida, { $conexion }, no existe en el router: mientras, se usa la detectada.
router-red-guardar = Guardar la conexión
router-red-guardada = Conexión a internet guardada.
router-red-redetectar = Volver a detectar
router-red-redetectada = Red del router detectada de nuevo.
router-error-cambio-en-curso = Hay un cambio en curso en el router: espera a que termine para cambiar la conexión a internet o volver a detectarla.
router-error-conexion-no-existe = El router no tiene ninguna conexión { $nombre }.

## Cambiar la contraseña del router
contrasena-router-titulo = Cambiar la contraseña del router
contrasena-router-texto = Cambia la contraseña de la web del router. Desde ese momento, el panel entra con la nueva y tú también tendrás que usarla para entrar en el router. El panel prueba la nueva antes de quedarse con ella y, si algo falla, vuelve a poner la anterior.
contrasena-router-nueva = Contraseña nueva
contrasena-router-nueva-ayuda = Entre { $min } y { $max } caracteres: letras sin tilde, números, espacios y signos del teclado inglés, sin «`». Sin espacio al principio ni al final, ni dos seguidos. Pulsa el ojo para comprobar lo que has escrito.
contrasena-router-solo-lectura = No se puede cambiar mientras el panel esté en solo lectura: primero confirma el firmware en la tarjeta «Firmware».
contrasena-router-cambiar = Cambiar la contraseña
contrasena-router-cambiando = Cambiando…
contrasena-router-cambiada = Contraseña del router cambiada: el panel ya entra con la nueva. Para entrar tú en la web del router, usa también la nueva.
contrasena-router-confirmar-titulo = ¿Cambiar la contraseña del router?
contrasena-router-confirmar-texto = El router pasará a tener la contraseña nueva. A partir de ese momento, para entrar en el router (también desde su propia web) se usa la nueva; el panel lo hará solo.
contrasena-router-confirmar-nota = Si el panel no consigue entrar con la nueva, vuelve a poner la anterior. Tarda unos segundos: no cierres la página.
contrasena-router-confirmar-boton = Cambiar la contraseña
# Las claves contrasena-router-error-* y contrasena-router-historial-nota
# (y panel-solo-lectura y comun-error-interno, en sus ficheros) se guardan
# en la base de datos de cada instalación, en el Historial: no se renombran
# ni se borran (solo se cambia su texto).
contrasena-router-error-corta = La contraseña nueva es demasiado corta: tiene que tener al menos { $n } caracteres.
contrasena-router-error-larga = La contraseña nueva es demasiado larga: como mucho { $n } caracteres.
contrasena-router-error-control = La contraseña nueva no puede llevar saltos de línea, tabuladores ni otros caracteres invisibles.
contrasena-router-error-caracter = La contraseña nueva solo puede llevar letras sin tilde, números, espacios y signos del teclado inglés, sin «`»: ni ñ, ni tildes, ni otros símbolos.
contrasena-router-error-espacios = La contraseña nueva no puede empezar ni acabar con un espacio, ni llevar dos espacios seguidos.
contrasena-router-error-igual = La contraseña nueva es la misma que ya tiene el router.
contrasena-router-error-en-curso = Hay otro cambio en curso en el router: espera a que termine para cambiar la contraseña.
contrasena-router-error-sin-router = El panel aún no sabe cómo entrar en el router: guarda primero su dirección, usuario y contraseña.
contrasena-router-error-sondeo = El panel no pudo pausar a tiempo sus lecturas del router y no ha cambiado nada. Vuelve a intentarlo en un momento.
contrasena-router-error-rechazada = El router no aceptó la contraseña nueva: sigue con la anterior, y el panel también.
contrasena-router-error-restaurada = El panel no pudo comprobar que la contraseña nueva funciona, así que ha vuelto a poner la anterior en el router. No ha cambiado nada.
contrasena-router-error-no-guardada = El router aceptó la contraseña nueva, pero el panel no pudo guardarla, así que ha vuelto a poner la anterior. No ha cambiado nada.
contrasena-router-error-grave = El panel no consigue entrar en el router ni con la contraseña nueva ni con la anterior. Para recuperar el acceso, entra en la web del router (prueba con la nueva y, si no, con la anterior) y pon a mano la contraseña que el panel tiene guardada: la ves pulsando el ojo en «Contraseña con la que el panel entra en el router», en esta misma página.
contrasena-router-historial-nota = Este cambio no se puede deshacer desde aquí: para poner otra contraseña, vuelve a cambiarla en Configuración del router.
contrasena-router-enlace = Cambiar la contraseña del router
contrasena-router-enlace-texto = Se cambia en Configuración del router, junto a la contraseña con la que entra el panel.

## Usuarios de la web del router (Ajustes → Seguridad → Usuarios)
# Las claves usuarios-router-error-* y usuarios-router-historial-nota quedan
# guardadas en el Historial: no se renombran ni se borran.
usuarios-router-contrasena = Contraseña
usuarios-router-protegido-panel = Es el usuario con el que entra el panel: no se puede borrar ni cambiar desde aquí.
usuarios-router-protegido-sistema = Es la cuenta del operador: no se puede borrar ni cambiar desde aquí.
usuarios-router-sin-credenciales = El panel no sabe con qué usuario entra en el router: desde aquí no se puede tocar ninguna cuenta.
usuarios-router-aviso-borrar = Si luego deshaces el borrado, el usuario vuelve con una contraseña nueva: la de ahora no se puede recuperar. La nueva solo se verá con el ojo, en el detalle del cambio en el Historial. Si es el panel el que lo devuelve solo (porque el cambio falla), esa contraseña no se guarda: el Historial te avisará para que la cambies.
usuarios-router-aviso-deshacer = El usuario vuelve con una contraseña nueva: la de antes no se puede recuperar. La verás con el ojo en el detalle de este cambio, en el Historial.
usuarios-router-error-protegido = Este usuario está protegido: es el que usa el panel para entrar en el router o la cuenta del operador. No se puede borrar ni cambiar desde aquí.
usuarios-router-error-existe = Ya hay un usuario con ese nombre en el router.
usuarios-router-error-nombre = El nombre tiene que tener de 4 a 20 letras sin tilde, números o «_», sin espacios.
usuarios-router-error-no-existe = Ese usuario ya no está en el router: vuelve a leer la lista.
usuarios-router-error-en-curso = Hay otro cambio en curso en el router: espera a que termine.
usuarios-router-error-rechazado = El router no aceptó el cambio: el usuario sigue como estaba.
usuarios-router-error-no-entra = El router respondió, pero el usuario no entra con la contraseña nueva. Prueba otra vez o cámbiala desde la web del router.
usuarios-router-error-grupo = El usuario ya entra con la contraseña nueva, pero el router no muestra el tipo de cuenta pedido. Vuelve a leer la lista.
usuarios-router-historial-nota = La contraseña nueva no se guarda en el panel: este cambio no se puede deshacer desde aquí.
usuarios-router-error-uno-cada-vez = Los usuarios se borran de uno en uno.
usuarios-router-error-nombre-baja = Ese nombre no se puede borrar desde el panel: lleva «:» o caracteres de control.
usuarios-router-error-grupo-sistema = Ese tipo de cuenta es el del operador: elige Administrador o Usuario.
usuarios-router-error-grupo-no-valido = Elige un tipo de cuenta: Administrador o Usuario.
usuarios-router-nota-revertido = Se volvió a crear el usuario borrado con una contraseña nueva que el panel no guarda: cámbiala en Ajustes → Seguridad → Cuentas de la web del router para poder usarlo.

## Funciones pendientes de comprobar con el router de verdad
validacion-real-pendiente-usuarios = Crear y borrar usuarios aún no se ha comprobado con el router de verdad: de momento no se puede.
validacion-real-pendiente-contrasena-otro = Cambiar la contraseña de otro usuario aún no se ha comprobado con el router de verdad: de momento no se puede.
validacion-real-pendiente-sin-ping = No responder al ping aún no se ha comprobado con el router de verdad: de momento no se puede.
