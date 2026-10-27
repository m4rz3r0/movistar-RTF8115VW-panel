# Funciones del panel

La API del router está en [api.md](api.md). El panel ofrece a su web unas
*server functions* de Leptos: `POST /api/<nombre>` con un cuerpo JSON, la
cookie de sesión (todas la exigen) y la protección CSRF del panel. Los errores
llegan traducidos al idioma de la sesión, y las respuestas con secretos van
con `Cache-Control: no-store`. Aquí están las de Ajustes, Mejoras, el
Historial y la contraseña del router. Las demás (Estado, la Biblioteca, el
asistente, los usuarios) se documentan en su código, junto a cada
`#[server(endpoint = "…")]`.

| Función | Qué hace |
|---|---|
| `leer_tema` | La página de un tema (`wifi`, `puertos`…): resumen, elementos con su estado, datos, recomendación principal y ajustes con el valor actual, acciones sobre varios elementos y cuánto hace que se leyó. En Puertos y Red local, también la red de casa (dispositivos, IP y máscara del router, IP fijas). Cada recomendación dice a qué lotes de Mejoras pertenece. Usa la caché corta si está; si una lectura falla, solo quedan «Sin leer» los elementos que la usan. Con `fresca` («Volver a leer»), sin caché. 404 si el tema no existe. `no-store`. |
| `preparar_accion` | Prepara una acción sobre hasta 20 elementos de un tema (quitar el WPS de dos redes, borrar tres reglas de puertos) como un solo cambio, con un paso por elemento y un solo deshacer. Lo que ya tiene el valor sale como «ya estaba así» y no se escribe. Las instancias de una lista se borran de mayor a menor id, comprobando antes de cada paso que siguen siendo las mismas. 400 si la acción no vale para alguno; 409 si ninguno cambia, si otro cambio se está escribiendo o si la cola está llena; 403 en solo lectura. |
| `preparar_ajuste` | Prepara el guardado de un ajuste: lee el router, calcula lo que cambia en cada paso y devuelve el resumen con su token. 409 si ya está así, si otro cambio se está escribiendo o si la cola está llena; 403 en solo lectura; 400 si un valor no vale. |
| `preparar_elemento` | Como `preparar_ajuste`, pero con varios ajustes de un elemento (lo esencial y lo avanzado de una red) en un solo cambio. |
| `leer_mejoras` | La página Mejoras: por lote, el estado de cada paso (hecho, pendiente, sin leer con su motivo, o no aplica), «x de y pendientes» y el estado del lote, más el contador. Lo que no se pudo leer no cuenta ni como hecho ni como pendiente. Con `fresca`, sin caché. `no-store`. |
| `contar_mejoras` | El contador de «Mejoras» de la barra lateral: los pasos pendientes de todos los lotes, sin repetir los compartidos; `null` si no se pudo leer nada. |
| `preparar_lote` | Prepara un lote (`id`, y en `opciones` el proveedor de DNS o `mantener`) como un solo cambio, con un paso por cada paso que se pudo leer y aplica. La hora neutra sale desmarcada. 403 en solo lectura; 404 si el lote no existe; 400 si el proveedor no existe; 409 si ya está aplicado, si solo faltan pasos sin leer, si otro cambio se está escribiendo o si la cola está llena; 502 si no se pudo leer nada. |
| `confirmar_ajuste` | Aplica lo preparado por las funciones anteriores (sin los pasos desmarcados) y devuelve el id del cambio. Si otro cambio se está vigilando, espera en la cola. 409 si la cola está llena; el token sigue valiendo. |
| `estado_cambio` | Estado de un cambio: `Aplicando`, `Vigilando` (segundos que faltan y `terminable`), `EnCola` (`puesto` y `quitable`), `Aplicado`, `Revertido`, `Fallido` o `Cancelado`, con la nota y el error traducidos. |
| `terminar_vigilancia` | «Todo bien, terminar»: da por buena la vigilancia del cambio `id`, que queda aplicado, y el Historial anota quién la terminó. Solo quien hizo el cambio (403 si no), mientras se vigila y si la última comprobación salió bien (409 si no). |
| `quitar_de_la_cola` | Quita de la cola el cambio `id`, que queda `Cancelado`. Solo quien lo encoló (403 si no); 409 si ya no espera. |
| `deshacer_ajuste` | Prepara el cambio que deshace uno aplicado desde Ajustes; se confirma con `confirmar_ajuste`. |
| `comparar_con_partida` | Lo que ha cambiado desde la foto de partida que tomó el asistente. Solo viajan los secretos que difieren. `no-store`. |
| `preparar_volver_a_partida` | Prepara volver un ajuste (o uno de sus campos) al valor de la foto de partida; se confirma con `confirmar_ajuste`. |
| `historial` | Una página del Historial: cambios y secretos revelados, del más reciente al más antiguo. |
| `deshacer` | Prepara el cambio inverso de uno aplicado; se confirma con `confirmar_cambio`. |
| `secretos_del_cambio` | El valor anterior y el nuevo de los secretos de un cambio. `no-store`. |
| `comprobar_contrasena_router` | Comprueba, sin escribir nada, que se puede cambiar la contraseña del router a la nueva. Exige reautenticación. |
| `cambiar_contrasena_router` | Cambia la contraseña del router; el panel pasa a usarla si entra con ella. Exige reautenticación; 409 con otro cambio en curso. |
| `leer_registro_router` | El registro del sistema del router: `texto`, `recortado` y `apagado` (vacío porque el registro está apagado). 503 sin router configurado; 502 si el router no lo da. `no-store`. |
| `estado_copia_router` | Por qué no se puede hacer ahora la copia de la configuración (`funcion-sin-comprobar` o `panel-solo-lectura`), o `null` si se puede. |
| `descargar_copia_router` | Guarda la configuración del router y descarga la copia: `nombre` (`config_<fecha>.tar`) y `datos` (en base64). 403 en solo lectura; 409 sin comprobar o con un cambio en curso; 503 sin router configurado; 502 si el router no la da. `no-store`. |

## Qué escribe el panel

Cada escritura o acción de `crates/catalogo/funciones.toml` lleva
`comprobada`: la referencia de la prueba en el router real (por ejemplo
`"ida y vuelta"`) o `false`. Sin el campo cuenta como `false`, y un test exige
que toda escritura lo lleve. Otro test fija la lista de las comprobadas
(`crates/catalogo/src/comprobadas.rs`): para marcar una función como
comprobada se cambia su línea y se añade allí.

Comprobadas:

- **Seguridad y sistema:** gestión remota y aviso al operador
  (`cbSetTr69Config`), hora (`cbSetTimeNtp`; `cbSetLocalTime` para fijarla a
  mano), registro (`cbSetSyslog`), espejo de la WAN (`cbSetDiagConfig`),
  reiniciar (`cbReboot`), copia de la configuración (`cbSaveConfig`,
  `cbDownloadConfig`) y cuentas de la web del router (`cbAddUser`,
  `cbDeleteUser`).
- **Cortafuegos:** filtros y reglas (`scbidirectionalflt.cmd` con
  `action=add`, `add.regla`, `modify`, `remove`, `add.filtro`,
  `modify.filtro` y `remove.filtro`).
- **WiFi:** cada red (`cbSetWifiAccPt`, también el WPS) y las radios
  (`cbSetWifi`).
- **Red de casa:** el grupo DHCP (`cbSetDhcpv4ServerPool`), sus reservas
  (`cbDhcpV4SvrPoolSetStatic`), IPv6 (`cbSetLanIPv6Config`) y relé DNS
  (`cbSetDnsRelay`).
- **Puertos, NAT y multicast:** reglas de puertos (`cbSetNatPortMapping`),
  UPnP (`cbSetUpnpCfg`), ALG y NAT loopback (`cbSetNatConfig`,
  `cbSetNatConfig.loopback`), IGMP (`cbSetMcastIgmpSetting`,
  `cbSetMcastIntf`) y RIP (`cbSetRipIntf`).
- **QoS:** reglas de clasificación (`cbSetClassification`).
- **Teléfono:** servidores (`cbSetVoiceSip`) y puerto físico
  (`cbSetVoicePhysical`).

Probadas y descartadas: `cbSetDdns` (crear una entrada inactiva colgó la web
del router hasta reiniciarlo; no se vuelve a probar), `cbSyncTime` (el router
no la confirma), `cbSetSystemTimeZone` y `cbSetNTPInfo` (su lectura no existe
en este router).

Entre las pendientes están las colas de QoS (`cbSetQueue` para el estado de
cada cola y `cbSetQueue.defecto` para la cola por defecto, tras la que la web
cierra la sesión), las opciones DHCP del grupo (`cbDhcpV4SvrPoolSetOpt`, en la
Biblioteca, con el valor siempre en hexadecimal) y la DMZ
(`cbSetNatPortMapping.dmz`).

### Lo que no está comprobado

Lo que no está comprobado solo se ve:

- **En la página**, sus campos salen como valores, sin interruptor, sin botón
  de guardar y sin acciones de la lista. Una recomendación sobre un campo sin
  comprobar se queda como información, sin botón ni lotes.
- **En Mejoras**, sus pasos no salen. «Cómo estaba al instalar» no ofrece
  volver a esos valores.
- **En la Biblioteca**, la función sale bloqueada, con su motivo.
- **El gestor** responde 409 sin escribir (`funcion-sin-comprobar`), venga la
  petición de donde venga. Deshacer y revertir lo ya hecho siguen
  funcionando.

La contraseña del propio router (`cbModifyPwd`) está sin comprobar y aun así
sigue activa: su flujo no pasa por el gestor y está protegido (prueba la
nueva, vuelve a la anterior si no entra y avisa). La prueba en el router real
se hará con él (TODO en el catálogo y en `contrasena_router.rs`).

Dos funciones están desactivadas en este router
(`crates/panel/src/validacion_real.rs`) aunque sus escrituras estén
comprobadas. La tarjeta solo enseña sus valores y una nota con el motivo, y
`cambiar_usuario_router` y `preparar_sin_ping` responden 409 sin escribir:

- **Cambiar la contraseña o el grupo de otra cuenta** de la web del router.
  Solo la cuenta `1234` entra en la web (`cbCheckLogin` responde `Result=4` a
  las demás), así que el panel no puede comprobar que la contraseña nueva
  funciona. Para cambiarla, borra la cuenta y créala de nuevo.
- **«No responder al ping desde internet»**: la regla se pone y se quita
  bien, pero el router sigue respondiendo porque otra regla del operador lo
  permite.

Las pruebas contra el simulador (router-sim) lo habilitan todo con
`RP_E2E_HABILITAR_TODO=1` (`e2e/arrancar.sh`, `Config::habilitar_todo`). Solo
lo atiende un binario de depuración; `deploy/pruebas/release-local.sh`
comprueba que el de release no lo hace.

### Casillas y otros marcadores del catálogo

En varias páginas de la web del router, un sí/no es una casilla sin `value`:
la web manda `on` si está marcada y nada si no lo está, y el router la da por
activa si el campo llega, valga lo que valga. El catálogo las marca con
`casilla` y el panel hace como la web. Pasa en el grupo DHCP, las ALG de NAT,
la clasificación QoS, el filtrado y el marcado L2, la WAN (`trigger`,
`dns_override`) y la línea directa del teléfono. No todas las CGI van así: la
de la WiFi guarda `ssid_hide=0` como «no».

En el grupo DHCP esto importa mucho. Mandar `dhcpv4_opt60=0` dejaba activas,
con el valor vacío, las condiciones del grupo (opción 60, 61, 77 y MAC), y
tras reiniciar el router dejaba de dar IP a los dispositivos nuevos. El panel
nunca deja activa una condición sin valor (o con una MAC todo ceros): si ya lo
estaba, la desactiva y el diff lo dice. Activar una sin valor da error.

Otros marcadores:

- `fijo`: un valor que la web manda siempre en un campo oculto y que la
  lectura no da (`set_nf_conntrack=1` en las ALG).
- `depende`: un campo que la web desactiva según otro y que por eso no viaja
  (el valor de una condición de QoS o del filtrado L2 con su casilla
  desmarcada, por ejemplo).
- `cgi`: dos funciones del catálogo que la web manda a la misma CGI con
  campos distintos (`cbSetQueue` y `cbSetQueue.defecto`; `cbSetNatConfig` y
  `cbSetNatConfig.loopback`).
- `ranuras`: una lista de tamaño fijo dentro de una función. Las IP
  reservadas del grupo DHCP (las que no reparte) ocupan 32 ranuras
  (`reserved_ipv4_addr_<i>` y `reserved_ipv4_mask_<i>`; una libre vale
  0.0.0.0). Como en la web, una nueva va a la ranura siguiente a la última
  usada y quitar una es ponerla a 0.0.0.0. A diferencia de la web, el panel
  manda además todo el grupo, para no desmarcar ninguna condición.

## Vigilancia y cola de cambios

Solo se vigila lo que puede cortar el acceso del panel al router o la
conexión a internet. Cada escritura del catálogo lo declara en `acceso` (el
catálogo no carga si falta) y el gestor decide en
`crates/panel/src/changes/acceso.rs`:

- `puede_cortar` (se vigila siempre): la red de casa (IP, máscara, DHCP y sus
  reservas, sus DNS, el relé de DNS), la WAN, el cortafuegos, rutas, colas y
  filtros, la contraseña del router, reinicios, firmware y restaurar la
  configuración.
- `wifi`: se vigila salvo que el panel compruebe que llega al router por cable
  (su IP en la lista de dispositivos del router y su MAC en ninguna red WiFi).
- `no_corta` (queda aplicado en cuanto la relectura muestra lo nuevo): UPnP,
  reglas de puertos, TR-069, hora y NTP, registro del sistema, DDNS, voz,
  multicast y las cuentas de la web del router.

Un cambio de varios pasos se vigila si alguno lo pide. Mientras se vigila:

- **«Todo bien, terminar»** da el cambio por bueno antes de tiempo. Sale en el
  aviso de Ajustes, en el diálogo del cambio y en su fila del Historial, solo
  para quien lo hizo.
- **La cola** guarda en orden lo que se confirma, como mucho 5 cambios
  (`MAX_COLA`). Justo antes de escribir, el gestor vuelve a leer el router y,
  si ya no está como cuando se revisó, no escribe (`Cancelado`: «el router ha
  cambiado desde que se revisó»). La cola solo sigue si el anterior queda
  aplicado; si se deshace o falla, los que esperaban quedan `Cancelado`. Si el
  panel se reinicia, lo que estaba en la cola queda `Cancelado`: nunca se
  aplica a ciegas.
- Detrás de un cambio de las credenciales del router no se encola nada (409).

## Mejoras: lotes, registro del sistema y DNS de la red de casa

Los lotes se declaran en `crates/ajustes/ajustes/lotes.toml`, con sus textos
en los `.ftl`. Un paso apunta a una acción de Ajustes o a un campo y su valor,
y puede repetirse en cada red WiFi (`para_cada`), depender de una condición
(`si`), salir desmarcado (`opcional`) o tomar el valor de una elección
(`elegir`). Un test comprueba que cada paso apunta a algo que existe y se
puede escribir, con valores válidos y textos en los dos idiomas.

| Lote | Pasos |
|---|---|
| Seguridad recomendada | Gestión remota del operador (TR-069) y su aviso periódico apagados; WPS fuera y WPA2-AES en cada red WiFi; UPnP apagado; dispositivos de la red de invitados aislados entre sí (si está encendida); espejo del tráfico apagado; registro sin envío remoto. Además, consejos sin botón para lo que la API no deja cambiar. |
| Privacidad máxima | Gestión remota y aviso periódico apagados; espejo apagado; registro sin envío remoto; DNS neutros en la red de casa, con proveedor a elegir (Quad9, Quad9 sin filtro, DNS4EU, Cloudflare, AdGuard o Cloudflare familiar, que filtra webs para adultos y malware) o «Mantener los actuales»; hora de un servidor neutro (`hora.roa.es`), opcional. |

Los dos ajustes que los lotes necesitan:

- **Registro del sistema** (`sistema.registro`, en Sistema): si el router
  envía su registro a otro equipo. Se lee con `cbGetSyslog` y se escribe con
  `cbSetSyslog` (`syslog_mode`: `2`, local y remoto; `0`, solo local; `1`,
  solo remoto, que se lee como no reconocido). Encender el envío exige el
  servidor. Las escrituras nunca llevan `export`, que exporta el registro.
- **DNS de la red de casa** (`red_local.dns_casa`, en Red local): los DNS que
  el router reparte por DHCP, solo en el grupo de id 0 (`ipv4_dns_addr_0` e
  `ipv4_dns_addr_1`). Nunca toca los de la WAN, el relé de DNS ni los demás
  grupos, que pueden afectar a la televisión, al teléfono o a otros
  dispositivos del operador. Los dispositivos preguntan a esos servidores sin
  cifrar.

  Un DNS principal que no es el router (su IP de casa, vacío o `0.0.0.0`) ni
  del operador (`del_operador` en el catálogo) es **propio**: la tarjeta no
  recomienda cambiarlo y Privacidad máxima da el paso por hecho («DNS
  propios»), salvo que el secundario sea el router o del operador. El
  principal de un proveedor de la lista sin secundario también cuenta como
  hecho («Quad9 (solo principal)»).

**El grupo DHCP se busca por su id.** `cbGetDhcpv4ServerPool` elige el grupo
por su posición y `cbSetDhcpv4ServerPool`, por su id, y en el router real no
coinciden: el grupo de casa (id 0) está en la posición 1. Los ajustes que leen
por posición y escriben por id lo declaran con `buscar` en su `leer` (los del
grupo DHCP de casa y los de cada radio WiFi): el panel busca la instancia con
ese id y, si no la encuentra, el ajuste queda «sin leer». El gestor de cambios
hace lo mismo con toda lectura pareada que pasa el id como posición
(`X_0i = {X_id}`).

**DMZ.** En el router real no tiene lectura propia: es una redirección de
puertos más, con `desc=DMZ` y sin protocolo ni puertos. Los dos ajustes leen
la misma lista y se la reparten con `solo` y `excluir` en su `leer`. Se
escribe con `cbSetNatPortMapping.dmz`, sin comprobar: en Puertos solo se ve.

**Red de invitados.** El panel solo cambia si aísla entre sí a sus
dispositivos (`isolation`). Si está separada de la red de casa
(`guest_networking`) solo se lee: ninguna escritura del catálogo lleva ese
campo.

Falta comprobar en el router real:

- que cargar `syslog.asp` no exporta el registro, y que guardar «no enviar»
  con el puerto remoto vacío o `0` no falla;
- que la televisión y el teléfono siguen tras apagar TR-069 y reiniciar, y
  que el descodificador no usa el grupo DHCP de casa;
- si 1.1.1.1 resuelve en este firmware;
- qué significa `guest_networking = 1` (se lee como «separada»);
- si la posición de `cbGetWifi` coincide con el id de la radio: las lecturas
  de estado de la WiFi (`hgu::read::wifi` y la detección de radios) aún toman
  la posición como id.

## Copia de la configuración (descargar)

Al pie de Ajustes → Sistema, «Descargar copia» baja la copia de seguridad de
la configuración del router, como `backup_restore.asp` en su web avanzada. En
un mismo turno de la sesión con el router, el panel:

1. guarda la configuración (`cbSaveConfig` con `save=1`), porque la copia
   solo lleva lo último guardado;
2. genera la copia (`cbSaveConfig` con `backup=1`, que da el nombre);
3. la descarga (`POST /cgi-bin/cbDownloadConfig.xml` con `fileName` y
   `sessionKey` en la URL).

El navegador la guarda con el nombre del router (`config_<fecha>.tar`). El
panel no acepta más de 16 MiB (`hgu::client::TOPE_COPIA`).

El fichero lleva todas las contraseñas del router, y la sección lo avisa. El
panel no lo anota en el registro, no lo guarda en ningún sitio y no enseña su
contenido. Como guarda la configuración, se trata como una escritura: no se
hace en solo lectura (403), con un cambio en curso (409) ni si las dos
funciones dejan de estar comprobadas; entonces el botón sale desactivado con
el motivo (`estado_copia_router`). No pasa por el gestor de cambios ni queda
en el Historial.

## Registro del router (ver y descargar)

Al pie de Ajustes → Sistema, «Leer el registro» enseña el registro del sistema
del router, como `syslog.asp` en su web avanzada. El panel pide al router que
lo exporte (`cbSetSyslog` con **solo** `export=EXPORT` y el `sessionKey` de
`syslog.asp`) y lo lee de `GET /syslog.log` con la misma sesión. La
exportación no cambia ningún ajuste: no pasa por el gestor, no queda en el
Historial y no invalida la caché de lecturas.

El texto sale con lo más nuevo abajo, un filtro de líneas y «Leído hace…».
«Descargar» guarda el registro entero, no solo lo filtrado, como
`registro-router-AAAAMMDD-HHMM.log`. Si pasa de 2 MiB
(`hgu::client::TOPE_REGISTRO`), solo llegan sus últimas líneas y la sección lo
dice. Si llega vacío y `syslog_enable` está apagado en `cbGetSyslog`, la
sección lo dice y remite al campo «Guardar el registro» de esa misma pestaña.

Falta comprobar en el router real qué tamaño suele tener, con qué tipo de
contenido se sirve y qué responde `/syslog.log` antes de la primera
exportación (el simulador da 404).

## Cortafuegos: editor de filtros y reglas

En Ajustes → Seguridad, el detalle del cortafuegos enseña los filtros con sus
reglas en el orden en que el router las mira. Como el editor oculto de la web
(`scbidirectionalflt_ed.asp`), permite:

- **Reglas:** añadir una al final de un filtro, cambiarla entera o borrarla.
  La carga es la de la web: el alta lleva `escape=true` y lo que la web
  deshabilita no viaja (los puertos solo con TCP o UDP, la máscara solo con
  dirección, etc.). La baja es `item=<FID>|<RID>`, nunca un FID suelto.
- **Filtros:** añadir uno vacío, cambiarlo (con reglas, sin cambiar de versión
  de IP) o borrarlo si está vacío: `item=<FID>` borraría también sus reglas y
  el deshacer no podría devolverlas.

Cada cambio va por la cola, con revisión, relectura, vigilancia y deshacer.
Antes de escribir, lo que enseñaba la página tiene que seguir igual en el
router. Solo se cambian o borran las reglas y los filtros que el panel sabe
volver a escribir igual: una regla del operador con el protocolo «todos», que
el editor no ofrece, se ve pero no se toca. Deshacer el borrado de una regla
la vuelve a crear al final de su filtro, como la web.
