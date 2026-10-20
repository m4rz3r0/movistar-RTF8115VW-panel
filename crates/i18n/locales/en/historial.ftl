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
historial-revelado = Revealed
historial-revelado-de = in
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
