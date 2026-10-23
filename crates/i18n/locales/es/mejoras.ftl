# Mejoras: lotes que aplican varios ajustes de un clic con un solo deshacer.

## Seguridad recomendada
mejoras-lote-seguridad = Seguridad recomendada
mejoras-lote-seguridad-desc = Recomendado si quieres cerrar las puertas que no usas. A favor: cierra el canal de gestión remota del operador, el emparejamiento por código (WPS) y la apertura automática de puertos (UPnP), y deja el cifrado de la WiFi en WPA2-AES. En contra: dejan de llegar las actualizaciones del router, que el operador solo envía por la gestión remota; su aplicación y el diagnóstico a distancia del soporte dejan de funcionar; alguna consola necesitará que abras sus puertos a mano, y los dispositivos WiFi muy antiguos (de antes de 2006, más o menos) no se conectarán.
mejoras-lote-seguridad-titulo = Mejoras, seguridad recomendada

## Privacidad máxima
mejoras-lote-privacidad = Privacidad máxima
mejoras-lote-privacidad-desc = Recomendado si quieres que el router cuente lo mínimo al operador y a terceros. A favor: el router deja de avisar al operador y de sacar su registro o una copia del tráfico, y tus consultas de nombres de webs (DNS) dejan de resolverse en el operador. En contra: lo mismo que al apagar la gestión remota; además, esas consultas siguen viajando sin cifrar, así que el operador las ve por el camino, y también el nombre de cada web que visitas: para ocultarlas hace falta DNS cifrado en cada dispositivo o una VPN.
mejoras-lote-privacidad-titulo = Mejoras, privacidad máxima

## Aviso común
mejoras-aviso-tr069 = Un reinicio de fábrica o un técnico pueden volver a encender la gestión remota. Si llamas al soporte, quizá te pidan encenderla. Conviene encenderla de vez en cuando para recibir las actualizaciones del router.

## Consejos sin botón
mejoras-consejo-wpa3 = Usa WPA3 y la protección de las tramas de gestión si algún día el router lo permite: este no lo admite.
mejoras-consejo-nombre_y_mac = No ocultes el nombre de la red ni filtres por MAC: no protegen, y ocultar el nombre hace que los móviles lo anuncien allá donde van.
mejoras-consejo-contrasena_admin = Cambia la contraseña de administración que trae la etiqueta del router (llegará al panel más adelante).
mejoras-consejo-reenvios = Revisa que ningún reenvío de puertos apunte al propio router.
mejoras-consejo-sin_api = Este panel no puede bloquear el ping desde internet ni apagar la compartición de archivos, el servidor multimedia o el USB.

## Pasos
mejoras-paso-gestion_remota = Apagar la gestión remota del operador
mejoras-paso-gestion_remota-desc = Recomendado si quieres cerrar el canal por el que el operador cambia el router a distancia y aceptas ocuparte tú de las actualizaciones. A favor: desaparece la vía de gestión remota más potente y el router deja de enviar su estado al operador. En contra: no llegan actualizaciones del router, que solo llegan por aquí; la aplicación del operador y el diagnóstico a distancia del soporte dejan de funcionar, y un reinicio de fábrica o un técnico pueden volver a encenderla.
mejoras-paso-aviso_operador = Apagar el aviso periódico al operador
mejoras-paso-aviso_operador-desc = Recomendado si apagas la gestión remota: la refuerza. A favor: el router deja de ponerse en contacto con el operador cada cierto tiempo. En contra: los mismos que al apagar la gestión remota.
mejoras-paso-wps = Quitar el WPS
mejoras-paso-wps-desc = Recomendado si no necesitas conectar dispositivos pulsando el botón del router, que es casi siempre. A favor: se cierra el emparejamiento por código, que se puede adivinar en unos 11 000 intentos. En contra: los dispositivos nuevos se conectan escribiendo la contraseña.
mejoras-paso-upnp = Apagar la apertura automática de puertos (UPnP)
mejoras-paso-upnp-desc = Recomendado si no tienes consolas ni programas que necesiten abrir puertos solos. A favor: ningún dispositivo ni programa malicioso abre puertos sin tu permiso. En contra: alguna consola pasará a una conexión restringida y tendrás que abrir sus puertos a mano.
mejoras-paso-wpa2 = Cifrar la WiFi con WPA2-AES
mejoras-paso-wpa2-desc = Recomendado si no tienes dispositivos WiFi de antes de 2006, más o menos. A favor: se quitan el cifrado antiguo y el modo mixto, que son más débiles. En contra: los dispositivos muy viejos no se conectarán.
mejoras-paso-aislar_invitados = Aislar la red de invitados
mejoras-paso-aislar_invitados-desc = Recomendado si usas la red de invitados para las visitas o para dispositivos del hogar conectados. A favor: los dispositivos de esa red no se ven entre sí. En contra: entre ellos no se puede enviar contenido a una tele ni imprimir.
mejoras-paso-espejo = Apagar el espejo del tráfico de internet
mejoras-paso-espejo-desc = Recomendado si no estás haciendo un diagnóstico de la conexión. A favor: ningún puerto de la red de casa recibe una copia de todo el tráfico de internet. En contra: ninguno; se enciende solo para diagnosticar.
mejoras-paso-registro = No enviar el registro del sistema a otro equipo
mejoras-paso-registro-desc = Recomendado si no tienes tu propio servidor para recoger registros, que es lo normal en casa. A favor: el registro, que puede decir qué dispositivos de tu casa se conectan y cuándo, no sale del router sin cifrar hacia otro equipo. En contra: si usas un servidor propio para los registros, deja de recibir el del router.
mejoras-paso-dns_casa = Usar DNS neutros en la red de casa
mejoras-paso-dns_casa-desc = Recomendado si no quieres que el operador resuelva o filtre tus consultas de nombres de webs y aceptas que siguen viajando en claro. A favor: el operador deja de resolverlas, y algunos proveedores bloquean dominios maliciosos. En contra: el operador las sigue viendo por el camino, pasas a confiar en otro proveedor y puedes perder alguna función del operador que dependa de su DNS.
mejoras-paso-hora_neutra = Tomar la hora de un servidor neutro
mejoras-paso-hora_neutra-desc = Recomendado si quieres depender lo menos posible de los servidores del operador. A favor: la hora viene del Real Instituto y Observatorio de la Armada. En contra: la ganancia de privacidad es mínima.

## Proveedores de DNS (la nota de cada uno, en el selector de la revisión)
mejoras-dns-mantener = Mantener los actuales
mejoras-dns-mantener-desc = No cambia los DNS que el router reparte ahora a la red de casa.
mejoras-dns-quad9 = Quad9
mejoras-dns-quad9-desc = Bloquea dominios maliciosos. Está en Suiza y no guarda tu dirección IP.
mejoras-dns-quad9_sin_filtro = Quad9 sin filtro
mejoras-dns-quad9_sin_filtro-desc = Igual que Quad9, pero sin bloquear nada.
mejoras-dns-dns4eu = DNS4EU
mejoras-dns-dns4eu-desc = Infraestructura y leyes de la Unión Europea. Aplica los bloqueos que ordenan los jueces.
mejoras-dns-cloudflare = Cloudflare
mejoras-dns-cloudflare-desc = Muy rápido. Es una empresa de Estados Unidos.
mejoras-dns-adguard = AdGuard
mejoras-dns-adguard-desc = Bloquea anuncios y rastreadores. Puede romper alguna web.
mejoras-dns-elegir = Servicio de DNS para la red de casa

## Estado de los lotes y su revisión
mejoras-motivo-sin-leer = No se pudo leer la pestaña { $pestana }.
mejoras-motivo-sin-dato = El router no da este dato.
mejoras-revision-sin-leer = { $n ->
    [one] Un paso no se pudo leer y no está en la revisión.
   *[other] { $n } pasos no se pudieron leer y no están en la revisión.
}
mejoras-error-lote = Ese lote de mejoras no existe.
mejoras-error-ya-aplicado = Este lote ya está aplicado: no hay nada que cambiar.
mejoras-error-proveedor = Ese servicio de DNS no está en la lista.
mejoras-motivo-pasos-sin-leer = { $n ->
    [one] Un paso no se pudo leer; lo demás ya está.
   *[other] { $n } pasos no se pudieron leer; lo demás ya está.
}
mejoras-error-lectura = No se ha podido leer ningún paso de este lote del router. Prueba de nuevo en un momento.
mejoras-error-sin-leer = { $n ->
    [one] Lo que se pudo leer ya está, pero un paso no se pudo leer. Vuelve a leer y prueba de nuevo.
   *[other] Lo que se pudo leer ya está, pero { $n } pasos no se pudieron leer. Vuelve a leer y prueba de nuevo.
}

## Página Mejoras
mejoras-titulo = Mejoras recomendadas para tu router
mejoras-desc = Grupos de ajustes que se revisan y se aplican de una vez, con un solo deshacer.
mejoras-estado-aplicado = Aplicado
mejoras-estado-pendientes = { $pendientes } de { $total } pendientes
mejoras-estado-sin-leer = Sin leer
mejoras-paso-estado-hecho = Hecho
mejoras-paso-estado-pendiente = Pendiente
mejoras-paso-estado-sin-leer = Sin leer
mejoras-paso-estado-no-aplica = No aplica
mejoras-paso-estado-dns-propios = DNS propios
mejoras-opcional = Opcional
mejoras-revisar = Revisar y aplicar
mejoras-volver-a-leer = Volver a leer
mejoras-mas-consejos = Más consejos
mejoras-pasos = Pasos
