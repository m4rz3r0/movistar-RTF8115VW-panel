# Página de Ajustes del router: textos comunes a todos los temas, errores al
# leer o escribir un valor, niveles y riesgos. Los textos de cada ajuste
# están en `ajustes-<tema>.ftl`.

## Página
ajustes-titulo = Ajustes del router
ajustes-desc = Cambia lo que quieras del router en lenguaje claro. Cada cambio se vigila y, si el router deja de responder, se deshace solo.
ajustes-mas-opciones = Más opciones
ajustes-mas-opciones-desc = Ajustes que casi nunca hace falta tocar.
ajustes-detalles-tecnicos = Detalles técnicos
ajustes-detalles-tecnicos-desc = Para quien quiera saber qué funciones del router se usan por debajo.
ajustes-detalles-lectura = Lectura
ajustes-detalles-escrituras = Escrituras
ajustes-detalles-parametros = Parámetros
ajustes-recomendado = Recomendado
ajustes-recomendado-desc = La opción que aconsejamos para la mayoría de las casas.
ajustes-solo-lectura = Ahora solo se pueden consultar los ajustes: el firmware del router no está probado con el panel. Puedes confirmarlo en Configuración del router.
ajustes-valor-desconocido = Valor no reconocido ({ $valor })
ajustes-valor-desconocido-desc = El router tiene puesto un valor que el panel no conoce. Se conserva tal cual mientras no elijas otro.
ajustes-efecto = Al aplicarlo
ajustes-temas = Temas de ajustes
ajustes-tema-elegir = Tema
ajustes-actualizando = Actualizando…
ajustes-volver = Ir a los ajustes de Internet
ajustes-solo-lectura-aviso = Ahora solo se pueden consultar los ajustes: el firmware del router aún no está probado con el panel.
ajustes-solo-lectura-enlace = Confírmalo en Configuración del router
ajustes-que-hace = ¿Qué hace esto?
ajustes-editar = Editar
ajustes-guardar = Guardar
ajustes-cancelar = Cancelar
ajustes-recomendado-valor = Recomendado: { $valor }
ajustes-recomendado-pros = A favor y en contra
ajustes-recomendado-marca = A favor:
ajustes-secreto-tapado-pedir = El router no deja leer esta clave: escríbela para guardar los cambios.
ajustes-secreto-sin-leer-desc = El router no deja leer el valor que tiene; si escribes uno, se guarda el nuevo.
ajustes-hueco = Este ajuste tendrá aquí su propia pantalla; todavía no está lista.
ajustes-lista-vacia = No hay ninguno configurado.
ajustes-lista-sin-editar = Por ahora esta lista solo se puede consultar.

## Temas
ajustes-tema-internet = Internet
ajustes-tema-internet-desc = Cómo sale tu casa a internet y qué servidores de nombres usa.
ajustes-tema-wifi = WiFi
ajustes-tema-wifi-desc = Tus redes inalámbricas: nombre, contraseña, seguridad y radios.
ajustes-tema-red_local = Red local
ajustes-tema-red_local-desc = Los dispositivos de casa, sus direcciones y las IP fijas.
ajustes-tema-puertos = Puertos
ajustes-tema-puertos-desc = Qué puede llegar desde internet a los dispositivos de tu casa.
ajustes-tema-seguridad = Seguridad
ajustes-tema-seguridad-desc = Quién puede entrar al router y la gestión remota del operador.
ajustes-tema-telefono = Teléfono
ajustes-tema-telefono-desc = La línea de teléfono fijo del router.
ajustes-tema-sistema = Sistema
ajustes-tema-sistema-desc = Hora, registro y otros ajustes generales del router.

## Niveles
ajustes-nivel-basico = Lo esencial
ajustes-nivel-basico-desc = Siempre a la vista.
ajustes-nivel-avanzado = Avanzado
ajustes-nivel-avanzado-desc = Bajo «Más opciones»: para quien sabe lo que busca.
ajustes-nivel-oculto = Oculto
ajustes-nivel-oculto-desc = No aparece en la web del operador.

## Riesgos
ajustes-riesgo-bajo = Riesgo bajo
ajustes-riesgo-bajo-desc = No corta la conexión. Se vigila unos segundos tras aplicarlo.
ajustes-riesgo-medio = Riesgo medio
ajustes-riesgo-medio-desc = Puede cortar un momento la WiFi o algún servicio. Se vigila un par de minutos y se deshace si algo falla.
ajustes-riesgo-alto = Riesgo alto
ajustes-riesgo-alto-desc = Puede dejarte sin internet o sin acceso al router. Se vigila varios minutos y se deshace si algo falla.

## Errores
ajustes-error-tema = Ese tema de ajustes no existe.
ajustes-error-lectura = No se ha podido leer este ajuste del router. Prueba de nuevo en un momento.
ajustes-error-vacio = Este campo no puede quedar vacío.
ajustes-error-corto = Demasiado corto: tiene que ocupar al menos lo que { $min } letras sin tilde. Cada letra con tilde o eñe ocupa como dos, el «€» como tres y un emoji como cuatro.
ajustes-error-largo = Demasiado largo: cabe lo que { $max } letras sin tilde. Cada letra con tilde o eñe ocupa como dos, el «€» como tres y un emoji como cuatro.
ajustes-error-fuera-de-rango = Tiene que ser un número entre { $min } y { $max }.
ajustes-error-fuera-de-rango-desde = Tiene que ser un número igual o mayor que { $min }.
ajustes-error-fuera-de-rango-hasta = Tiene que ser un número igual o menor que { $max }.
ajustes-error-numero = Tiene que ser un número entero, sin decimales.
ajustes-error-ip = No es una dirección IP válida (cuatro números del 0 al 255 separados por puntos).
ajustes-error-mac = No es una dirección MAC válida (seis pares de cifras y letras separados por dos puntos).
ajustes-error-opcion = Elige una de las opciones.
ajustes-error-control = Lleva caracteres que no se pueden escribir, como saltos de línea.
ajustes-error-ascii = Solo admite letras sin tilde ni eñe, cifras y signos corrientes.
ajustes-error-solo-lectura = Este valor solo se puede consultar.
ajustes-error-sin-cambios = Ya está así: el router tiene esos valores y no hay nada que cambiar.
ajustes-error-bloqueada = El panel aún no puede cambiar este ajuste con seguridad.
ajustes-error-router = El router no ha podido hacer el cambio. Prueba de nuevo en un momento.
ajustes-error-invalida = El panel no ha podido preparar este cambio. Recarga la página y prueba de nuevo.
ajustes-error-ajuste = Ese ajuste no existe.
ajustes-error-campo = Ese ajuste no tiene ese dato. Recarga la página y prueba de nuevo.
cambio-error-secreto-tapado = No se puede guardar: en la misma escritura va una clave que el router no deja leer o da tapada, y reenviarla podría borrarla. Escríbela para guardar los cambios.
cambio-sin-deshacer-tapado = Este cambio no se puede volver atrás: el router no deja leer el valor anterior de «{ $campos }». Si hace falta, escríbelo otra vez en Ajustes.
cambio-aviso-tapado = «{ $campos }» se queda como se escribió: el router no deja leer el valor anterior.
ajustes-error-cambio = Ese cambio no se hizo desde Ajustes. Puedes deshacerlo desde el Historial.
ajustes-error-no-deshacer = Ese cambio ya no se puede deshacer: solo se deshace un cambio ya aplicado (cuando termina la vigilancia) que no se haya deshecho antes.
ajustes-error-requiere = Hace falta elegirlo para «{ $campo }».

## Guardar un ajuste: resumen y confirmación
ajustes-grupo-revisar = Revisa el cambio
ajustes-grupo-deshacer-titulo = Volver a como estaba
ajustes-grupo-intro = Esto es lo que va a cambiar en el router.
ajustes-grupo-ajuste-detalle = { $ajuste }: { $detalle }
ajustes-grupo-paso = Paso { $n }
ajustes-grupo-pasos-desc = Desmarca un paso si no quieres aplicarlo ahora.
ajustes-grupo-ya-esta = Ya está así
ajustes-grupo-ya-esta-desc = El router ya tiene estos valores: este paso no se escribe.
ajustes-grupo-col-dato = Dato
ajustes-grupo-col-antes = Ahora
ajustes-grupo-col-despues = Después
ajustes-grupo-vacio = (vacío)
ajustes-grupo-vigilancia = Tras aplicarlo, el panel vigila el router durante { $plazo } y lo deshace solo si deja de responder o se cae internet.
ajustes-grupo-ninguno = Marca al menos un paso para aplicar el cambio.
ajustes-grupo-aplicar = Aplicar
ajustes-grupo-aplicando = Aplicando…
ajustes-grupo-cancelar = Cancelar
ajustes-grupo-cerrar = Cerrar
ajustes-grupo-deshacer = Deshacer
ajustes-grupo-preparando = Preparando…
ajustes-grupo-estado-aplicando = Aplicando el cambio y comprobando que el router lo tiene…
ajustes-grupo-estado-vigilando = Aplicado. Vigilando el router: quedan { $resto }.
ajustes-grupo-estado-aplicado = Hecho: el router tiene ya el cambio.
ajustes-grupo-estado-revertido = El router dejó de responder bien y el cambio se ha deshecho solo.
ajustes-grupo-estado-fallido = No se ha podido aplicar el cambio. El router sigue como estaba.
ajustes-grupo-estado-parcial = No se ha podido aplicar el cambio y una parte no ha vuelto a como estaba. Revísalo en el Historial.

## Cómo estaba al instalar (foto de partida)
ajustes-foto-titulo = Cómo estaba al instalar
ajustes-foto-desc = Lo que ha cambiado en el router desde que instalaste el panel. Si algo no te convence, puedes volver al valor de entonces.
ajustes-foto-enlace = Ver qué ha cambiado desde que instalaste el panel
ajustes-foto-tomada = Comparado con cómo estaba el router el { $fecha }.
ajustes-foto-sin-foto = No hay con qué comparar: este panel se instaló antes de que el asistente guardara cómo estaba el router. Lo que cambies desde el panel queda en el Historial.
ajustes-foto-sin-diferencias = Todo está como al instalar el panel.
ajustes-foto-ir-historial = Ver el Historial
ajustes-foto-ir-ajustes = Volver a los ajustes
ajustes-foto-ver-tema = Ver en { $tema }
ajustes-foto-al-instalar = Al instalar
ajustes-foto-ahora = Ahora
ajustes-foto-volver = Volver a este valor
ajustes-foto-alta = Añadido después de instalar
ajustes-foto-baja = Estaba al instalar y ya no está
ajustes-foto-lista-nota = Los cambios en las listas solo se muestran aquí: para deshacerlos, ve a su tema.
ajustes-foto-sin-dato = No se guardó cómo estaban
ajustes-foto-sin-dato-desc = Al instalar el panel no se guardó cómo estaban estos ajustes, así que no se pueden comparar. Es normal si el panel se instaló con una versión anterior, que guardaba menos; no afecta al router ni a tus cambios.
ajustes-foto-sin-leer = Ahora no se han podido leer del router estos ajustes; prueba de nuevo en un momento:
ajustes-foto-error-leer = No se ha podido leer la foto de cómo estaba el router. Prueba de nuevo en un momento.
ajustes-foto-error-cifrado = No se puede leer cómo estaba el router al instalar: el panel ya no tiene la clave con la que lo guardó. Pasa si se ha cambiado la clave de cifrado del panel.
ajustes-foto-error-sin-foto = No hay foto de cómo estaba el router al instalar el panel.
ajustes-foto-error-sin-dato = De este ajuste no se guardó cómo estaba al instalar.
ajustes-foto-error-lista = Las listas no se pueden volver a como estaban desde aquí: cámbialas en su tema.
ajustes-foto-error-desconocido = Al instalar, el router tenía aquí un valor que el panel no reconoce, así que no puede volver a ponerlo.
ajustes-foto-error-secreto = Al instalar, el router no dejaba leer esta clave, así que el panel no puede volver a ponerla.

## Si cada red emite: insignia de la tarjeta de una red y nota de la de una radio
ajustes-wifi-red-encendida = Encendida
ajustes-wifi-red-apagada = Apagada
ajustes-wifi-red-sin-radio = No emite: la radio está apagada
ajustes-wifi-radio-sin-redes = La radio está encendida, pero ninguna de sus redes emite: en esta banda no hay WiFi.
ajustes-wifi-red-no-emite = Encendida, pero no emite
ajustes-wifi-radio-ocultas = { $n ->
    [one] La radio emite una red que el panel no muestra.
   *[other] La radio emite { $n } redes que el panel no muestra.
}
ajustes-wifi-radio-ocultas-enlace = Ver en la Biblioteca
ajustes-wifi-radio-apagada = Apagada
ajustes-wifi-radio-sin-redes-corto = Encendida, sin redes
ajustes-wifi-radio-redes-sin-leer-corto = Encendida, redes sin leer
ajustes-wifi-radio-redes-sin-leer = La radio está encendida, pero alguna de sus redes no se ha podido leer: no se sabe si emite alguna.

## Elementos de cada pestaña: estado, papel, datos, recomendación y acciones
ajustes-estado-sin-leer = Sin leer
ajustes-estado-desconocido = Sin datos
ajustes-estado-leido = Leído
ajustes-papel-principal = Principal
ajustes-papel-invitados = Invitados
ajustes-papel-radio = Radio
ajustes-dato-canal_y_ancho = Canal y ancho
ajustes-dato-canal-auto = Canal automático
ajustes-dato-canal-numero = Canal { $canal }
ajustes-dato-canal-y-ancho = { $canal } · { $ancho } MHz
ajustes-dato-conectados = Dispositivos conectados
ajustes-recomendacion-aplicar = Aplicar lo recomendado
ajustes-accion-encender = Encender
ajustes-accion-apagar = Apagar
ajustes-accion-quitar_wps = Quitar WPS
ajustes-accion-ocultar_nombre = Ocultar el nombre
ajustes-accion-mostrar_nombre = Mostrar el nombre
ajustes-accion-aislar = Aislar los dispositivos
ajuste-accion-encender-titulo = Redes WiFi, encender
ajuste-accion-apagar-titulo = Redes WiFi, apagar
ajuste-accion-quitar_wps-titulo = Redes WiFi, quitar el WPS
ajuste-accion-ocultar_nombre-titulo = Redes WiFi, ocultar el nombre
ajuste-accion-mostrar_nombre-titulo = Redes WiFi, mostrar el nombre
ajuste-accion-aislar-titulo = Redes WiFi, aislar los dispositivos
ajustes-accion-demasiados = Como mucho { $max } a la vez: desmarca alguno.
ajustes-accion-no-aplica = Esa acción no se puede aplicar a todo lo marcado.
ajustes-accion-ya-estaba = Todas las marcadas ya estaban así: no hay nada que cambiar.
ajustes-accion-tapado = Esta acción no vale para «{ $ajuste }»: guardarlo exige escribir una clave que el router no deja leer. Cámbialo desde su detalle.
ajustes-recomendacion-ahora = Ahora: { $campo }, { $valor }.
ajustes-problema-wps-true = El WPS está activado: quien llegue al router puede conectarse pulsando su botón, sin la contraseña.
ajustes-problema-activa-false = La red está apagada: ningún dispositivo puede conectarse a ella.
ajustes-problema-activa-true = La red está encendida: si ahora no tienes visitas, es una red más que se ve desde fuera.
ajustes-problema-aislamiento-false = Los dispositivos de esta red pueden verse entre ellos: las visitas verían los tuyos.
ajustes-problema-aislamiento-true = Los dispositivos de esta red no se ven entre ellos: no podrás, por ejemplo, enviar contenido a la tele.
ajustes-problema-visible-false = El nombre de la red está oculto: hay que escribirlo a mano para conectarse.
ajustes-problema-seguridad-abierta = La red está abierta: cualquiera que esté cerca puede conectarse sin contraseña.
ajustes-leido-hace = { $s ->
    [0] Leído ahora mismo
    [one] Leído hace 1 s
   *[other] Leído hace { $s } s
}
ajustes-leido-hace-min = Leído hace { $min } min
ajustes-apagada-desde = Apagada desde { $fecha }
ajustes-elemento-no-existe = No hay nada que se llame «{ $id }» en esta pestaña. Elige una de sus tarjetas.

## Rejilla de elementos: marca, barra, tarjeta, detalle y aviso del cambio
ajustes-resumen = Resumen del tema
ajustes-marcar = Marcar { $nombre }
ajustes-barra = Acciones para lo marcado
ajustes-marcadas = { $n ->
    [one] 1 marcada
   *[other] { $n } marcadas
}
ajustes-desmarcar-todo = Desmarcar todo
ajustes-barra-sin-leer = No entra lo que no se ha podido leer: { $nombres }.
ajustes-barra-ninguna = Ninguna acción vale para todo lo marcado.
ajustes-insignia-aplicando = Aplicando…
ajustes-insignia-comprobando = Comprobando…
ajustes-interruptor-si = { $nombre } encendida
ajustes-interruptor-no = { $nombre } apagada
ajustes-abrir = Abrir
ajustes-abrir-nombre = Abrir { $nombre }
ajustes-volver-a-leer = Volver a leer
ajustes-detalle-cerrar = Cerrar el detalle
ajustes-detalle-volver = ‹ { $tema }
ajustes-detalle-basico = Lo esencial
ajustes-descartar = Descartar
ajustes-guardar-cambios = Guardar cambios
ajustes-sin-guardar-titulo = Tienes cambios sin guardar
ajustes-sin-guardar-texto = Si sigues, se pierden los cambios de «{ $nombre }».
ajustes-sin-guardar-seguir = Seguir editando
ajustes-sin-guardar-descartar = Descartar los cambios
ajustes-aviso-cambio = { $titulo }: { $campo }, { $antes } → { $despues }
ajustes-aviso-varios = { $titulo }: { $n ->
    [one] un cambio
   *[other] { $n } cambios
}
ajustes-aviso-cerrar = Cerrar el aviso
ajustes-aviso-conectados = { $n ->
    [one] El dispositivo conectado se queda sin WiFi.
   *[other] Los { $n } dispositivos conectados se quedan sin WiFi.
}

## Franja de resumen de la WiFi
ajustes-resumen-de = { $n } de { $total }
ajustes-resumen-sin-dato = —
ajustes-resumen-wifi-redes = Redes encendidas
ajustes-resumen-wifi-redes-detalle = Las que emiten ahora mismo.
ajustes-resumen-wifi-radios = Radios encendidas
ajustes-resumen-wifi-radios-detalle = Las bandas de 2,4 y 5 GHz.
ajustes-resumen-wifi-conectados = Dispositivos conectados
ajustes-resumen-wifi-conectados-detalle = En todas las redes del panel.
ajustes-resumen-recomendaciones = Recomendaciones pendientes
ajustes-resumen-recomendaciones-detalle = Cada tarjeta enseña la más importante.
