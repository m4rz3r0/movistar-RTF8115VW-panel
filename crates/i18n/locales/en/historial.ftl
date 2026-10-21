# History of changes and revealed secrets.
historial-titulo = History
historial-sub = Changes made from the panel and revealed secrets, newest first.
historial-vacio = No changes yet
historial-vacio-sub = Changes you make from the panel and secrets you reveal will show up here.
historial-pagina-vacia = There are no entries on this page.
historial-volver-recientes = Back to the newest
historial-paginas = History pages
historial-mas-recientes = ← Newer
historial-mas-antiguos = Older →
historial-pagina-de = Page { $pagina } of { $paginas } · { $total ->
    [one] { $total } entry
   *[other] { $total } entries
}
historial-sin-campos = No fields
historial-campos = { $n ->
    [one] { $n } field
   *[other] { $n } fields
}: { $nombres }
historial-instancia-sep = { " · " }instance { $i }
historial-instancia-par = { " " }(instance { $i })
historial-revelado-de = in
historial-revelado-campo = Shown: { $campo }
historial-revelado-clave = A password was shown
historial-tecnico = Technical details
historial-cambio-router = Router change
historial-clave-sin-leer = (the router won't let it be read)
historial-valor-anterior = { $campo }, previous value
historial-valor-nuevo = { $campo }, new value
historial-secreto = Secret
historial-creado = Created
historial-ultimo-estado = Last status change
historial-deshecho-por = Undone by
historial-deshace-a = Undoes
historial-deshacer = Undo
historial-deshacer-lector = { " " }change #{ $id }
historial-error-deshacer = Cannot undo: { $motivo }.
historial-pasos = { " · " }{ $n ->
    [one] { $n } step
   *[other] { $n } steps
}
historial-pasos-lista = Steps of the change
historial-paso-num = Step { $n }
historial-paso-pendiente = not applied
historial-paso-omitido = skipped
historial-paso-aplicando = applying
historial-paso-aplicado = applied
historial-paso-fallido = failed
historial-paso-revertido = reverted
# Notes and errors from the change manager: stored as keys in each
# installation's database and shown in the language of whoever views the
# History. Never rename or delete them (only change their text).
nota-paso = step { $n }: { $nota }
nota-paso-nombre = step { $n } ({ $nombre }): { $nota }
nota-verificacion-limitada = limited check: the router doesn't show the fields written
nota-reinicio-sin-respuesta = the router didn't answer the restart command ({ $error })
nota-sin-confirmar = the router didn't confirm the write ({ $error })
nota-releido-aplicada = read again: applied
nota-sin-campos-escritos = the router doesn't show the fields written
nota-sin-releer = it couldn't be read again either ({ $error })
nota-wan-caida = the internet connection was already down before the change: only whether the router answers is watched
nota-retomada = watching resumed after the panel restarted
cambio-error-router = the router didn't make the change
cambio-error-verificacion = the router doesn't show the values written
cambio-error-preparar = the step couldn't be prepared
cambio-error-revertido = the previous values were put back
cambio-error-sin-revertir = the previous values couldn't be put back
cambio-error-sin-inversa = it can't be put back: the previous values aren't known
cambio-error-router-no-responde = the router stopped answering
cambio-error-internet-caida = the internet connection went down
cambio-error-reinicio-sin-vuelta = the router didn't come back after restarting within { $s } s
cambio-error-accion = an action can't be undone
cambio-error-interrumpido = interrupted by a panel restart before finishing
cambio-error-ningun-paso = no step was left applied
cambio-error-sin-pasos = interrupted: the change has no steps
cambio-error-no-aplicado = interrupted: the change wasn't left applied
cambio-error-reinicio-incierto = interrupted: it isn't known whether the router restarted
cambio-error-sin-respuesta-tras-panel = the router doesn't answer after the panel restarted
cambio-error-sin-comprobar = interrupted: it can't be checked how the router was left
cambio-error-antes-de-aplicar = interrupted before applying: the router has the previous values
cambio-error-sin-cambio = interrupted: the router doesn't show the change
cambio-error-paso-ilegible = the saved step can't be read
