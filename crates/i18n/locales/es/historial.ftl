# Historial de cambios y secretos revelados.
historial-titulo = Historial
historial-sub = Cambios hechos desde el panel y secretos revelados, del más reciente al más antiguo.
historial-vacio = Todavía no hay cambios
historial-vacio-sub = Aquí aparecerán los cambios que hagas desde el panel y los secretos que reveles.
historial-pagina-vacia = No hay entradas en esta página.
historial-volver-recientes = Volver a las más recientes
historial-paginas = Páginas del historial
historial-mas-recientes = ← Más recientes
historial-mas-antiguos = Más antiguos →
historial-pagina-de = Página { $pagina } de { $paginas } · { $total ->
    [one] { $total } entrada
   *[other] { $total } entradas
}
historial-sin-campos = Sin campos
historial-campos = { $n ->
    [one] { $n } campo
   *[other] { $n } campos
}: { $nombres }
historial-instancia-sep = { " · " }instancia { $i }
historial-instancia-par = { " " }(instancia { $i })
historial-revelado-de = de
historial-revelado-campo = Se mostró «{ $campo }»
historial-revelado-clave = Se mostró una clave
historial-tecnico = Detalles técnicos
historial-cambio-router = Cambio en el router
historial-clave-sin-leer = (el router no deja leerla)
historial-valor-anterior = { $campo }, valor anterior
historial-valor-nuevo = { $campo }, valor nuevo
historial-secreto = Secreto
historial-creado = Creado
historial-ultimo-estado = Último cambio de estado
historial-deshecho-por = Deshecho por
historial-deshace-a = Deshace a
historial-deshacer = Deshacer
historial-deshacer-lector = { " " }el cambio nº { $id }
historial-error-deshacer = No se puede deshacer: { $motivo }.
historial-pasos = { " · " }{ $n ->
    [one] { $n } paso
   *[other] { $n } pasos
}
historial-pasos-lista = Pasos del cambio
historial-paso-num = Paso { $n }
historial-paso-pendiente = sin aplicar
historial-paso-omitido = omitido
historial-paso-aplicando = aplicando
historial-paso-aplicado = aplicado
historial-paso-fallido = fallido
historial-paso-revertido = revertido
historial-paso-cancelado = cancelado
# Notas y errores del gestor de cambios: se guardan como claves en la base
# de datos de cada instalación y se pintan en el idioma de quien mira el
# Historial. No se renombran ni se borran (solo se cambia su texto).
nota-paso = paso { $n }: { $nota }
nota-paso-nombre = paso { $n } ({ $nombre }): { $nota }
nota-desconocida = algo no salió como se esperaba (más en «Detalles técnicos»)
nota-verificacion-limitada = verificación limitada: el router no muestra los campos escritos
nota-reinicio-sin-respuesta = el router no respondió a la orden de reinicio
nota-sin-confirmar = el router no confirmó la escritura
nota-releido-aplicada = releído: aplicada
nota-sin-campos-escritos = el router no muestra los campos escritos
nota-sin-releer = tampoco se pudo releer
nota-wan-caida = la conexión a internet ya estaba caída antes del cambio: solo se vigila que el router responda
nota-retomada = vigilancia retomada tras un reinicio del panel
nota-sin-vigilancia = aplicado sin vigilancia: no afecta a la conexión del panel con el router
nota-vigilancia-terminada = { $usuario } dio la vigilancia por terminada antes de tiempo
nota-cola-anterior-deshecho = no se aplicó porque el cambio anterior se deshizo
nota-cola-anterior-no-aplicado = no se aplicó porque el cambio anterior no llegó a aplicarse
nota-cola-router-cambiado = no se aplicó: el router ha cambiado desde que lo revisaste; vuelve a prepararlo
nota-cola-no-preparado = no se aplicó: al llegarle el turno no se pudo preparar
nota-cola-quitado = { $usuario } lo quitó de la cola
nota-cola-interrumpida = no se aplicó: el panel se detuvo o se reinició antes de llegarle el turno
cambio-error-router = el router no hizo el cambio
cambio-error-verificacion = el router no muestra los valores escritos
cambio-error-preparar = no se pudo preparar el paso
cambio-error-revertido = se volvió a los valores anteriores
cambio-error-sin-revertir = no se pudo volver a los valores anteriores
cambio-error-sin-inversa = no se puede volver atrás: no se conocen los valores anteriores
cambio-error-router-no-responde = el router dejó de responder
vigilancia-terminar = Todo bien, terminar
vigilancia-terminando = Terminando…
cola-quitar = Quitar de la cola
cola-quitando = Quitando…
cambio-terminar-no-vigila = Ese cambio ya no se está vigilando.
cambio-terminar-otro-usuario = Solo quien hizo el cambio puede terminar su vigilancia.
cambio-terminar-fallando = El router no respondió bien en la última comprobación: espera a que se recupere o a que el cambio se deshaga solo.
cambio-quitar-no-en-cola = Ese cambio ya no está esperando en la cola.
cambio-quitar-otro-usuario = Solo quien lo puso en la cola puede quitarlo.
cambio-error-internet-caida = se cayó la conexión a internet
cambio-error-reinicio-sin-vuelta = el router no volvió tras el reinicio en { $s } s
cambio-error-accion = una acción no se puede deshacer
cambio-error-interrumpido = interrumpido por un reinicio del panel antes de terminar
cambio-error-ningun-paso = no quedó ningún paso aplicado
cambio-error-sin-pasos = interrumpido: el cambio no tiene pasos
cambio-error-no-aplicado = interrumpido: el cambio no quedó aplicado
cambio-error-reinicio-incierto = interrumpido: no se sabe si el router llegó a reiniciarse
cambio-error-sin-respuesta-tras-panel = el router no responde tras reiniciar el panel
cambio-error-sin-comprobar = interrumpido: no se puede comprobar cómo quedó el router
cambio-error-antes-de-aplicar = interrumpido antes de aplicar: el router tiene los valores anteriores
cambio-error-sin-cambio = interrumpido: el router no muestra el cambio
cambio-error-paso-ilegible = no se puede leer el paso guardado
