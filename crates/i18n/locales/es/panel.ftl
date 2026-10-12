# Ajustes del panel.
panel-solo-lectura = El panel está en solo lectura: el firmware del router no se ha probado con el panel. Confírmalo en Router → Conexión con el router para poder hacer cambios.

## Página
panel-titulo = Ajustes del panel
panel-sub = Idioma y zona horaria, privacidad e instancia y usuarios del panel. Se guardan en la base de datos del panel.
panel-sub-router = La conexión con el router está en
panel-sub-router-enlace = Router → Conexión con el router
panel-guardado = Guardado.

## Panel
panel-panel = Panel
panel-idioma = Idioma por defecto
panel-idioma-navegador = El del navegador
panel-idioma-ayuda = Para quien no ha elegido idioma con el selector del menú, que es donde se cambia el tuyo.
panel-zona = Zona horaria
panel-zona-ayuda = La de las fechas y gráficas del panel. Península y Canarias cambian solas con el horario de verano; UTC y «Otra» son un desfase fijo.
panel-zona-peninsular = Península (hora de Madrid)
panel-zona-canarias = Canarias
panel-zona-utc = UTC
panel-zona-otra = Otra (UTC±HH:MM)
panel-zona-desfase = Desfase respecto a UTC

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
