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
historial-revelado-ajuste = Showed “{ $campo }” for { $ajuste }
historial-revelado-anterior-ajuste = Showed the previous value of “{ $campo }” for { $ajuste } (change no. { $id })
historial-revelado-anterior = Showed the previous value of “{ $campo }” (change no. { $id })
historial-mostrar-anterior = Show the previous value
historial-anteriores = Previous values of the passwords
historial-error-sin-anterior = That step did not keep a previous value for that field.
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
