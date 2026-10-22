# Funciones del panel: Ajustes, Mejoras, Historial y contraseña del router

La API del router está en [api.md](api.md). El panel, a su vez, ofrece a su
web unas *server functions* de Leptos: `POST /api/<nombre>` con un cuerpo JSON, la
cookie de sesión (todas la exigen) y la protección CSRF del panel. Los errores
llevan el mensaje ya traducido al idioma de la sesión. Las que devuelven
secretos van con `Cache-Control: no-store`. Aquí están las de Ajustes,
Mejoras, el Historial y la contraseña del router (fase 2), no todas: el resto
(Estado, la Biblioteca, el asistente, los usuarios) se documenta en su
código, junto a cada `#[server(endpoint = "…")]`.

| Función | Qué hace |
|---|---|
| `leer_tema` | La página entera de un tema (`wifi`, `puertos`…): la franja de resumen (en los temas en rejilla), los elementos con su estado, sus datos, su recomendación principal y sus ajustes con el valor actual del router, las acciones sobre varios elementos, cuánto hace que se leyó y, en Puertos y Red local, la red de casa (los dispositivos, la IP y la máscara del router y las IP fijas). Cada recomendación dice a qué lotes de Mejoras pertenece su campo (sale del catálogo, sin lecturas de más). Cada lectura se hace una vez, de la caché corta si está; si una falla, solo los elementos que la usan quedan «Sin leer». Con `fresca` («Volver a leer»), nada del tema sale de la caché. 404 si el tema no existe. `no-store`. |
| `preparar_accion` | Prepara una acción sobre varios elementos de un tema (quitar el WPS de dos redes, borrar tres reglas de puertos), como mucho 20: un solo cambio con un paso por elemento, que se confirma con `confirmar_ajuste` y se deshace entero. Un elemento que ya tiene el valor sale como «ya estaba así» y no se escribe. Al borrar varias instancias de una lista, de mayor a menor id y comprobando, antes de cada paso, que la instancia sigue siendo la misma. 400 si la acción no se aplica a alguno; 409 si ninguno cambia o hay otro cambio en curso; 403 en solo lectura. |
| `preparar_ajuste` | Prepara el guardado de un ajuste: lee el router, calcula lo que cambia en cada paso y devuelve el resumen con su token. 409 si ya está así o hay otro cambio en curso; 403 en solo lectura; 400 si un valor no vale. |
| `preparar_elemento` | Prepara el guardado del detalle de un elemento: los cambios de varios de sus ajustes (lo esencial y lo avanzado de una red) en un solo cambio, con un paso por escritura y un solo deshacer; se confirma con `confirmar_ajuste`. Errores como `preparar_ajuste`. |
| `leer_mejoras` | La página Mejoras, su única función: lee (de la caché corta si está) las pestañas que tocan los lotes y da, por lote, el estado de cada paso (hecho, pendiente, sin leer con su motivo, o no aplica), «x de y pendientes» y el estado del lote (aplicado, pendiente o sin leer); además, el contador. Lo que no se pudo leer no se da por hecho ni por pendiente. Con `fresca` («Volver a leer»), nada sale de la caché. `no-store`. |
| `contar_mejoras` | El contador de «Mejoras» en la barra lateral: los pasos pendientes de todos los lotes, sin repetir los compartidos, con el mismo cálculo que `leer_mejoras`; `null` si no se pudo leer nada. Se pide al cargar la página y tras un cambio, no en cada navegación mientras la caché siga viva. |
| `preparar_lote` | Prepara un lote (`id`, `opciones` con el proveedor de DNS) como un solo cambio, con un paso por cada paso que se pudo leer y aplica, agrupados por pestaña; la hora neutra sale desmarcada. Lo ya hecho sale como «ya estaba así». Se confirma con `confirmar_ajuste` (sin los pasos desmarcados) y se deshace entero. 403 en solo lectura (antes de leer nada); 404 si el lote no existe; 400 si el proveedor no existe; 409 si ya está aplicado, si solo faltan pasos sin leer o hay otro cambio en curso; 502 si no se pudo leer nada. |
| `confirmar_ajuste` | Aplica el ajuste preparado (sin los pasos desmarcados) y devuelve el id del cambio, que se sigue con `estado_cambio`. |
| `deshacer_ajuste` | Prepara el cambio que deshace uno aplicado desde Ajustes; se confirma con `confirmar_ajuste`. |
| `comparar_con_partida` | Lo que ha cambiado desde la foto de partida que tomó el asistente. Solo viajan los secretos que difieren. `no-store`. |
| `preparar_volver_a_partida` | Prepara volver un ajuste (o uno de sus campos) al valor de la foto de partida; se confirma con `confirmar_ajuste`. |
| `historial` | Una página del Historial: cambios y secretos revelados, del más reciente al más antiguo. |
| `deshacer` | Prepara el cambio inverso de uno aplicado; se confirma con `confirmar_cambio`. |
| `secretos_del_cambio` | El valor anterior y el nuevo de los secretos de un cambio, para su detalle desplegado. `no-store`. |
| `comprobar_contrasena_router` | Comprueba, sin escribir nada, que se puede cambiar la contraseña del router a la nueva. Exige reautenticación. |
| `cambiar_contrasena_router` | Cambia la contraseña del router y el panel pasa a usarla si entra con ella. Exige reautenticación; 409 con otro cambio en curso. |

## Mejoras: lotes, registro del sistema y DNS de la red de casa

Los lotes se declaran en `crates/ajustes/ajustes/lotes.toml`: cada uno con
sus textos (en los `.ftl`, en español y en inglés) y sus pasos. Un paso
apunta a una acción de Ajustes o a un campo y su valor, y puede repetirse
en cada red WiFi (`para_cada`), aplicar solo si se cumple una condición
(`si`: aislar la red de invitados solo si está encendida), salir
desmarcado (`opcional`: la hora neutra) o tomar el valor de una elección
(`elegir`: el proveedor de DNS). Un test del catálogo comprueba que cada
paso apunta a algo que existe y se puede escribir, con valores válidos y
sus textos en los dos idiomas.

| Lote | Pasos |
|---|---|
| Seguridad recomendada | Gestión remota del operador (TR-069) y su aviso periódico apagados; WPS fuera y WPA2-AES en cada red WiFi; UPnP apagado; red de invitados aislada (si está encendida); espejo del tráfico apagado; registro sin envío remoto. Además, consejos sin botón (lo que la API no deja cambiar). |
| Privacidad máxima | Gestión remota y aviso periódico apagados; espejo apagado; registro sin envío remoto; DNS neutros en la red de casa, con proveedor a elegir (Quad9, Quad9 sin filtro, DNS4EU, Cloudflare o AdGuard); hora de un servidor neutro (`hora.roa.es`), opcional. |

Los dos ajustes que los lotes necesitan:

- **Registro del sistema** (`sistema.registro`, en Sistema): si el router
  envía su registro a otro equipo. Se lee con `cbGetSyslog` y se escribe
  con `cbSetSyslog` (`syslog_mode`: `2`, local y remoto; `0`, solo local;
  `1`, solo remoto, se lee como no reconocido). Encender el envío exige el
  servidor. **Las escrituras del registro nunca llevan `export`**: ese
  parámetro exporta el registro y no está en el catálogo de la función.
  Antes de escribir, el panel carga `syslog.asp` para obtener el
  `sessionKey`.
- **DNS de la red de casa** (`red_local.dns_casa`, en Red local): los DNS
  que el router reparte por DHCP, solo en el pool 0 (`cbGetDhcpv4ServerPool`
  y `cbSetDhcpv4ServerPool`, `ipv4_dns_addr_0` y `ipv4_dns_addr_1`). Nunca
  los de la WAN ni el relé de DNS, ni los pools 1 y 2 (los del
  descodificador): pueden afectar a la televisión y al teléfono. El router
  sigue reenviando las consultas sin cifrar.

Todo esto se ha probado contra el simulador (`router-sim`). Lo pendiente de
comprobar en el router real (fase 3): que el descodificador no usa el pool
0; que `cbSetSyslog` sin `export` cambia `syslog_mode` y que cargar
`syslog.asp` no exporta el registro; que guardar «no enviar» con el puerto
remoto vacío o `0` no falla; qué aísla exactamente la red de invitados; que
la televisión y el teléfono siguen tras apagar TR-069 y reiniciar; y si
1.1.1.1 resuelve en este firmware.
