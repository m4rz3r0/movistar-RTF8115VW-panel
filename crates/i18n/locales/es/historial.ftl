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
historial-revelado = Revelado
historial-revelado-de = de
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
# Notas del gestor de cambios (se guardan como claves y se pintan en el
# idioma de quien mira el Historial).
nota-paso = paso { $n }: { $nota }
nota-verificacion-limitada = verificación limitada: el router no muestra los campos escritos
nota-reinicio-sin-respuesta = el router no respondió a la orden de reinicio ({ $error })
nota-sin-confirmar = el router no confirmó la escritura ({ $error })
nota-releido-aplicada = releído: aplicada
nota-sin-campos-escritos = el router no muestra los campos escritos
nota-sin-releer = tampoco se pudo releer ({ $error })
nota-wan-caida = la WAN ya estaba caída antes del cambio: solo se vigila el acceso al router
nota-retomada = vigilancia retomada tras un reinicio del panel
