# Asistente de primera instalación (/instalar).
instalar-titulo = Configurar el panel
instalar-pasos = Pasos de la instalación
instalar-paso-admin = Administrador
instalar-paso-router = Router
instalar-paso-lectura = Lectura inicial
instalar-paso-fin = Listo

## Administrador
instalar-admin-texto = Crea el usuario con el que entrarás en el panel.
instalar-token = Token de instalación
instalar-token-ayuda = Está en el registro del servicio y en el fichero instalar.token, junto a la base de datos del panel. Solo sirve una vez.
instalar-usuario = Usuario del panel
instalar-contrasena = Contraseña (al menos { $n } caracteres)
instalar-repetir = Repite la contraseña
instalar-no-coinciden = Las contraseñas no coinciden
instalar-crear-admin = Crear el administrador

## Router
instalar-router-texto = Dirección de la web del router y su usuario y contraseña de configuración. El panel prueba a entrar antes de guardarlos.
instalar-router-url = Dirección del router
instalar-router-url-ejemplo = http://192.168.1.1
instalar-router-usuario = Usuario del router
instalar-router-contrasena = Contraseña del router
instalar-router-usuario-ejemplo = 1234
instalar-router-url-sugerida = Sugerida a partir de la red de este equipo: compruébala.
instalar-probar = Probar
instalar-guardar = Guardar y seguir
instalar-detectado = { $modelo } con el firmware { $firmware }
instalar-firmware-probado = Firmware probado con el panel
instalar-firmware-no-probado = Firmware sin probar: el panel quedará en solo lectura hasta que lo confirmes en Configuración del router → Conexión con el router

## Lectura inicial
instalar-lectura-texto = El panel lee ahora toda la configuración del router y la guarda cifrada como foto de partida, para poder compararla y volver a ella más adelante. Puede tardar un minuto.
instalar-leer = Leer la configuración
instalar-leyendo = Leyendo…
instalar-leidas = { $n ->
    [one] Leída { $n } función del router.
   *[other] Leídas { $n } funciones del router.
}
instalar-fallidas = { $n ->
    [one] No se pudo leer { $n } función: el registro del servicio dice cuál.
   *[other] No se pudieron leer { $n } funciones: el registro del servicio dice cuáles.
}

## Fin
instalar-fin-texto = El panel ya está configurado.
instalar-red = Red del router detectada:
instalar-red-sin-detectar = No se pudo detectar la red del router: puedes volver a intentarlo en Configuración del router → Conexión con el router.
instalar-ir-estado = Ir al estado

## Errores
instalar-error-token = Token de instalación no válido
instalar-error-terminada = La instalación ya está hecha
instalar-error-red = El asistente de instalación solo responde desde la red local
instalar-error-usuario = Nombre de usuario no válido: de 1 a 64 caracteres, sin espacios
instalar-error-contrasena-corta = La contraseña debe tener al menos { $n } caracteres
instalar-error-contrasena-larga = La contraseña es demasiado larga: como mucho { $n } bytes
instalar-error-url = Dirección no válida: escribe algo como http://192.168.1.1
instalar-error-faltan = Faltan el usuario o la contraseña del router
instalar-error-router = No se puede entrar en el router: { $error }
instalar-error-sin-router = Antes hay que configurar el router
