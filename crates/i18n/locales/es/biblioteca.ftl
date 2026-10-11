# Explorador (biblioteca de funciones del router): la interfaz. Lo que viene
# del catálogo (áreas, descripciones, etiquetas) no se traduce.
biblioteca-titulo = Explorador
biblioteca-inicio-texto = Las lecturas se ejecutan al abrirlas, con los secretos enmascarados. Las escrituras pasan por el gestor de cambios: ves el antes y el después, el panel vigila el router tras aplicarlas y revierte si deja de responder.
biblioteca-inicio-total = { $n } funciones del router, de la web básica, la avanzada y las que no enlaza ningún menú.
biblioteca-buscar = Buscar función
biblioteca-buscar-placeholder = Nombre o descripción
biblioteca-areas = Áreas
biblioteca-ninguna-coincide = Ninguna función coincide
biblioteca-funciones = { $n ->
    [one] { $n } función
   *[other] { $n } funciones
}
biblioteca-funciones-parcial = { $n } funciones (se muestran { $mostradas })
biblioteca-n-lecturas = { $n ->
    [one] { $n } lectura
   *[other] { $n } lecturas
}
biblioteca-n-escrituras = { $n ->
    [one] { $n } escritura
   *[other] { $n } escrituras
}
biblioteca-n-acciones = { $n ->
    [one] { $n } acción
   *[other] { $n } acciones
}
biblioteca-area-desconocida = Área desconocida
biblioteca-elige-area = Elige un área del lateral.
biblioteca-error-conexion = No se puede contactar con el panel; comprueba la conexión y vuelve a intentarlo.

## Etiquetas
biblioteca-tipo-lectura = Lectura
biblioteca-tipo-escritura = Escritura
biblioteca-tipo-accion = Acción
biblioteca-origen-basica = Básica
biblioteca-origen-avanzada = Avanzada
biblioteca-origen-oculta = Oculta
biblioteca-origen-incompleta = Incompleta
biblioteca-origen-basica-ayuda = De la web básica del router
biblioteca-origen-avanzada-ayuda = De la web avanzada, enlazada desde su menú
biblioteca-origen-oculta-ayuda = Funciona, pero ningún menú del router la enlaza
biblioteca-origen-incompleta-ayuda = Restos sin terminar: puede no responder
biblioteca-riesgo-bajo = Riesgo bajo
biblioteca-riesgo-medio = Riesgo medio
biblioteca-riesgo-alto = Riesgo alto
biblioteca-riesgo-vigilancia = { $riesgo } · vigilancia { $vigilancia }
biblioteca-bloqueada = Bloqueada
biblioteca-bloqueada-motivo = Bloqueada:
biblioteca-campo-bool = Sí/no
biblioteca-campo-enum = Opción
biblioteca-campo-int = Entero
biblioteca-campo-texto = Texto
biblioteca-campo-secreto = Secreto
biblioteca-cambio-preparado = preparado
biblioteca-cambio-aplicando = aplicando
biblioteca-cambio-vigilando = vigilando
biblioteca-cambio-aplicado = aplicado
biblioteca-cambio-revertido = revertido
biblioteca-cambio-fallido = fallido
biblioteca-cambio-revertido-parcial = revertido en parte

## Ficha
biblioteca-pagina-router = Página del router
biblioteca-lectura-pareada = Lectura pareada
biblioteca-lista = Lista
biblioteca-lista-por = por
biblioteca-lista-borrar = , admite borrar
biblioteca-parametros = Parámetros
biblioteca-col-parametro = Parámetro
biblioteca-col-etiqueta = Etiqueta
biblioteca-col-tipo = Tipo
biblioteca-col-opciones = Opciones
biblioteca-col-campo = Campo
biblioteca-col-valor = Valor
biblioteca-col-antes = Antes
biblioteca-col-despues = Después

## Lectura
biblioteca-lectura = Lectura
biblioteca-leyendo = Leyendo…
biblioteca-leer-de-nuevo = Leer de nuevo
biblioteca-sin-datos = La respuesta no trae datos.
biblioteca-xml = XML de la respuesta (enmascarado)
biblioteca-vacio-par = (vacío)
biblioteca-vacio = vacío
biblioteca-valor-desconocido = (valor desconocido: { $valor })
biblioteca-visible-30 = (visible durante 30 s)
biblioteca-revelar = Revelar
biblioteca-instancias = { $n ->
    [one] { $n } instancia
   *[other] { $n } instancias
}
biblioteca-instancias-parcial = { $n } instancias (se muestran { $mostradas })

## Escritura
biblioteca-instancia-trabajar = Instancia sobre la que trabajar:
biblioteca-cargar = Cargar
biblioteca-instancias-titulo = Instancias
biblioteca-anadir = Añadir
biblioteca-ejecutar = Ejecutar
biblioteca-editar = Editar
biblioteca-editar-instancia = Editar la instancia { $i }
biblioteca-nueva-instancia = Nueva instancia
biblioteca-la-instancia = { " " }la instancia { $i }
biblioteca-sin-instancias = No hay instancias
biblioteca-acciones = Acciones
biblioteca-leyendo-actuales = Leyendo los valores actuales…
biblioteca-ayuda-secreto-editar = Déjalo como está para no cambiarlo.
biblioteca-ayuda-secreto = Vacío: no se envía.
biblioteca-ayuda-ip = Dirección IPv4, p. ej. 192.168.1.60.
biblioteca-ayuda-mac = Seis pares hexadecimales separados por «:».
biblioteca-valor-actual = { $valor } (valor actual)
biblioteca-sin-cambios = Sin cambios.
biblioteca-se-cambiara = Se cambiará.
biblioteca-no-editables = No editables (la lectura del router no los muestra y sin valor anterior no se podría deshacer):
biblioteca-corrige-campos = { $n ->
    [one] Corrige el campo marcado antes de revisar.
   *[other] Corrige los campos marcados antes de revisar.
}
biblioteca-sin-campos-editables = La lectura del router no muestra ninguno de los campos de esta función: no se puede editar sin poder deshacer.
biblioteca-revisando = Revisando…
biblioteca-revisar-cambios = Revisar cambios
biblioteca-validar-ip = no es una dirección IPv4 (p. ej. 192.168.1.60)
biblioteca-validar-mac = no es una MAC (seis pares hexadecimales con «:»)
biblioteca-validar-bool = debe ser 0 o 1
biblioteca-validar-enum = no es una de las opciones
biblioteca-validar-int = debe ser un número entero
biblioteca-validar-control = contiene caracteres no válidos

## Diálogos
biblioteca-diff-vacio = Sin campos que comparar.
biblioteca-dialogo-deshacer = Deshacer el cambio nº { $n }
biblioteca-dialogo-borrar = Borrar la instancia
biblioteca-dialogo-accion = Ejecutar la acción
biblioteca-dialogo-revisar = Revisar cambios
biblioteca-dialogo-instancia = , instancia { $i }
biblioteca-nota-accion = Una acción no se puede revertir. Tras ejecutarla, el panel comprueba el router durante { $vigilancia }.
biblioteca-nota-cambio = Tras aplicarlo, el panel vigila el router durante { $vigilancia }: si deja de responder o se cae internet, vuelve a los valores anteriores.
biblioteca-escribe = Escribe
biblioteca-para-confirmar = para confirmar
biblioteca-escribe-mayusculas = Escribe { $palabra } en mayúsculas
biblioteca-aplicando-comprobando = Aplicando el cambio y comprobando que el router lo muestra…
biblioteca-cambio-comprobando = Cambio nº { $id }: comprobando…
biblioteca-cambio-estado = Cambio nº { $id }: { $estado }
biblioteca-vigilando-quedan = Vigilando el router: quedan { $resto }.
biblioteca-vigilando = Vigilando el router…
biblioteca-aplicado-verificado = Aplicado y verificado.
biblioteca-aplicado-nota = Aplicado ({ $nota }).
biblioteca-cerrar = Cerrar
biblioteca-cancelar = Cancelar
biblioteca-aplicando = Aplicando…
biblioteca-deshacer = Deshacer
biblioteca-borrar = Borrar
biblioteca-aplicar = Aplicar
biblioteca-revelar-titulo = Revelar un secreto
biblioteca-revelar-para-ver = Para ver
biblioteca-revelar-texto = {", "}escribe la contraseña del panel. Se mostrará 30 s y quedará anotado en el historial (sin el valor).
biblioteca-contrasena-panel = Contraseña del panel
biblioteca-comprobando = Comprobando…

## Errores del servidor
biblioteca-error-bloqueada = Función bloqueada: { $motivo }.
biblioteca-error-sin-cambios = No hay nada que cambiar: los valores coinciden con los del router.
biblioteca-error-en-curso = Hay otro cambio en curso; espera a que termine y vuelve a intentarlo.
biblioteca-error-router = { $detalle }. Comprueba la conexión con el router y vuelve a intentarlo.
biblioteca-error-verificacion = El router no muestra el cambio ({ $detalle }). No se ha revertido nada: revisa los valores en su página.
biblioteca-error-palabra = La palabra de confirmación no coincide: escríbela exactamente como se indica.
biblioteca-error-token = El cambio preparado ha caducado o ya se usó: pulsa «Revisar cambios» de nuevo.
biblioteca-error-invalida = Petición no válida: { $detalle }. Corrige el valor y revisa de nuevo.
biblioteca-error-no-existe-funcion = No existe la función { $nombre }
biblioteca-error-no-existe-cambio = No existe el cambio { $id }
biblioteca-error-pagina-accion = Esta página dispara una acción al cargarse: el panel no la carga.
biblioteca-error-no-lectura = { $nombre } no es una lectura.
biblioteca-error-param-desconocido = { $funcion }: parámetro desconocido { $param }
biblioteca-error-lectura-router = El router no ha respondido a la lectura: { $error }
biblioteca-error-no-identifica = { $param } no identifica instancias de { $funcion }
biblioteca-error-lectura-sin-formulario = { $nombre } es una lectura: no tiene formulario
biblioteca-error-intentos = Demasiados intentos: espera { $s } s y vuelve a probar.
biblioteca-error-contrasena = Contraseña del panel incorrecta.
biblioteca-error-campo-desaparecido = La lectura ya no muestra el campo { $campo }; vuelve a leer.
