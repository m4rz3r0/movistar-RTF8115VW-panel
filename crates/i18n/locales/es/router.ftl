# Router: ajustes del router. Su primera página es la conexión.
router-conexion = Conexión con el router
router-conexion-sub = Dirección de la web del router y el usuario y la contraseña con los que entra el panel, y el firmware que tiene. Se guardan en la base de datos del panel.

## Credenciales
router-credenciales = Credenciales del router
router-texto = Dirección de la web del router y su usuario de configuración. La contraseña guardada no se muestra: para cambiar algo, escribe la contraseña del router; el panel prueba a entrar antes de guardar.
router-sin-configurar = El router aún no está configurado.
router-url = Dirección del router
router-usuario = Usuario del router
router-contrasena = Contraseña del router
router-probar = Probar la conexión
router-guardar = Guardar las credenciales
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
