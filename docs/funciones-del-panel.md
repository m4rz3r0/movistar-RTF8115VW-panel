# Funciones del panel: Ajustes, Historial y contraseña del router

La API del router está en [api.md](api.md). El panel, a su vez, ofrece a su
web unas *server functions* de Leptos: `POST /api/<nombre>` con un cuerpo JSON, la
cookie de sesión (todas la exigen) y la protección CSRF del panel. Los errores
llevan el mensaje ya traducido al idioma de la sesión. Las que devuelven
secretos van con `Cache-Control: no-store`. Aquí están las de Ajustes, el
Historial y la contraseña del router (fase 2), no todas: el resto
(Estado, la Biblioteca, el asistente, los usuarios) se documenta en su
código, junto a cada `#[server(endpoint = "…")]`.

| Función | Qué hace |
|---|---|
| `leer_tema` | La página entera de un tema (`wifi`, `puertos`…): la franja de resumen (en los temas en rejilla), los elementos con su estado, sus datos, su recomendación principal y sus ajustes con el valor actual del router, las acciones sobre varios elementos, cuánto hace que se leyó y, en Puertos y Red local, la red de casa (los dispositivos, la IP y la máscara del router y las IP fijas). Cada lectura se hace una vez, de la caché corta si está; si una falla, solo los elementos que la usan quedan «Sin leer». Con `fresca` («Volver a leer»), nada del tema sale de la caché. 404 si el tema no existe. `no-store`. |
| `preparar_accion` | Prepara una acción sobre varios elementos de un tema (quitar el WPS de dos redes, borrar tres reglas de puertos), como mucho 20: un solo cambio con un paso por elemento, que se confirma con `confirmar_ajuste` y se deshace entero. Un elemento que ya tiene el valor sale como «ya estaba así» y no se escribe. Al borrar varias instancias de una lista, de mayor a menor id y comprobando, antes de cada paso, que la instancia sigue siendo la misma. 400 si la acción no se aplica a alguno; 409 si ninguno cambia o hay otro cambio en curso; 403 en solo lectura. |
| `preparar_ajuste` | Prepara el guardado de un ajuste: lee el router, calcula lo que cambia en cada paso y devuelve el resumen con su token. 409 si ya está así o hay otro cambio en curso; 403 en solo lectura; 400 si un valor no vale. |
| `preparar_elemento` | Prepara el guardado del detalle de un elemento: los cambios de varios de sus ajustes (lo esencial y lo avanzado de una red) en un solo cambio, con un paso por escritura y un solo deshacer; se confirma con `confirmar_ajuste`. Errores como `preparar_ajuste`. |
| `confirmar_ajuste` | Aplica el ajuste preparado (sin los pasos desmarcados) y devuelve el id del cambio, que se sigue con `estado_cambio`. |
| `deshacer_ajuste` | Prepara el cambio que deshace uno aplicado desde Ajustes; se confirma con `confirmar_ajuste`. |
| `comparar_con_partida` | Lo que ha cambiado desde la foto de partida que tomó el asistente. Solo viajan los secretos que difieren. `no-store`. |
| `preparar_volver_a_partida` | Prepara volver un ajuste (o uno de sus campos) al valor de la foto de partida; se confirma con `confirmar_ajuste`. |
| `historial` | Una página del Historial: cambios y secretos revelados, del más reciente al más antiguo. |
| `deshacer` | Prepara el cambio inverso de uno aplicado; se confirma con `confirmar_cambio`. |
| `secretos_del_cambio` | El valor anterior y el nuevo de los secretos de un cambio, para su detalle desplegado. `no-store`. |
| `comprobar_contrasena_router` | Comprueba, sin escribir nada, que se puede cambiar la contraseña del router a la nueva. Exige reautenticación. |
| `cambiar_contrasena_router` | Cambia la contraseña del router y el panel pasa a usarla si entra con ella. Exige reautenticación; 409 con otro cambio en curso. |
