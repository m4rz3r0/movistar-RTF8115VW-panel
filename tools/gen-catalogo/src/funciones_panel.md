## Funciones del panel

Lo anterior es la API del router. El panel, a su vez, ofrece a su web unas
*server functions* de Leptos: `POST /api/<nombre>` con un cuerpo JSON, la
cookie de sesión (todas la exigen) y la protección CSRF del panel. Los errores
llevan el mensaje ya traducido al idioma de la sesión. Las que devuelven
secretos van con `Cache-Control: no-store`. Estas son las de Ajustes, el
Historial y la contraseña del router (fase 2):

| Función | Qué hace |
|---|---|
| `leer_tema` | Los ajustes de un tema (`wifi`, `puertos`…) con su valor actual del router. Cada lectura se hace una vez; si una falla, solo sus ajustes quedan sin leer. 404 si el tema no existe. `no-store`. |
| `leer_dispositivos` | Los dispositivos que el router conoce en la red de casa, con su conexión y su IP fija. `no-store`. |
| `leer_red_casa` | La IP y la máscara del router en la red de casa, para validar una IP fija antes de enviarla. |
| `leer_reservas` | Las IP de las IP fijas, para avisar de una regla de puertos cuyo dispositivo ya no está. |
| `preparar_ajuste` | Prepara el guardado de un ajuste: lee el router, calcula lo que cambia en cada paso y devuelve el resumen con su token. 409 si ya está así o hay otro cambio en curso; 403 en solo lectura; 400 si un valor no vale. |
| `confirmar_ajuste` | Aplica el ajuste preparado (sin los pasos desmarcados) y devuelve el id del cambio, que se sigue con `estado_cambio`. |
| `deshacer_ajuste` | Prepara el cambio que deshace uno aplicado desde Ajustes; se confirma con `confirmar_ajuste`. |
| `comparar_con_partida` | Lo que ha cambiado desde la foto de partida que tomó el asistente. Solo viajan los secretos que difieren. `no-store`. |
| `preparar_volver_a_partida` | Prepara volver un ajuste (o uno de sus campos) al valor de la foto de partida; se confirma con `confirmar_ajuste`. |
| `historial` | Una página del Historial: cambios y secretos revelados, del más reciente al más antiguo. |
| `deshacer` | Prepara el cambio inverso de uno aplicado; se confirma con `confirmar_cambio`. |
| `secretos_del_cambio` | El valor anterior y el nuevo de los secretos de un cambio, para su detalle desplegado. `no-store`. |
| `comprobar_contrasena_router` | Comprueba, sin escribir nada, que se puede cambiar la contraseña del router a la nueva. Exige reautenticación. |
| `cambiar_contrasena_router` | Cambia la contraseña del router y el panel pasa a usarla si entra con ella. Exige reautenticación; 409 con otro cambio en curso. |
