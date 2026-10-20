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
