# Library of router functions (route /explorador): the interface. What comes from the
# catalogue (areas, descriptions, labels) is not translated.
biblioteca-titulo = Library
biblioteca-inicio-texto = Reads run when you open them; passwords and keys are hidden, with an eye button to show them. Writes go through the change manager: you see the before and after, and the panel watches the router after applying them and reverts if it stops responding.
biblioteca-inicio-total = { $n } router functions, from the basic web interface, the advanced web interface and those no menu links to.
biblioteca-buscar = Find a function
biblioteca-buscar-placeholder = Name or description
biblioteca-areas = Areas
biblioteca-ninguna-coincide = No function matches
biblioteca-funciones = { $n ->
    [one] { $n } function
   *[other] { $n } functions
}
biblioteca-funciones-parcial = { $n } functions (showing { $mostradas })
biblioteca-n-lecturas = { $n ->
    [one] { $n } read
   *[other] { $n } reads
}
biblioteca-n-escrituras = { $n ->
    [one] { $n } write
   *[other] { $n } writes
}
biblioteca-n-acciones = { $n ->
    [one] { $n } action
   *[other] { $n } actions
}
biblioteca-area-desconocida = Unknown area
biblioteca-elige-area = Pick an area from the sidebar.
biblioteca-error-conexion = Cannot reach the panel; check the connection and try again.

## Labels
biblioteca-tipo-lectura = Read
biblioteca-tipo-escritura = Write
biblioteca-tipo-accion = Action
biblioteca-origen-basica = Basic
biblioteca-origen-avanzada = Advanced
biblioteca-origen-oculta = Hidden
biblioteca-origen-incompleta = Incomplete
biblioteca-origen-basica-ayuda = From the router's basic web interface
biblioteca-origen-avanzada-ayuda = From the advanced web interface, linked from its menu
biblioteca-origen-oculta-ayuda = Works, but no router menu links to it
biblioteca-origen-incompleta-ayuda = Unfinished leftovers: may not respond
biblioteca-riesgo-bajo = Low risk
biblioteca-riesgo-medio = Medium risk
biblioteca-riesgo-alto = High risk
biblioteca-riesgo-vigilancia = { $riesgo } · watch { $vigilancia }
biblioteca-bloqueada = Blocked
biblioteca-bloqueada-motivo = Blocked:
biblioteca-campo-bool = Yes/no
biblioteca-campo-enum = Option
biblioteca-campo-int = Integer
biblioteca-campo-texto = Text
biblioteca-campo-secreto = Secret
biblioteca-cambio-preparado = prepared
biblioteca-cambio-aplicando = applying
biblioteca-cambio-vigilando = watching
biblioteca-cambio-aplicado = applied
biblioteca-cambio-revertido = reverted
biblioteca-cambio-fallido = failed
biblioteca-cambio-revertido-parcial = partly reverted

## Function page
biblioteca-pagina-router = Router page
biblioteca-lectura-pareada = Paired read
biblioteca-lista = List
biblioteca-lista-por = by
biblioteca-lista-borrar = , can delete
biblioteca-parametros = Parameters
biblioteca-col-parametro = Parameter
biblioteca-col-etiqueta = Label
biblioteca-col-tipo = Type
biblioteca-col-opciones = Options
biblioteca-col-campo = Field
biblioteca-col-valor = Value
biblioteca-col-antes = Before
biblioteca-col-despues = After

## Read
biblioteca-lectura = Read
biblioteca-leyendo = Reading…
biblioteca-leer-de-nuevo = Read again
biblioteca-sin-js = JavaScript is turned off in your browser.
biblioteca-sin-hidratar = The Library reads from the router in the browser and needs JavaScript. If it is on and this notice does not go away, reload the page.
biblioteca-sin-datos = The response has no data.
biblioteca-xml = Response XML
biblioteca-xml-mostrar-claves = Show keys in the XML
biblioteca-xml-ocultar-claves = Hide keys in the XML
biblioteca-vacio-par = (empty)
biblioteca-vacio = empty
biblioteca-valor-desconocido = (unknown value: { $valor })
biblioteca-instancias = { $n ->
    [one] { $n } instance
   *[other] { $n } instances
}
biblioteca-instancias-parcial = { $n } instances (showing { $mostradas })

## Write
biblioteca-instancia-trabajar = Instance to work on:
biblioteca-cargar = Load
biblioteca-instancias-titulo = Instances
biblioteca-anadir = Add
biblioteca-ejecutar = Run
biblioteca-editar = Edit
biblioteca-editar-instancia = Edit instance { $i }
biblioteca-nueva-instancia = New instance
biblioteca-la-instancia = { " " }instance { $i }
biblioteca-sin-instancias = No instances
biblioteca-acciones = Actions
biblioteca-leyendo-actuales = Reading the current values…
biblioteca-ayuda-secreto-editar = Leave it as it is to keep it.
biblioteca-ayuda-secreto = Empty: not sent.
biblioteca-ayuda-ip = IPv4 address, e.g. 192.168.1.100.
biblioteca-ayuda-mac = Six hexadecimal pairs separated by “:”.
biblioteca-valor-actual = { $valor } (current value)
biblioteca-sin-cambios = Unchanged.
biblioteca-se-cambiara = Will be changed.
biblioteca-no-editables = Not editable (the router's read does not show them, and without the previous value it could not be undone):
biblioteca-corrige-campos = { $n ->
    [one] Fix the marked field before reviewing.
   *[other] Fix the marked fields before reviewing.
}
biblioteca-sin-campos-editables = The router's read shows none of this function's fields: it cannot be edited without a way to undo.
biblioteca-revisando = Reviewing…
biblioteca-revisar-cambios = Review changes
biblioteca-validar-ip = not an IPv4 address (e.g. 192.168.1.100)
biblioteca-validar-mac = not a MAC (six hexadecimal pairs with “:”)
biblioteca-validar-bool = must be 0 or 1
biblioteca-validar-enum = not one of the options
biblioteca-validar-int = must be a whole number
biblioteca-validar-control = contains invalid characters

## Dialogs
biblioteca-diff-vacio = No fields to compare.
biblioteca-dialogo-deshacer = Undo change #{ $n }
biblioteca-dialogo-borrar = Delete the instance
biblioteca-dialogo-accion = Run the action
biblioteca-dialogo-revisar = Review changes
biblioteca-dialogo-instancia = , instance { $i }
biblioteca-nota-accion = An action cannot be reverted. After running it, the panel checks the router for { $vigilancia }.
biblioteca-nota-cambio = After applying it, the panel watches the router for { $vigilancia }: if it stops responding or internet drops, it goes back to the previous values.
biblioteca-escribe = Type
biblioteca-para-confirmar = to confirm
biblioteca-escribe-mayusculas = Type { $palabra } in capitals
biblioteca-aplicando-comprobando = Applying the change and checking that the router shows it…
biblioteca-cambio-comprobando = Change #{ $id }: checking…
biblioteca-cambio-estado = Change #{ $id }: { $estado }
biblioteca-vigilando-quedan = Watching the router: { $resto } left.
biblioteca-vigilando = Watching the router…
biblioteca-aplicado-verificado = Applied and verified.
biblioteca-aplicado-nota = Applied ({ $nota }).
biblioteca-cerrar = Close
biblioteca-cancelar = Cancel
biblioteca-aplicando = Applying…
biblioteca-deshacer = Undo
biblioteca-borrar = Delete
biblioteca-aplicar = Apply

## Server errors
biblioteca-error-bloqueada = Function blocked: { $motivo }.
biblioteca-error-sin-cambios = Nothing to change: the values match the router's.
biblioteca-error-en-curso = Another change is in progress; wait for it to finish and try again.
biblioteca-error-router = { $detalle }. Check the connection to the router and try again.
biblioteca-error-verificacion = The router does not show the change ({ $detalle }). Nothing has been reverted: check the values on its page.
biblioteca-error-palabra = The confirmation word does not match: type it exactly as shown.
biblioteca-error-token = The prepared change has expired or was already used: press “Review changes” again.
biblioteca-error-invalida = Invalid request: { $detalle }. Fix the value and review again.
biblioteca-error-no-existe-funcion = Function { $nombre } does not exist
biblioteca-error-no-existe-cambio = Change { $id } does not exist
biblioteca-error-pagina-accion = This page triggers an action when loaded: the panel does not load it.
biblioteca-error-no-lectura = { $nombre } is not a read.
biblioteca-error-param-desconocido = { $funcion }: unknown parameter { $param }
biblioteca-error-lectura-router = The router did not answer the read: { $error }
biblioteca-error-no-identifica = { $param } does not identify instances of { $funcion }
biblioteca-error-lectura-sin-formulario = { $nombre } is a read: it has no form
