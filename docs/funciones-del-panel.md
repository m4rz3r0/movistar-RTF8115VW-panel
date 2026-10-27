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
| `preparar_accion` | Prepara una acción sobre varios elementos de un tema (quitar el WPS de dos redes, borrar tres reglas de puertos), como mucho 20: un solo cambio con un paso por elemento, que se confirma con `confirmar_ajuste` y se deshace entero. Un elemento que ya tiene el valor sale como «ya estaba así» y no se escribe. Al borrar varias instancias de una lista, de mayor a menor id y comprobando, antes de cada paso, que la instancia sigue siendo la misma. 400 si la acción no se aplica a alguno; 409 si ninguno cambia, si otro cambio se está escribiendo o si la cola está llena; 403 en solo lectura. |
| `preparar_ajuste` | Prepara el guardado de un ajuste: lee el router, calcula lo que cambia en cada paso y devuelve el resumen con su token. 409 si ya está así, si otro cambio se está escribiendo o si la cola está llena (mientras otro se vigila, sí se prepara: se confirmará en la cola); 403 en solo lectura; 400 si un valor no vale. |
| `preparar_elemento` | Prepara el guardado del detalle de un elemento: los cambios de varios de sus ajustes (lo esencial y lo avanzado de una red) en un solo cambio, con un paso por escritura y un solo deshacer; se confirma con `confirmar_ajuste`. Errores como `preparar_ajuste`. |
| `leer_mejoras` | La página Mejoras, su única función: lee (de la caché corta si está) las pestañas que tocan los lotes y da, por lote, el estado de cada paso (hecho, pendiente, sin leer con su motivo, o no aplica), «x de y pendientes» y el estado del lote (aplicado, pendiente o sin leer); además, el contador. Lo que no se pudo leer no se da por hecho ni por pendiente. Con `fresca` («Volver a leer»), nada sale de la caché. `no-store`. |
| `contar_mejoras` | El contador de «Mejoras» en la barra lateral: los pasos pendientes de todos los lotes, sin repetir los compartidos, con el mismo cálculo que `leer_mejoras`; `null` si no se pudo leer nada. Se pide al cargar la página y tras un cambio, no en cada navegación mientras la caché siga viva. |
| `preparar_lote` | Prepara un lote (`id`, `opciones` con el proveedor de DNS o `mantener`, «Mantener los actuales») como un solo cambio, con un paso por cada paso que se pudo leer y aplica, agrupados por pestaña; la hora neutra sale desmarcada. Lo ya hecho sale como «ya estaba así». Se confirma con `confirmar_ajuste` (sin los pasos desmarcados) y se deshace entero. 403 en solo lectura (antes de leer nada); 404 si el lote no existe; 400 si el proveedor no existe; 409 si ya está aplicado, si solo faltan pasos sin leer, si otro cambio se está escribiendo o si la cola está llena; 502 si no se pudo leer nada. |
| `confirmar_ajuste` | Aplica el ajuste preparado (sin los pasos desmarcados) y devuelve el id del cambio, que se sigue con `estado_cambio`. Si otro cambio se está vigilando, el cambio espera en la cola (`EnCola`, con su puesto) y se aplica cuando el anterior quede aplicado. 409 si la cola está llena: el token sigue valiendo. |
| `estado_cambio` | Estado de un cambio para quien lo sigue: `Aplicando`, `Vigilando` (con los segundos que faltan y `terminable`), `EnCola` (con `puesto` y `quitable`), `Aplicado`, `Revertido`, `Fallido` o `Cancelado`, con la nota y el error ya traducidos. |
| `terminar_vigilancia` | «Todo bien, terminar»: da por buena ya la vigilancia del cambio `id`, que queda aplicado; el Historial anota quién la terminó. Solo quien hizo el cambio (403 si no), mientras se vigila y si la última comprobación salió bien (409 si no). Lo que esperaba en la cola se aplica detrás. |
| `quitar_de_la_cola` | Quita de la cola el cambio `id`: queda `Cancelado`, con quién lo quitó. Solo quien lo encoló (403 si no); 409 si ya no espera (se está aplicando o ya terminó). Los demás de la cola siguen. |
| `deshacer_ajuste` | Prepara el cambio que deshace uno aplicado desde Ajustes; se confirma con `confirmar_ajuste`. |
| `comparar_con_partida` | Lo que ha cambiado desde la foto de partida que tomó el asistente. Solo viajan los secretos que difieren. `no-store`. |
| `preparar_volver_a_partida` | Prepara volver un ajuste (o uno de sus campos) al valor de la foto de partida; se confirma con `confirmar_ajuste`. |
| `historial` | Una página del Historial: cambios y secretos revelados, del más reciente al más antiguo. |
| `deshacer` | Prepara el cambio inverso de uno aplicado; se confirma con `confirmar_cambio`. |
| `secretos_del_cambio` | El valor anterior y el nuevo de los secretos de un cambio, para su detalle desplegado. `no-store`. |
| `comprobar_contrasena_router` | Comprueba, sin escribir nada, que se puede cambiar la contraseña del router a la nueva. Exige reautenticación. |
| `cambiar_contrasena_router` | Cambia la contraseña del router y el panel pasa a usarla si entra con ella. Exige reautenticación; 409 con otro cambio en curso. |
| `leer_registro_router` | El registro del sistema del router (`texto`, con lo más nuevo al final, y `recortado`): lo exporta y lo lee como la web (ver «Registro del router»). Solo lectura: no pasa por el gestor ni cambia ningún ajuste. 503 sin router configurado; 502 si el router no lo da. `no-store`. |

## Lo que el panel escribe: solo lo comprobado en el router real

Cada escritura o acción de `crates/catalogo/funciones.toml` lleva
`comprobada`: la referencia de la prueba en el router real
(`comprobada = "E3, cambio 14"`) o `false`. Es una lista blanca que falla
cerrada: sin el campo, como `false` (un test pide que toda escritura lo
lleve, y otro fija la lista de la 1.0). Marcar una función como comprobada
es cambiar esa línea.

En la 1.0 están comprobadas:

- **Seguridad y sistema:** gestión remota y aviso al operador
  (`cbSetTr69Config`), hora (`cbSetTimeNtp`), registro (`cbSetSyslog`),
  diagnóstico (`cbSetDiagConfig`), reiniciar el router (`cbReboot`), cuentas de la web del router
  (`cbAddUser`, `cbDeleteUser`) y el alta y la baja de la regla del
  cortafuegos (`scbidirectionalflt.cmd?action=add`/`remove`).
- **WiFi:** cada red (`cbSetWifiAccPt`, también el WPS) y las radios
  (`cbSetWifi`).
- **Red de casa:** las reservas del DHCP (`cbDhcpV4SvrPoolSetStatic`),
  IPv6 de la red (`cbSetLanIPv6Config`) y relé DNS (`cbSetDnsRelay`).
- **Puertos y multicast:** reglas de puertos (`cbSetNatPortMapping`), UPnP
  (`cbSetUpnpCfg`), IGMP (`cbSetMcastIgmpSetting`, `cbSetMcastIntf`) y RIP
  (`cbSetRipIntf`).
- **Teléfono:** servidores (`cbSetVoiceSip`) y puerto físico
  (`cbSetVoicePhysical`).

Probadas y descartadas (2026-10-26):

- `cbSetDdns`: crear una entrada inactiva colgó la web del router hasta
  reiniciarlo. No se vuelve a probar.
- `cbSetNatConfig`: los ALG no cambian. Causa encontrada después (ver
  abajo): son casillas que van por presencia, y la web manda siempre
  `set_nf_conntrack=1`. Además mandaba los campos del NAT loopback, que
  la web de este firmware no ofrece; ahora son otra función,
  `cbSetNatConfig.loopback`. Arreglada; falta volver a probarla.
- `cbSetQueue`: estaba mal modelada. La web la usa de dos formas: el
  estado de cada cola (solo el id y `adm_state`) y la cola por defecto (el
  id y `default_traffic_class=-1`; después la web cierra la sesión). El
  panel mandaba las dos cosas a la vez y reenviaba la clase leída. Ahora
  son `cbSetQueue` y `cbSetQueue.defecto` (`cgi = "cbSetQueue"`), y faltan
  por probar.
- `cbSyncTime`: el router no la confirma.
- `cbSetSystemTimeZone` y `cbSetNTPInfo`: su lectura no existe en este
  router.

Arreglada y pendiente de comprobar otra vez:

- `cbSetDhcpv4ServerPool` (el grupo DHCP de casa). Los cambios 39 y 40
  cambiaron y deshicieron la concesión y el router los dio por aplicados,
  pero dejaron activas, con el valor vacío, las cuatro condiciones del
  grupo (opción 60, 61, 77 y MAC). Tras el reinicio del cambio 57 el router
  dejó de dar IP nueva a cualquier dispositivo: ninguno encaja en esas
  condiciones. La causa: en la web del router son casillas sin `value`; la
  web las manda como `on` si están marcadas y no las manda si no lo están,
  y el router las da por marcadas si el campo llega, valga lo que valga
  (captura del 2026-10-26). El panel mandaba `dhcpv4_opt60=0`, y así con
  las cuatro. Ahora el catálogo las marca como `casilla`: el panel manda
  `on` o nada, como la web, y nunca deja activa una condición con el valor
  vacío (o una MAC todo ceros): si ya lo estaba, la desactiva, y el diff
  lo dice; activar una sin valor da error. Todo lo demás se cambia como en
  la web oficial, también el grupo del operador (id 1, TEF_IOT). Hasta que
  la prueba en el router real (cambiar y deshacer la concesión, también
  tras reiniciar, y comprobar que el router da IP nueva) pase, sigue con
  `comprobada = false`, y con ella los ajustes de DHCP y DNS de la red de
  casa.
- IP reservadas del grupo (las que no reparte), en la Biblioteca
  (`cbSetDhcpv4ServerPool`). El router las guarda en 32 ranuras
  (`reserved_ipv4_addr_<i>`/`reserved_ipv4_mask_<i>`, dentro de
  `<reserved_ipv4>`; una libre vale 0.0.0.0) y el catálogo las declara con
  `ranuras = 32`. El formulario del grupo enseña las ranuras usadas y una
  pareja vacía para una nueva. Como la web oficial, una nueva va a la
  ranura siguiente a la última usada (si esa era la última, a la primera
  libre; sin máscara, una sola dirección) y quitar una es poner su IP a
  0.0.0.0, que manda también la máscara a 0.0.0.0. A diferencia de la web,
  que manda solo la ranura y el id, el panel manda además todo el grupo
  como al editarlo, para no desmarcar ninguna condición; las demás ranuras
  no se reenvían nunca, tampoco al cambiar la concesión.
- Opciones DHCP del grupo (`cbDhcpV4SvrPoolSetOpt`), en la Biblioteca: como
  la web, el código va de 0 a 254, el valor en hexadecimal sin espacios y,
  con la longitud vacía, la de los bytes del valor; al crear, el id `-1`, y
  al borrar, solo los dos ids con `del=DEL`. Para los códigos 224 a 254 la
  web convierte el texto a hexadecimal; el panel lo pide ya en hexadecimal,
  que es lo que guarda y devuelve el router y lo que compara la relectura.
  Las dos siguen con `comprobada = false` hasta probarlas en el router real.

**Casillas y campos fijos (2026-10-27).** En varias páginas de la web del
router, un sí/no es una casilla sin `value`: la web la manda como `on` si
está marcada y no la manda si no lo está, y el router la da por activa si
el campo llega, valga lo que valga. El catálogo las marca con `casilla`
(el panel manda `on` o nada, como la web): el grupo DHCP, las ALG de NAT,
la clasificación QoS, el filtrado y el marcado L2, la WAN (`trigger`,
`dns_override`) y la línea directa del teléfono. Un CGI no tiene por qué
hacerlo así: el de la WiFi guarda `ssid_hide=0` como «no», así que
`ssid_hide` y el prefijo ULA de IPv6, ya comprobados, siguen como 0/1
hasta probarlos. `fijo` es un valor que la web manda siempre en un campo
oculto y la lectura no da (`set_nf_conntrack=1`). El paso directo de fax
y módem es un desplegable (desactivado, automático, siempre), no un sí/no.
`depende` marca lo que la web desactiva según otro campo, y que por eso
no viaja: el valor de cada condición de la clasificación QoS y del
filtrado L2 mientras su casilla está desmarcada, el VLAN y la prioridad
del marcado L2 si no se sobrescriben, y el orden de una regla en la
posición primera o última. `cgi` junta dos funciones del catálogo que la
web manda a la misma CGI con campos distintos: las colas de QoS
(`cbSetQueue` y `cbSetQueue.defecto`) y el NAT
(`cbSetNatConfig` y `cbSetNatConfig.loopback`).

Lo demás solo se ve:

- **En la página**, sus campos salen como valores, sin notas por campo; sin
  interruptor, sin botón de guardar y sin acciones de la lista (añadir,
  borrar, fijar la IP de un dispositivo). Una recomendación de un campo
  sin comprobar se queda como información: dice qué pasa y por qué
  conviene cambiarlo, sin botón ni lotes, detrás de las que se pueden
  aplicar.
- **En Mejoras**, los pasos sin comprobar no salen (un lote sin ninguno
  tampoco). «Cómo estaba al instalar» no ofrece volver a esos valores.
- **En la Biblioteca**, la función sale bloqueada, con su motivo.
- **El gestor** responde 409 sin escribir (`funcion-sin-comprobar`), venga
  la petición de donde venga. El deshacer y la reversión de lo ya hecho
  siguen funcionando.

La contraseña del propio router (`cbModifyPwd`) está pendiente de la E4 y
sigue activa: su flujo no pasa por el gestor, está protegido (prueba la
nueva, vuelve a la anterior y avisa) y la E4 se hace con él (TODO en el
catálogo y en `contrasena_router.rs`).

Aparte, dos funciones del panel están desactivadas en este router
(`crates/panel/src/validacion_real.rs`), aunque sus escrituras estén
comprobadas:

- **Cambiar la contraseña o el grupo de otra cuenta** de la web del
  router: en este router solo la cuenta `1234` entra en la web
  (`cbCheckLogin` responde `Result=4` a las demás), así que el panel no
  puede comprobar que la contraseña nueva funciona. Para cambiarla, se
  borra la cuenta y se crea de nuevo. Crear y borrar cuentas sí funciona.
- **«No responder al ping desde internet»**: la regla del panel se pone y
  se quita bien, pero el router sigue respondiendo al ping porque otra
  regla del operador lo permite.

La tarjeta no las ofrece: solo enseña sus valores y una nota de una línea
con el motivo. `preparar_sin_ping` y `cambiar_usuario_router` responden 409
sin escribir.

Las pruebas contra router-sim lo habilitan todo (el catálogo entero y las
dos funciones) con un solo interruptor: `Config::habilitar_todo`, que en el
e2e pone `RP_E2E_HABILITAR_TODO=1` (`e2e/arrancar.sh`). Solo lo atiende un
binario de depuración; `deploy/pruebas/release-local.sh` comprueba que el
de release no lo hace.

## Vigilancia y cola de cambios

Solo se vigila lo que puede cortar el acceso del panel al router o la
conexión a internet. Cada escritura del catálogo de funciones lo declara
(`acceso` en `crates/catalogo/funciones.toml`; el catálogo no carga si
alguna escritura no lo lleva) y el gestor decide en
`crates/panel/src/changes/acceso.rs`:

- `puede_cortar` (se vigila siempre): la red de casa (IP, máscara, DHCP y
  sus reservas, sus DNS, el relé de DNS), la WAN, el cortafuegos, rutas,
  colas y filtros, la contraseña del router, reinicios, firmware y
  restaurar la configuración.
- `wifi`: se vigila salvo que el panel compruebe que llega al router por
  cable (su IP en la lista de dispositivos del router y su MAC en ninguna
  red WiFi); por WiFi, o si no se sabe, se vigila.
- `no_corta` (sin vigilancia: queda aplicado en cuanto la relectura
  muestra lo nuevo): UPnP, reglas de puertos, TR-069, hora y NTP, registro
  del sistema, DDNS, voz, multicast y las cuentas de la web del router.

Un cambio de varios pasos se vigila si alguno lo pide.

Mientras un cambio se vigila, el gestor no se queda bloqueado:

- **«Todo bien, terminar»** (`terminar_vigilancia`) da el cambio por bueno
  antes de tiempo. Aparece en el aviso de Ajustes, en el diálogo del
  cambio y en su fila del Historial, solo para quien lo hizo.
- **La cola**: lo que se confirma mientras tanto espera, como mucho
  5 cambios (`MAX_COLA`), en orden. Su token ya no caduca; justo antes de
  escribir, el gestor vuelve a leer el router y, si ya no está como cuando
  se revisó, no escribe (`Cancelado`: «el router ha cambiado desde que se
  revisó»). Solo se sigue con la cola si el anterior queda aplicado: si se
  deshace o falla, los que esperaban quedan `Cancelado` con el motivo. Si
  el panel se detiene o se reinicia, lo que estaba en la cola queda
  `Cancelado` al arrancar; nunca se aplica a ciegas.
- Detrás de un cambio de las credenciales del router no se encola nada
  (409, como siempre).

## Mejoras: lotes, registro del sistema y DNS de la red de casa

Los lotes se declaran en `crates/ajustes/ajustes/lotes.toml`: cada uno con
sus textos (en los `.ftl`, en español y en inglés) y sus pasos. Un paso
apunta a una acción de Ajustes o a un campo y su valor, y puede repetirse
en cada red WiFi (`para_cada`), aplicar solo si se cumple una condición
(`si`: aislar entre sí los dispositivos de la red de invitados solo si
está encendida), salir
desmarcado (`opcional`: la hora neutra) o tomar el valor de una elección
(`elegir`: el proveedor de DNS). Un test del catálogo comprueba que cada
paso apunta a algo que existe y se puede escribir, con valores válidos y
sus textos en los dos idiomas.

| Lote | Pasos |
|---|---|
| Seguridad recomendada | Gestión remota del operador (TR-069) y su aviso periódico apagados; WPS fuera y WPA2-AES en cada red WiFi; UPnP apagado; dispositivos de la red de invitados aislados entre sí (si está encendida); espejo del tráfico apagado; registro sin envío remoto. Además, consejos sin botón (lo que la API no deja cambiar). |
| Privacidad máxima | Gestión remota y aviso periódico apagados; espejo apagado; registro sin envío remoto; DNS neutros en la red de casa, con proveedor a elegir (Quad9, Quad9 sin filtro, DNS4EU, Cloudflare, AdGuard o Cloudflare familiar, que filtra webs para adultos y malware) o «Mantener los actuales»: con DNS propios (ni el router ni del operador) el paso ya está hecho («DNS propios») y se mantienen salvo que se elija un proveedor; hora de un servidor neutro (`hora.roa.es`), opcional. |

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
  que el router reparte por DHCP, solo en el pool de id 0
  (`cbGetDhcpv4ServerPool` y `cbSetDhcpv4ServerPool`, `ipv4_dns_addr_0` y
  `ipv4_dns_addr_1`). Nunca los de la WAN ni el relé de DNS, ni los demás
  pools: pueden afectar a la televisión, al teléfono o a otros dispositivos
  del operador. Cada dispositivo pregunta directamente a esos servidores,
  sin cifrar. Un DNS principal que no es el router (su IP de casa, vacío o
  `0.0.0.0`) ni del operador (`del_operador` en el catálogo: los DNS
  públicos de Movistar) es **propio**: la tarjeta no recomienda cambiarlo y
  el paso de Privacidad máxima lo da por hecho («DNS propios»), salvo que
  el secundario sea el router o del operador (entonces sigue pendiente).
  El principal de un proveedor de la lista sin secundario también cuenta
  como hecho, y el paso dice el proveedor («Quad9 (solo principal)»).

**El pool DHCP se busca por su id (0.5.1).** `cbGetDhcpv4ServerPool` elige
el pool por su **posición** (`rtm_cfg_dhcpv4_svr_pool_0i`) y
`cbSetDhcpv4ServerPool`, por su **id** (`rtm_cfg_dhcpv4_svr_pool_id`), y en
el router real no coinciden: en la posición 0 está el pool de id 1 (otros
dispositivos del operador) y en la 1, el de casa (id 0). Los ajustes que
leen una lista por posición y escriben por id lo declaran con `buscar` en
su `leer` (los del pool DHCP de casa y los de cada radio WiFi): el panel
lee la posición del id y, si no es esa instancia, las demás desde la 0,
hasta dar con la de su id; si no está, el ajuste queda «sin leer» y nunca
toma otra. El gestor de cambios hace lo mismo con toda lectura pareada que
pasa el id como posición (`X_0i = {X_id}`), para comprobar y para el
«antes» del deshacer. El simulador tiene los dos pools en el orden del
router real.

**Red de invitados.** Tiene dos valores: si está separada de la red de casa
(`guest_networking`) y si aísla entre sí a sus dispositivos (`isolation`).
El panel solo sabe cambiar el segundo: ninguna escritura del catálogo de
funciones tiene el primero (ni `cbSetWifiAccPt`), así que «Separada de la
red de casa» se enseña, pero solo se lee.

Todo esto se ha probado contra el simulador (`router-sim`). Lo pendiente de
comprobar en el router real (fase 3): que el descodificador no usa el pool
de casa; que `cbSetSyslog` sin `export` cambia `syslog_mode` y que cargar
`syslog.asp` no exporta el registro; que guardar «no enviar» con el puerto
remoto vacío o `0` no falla; que la televisión y el teléfono siguen tras
apagar TR-069 y reiniciar; si 1.1.1.1 resuelve en este firmware; qué
significa `guest_networking = 1` (se lee como «separada»); y si la
posición de `cbGetWifi` coincide con el id de la radio, porque las
lecturas de estado de la WiFi (`hgu::read::wifi` y la detección de radios)
aún toman la posición como id.

## Registro del router (ver y descargar)

Al pie de Ajustes → Sistema, «Registro del router» enseña el registro del
sistema del router, como la página Syslog de su web avanzada
(`syslog.asp`), que al cargarse lo exporta y deja descargarlo. Solo se lee
al pulsar «Leer el registro» (`leer_registro_router`): el panel pide al
router que lo exporte (`cbSetSyslog` con **solo** `export=EXPORT` y el
`sessionKey` de `syslog.asp`, sin ningún parámetro de configuración) y lo
lee de `GET /syslog.log` con la misma sesión
(`hgu::Client::registro_del_sistema`). La exportación no cambia ningún
ajuste: no pasa por el gestor de cambios, no queda en el Historial y no
invalida la caché de lecturas. La Biblioteca sigue sin ofrecer `export` ni
abrir `syslog.asp`.

El texto se ve en un bloque que se desplaza, con lo más nuevo abajo, con un
filtro de líneas (cada palabra, sin mayúsculas) y «Leído hace…».
«Descargar» guarda el registro entero (no solo lo filtrado) como
`registro-router-AAAAMMDD-HHMM.log`, con la hora de la lectura. Si el
registro pasa de 2 MiB (`hgu::client::TOPE_REGISTRO`), solo llegan sus
últimas líneas enteras y la sección lo dice; el panel nunca guarda en
memoria mucho más que eso mientras lo lee.

**Pendiente de comprobar en el router real (T12).** Que `cbSetSyslog` con
solo `export=EXPORT` responde bien y no cambia lo que da `cbGetSyslog`;
que el registro está en `/syslog.log` y se sirve con la sesión del panel
(y con qué tipo de contenido); qué tamaño tiene; y qué responde
`/syslog.log` antes de la primera exportación. El simulador supone que el
fichero no existe (404) hasta la primera exportación desde el arranque y
que después se sirve hasta el siguiente reinicio.
