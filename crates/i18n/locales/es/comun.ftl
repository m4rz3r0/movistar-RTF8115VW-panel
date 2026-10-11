# Marco, errores comunes, gráficos, tablas y fechas.

## Marco
comun-saltar = Saltar al contenido
comun-marca-inicio = RTF8115VW, inicio
comun-marca-sub = Panel del router
comun-menu = Menú
comun-nav-principal = Principal
comun-nav-usuario = Usuario
comun-nav-estado = Estado
comun-nav-explorador = Explorador
comun-nav-historial = Historial
comun-nav-router = Router
comun-salir = Salir
comun-idioma = Idioma
# Cada idioma, con su propio nombre (igual en todos los ficheros).
comun-idioma-es = Español
comun-idioma-en = English
comun-no-encontrada = Página no encontrada
comun-volver-al-estado = Volver al estado
comun-cargando = Cargando…

## Errores
comun-error-conexion = No se puede contactar con el panel
comun-error-interno = Error interno del panel
comun-error-interno-id = Error interno del panel (id { $id })
comun-error-sesion = Sesión no iniciada o caducada
comun-error-idioma = No se puede cambiar el idioma; inténtalo de nuevo

## Valores
comun-si = Sí
comun-no = No
comun-valor-oculto = (valor oculto)
comun-sin-valor = sin valor
comun-sin-datos = Sin datos
comun-sin-datos-min = sin datos
comun-conectado = Conectado
comun-conectado-min = conectado
comun-sin-conexion = Sin conexión
comun-sin-conexion-min = sin conexión
comun-ahora = ahora
comun-en-curso = en curso

## Gráficos
comun-col-hora = Hora
comun-col-inicio = Inicio
comun-col-fin = Fin
comun-col-duracion = Duración
comun-sin-datos-intervalo = Sin datos en este intervalo
comun-ver-datos-tabla = Ver los datos en una tabla
comun-ver-cortes-tabla = Ver los cortes en una tabla
comun-sin-cortes = Sin cortes
comun-sin-cortes-intervalo = Sin cortes en este intervalo
comun-cortes = { $n ->
    [one] { $n } corte
   *[other] { $n } cortes
}
comun-disponible = { $disponible } disponible
comun-disponible-caido = { $disponible } disponible · { $caido } sin conexión
comun-franja-aria = Disponibilidad de internet: { $disponible }, { $cortes ->
        [one] { $cortes } corte
       *[other] { $cortes } cortes
    }, { $caido } sin conexión
comun-serie-resumen = { $serie }: última { $ultima }, máxima { $maxima }
comun-serie-sin-datos = { $serie }: sin datos

## Fechas: día y mes ($dia y $mes con dos cifras; $n_dia y $n_mes, números)
comun-fecha-dia-mes = { $dia }/{ $mes }
comun-fecha-dia-mes-anio = { $dia }/{ $mes }/{ $anio }
