# Ajustes del panel.
panel-solo-lectura = El panel está en solo lectura: el firmware del router no se ha probado con el panel. Confírmalo en Ajustes del panel para poder hacer cambios.

## Página
panel-titulo = Ajustes del panel
panel-sub = Router, parámetros de esta instalación y usuarios del panel. Se guardan en la base de datos del panel.
panel-guardado = Guardado.

## Router
panel-router = Router
panel-router-texto = Dirección de la web del router y su usuario de configuración. La contraseña guardada no se muestra: para cambiar algo, escribe la contraseña del router; el panel prueba a entrar antes de guardar.
panel-router-sin-configurar = El router aún no está configurado.
panel-router-url = Dirección del router
panel-router-usuario = Usuario del router
panel-router-contrasena = Contraseña del router
panel-router-probar = Probar la conexión
panel-router-guardar = Guardar las credenciales
panel-router-guardadas = Credenciales guardadas: el panel ya usa las nuevas.
panel-firmware = Firmware: { $firmware }
panel-firmware-desconocido = Firmware: aún sin leer
panel-modo-solo-lectura = Solo lectura: el firmware no se ha probado con el panel
panel-modo-escritura = Se pueden hacer cambios
panel-confirmar-texto = Si sabes que este firmware se comporta como el probado, confírmalo para poder hacer cambios. El gestor de cambios sigue verificando y deshaciendo lo que falle.
panel-confirmar = Confirmar el firmware

## Panel
panel-panel = Panel
panel-idioma = Idioma por defecto
panel-zona = Zona horaria
panel-zona-ayuda = Desfase fijo respecto a UTC: no cambia solo con el horario de verano.

## Privacidad e instancia
panel-privacidad = Privacidad e instancia
panel-privacidad-texto = Parámetros que usan las acciones y las recomendaciones. Todos son opcionales.
panel-dns = DNS preferidos
panel-dns-ayuda = Hasta { $n } direcciones IPv4 o IPv6, separadas por comas o espacios. Vacío: los del operador.
panel-dns-sugerencias = Sugerencias públicas
panel-dns-usar = Usar { $nombre }
panel-dns-politica = política de privacidad de { $nombre }
panel-dns-quad9 = Quad9
panel-dns-quad9-ips = 9.9.9.9
panel-dns-quad9-politica = https://quad9.net/privacy/policy/
panel-dns-mullvad = Mullvad
panel-dns-mullvad-ips = 194.242.2.2
panel-dns-mullvad-politica = https://mullvad.net/help/privacy-policy
panel-dns-cloudflare = Cloudflare
panel-dns-cloudflare-ips = 1.1.1.1
panel-dns-cloudflare-politica = https://developers.cloudflare.com/1.1.1.1/privacy/public-dns-resolver/
panel-confianza = Dispositivos de confianza
panel-confianza-ayuda = Direcciones MAC, separadas por comas o espacios.
panel-confianza-equipos = Equipos que ve el router
panel-confianza-anadir = Añadir { $nombre }
panel-equipo-sin-nombre = (sin nombre)
panel-mantenimiento = Horario de mantenimiento
panel-mantenimiento-ayuda = Como 03:00-05:00. Vacío: sin horario.
panel-servidor = Servidor propio
panel-servidor-ayuda = IP o nombre de un servidor de la red, si tienes uno. Vacío: ninguno.
panel-guardar = Guardar

## Usuarios
panel-usuarios = Usuarios del panel
panel-usuarios-texto = Pueden entrar en el panel con todos los permisos. También se gestionan con la orden panel user.
panel-usuario-tu = (tú)
panel-usuario-borrar = Borrar
panel-usuario-borrar-de = Borrar a { $usuario }
panel-usuario-contrasena = Cambiar la contraseña
panel-usuario-contrasena-de = Cambiar la contraseña de { $usuario }
panel-usuario-nuevo = Usuario nuevo
panel-usuario-nombre = Nombre
panel-usuario-clave = Contraseña (al menos { $n } caracteres)
panel-usuario-crear = Crear el usuario
panel-usuario-clave-nueva = Contraseña nueva (al menos { $n } caracteres)
panel-usuario-cambiar = Cambiar
panel-cancelar = Cancelar

## Reautenticación
panel-reautenticar = Confirma que eres tú
panel-reautenticar-texto = Para esto hace falta la contraseña del panel. No se volverá a pedir en 5 minutos.
panel-reautenticar-contrasena = Contraseña del panel
panel-reautenticar-seguir = Seguir

## Errores
panel-error-contrasena = Contraseña del panel incorrecta.
panel-error-cambio-en-curso = Hay un cambio en curso en el router: espera a que termine para cambiar sus credenciales.
panel-error-dns = No es una dirección IPv4 ni IPv6: { $valor }
panel-error-demasiados-dns = Como mucho { $n } servidores DNS.
panel-error-mac = No es una dirección MAC: { $valor }
panel-error-mantenimiento = El horario de mantenimiento va como 03:00-05:00.
panel-error-zona = Zona horaria no válida: elige una de la lista.
panel-error-servidor = El servidor propio va como IP o nombre, sin http:// ni rutas.
panel-error-usuario-existe = Ese usuario ya existe.
panel-error-usuario-no-existe = Ese usuario no existe.
panel-error-ultimo-usuario = No se puede borrar el último usuario del panel.
panel-error-usuario-propio = No puedes borrar tu propio usuario.
