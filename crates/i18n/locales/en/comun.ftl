# Frame, common errors, charts, tables and dates.

## Frame
comun-saltar = Skip to content
comun-marca-inicio = RTF8115VW, home
comun-marca-sub = Router panel
comun-menu = Menu
comun-nav-principal = Main
comun-nav-estado = Status
comun-nav-ajustes = Settings
comun-nav-historial = History
comun-nav-biblioteca = Library
comun-nav-router = Router settings
# Screen readers only, after the menu's username (which opens “Panel
# settings”).
comun-usuario-ajustes = Panel settings
comun-salir = Sign out
comun-idioma = Language
# Each language, in its own name (the same in every file).
comun-idioma-es = Español
comun-idioma-en = English
comun-no-encontrada = Page not found
comun-volver-al-estado = Back to status
comun-cargando = Loading…

## Errors
comun-error-conexion = Cannot reach the panel
comun-error-interno = Internal panel error
comun-error-interno-id = Internal panel error (id { $id })
comun-error-sesion = Not signed in or session expired
comun-error-idioma = Cannot change the language; try again

## Values
comun-si = Yes
comun-no = No
comun-valor-oculto = (hidden value)
secreto-mostrar = Show
secreto-copiar = Copy
secreto-copiado = Copied
secreto-cuenta = It will be hidden again in { $s } s.
secreto-vacio = (empty)
comun-sin-valor = no value
comun-sin-datos = No data
comun-sin-datos-min = no data
comun-conectado = Connected
comun-conectado-min = connected
comun-sin-conexion = Disconnected
comun-sin-conexion-min = disconnected
comun-ahora = now
comun-en-curso = ongoing

## Charts
comun-col-hora = Time
comun-col-inicio = Start
comun-col-fin = End
comun-col-duracion = Duration
comun-sin-datos-intervalo = No data in this range
comun-ver-datos-tabla = Show the data as a table
comun-ver-cortes-tabla = Show the outages as a table
comun-sin-cortes = No outages
comun-sin-cortes-intervalo = No outages in this range
comun-cortes = { $n ->
    [one] { $n } outage
   *[other] { $n } outages
}
comun-disponible = { $disponible } available
comun-disponible-caido = { $disponible } available · { $caido } disconnected
comun-franja-aria = Internet availability: { $disponible }, { $cortes ->
        [one] { $cortes } outage
       *[other] { $cortes } outages
    }, { $caido } disconnected
comun-serie-resumen = { $serie }: latest { $ultima }, peak { $maxima }
comun-serie-sin-datos = { $serie }: no data

## Dates: day and month ($dia and $mes with two digits; $n_dia and $n_mes, numbers)
comun-fecha-dia-mes = { $n_dia } { $n_mes ->
        [1] Jan
        [2] Feb
        [3] Mar
        [4] Apr
        [5] May
        [6] Jun
        [7] Jul
        [8] Aug
        [9] Sep
        [10] Oct
        [11] Nov
       *[12] Dec
    }
comun-fecha-dia-mes-anio = { $n_dia } { $n_mes ->
        [1] Jan
        [2] Feb
        [3] Mar
        [4] Apr
        [5] May
        [6] Jun
        [7] Jul
        [8] Aug
        [9] Sep
        [10] Oct
        [11] Nov
       *[12] Dec
    } { $anio }
