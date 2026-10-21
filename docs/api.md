# API del router Askey RTF8115VW (Movistar/O2)

<!-- Generado por `cargo run -p gen-catalogo -- --api-md` a partir de
crates/catalogo/funciones.toml. No editar a mano. -->

## Cómo se llama

- **UI avanzada**: `POST /cgi-bin/<función>.xml` con los parámetros como
  formulario. Inicio de sesión con `cbCheckLogin.xml` (`rqUsername`,
  `rqPasswd` ofuscados con XOR 0x1f, `rqTimeout`); la respuesta es
  `<Response><función>…<Result>0</Result></función></Response>` (`Result` 0
  = correcto, 1 = error con `ErrorString`).
- Las escrituras llevan además `sessionKey`, la clave de sesión que da la
  página indicada. Las listas se leen con `all=ALL` (ids) y cada
  instancia con `<obj>_0i=<id>` o `<obj>_id=<id>`; los borrados llevan
  `del=DEL`; `-1` como id crea una instancia nueva.
- Los campos IP/MAC partidos (`<nombre>-0..-3` o `-0..-5`) se envían unidos
  en `<nombre>` (`beforeSendForm` de `tabs.js`).
- **UI básica**: `GET /cgi-bin/<página>.cmd|.cgi|.wl?…` con los valores en
  la URL, `sessionKey` y la cookie `_httpdSessionId_`; las claves WiFi y
  contraseñas van ofuscadas con XOR 0x1f.
- Riesgo: vigilancia tras el cambio de 30 s (bajo), 2 min (medio) o 5 min
  (alto). Las lecturas no tienen riesgo. Las funciones **bloqueadas** no se
  invocan nunca desde el panel.
- En la lectura pareada, `{param}` es el valor de ese parámetro en la
  escritura. Elementos marcados con 🔒 se enmascaran siempre
  (`crates/catalogo/secretos.toml`).

## Resumen

| Área | Funciones | Lecturas | Escrituras | Acciones | Bloqueadas |
|---|---|---|---|---|---|
| [WAN](#wan) | 14 | 8 | 6 | 0 | 0 |
| [LAN/DHCP](#landhcp) | 22 | 12 | 10 | 0 | 0 |
| [WiFi](#wifi) | 20 | 11 | 8 | 1 | 0 |
| [NAT](#nat) | 16 | 6 | 10 | 0 | 0 |
| [Cortafuegos](#cortafuegos) | 4 | 1 | 3 | 0 | 0 |
| [DNS](#dns) | 6 | 3 | 3 | 0 | 0 |
| [IPv6](#ipv6) | 8 | 2 | 6 | 0 | 0 |
| [Multicast/IGMP](#multicastigmp) | 5 | 3 | 2 | 0 | 0 |
| [QoS](#qos) | 7 | 4 | 3 | 0 | 0 |
| [VoIP](#voip) | 7 | 4 | 3 | 0 | 0 |
| [TR-069](#tr-069) | 2 | 1 | 1 | 0 | 0 |
| [Usuarios](#usuarios) | 11 | 2 | 2 | 7 | 7 |
| [Sistema](#sistema) | 30 | 10 | 4 | 16 | 10 |
| [Ocultas](#ocultas) | 8 | 8 | 0 | 0 | 0 |

## WAN

### `cbGetGPONInfo`

*lectura* · UI avanzada · riesgo bajo · página `summary.asp`

Estado de la fibra (GPON): estado de la ONT, serie y contraseña GPON (enmascaradas).

Elementos de la respuesta: `AcsSerialNumber` 🔒, `GponSerialNumber` 🔒, `Opstatus`, `Password` 🔒

### `cbGetIpIntf`

*lectura* · UI avanzada · riesgo bajo · página `ipv4.asp`

Lista de interfaces IP (LAN y WAN) con `all=ALL`; la usan muchas páginas para rellenar desplegables.

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `all` | Todas (siempre «ALL») | text |  |

Elementos de la respuesta: `adm_state`, `desc`, `dev_name`, `intf_id`, `intf_type`, `ipv4_adm_state`, `ipv4_opr_status`, `ipv6_adm_state`, `ipv6_opr_status`, `mtu`, `name`, `opr_status`, `rt_id`, `rtm_cfg_ip_intf`, `rtm_cfg_ip_intf_0i`, `rtm_cfg_ip_intf_id`, `type`

### `cbGetRipIntf`

*lectura* · UI avanzada · riesgo bajo · página `rip.asp`

Configuración de RIP por interfaz.

Elementos de la respuesta: `accept_ra`, `adm_state`, `ip_intf_id`, `rip_intf_0i`, `rip_intf_id`, `rtm_cfg_rip_intf`, `send_ra`

### `cbSetRipIntf`

*escritura* · UI avanzada · riesgo alto · página `rip.asp`

Activa RIP y el envío/recepción de anuncios en una interfaz.

Lectura pareada: `cbGetRipIntf`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `accept_ra` | Aceptar anuncios RIP | bool |  |
| `send_ra` | Enviar anuncios RIP | bool |  |
| `rip_intf_id` | Interfaz RIP (id) | int |  |

### `cbGetRoutingV4Rule`

*lectura* · UI avanzada · riesgo bajo · página `default_gateway.asp`

Rutas IPv4 (estáticas y puerta de enlace por defecto con `desc=Default`).

Lista: instancia por `rtm_rulev4_0i`; `all=ALL` lista todas

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `all` | Todas (siempre «ALL») | text |  |
| `desc` | Descripción | text |  |
| `rtm_rulev4_0i` | Ruta IPv4 (índice) | int |  |

Elementos de la respuesta: `adm_state`, `desc`, `dst_addr`, `dst_mask`, `fwd_policy`, `gw_addr`, `ip_intf_id`, `is_link`, `is_static`, `is_tef_hidden`, `metric`, `opr_ip_intf_id`, `opr_ipv4_addr_id`, `opr_status`, `rtm_cfg_rulev4`, `rtm_rulev4_0i`, `rtm_rulev4_id`, `type`

### `cbSetRoutingV4Rule`

*escritura* · UI avanzada · riesgo alto · página `default_gateway.asp`

Crea, modifica o borra una ruta IPv4 o la puerta de enlace por defecto.

Lectura pareada: `cbGetRoutingV4Rule` con `rtm_rulev4_0i={rtm_rulev4_id}`

Lista: instancia por `rtm_rulev4_id`; `del=DEL` borra

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `ip_intf_id` | Interfaz IP | int |  |
| `rtm_rulev4_id` | Ruta (id; -1 = nueva) | int |  |
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `metric` | Métrica | int |  |
| `fwd_policy` | Política | text |  |
| `dst_addr` | Destino | ip (Ip4) |  |
| `dst_mask` | Máscara de destino | ip (Ip4) |  |
| `gw_addr` | Puerta de enlace | ip (Ip4) |  |

### `cbGetRoutingV6Rule`

*lectura* · UI avanzada · riesgo bajo · página `default_gateway.asp`

Rutas IPv6 (estáticas y puerta de enlace por defecto con `desc=Default`).

Lista: instancia por `rtm_rulev6_0i`; `all=ALL` lista todas

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `all` | Todas (siempre «ALL») | text |  |
| `desc` | Descripción | text |  |
| `rtm_rulev6_0i` | Ruta IPv6 (índice) | int |  |

Elementos de la respuesta: `adm_state`, `desc`, `dst_addr`, `dst_prefix`, `fwd_policy`, `ip_intf_id`, `is_link`, `is_static`, `is_tef_hidden`, `metric`, `next_hop_addr`, `opr_ip_intf_id`, `opr_ipv6_addr_id`, `opr_status`, `rtm_cfg_rulev6`, `rtm_rulev6_0i`, `rtm_rulev6_id`, `type`

### `cbSetRoutingV6Rule`

*escritura* · UI avanzada · riesgo alto · página `default_gateway.asp`

Crea, modifica o borra una ruta IPv6 o la puerta de enlace IPv6 por defecto.

Lectura pareada: `cbGetRoutingV6Rule` con `rtm_rulev6_0i={rtm_rulev6_id}`

Lista: instancia por `rtm_rulev6_id`; `del=DEL` borra

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `ip_intf_id` | Interfaz IP | int |  |
| `rtm_rulev6_id` | Ruta (id; -1 = nueva) | int |  |
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `dst_addr` | Destino | text |  |
| `dst_prefix` | Longitud de prefijo | int |  |
| `next_hop_addr` | Siguiente salto | text |  |
| `metric` | Métrica | int |  |
| `fwd_policy` | Política | text |  |

### `cbGetStatistic`

*lectura* · UI avanzada · riesgo bajo · página `wanstats.asp`

Contadores de tráfico por interfaz (bytes, paquetes, errores); la WAN PPP es `ip2`.

Elementos de la respuesta: `desc`, `dpm_cfg_eth`, `dpm_cfg_eth_id`, `dpm_wifi_ap`, `dpm_wifi_ap_id`, `name`, `rtm_cfg_ip_intf`, `rtm_cfg_ip_intf_id`, `rx_byte`, `rx_pkt`, `rx_pkt_broadcast`, `rx_pkt_discard`, `rx_pkt_err`, `rx_pkt_multicast`, `rx_pkt_unicast`, `tx_byte`, `tx_pkt`, `tx_pkt_broadcast`, `tx_pkt_discard`, `tx_pkt_err`, `tx_pkt_multicast`, `tx_pkt_unicast`

### `cbGetWanIPIntf`

*lectura* · UI avanzada · riesgo bajo · página `wanintf.asp`

Conexiones WAN (PPPoE/DHCP/estática, VLAN, IPv6): estado operativo, IP y credenciales PPP (enmascaradas). Sin parámetros, todas; con `ip_intf_0i`, una.

Lista: instancia por `ip_intf_0i`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `ip_intf_0i` | Conexión WAN (índice) | int |  |

Elementos de la respuesta: `ac_name`, `adm_state`, `dhcpv4_req_opt60_value`, `dhcpv4_req_opt61_value`, `dhcpv4_req_opt77_value`, `dis_ipv4_addr`, `dis_ipv4_gw`, `dis_ipv6_addr`, `dns_override`, `dnsv4_svr0`, `dnsv4_svr1`, `idle_dis_time`, `ip_intf_0i`, `ip_intf_id`, `ipcpv4_req_local_addr`, `ipv4_adm_state`, `ipv4_opr_status`, `ipv6_addr_type`, `ipv6_adm_state`, `ipv6_opr_status`, `ipv6_type`, `ipv6_unnumbered`, `mll_wanconn_data`, `mtu`, `name`, `opr_app_mask`, `opr_status`, `password` 🔒, `ppp_passthru`, `service_name`, `static_ipv4_addr`, `static_ipv4_gw`, `static_ipv4_mask`, `trigger`, `untagged`, `username` 🔒, `vlan8021p`, `vlanid`, `wan_type`

### `cbSetWanIPIntf`

*escritura* · UI avanzada · riesgo alto · página `wanintf.asp`

Crea, modifica o borra una conexión WAN: tipo, VLAN, credenciales PPP, IP estática, DNS e IPv6. Puede dejar sin internet.

Lectura pareada: `cbGetWanIPIntf` con `ip_intf_0i={ip_intf_id}`

Lista: instancia por `ip_intf_id`; `del=DEL` borra

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `untagged` | Sin etiqueta VLAN | enum | `0` Con etiqueta, `1` Sin etiqueta |
| `vlanid` | VLAN | int |  |
| `vlan8021p` | Prioridad 802.1p | enum | `0` 0, `1` 1, `2` 2, `3` 3, `4` 4, `5` 5, `6` 6, `7` 7 |
| `wan_type` | Tipo de conexión | enum | `0` PPPoE, `1` DHCP, `2` IP estática |
| `username` | Usuario PPP | secret |  |
| `password` | Contraseña | secret |  |
| `service_name` | Nombre de servicio PPPoE | text |  |
| `ac_name` | Concentrador PPPoE (AC) | text |  |
| `trigger` | Conexión bajo demanda | bool |  |
| `idle_dis_time` | Desconexión por inactividad (s) | int |  |
| `dhcpv4_req_opt60_value` | Opción 60 enviada | text |  |
| `dhcpv4_req_opt61_value` | Opción 61 enviada | text |  |
| `dhcpv4_req_opt77_value` | Opción 77 enviada | text |  |
| `dns_override` | DNS manuales | bool |  |
| `ipv6_type` | IPv6 | enum | `0` Doble pila, `1` Solo IPv6, `2` Desactivado |
| `ipv6_unnumbered` | IPv6 sin numerar | enum | `1` Activado, `0` Desactivado |
| `ipv6_addr_type` | Direccionamiento IPv6 | enum | `1` SLAAC, `0` DHCP |
| `ip_intf_id` | Interfaz IP | int |  |
| `dnsv4_svr0` | DNS primario | ip (Ip4) |  |
| `dnsv4_svr1` | DNS secundario | ip (Ip4) |  |
| `static_ipv4_addr` | IP estática | ip (Ip4) |  |
| `static_ipv4_gw` | Puerta de enlace estática | ip (Ip4) |  |
| `static_ipv4_mask` | Máscara estática | ip (Ip4) |  |

### `te_mm.cmd?action=Apply`

*escritura* · UI básica · riesgo alto · página `te_multipuesto_monopuesto.asp`

Cambia entre multipuesto (router) y monopuesto (puente) con usuario y contraseña PPPoE. Puede dejar sin internet.

Lectura pareada: `te_multipuesto_monopuesto.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `userselection` | Modo (multipuesto/monopuesto) | text |  |
| `PPPoEUsername` | Usuario PPPoE | secret |  |
| `PPPoEPassword` | Contraseña PPPoE | secret |  |
| `curWebPage` | Página de retorno | text |  |

### `te_mrl.cmd?action=wanApply`

*escritura* · UI básica · riesgo alto · página `te_mapa_red_local.asp`

Cambia el modo WAN (multipuesto/monopuesto) y las credenciales PPPoE desde el mapa de red de la UI básica.

Lectura pareada: `te_mapa_red_local.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `userselection` | Modo (multipuesto/monopuesto) | text |  |
| `PPPoEUsername` | Usuario PPPoE | secret |  |
| `PPPoEPassword` | Contraseña PPPoE | secret |  |
| `curWebPage` | Página de retorno | text |  |

### `te_multipuesto_monopuesto.asp`

*lectura* · UI básica · riesgo bajo · página `te_multipuesto_monopuesto.asp`

Página básica del modo WAN; sus variables (`userselection`, `wanPpp`) dan el modo y las credenciales PPPoE (enmascaradas).

## LAN/DHCP

### `cbDhcpV4SvrPoolGetClt`

*lectura* · UI avanzada · riesgo bajo · página `dhcp.asp`

Clientes con concesión DHCP de un pool (IP, MAC, tiempo restante).

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `rtm_cfg_dhcpv4_svr_pool_id` | Pool DHCP | int |  |
| `all` | Todas (siempre «ALL») | text |  |

Elementos de la respuesta: `dhcpv4_opt`, `ipv4_addr`, `is_active`, `lease_remaining`, `mac_addr`, `rtm_dhcpv4_svr_pool_clt`, `rtm_dhcpv4_svr_pool_clt_0i`, `rtm_dhcpv4_svr_pool_clt_addr`, `rtm_dhcpv4_svr_pool_clt_addr_0i`, `rtm_dhcpv4_svr_pool_clt_addr_id`, `rtm_dhcpv4_svr_pool_clt_id`, `rtm_dhcpv4_svr_pool_clt_opt`, `rtm_dhcpv4_svr_pool_clt_opt_0i`, `rtm_dhcpv4_svr_pool_clt_opt_id`

### `cbDhcpV4SvrPoolGetOpt`

*lectura* · UI avanzada · riesgo bajo · página `dhcp.asp`

Opciones DHCP que entrega un pool.

Lista: instancia por `rtm_dhcpv4_svr_pool_opt_id`; `all=ALL` lista todas

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `rtm_cfg_dhcpv4_svr_pool_id` | Pool DHCP | int |  |
| `rtm_dhcpv4_svr_pool_opt_id` | Opción (id; -1 = nueva) | int |  |
| `all` | Todas (siempre «ALL») | text |  |

Elementos de la respuesta: `adm_state`, `opt_code`, `opt_len`, `opt_value`, `rtm_dhcpv4_svr_pool_opt`, `rtm_dhcpv4_svr_pool_opt_0i`, `rtm_dhcpv4_svr_pool_opt_id`

### `cbDhcpV4SvrPoolSetOpt`

*escritura* · UI avanzada · riesgo bajo · página `dhcp.asp`

Crea, modifica o borra una opción DHCP de un pool (valor en hexadecimal).

Lectura pareada: `cbDhcpV4SvrPoolGetOpt` con `rtm_cfg_dhcpv4_svr_pool_id={rtm_cfg_dhcpv4_svr_pool_id}`, `rtm_dhcpv4_svr_pool_opt_id={rtm_dhcpv4_svr_pool_opt_id}`

Lista: instancia por `rtm_dhcpv4_svr_pool_opt_id`; `del=DEL` borra

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `opt_code` | Código de opción | int |  |
| `opt_len` | Longitud | int |  |
| `opt_value` | Valor (hexadecimal) | text |  |
| `rtm_dhcpv4_svr_pool_opt_id` | Opción (id; -1 = nueva) | int |  |
| `rtm_cfg_dhcpv4_svr_pool_id` | Pool DHCP | int |  |

### `cbDhcpV4SvrPoolGetStatic`

*lectura* · UI avanzada · riesgo bajo · página `dhcp.asp`

Reservas (IP fija por MAC) de un pool DHCP.

Lista: instancia por `rtm_dhcpv4_svr_pool_static_id`; `all=ALL` lista todas

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `rtm_cfg_dhcpv4_svr_pool_id` | Pool DHCP | int |  |
| `rtm_dhcpv4_svr_pool_static_id` | Reserva (id; -1 = nueva) | int |  |
| `all` | Todas (siempre «ALL») | text |  |

Elementos de la respuesta: `adm_state`, `ipv4_addr`, `mac_addr`, `rtm_dhcpv4_svr_pool_static`, `rtm_dhcpv4_svr_pool_static_0i`, `rtm_dhcpv4_svr_pool_static_id`

### `cbDhcpV4SvrPoolSetStatic`

*escritura* · UI avanzada · riesgo bajo · página `dhcp.asp`

Crea, modifica o borra una reserva DHCP (IP fija por MAC).

Lectura pareada: `cbDhcpV4SvrPoolGetStatic` con `rtm_cfg_dhcpv4_svr_pool_id={rtm_cfg_dhcpv4_svr_pool_id}`, `rtm_dhcpv4_svr_pool_static_id={rtm_dhcpv4_svr_pool_static_id}`

Lista: instancia por `rtm_dhcpv4_svr_pool_static_id`; `del=DEL` borra

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `rtm_dhcpv4_svr_pool_static_id` | Reserva (id; -1 = nueva) | int |  |
| `rtm_cfg_dhcpv4_svr_pool_id` | Pool DHCP | int |  |
| `ipv4_addr` | Dirección IPv4 | ip (Ip4) |  |
| `mac_addr` | Dirección MAC | mac (Mac6) |  |

### `cbGetArpCache`

*lectura* · UI avanzada · riesgo bajo · página `arpstats.asp`

Tabla ARP del router.

Elementos de la respuesta: `arp_record`, `device`, `hwaddr`, `ipaddr`, `status`

### `cbGetDhcpv4ServerPool`

*lectura* · UI avanzada · riesgo bajo · página `dhcp.asp`

Pools del servidor DHCPv4: rango, máscara, puerta de enlace, DNS, concesión y condiciones por opción.

Lista: instancia por `rtm_cfg_dhcpv4_svr_pool_0i`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `rtm_cfg_dhcpv4_svr_pool_0i` | Pool DHCP | int |  |

Elementos de la respuesta: `adm_state`, `dhcpv4_opt60`, `dhcpv4_opt60_ex`, `dhcpv4_opt60_matching_mode`, `dhcpv4_opt60_value`, `dhcpv4_opt61`, `dhcpv4_opt61_ex`, `dhcpv4_opt61_value`, `dhcpv4_opt77`, `dhcpv4_opt77_ex`, `dhcpv4_opt77_value`, `domain_name`, `ip_intf_id`, `ipv4_addr_max`, `ipv4_addr_min`, `ipv4_dns_addr_0`, `ipv4_dns_addr_1`, `ipv4_gw_addr`, `ipv4_mask`, `lease_time`, `mac_addr`, `mac_addr_ex`, `mac_addr_mask`, `mac_addr_value`, `opr_status`, `order`, `reserved_ipv4`, `reserved_ipv4_addr_0`, `reserved_ipv4_addr_1`, `reserved_ipv4_addr_10`, `reserved_ipv4_addr_11`, `reserved_ipv4_addr_12`, `reserved_ipv4_addr_13`, `reserved_ipv4_addr_14`, `reserved_ipv4_addr_15`, `reserved_ipv4_addr_16`, `reserved_ipv4_addr_17`, `reserved_ipv4_addr_18`, `reserved_ipv4_addr_19`, `reserved_ipv4_addr_2`, `reserved_ipv4_addr_20`, `reserved_ipv4_addr_21`, `reserved_ipv4_addr_22`, `reserved_ipv4_addr_23`, `reserved_ipv4_addr_24`, `reserved_ipv4_addr_25`, `reserved_ipv4_addr_26`, `reserved_ipv4_addr_27`, `reserved_ipv4_addr_28`, `reserved_ipv4_addr_29`, `reserved_ipv4_addr_3`, `reserved_ipv4_addr_30`, `reserved_ipv4_addr_31`, `reserved_ipv4_addr_4`, `reserved_ipv4_addr_5`, `reserved_ipv4_addr_6`, `reserved_ipv4_addr_7`, `reserved_ipv4_addr_8`, `reserved_ipv4_addr_9`, `reserved_ipv4_mask_0`, `reserved_ipv4_mask_1`, `reserved_ipv4_mask_10`, `reserved_ipv4_mask_11`, `reserved_ipv4_mask_12`, `reserved_ipv4_mask_13`, `reserved_ipv4_mask_14`, `reserved_ipv4_mask_15`, `reserved_ipv4_mask_16`, `reserved_ipv4_mask_17`, `reserved_ipv4_mask_18`, `reserved_ipv4_mask_19`, `reserved_ipv4_mask_2`, `reserved_ipv4_mask_20`, `reserved_ipv4_mask_21`, `reserved_ipv4_mask_22`, `reserved_ipv4_mask_23`, `reserved_ipv4_mask_24`, `reserved_ipv4_mask_25`, `reserved_ipv4_mask_26`, `reserved_ipv4_mask_27`, `reserved_ipv4_mask_28`, `reserved_ipv4_mask_29`, `reserved_ipv4_mask_3`, `reserved_ipv4_mask_30`, `reserved_ipv4_mask_31`, `reserved_ipv4_mask_4`, `reserved_ipv4_mask_5`, `reserved_ipv4_mask_6`, `reserved_ipv4_mask_7`, `reserved_ipv4_mask_8`, `reserved_ipv4_mask_9`, `rtm_cfg_dhcpv4_svr_pool`, `rtm_cfg_dhcpv4_svr_pool_0i`, `rtm_cfg_dhcpv4_svr_pool_id`

### `cbSetDhcpv4ServerPool`

*escritura* · UI avanzada · riesgo medio · página `dhcp.asp`

Configura un pool DHCPv4 (rango, máscara, puerta de enlace, DNS, concesión, IP reservadas y condiciones).

Lectura pareada: `cbGetDhcpv4ServerPool` con `rtm_cfg_dhcpv4_svr_pool_0i={rtm_cfg_dhcpv4_svr_pool_id}`

Lista: instancia por `rtm_cfg_dhcpv4_svr_pool_id`; `del=DEL` borra

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `rtm_cfg_dhcpv4_svr_pool_id` | Pool DHCP | int |  |
| `reserved_ipv4_addr` | IP reservada (no se reparte) | ip (Ip4) |  |
| `reserved_ipv4_mask` | Máscara de la reservada | ip (Ip4) |  |
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `ip_intf_id` | Interfaz IP | int |  |
| `order_p` | Posición | enum | `1` Primera, `2` Según el orden, `3` Última |
| `order` | Orden | int |  |
| `lease_time` | Duración de la concesión (s) | int |  |
| `lease_time_u` | Concesión ilimitada | bool |  |
| `dhcpv4_opt60` | Condición por opción 60 (vendor ID) | bool |  |
| `dhcpv4_opt60_value` | Valor de la opción 60 | text |  |
| `dhcpv4_opt60_ex` | Opción 60: incluir/excluir | enum | `0` Incluir, `1` Excluir |
| `dhcpv4_opt60_matching_mode` | Opción 60: comparación | enum | `0` Exacta, `1` Prefijo, `2` Sufijo, `3` Subcadena |
| `dhcpv4_opt61` | Condición por opción 61 (client ID) | bool |  |
| `dhcpv4_opt61_value` | Valor de la opción 61 (hex) | text |  |
| `dhcpv4_opt61_ex` | Opción 61: incluir/excluir | enum | `0` Incluir, `1` Excluir |
| `dhcpv4_opt77` | Condición por opción 77 (user class) | bool |  |
| `dhcpv4_opt77_value` | Valor de la opción 77 (hex) | text |  |
| `dhcpv4_opt77_ex` | Opción 77: incluir/excluir | enum | `0` Incluir, `1` Excluir |
| `mac_addr` | Dirección MAC | bool |  |
| `mac_addr_ex` | MAC: incluir/excluir | enum | `0` Incluir, `1` Excluir |
| `ipv4_addr_max` | Última IP del rango | ip (Ip4) |  |
| `ipv4_addr_min` | Primera IP del rango | ip (Ip4) |  |
| `ipv4_dns_addr_0` | DNS primario | ip (Ip4) |  |
| `ipv4_dns_addr_1` | DNS secundario | ip (Ip4) |  |
| `ipv4_gw_addr` | Puerta de enlace | ip (Ip4) |  |
| `ipv4_mask` | Máscara | ip (Ip4) |  |
| `mac_addr_mask` | Máscara de MAC | mac (Mac6) |  |
| `mac_addr_value` | MAC de la condición | mac (Mac6) |  |

### `cbGetLanHostList`

*lectura* · UI avanzada · riesgo bajo · página `hoststats.asp`

Equipos conocidos en la LAN: IP, MAC, nombre, interfaz y si tienen reserva.

Elementos de la respuesta: `associated_type`, `dhcpv4_lease_remaining`, `dhcpv4_opt12`, `dhcpv4_opt125`, `dhcpv4_opt60`, `dhcpv4_opt61`, `dhcpv4_opt77`, `dhcpv4_svr_pool_clt_addr_id`, `dhcpv4_svr_pool_clt_id`, `dhcpv4_svr_pool_id`, `hostname`, `intf_id`, `intf_type`, `ip_intf_id`, `ipv4_addr`, `ipv6_addr`, `is_active`, `mac_addr`, `rtm_cfg_host`, `rtm_cfg_host_0i`

### `cbGetLanIPv4Addr`

*lectura* · UI avanzada · riesgo bajo · página `ipv4.asp`

Direcciones IPv4 de las interfaces LAN (la IP del router).

Lista: instancia por `ipv4_addr_id`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `ipv4_addr_id` | Dirección (id) | int |  |
| `ip_intf_id` | Interfaz IP | int |  |

Elementos de la respuesta: `adm_state`, `ip_intf_id`, `ipv4_addr`, `ipv4_addr_0i`, `ipv4_addr_id`, `ipv4_addr_type`, `ipv4_mask`, `opr_status`, `rtm_cfg_ipv4_addr`

### `cbSetLanIPv4Addr`

*escritura* · UI avanzada · riesgo alto · página `ipv4.asp`

Cambia, añade o borra la IP y máscara LAN del router. Cambia la dirección del propio panel: riesgo alto.

Lectura pareada: `cbGetLanIPv4Addr` con `ip_intf_id={ip_intf_id}`, `ipv4_addr_id={ipv4_addr_id}`

Lista: instancia por `ipv4_addr_id`; `del=DEL` borra

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `ip_intf_id` | Interfaz IP | int |  |
| `ipv4_addr_id` | Dirección (id) | int |  |
| `ipv4_addr` | Dirección IPv4 | ip (Ip4) |  |
| `ipv4_mask` | Máscara | ip (Ip4) |  |

### `cbGetLanIPv4Intf`

*lectura* · UI avanzada · riesgo bajo · página `ipv4.asp`

Interfaces LAN IPv4 (puente, enlace Ethernet, IGMP snooping).

Elementos de la respuesta: `adm_state`, `br_id`, `eth_link_id`, `igmp_snooping`, `ip_intf_id`, `ipv4_adm_state`, `ipv4_opr_status`, `lan_ipv4_interface`

### `cbSetLanIPv4Intf`

*escritura* · UI avanzada · riesgo alto · página `ipv4.asp`

Activa una interfaz LAN y su IGMP snooping. Puede dejar sin acceso al router.

Lectura pareada: `cbGetLanIPv4Intf`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `igmp_snooping` | IGMP snooping | enum | `1` Activado, `0` Desactivado |
| `ip_intf_id` | Interfaz IP | int |  |
| `eth_link_id` | Enlace Ethernet (id) | int |  |
| `br_id` | Puente (id) | int |  |

### `cbGetTR98L2Bridge`

*lectura* · UI avanzada · riesgo bajo · página `bridging.asp`

Puentes de nivel 2 (TR-098): nombre, VLAN por defecto e interfaces.

Lista: instancia por `mll_bridge_0i`; `all=ALL` lista todas

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `all` | Todas (siempre «ALL») | text |  |
| `mll_bridge_0i` | Puente (índice) | int |  |

Elementos de la respuesta: `adm_state`, `default_vid_value`, `mll_bridge_0i`, `mll_bridge_id`, `mll_tr098_dpm_cfg_br`, `name`, `opr_state`

### `cbSetTR98L2Bridge`

*escritura* · UI avanzada · riesgo alto · página `bridging.asp`

Crea, modifica o borra un puente de nivel 2. Puede aislar la LAN: riesgo alto.

Lectura pareada: `cbGetTR98L2Bridge` con `mll_bridge_0i={mll_bridge_id}`

Lista: instancia por `mll_bridge_id`; `del=DEL` borra

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `name` | Nombre | text |  |
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `default_vid_value` | VLAN por defecto | int |  |
| `mll_bridge_id` | Puente (id; -1 = nuevo) | int |  |

### `cbGetTR98L2Filtering`

*lectura* · UI avanzada · riesgo bajo · página `filtering.asp`

Filtros de puente: qué interfaz entra en qué puente y con qué condiciones (VLAN, EtherType, MAC).

Lista: instancia por `mll_br_filter_0i`; `all=ALL` lista todas

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `all` | Todas (siempre «ALL») | text |  |
| `mll_br_filter_0i` | Filtro (índice) | int |  |

Elementos de la respuesta: `adm_state`, `cls_admit_only_vlan_tagged`, `cls_dst_mac`, `cls_dst_mac_addr`, `cls_dst_mac_mask`, `cls_ethertype`, `cls_ethertype_value`, `cls_src_mac`, `cls_src_mac_addr`, `cls_src_mac_mask`, `cls_vid_value`, `cls_vid_value_active`, `filter_br_id`, `filter_intf`, `mll_br_filter_0i`, `mll_br_filter_id`, `mll_tr098_dpm_cfg_ingress_filter`, `mll_tr098_dpm_cfg_ingress_filter_totalCnt`, `opr_status`, `order`, `totalCnt`

### `cbSetTR98L2Filtering`

*escritura* · UI avanzada · riesgo alto · página `filtering.asp`

Crea, modifica o borra un filtro de puente. Puede aislar la LAN: riesgo alto.

Lectura pareada: `cbGetTR98L2Filtering` con `mll_br_filter_0i={mll_br_filter_id}`

Lista: instancia por `mll_br_filter_id`; `del=DEL` borra

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `order_p` | Posición | enum | `1` Primera, `2` Según el orden, `3` Última |
| `order` | Orden | int |  |
| `filter_intf` | Interfaz de entrada | text |  |
| `filter_br_id` | Puente asociado | int |  |
| `cls_admit_only_vlan_tagged` | Paquetes admitidos | enum | `0` Todos, `1` Solo con etiqueta |
| `cls_vid_value` | VLAN (1-4094) | int |  |
| `cls_ethertype` | Comprobar EtherType | bool |  |
| `cls_ethertype_value` | EtherType | text |  |
| `cls_src_mac` | Comprobar MAC de origen | bool |  |
| `cls_src_mac_addr` | MAC de origen | mac |  |
| `cls_src_mac_mask` | Máscara de MAC de origen | mac |  |
| `cls_dst_mac` | Comprobar MAC de destino | bool |  |
| `cls_dst_mac_addr` | MAC de destino | mac |  |
| `cls_dst_mac_mask` | Máscara de MAC de destino | mac |  |
| `mll_br_filter_id` | Filtro (id; -1 = nuevo) | int |  |

### `te_mapa_red_local.asp`

*lectura* · UI básica · riesgo bajo · página `te_mapa_red_local.asp`

Mapa de red de la UI básica; sus variables dan LAN, DHCP, modo WAN y credenciales PPPoE (enmascaradas).

### `te_mapa_red_local.cgi`

*escritura* · UI básica · riesgo alto · página `te_mapa_red_local.asp`

Igual que te_red_local.cgi, desde el mapa de red de la UI básica.

Lectura pareada: `te_mapa_red_local.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `ethIpAddress` | IP del router | ip |  |
| `ethSubnetMask` | Máscara de la LAN | ip |  |
| `enblDhcpSrv` | Servidor DHCP | bool |  |
| `dhcpEthStart` | Primera IP de DHCP | ip |  |
| `dhcpEthEnd` | Última IP de DHCP | ip |  |
| `lanHostDns` | DNS anunciado | text |  |
| `lanHostDhcp` | Reservas DHCP | text |  |
| `curWebPage` | Página de retorno | text |  |

### `te_mrl.cmd?action=setpc`

*escritura* · UI básica · riesgo bajo · página `te_mapa_red_local.asp`

Pone nombre y tipo a un equipo del mapa de red de la UI básica.

Lectura pareada: `te_mapa_red_local.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `hostName` | Nombre del equipo | text |  |
| `deviceType` | Tipo de equipo | text |  |
| `deviceIPAddress` | IP del equipo | ip |  |
| `curWebPage` | Página de retorno | text |  |
| `deviceMac` | MAC del equipo | mac |  |

### `te_red_local.asp`

*lectura* · UI básica · riesgo bajo · página `te_red_local.asp`

Página básica de la red local; sus variables dan IP y máscara del router, DHCP y rango.

### `te_red_local.cgi`

*escritura* · UI básica · riesgo alto · página `te_red_local.asp`

Cambia desde la UI básica la IP y máscara del router, el servidor DHCP y su rango. Cambia la dirección del propio panel.

Lectura pareada: `te_red_local.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `ethIpAddress` | IP del router | ip |  |
| `ethSubnetMask` | Máscara de la LAN | ip |  |
| `enblDhcpSrv` | Servidor DHCP | bool |  |
| `dhcpEthStart` | Primera IP de DHCP | ip |  |
| `dhcpEthEnd` | Última IP de DHCP | ip |  |
| `lanHostDns` | DNS anunciado | text |  |
| `lanHostDhcp` | Reservas DHCP | text |  |

## WiFi

### `cbGetWifi`

*lectura* · UI avanzada · riesgo bajo · página `wifi.asp`

Configuración de una radio (0 = 2,4 GHz, 1 = 5 GHz): estándar, canal, ancho, potencia, roaming.

Lista: instancia por `dpm_cfg_wifi_0i`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `dpm_cfg_wifi_0i` | Radio (0 = 2,4 GHz, 1 = 5 GHz) | int |  |

Elementos de la respuesta: `adm_bandwidth`, `adm_channel`, `adm_guard_interval`, `adm_tx_preamble`, `adm_tx_pwr_ratio`, `auto_channel`, `beacon_adv`, `beacon_interval`, `beamforming`, `chn_40mhz_ext_pos`, `country_code`, `dpm_cfg_wifi`, `dpm_cfg_wifi_0i`, `dpm_cfg_wifi_id`, `dtim_interval`, `frag_threshold`, `max_clt_count`, `mcs`, `opr_bandwidth`, `opr_channel`, `opr_guard_interval`, `opr_off_cac`, `opr_tx_preamble`, `opr_tx_pwr_ratio`, `qhop`, `radio_adm_state`, `radio_opr_status`, `roaming_adm_state`, `roaming_role`, `rts_threshold`, `scs`, `spatial_streams`, `standard`, `tef_opr_status`, `wifi_bg_rate`, `wifi_off_cac`

### `cbSetWifi`

*escritura* · UI avanzada · riesgo medio · página `wifi.asp`

Configura una radio WiFi: activación, país, estándar, canal, ancho, potencia y opciones avanzadas. Los valores válidos dependen de la banda.

Lectura pareada: `cbGetWifi` con `dpm_cfg_wifi_0i={dpm_cfg_wifi_id}`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `radio_adm_state` | Radio activada | enum | `1` Activado, `0` Desactivado |
| `country_code` | País | enum | `UN` Sin país (UN), `AR` Argentina, `BR` Brasil, `CL` Chile, `CO` Colombia, `ES` España, `PE` Perú |
| `standard` | Estándar (2.4: 2 b, 4 g, 8 n, 6 b/g, 12 g/n, 14 b/g/n; 5: 9 a/n, 25 a/n/ac) | int |  |
| `adm_bandwidth` | Ancho de canal (1 = 20, 2 = 20/40, 3 = 20/40/80 MHz) | int |  |
| `chn_40mhz_ext_pos` | Canal secundario (1 = superior, 2 = inferior) | int |  |
| `adm_channel` | Canal (0 = automático) | int |  |
| `wifi_bg_rate` | Velocidad b/g (0 = auto) | int |  |
| `mcs` | MCS (-1 = auto) | int |  |
| `spatial_streams` | Flujos espaciales | enum | `1` 1, `2` 2, `3` 3, `4` 4 |
| `frag_threshold` | Umbral de fragmentación | int |  |
| `rts_threshold` | Umbral RTS | int |  |
| `beacon_interval` | Intervalo de baliza (ms) | int |  |
| `dtim_interval` | Periodo DTIM | int |  |
| `adm_tx_pwr_ratio` | Potencia de emisión | enum | `255` Automático, `100` 100%, `90` 90%, `80` 80%, `70` 70%, `60` 60%, `50` 50%, `40` 40%, `30` 30%, `20` 20%, `10` 10%, `0` 0% |
| `adm_tx_preamble` | Preámbulo | enum | `1` Corto, `2` Largo |
| `adm_guard_interval` | Intervalo de guarda | enum | `0` Automático, `1` 400ns, `2` 800ns |
| `dpm_cfg_wifi_id` | Radio (0 = 2,4 GHz, 1 = 5 GHz) | int |  |
| `scs` | Selección inteligente de canal | enum | `1` Activado, `0` Desactivado |
| `wifi_off_cac` | Sin CAC (DFS) | enum | `1` Activado, `0` Desactivado |
| `qhop` | QHop | enum | `1` Activado, `0` Desactivado |
| `beamforming` | Beamforming | enum | `1` Activado, `0` Desactivado |
| `roaming_adm_state` | Roaming | enum | `1` Activado, `0` Desactivado |
| `roaming_role` | Papel en el roaming | enum | `1` Principal, `0` Secundario |

### `cbGetWifiAccPt`

*lectura* · UI avanzada · riesgo bajo · página `wifi.asp`

Configuración de un punto de acceso: SSID, seguridad, claves (enmascaradas), WMM, WPS y aislamiento.

Lista: instancia por `dpm_cfg_wifi_acc_pt_id`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `dpm_cfg_wifi_acc_pt_0i` | Punto de acceso (índice) | int |  |
| `dpm_cfg_wifi_acc_pt_id` | Punto de acceso (id) | int |  |

Elementos de la respuesta: `adm_state`, `authentication`, `bssid`, `dpm_cfg_wifi_acc_pt`, `dpm_cfg_wifi_acc_pt_0i`, `dpm_cfg_wifi_acc_pt_id`, `dpm_cfg_wifi_ssid_id`, `encryption`, `guest_networking`, `isolation`, `key_phrase` 🔒, `max_clt_count`, `opr_status`, `pre_shared_key` 🔒, `pri_svr_ip`, `pri_svr_port`, `pri_svr_secret` 🔒, `regen_key_interval`, `retry_limit`, `sec_svr_ip`, `sec_svr_port`, `sec_svr_secret` 🔒, `ssid`, `ssid_hide`, `wep_key1_data` 🔒, `wep_key1_type`, `wep_key2_data` 🔒, `wep_key2_type`, `wep_key3_data` 🔒, `wep_key3_type`, `wep_key4_data` 🔒, `wep_key4_type`, `wep_key_idx`, `wifi_pmf`, `wifi_ssid_id`, `wmm`, `wmm_no_ack`, `wmm_ps`, `wps_adm_state`, `wps_device_pin` 🔒, `wps_method`, `wps_opr_state`, `wps_proc_status`, `wps_setup_lock`, `wps_status_string`

### `cbListWifiAccPt`

*lectura* · UI avanzada · riesgo bajo · página `wifi.asp`

Puntos de acceso (SSID) de una radio.

Lista: instancia por `dpm_cfg_wifi_id`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `dpm_cfg_wifi_id` | Radio (0 = 2,4 GHz, 1 = 5 GHz) | int |  |

Elementos de la respuesta: `adm_state`, `authentication`, `bssid`, `dpm_cfg_wifi_acc_pt`, `dpm_cfg_wifi_acc_pt_0i`, `dpm_cfg_wifi_acc_pt_id`, `dpm_cfg_wifi_ssid_id`, `encryption`, `guest_networking`, `isolation`, `key_phrase` 🔒, `max_clt_count`, `opr_status`, `pre_shared_key` 🔒, `pri_svr_ip`, `pri_svr_port`, `pri_svr_secret` 🔒, `regen_key_interval`, `retry_limit`, `sec_svr_ip`, `sec_svr_port`, `sec_svr_secret` 🔒, `ssid`, `ssid_hide`, `wep_key1_data` 🔒, `wep_key1_type`, `wep_key2_data` 🔒, `wep_key2_type`, `wep_key3_data` 🔒, `wep_key3_type`, `wep_key4_data` 🔒, `wep_key4_type`, `wep_key_idx`, `wifi_pmf`, `wifi_ssid_id`, `wmm`, `wmm_no_ack`, `wmm_ps`, `wps_adm_state`, `wps_device_pin` 🔒, `wps_method`, `wps_opr_state`, `wps_proc_status`, `wps_setup_lock`, `wps_status_string`

### `cbSetWifiAccPt`

*escritura* · UI avanzada · riesgo medio · página `wifi.asp`

Configura un punto de acceso: SSID, ocultación, seguridad y clave, clientes máximos, WMM, aislamiento y WPS. Con `validation_only=1` solo valida.

Lectura pareada: `cbGetWifiAccPt` con `dpm_cfg_wifi_acc_pt_id={dpm_cfg_wifi_acc_pt_id}`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `ssid` | SSID | text |  |
| `ssid_hide` | Ocultar SSID | bool |  |
| `authentication` | Autenticación | enum | `0` Ninguna, `3` WPA2-PSK, `4` WPA/WPA2-PSK mixto |
| `encryption` | Cifrado | enum | `0` Ninguna, `1` WEP 64, `2` WEP 128, `3` TKIP, `4` AES, `5` TKIP-AES |
| `wep_key_idx` | Clave WEP activa | enum | `1` Clave 1, `2` Clave 2, `3` Clave 3, `4` Clave 4 |
| `wep_key1_data` | Clave WEP 1 | secret |  |
| `wep_key2_data` | Clave WEP 2 | secret |  |
| `wep_key3_data` | Clave WEP 3 | secret |  |
| `wep_key4_data` | Clave WEP 4 | secret |  |
| `key_phrase` | Clave WiFi (WPA) | secret |  |
| `regen_key_interval` | Renovación de clave de grupo (s) | int |  |
| `max_clt_count` | Máximo de clientes | int |  |
| `wmm` | WMM | enum | `1` Activado, `0` Desactivado |
| `wmm_ps` | Ahorro de energía WMM | enum | `1` Activado, `0` Desactivado |
| `isolation` | Aislamiento de clientes | enum | `1` Activado, `0` Desactivado |
| `wps_adm_state` | WPS activado | bool |  |
| `dpm_cfg_wifi_ssid_id` | SSID (id) | int |  |
| `dpm_cfg_wifi_acc_pt_id` | Punto de acceso (id) | int |  |
| `validation_only` | Solo validar (1) o aplicar (0) | bool |  |

### `cbListWifiAccPtAssociation`

*lectura* · UI avanzada · riesgo bajo · página `wifi.asp`

Estaciones asociadas a una radio (MAC, RSSI, velocidades).

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `dpm_cfg_wifi_id` | Radio (0 = 2,4 GHz, 1 = 5 GHz) | int |  |

Elementos de la respuesta: `dpm_cfg_wifi_acc_pt_0i`, `dpm_cfg_wifi_acc_pt_id`, `mac_addr`, `rssi`, `rx_rate`, `ssid`, `tx_rate`, `wifi_association`, `wifi_association_group`

### `cbGetWifiWds`

*lectura* · UI avanzada · riesgo bajo · página `wifi.asp`

Enlaces WDS (puente inalámbrico) configurados.

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `dpm_cfg_wifi_wds_0i` | Enlace WDS (índice) | int |  |

### `cbSetWifiWds`

*escritura* · UI avanzada · riesgo medio · página `wifi.asp`

Crea, modifica o borra un enlace WDS con otro punto de acceso.

Lectura pareada: `cbGetWifiWds` con `dpm_cfg_wifi_wds_0i={dpm_cfg_wifi_wds_id}`

Lista: instancia por `dpm_cfg_wifi_wds_id`; `del=DEL` borra

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `dpm_cfg_wifi_wds_id` | Enlace WDS (id; -1 = nuevo) | int |  |
| `wds_adm_state` | WDS activado | enum | `1` Activado, `0` Desactivado |
| `wifi_acc_pt_id` | Punto de acceso | int |  |
| `peer_bssid` | MAC del otro extremo | mac |  |

### `cbTriggerWifiAccPtWpsProc`

*acción* · UI avanzada · riesgo medio · página `wifi.asp`

Arranca un emparejamiento WPS (formulario WiFiWPSStartForm, que no aparece en las páginas capturadas: sus campos no se conocen).

### `te_wifi.asp`

*lectura* · UI básica · riesgo bajo · página `te_wifi.asp`

Página básica de la WiFi 2,4 GHz; sus variables dan SSID, seguridad, canal, filtro MAC y clave (enmascarada).

### `te_wifi.cmd?action=save`

*escritura* · UI básica · riesgo medio · página `te_wifi.asp`

Guarda la lista del filtro MAC de la WiFi 2,4 GHz (UI básica).

Lectura pareada: `te_wifi.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `wlMACaddr` | Lista de MAC (MAC1,-MAC2: «-» desactivada) | text |  |
| `unitNum` | Radio (unidad) | int |  |
| `curWebPage` | Página de retorno | text |  |

### `te_wifi.wl`

*escritura* · UI básica · riesgo medio · página `te_wifi.asp`

Configura la WiFi 2,4 GHz desde la UI básica: SSID, ocultación, seguridad, clave (XOR 0x1f), activación, canal y filtro MAC.

Lectura pareada: `te_wifi.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `wlSsidIdx` | SSID (índice) | int |  |
| `ssid_id` | SSID (id) | int |  |
| `acc_pt_id` | Punto de acceso (id) | int |  |
| `wlSsid` | Nombre de la red (SSID) | text |  |
| `wlHide` | Ocultar SSID | bool |  |
| `wlAuthMode` | Seguridad | enum | `00` Abierta, `02` WEP, `34` WPA2-PSK, `44` WPA/WPA2-PSK mixto |
| `wlKeyIndex` | Clave WEP activa | int |  |
| `wlKey1` | Clave WEP 1 (XOR 0x1f) | secret |  |
| `wlKey2` | Clave WEP 2 (XOR 0x1f) | secret |  |
| `wlKey3` | Clave WEP 3 (XOR 0x1f) | secret |  |
| `wlKey4` | Clave WEP 4 (XOR 0x1f) | secret |  |
| `wlWpaPsk` | Clave WiFi (XOR 0x1f) | secret |  |
| `wlEnbl` | WiFi activada | bool |  |
| `wlChannel` | Canal | int |  |
| `wlSyncNvram` | Guardar (siempre 1) | int |  |
| `wlFltMacMode` | Filtro MAC activado | bool |  |
| `wlFltMacList` | MAC permitidas (separadas por comas) | text |  |
| `unitNum` | Radio (unidad) | int |  |
| `curWebPage` | Página de retorno | text |  |

### `te_wifi_5ghz.asp`

*lectura* · UI básica · riesgo bajo · página `te_wifi_5ghz.asp`

Página básica de la WiFi 5 GHz; sus variables dan SSID, seguridad, canal, filtro MAC y clave (enmascarada).

### `te_wifi_5ghz.cmd?action=save`

*escritura* · UI básica · riesgo medio · página `te_wifi_5ghz.asp`

Guarda la lista del filtro MAC de la WiFi 5 GHz (UI básica).

Lectura pareada: `te_wifi_5ghz.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `wlMACaddr` | Lista de MAC (MAC1,-MAC2: «-» desactivada) | text |  |
| `unitNum` | Radio (unidad) | int |  |
| `curWebPage` | Página de retorno | text |  |

### `te_wifi_5ghz.wl`

*escritura* · UI básica · riesgo medio · página `te_wifi_5ghz.asp`

Configura la WiFi 5 GHz desde la UI básica (mismos campos que te_wifi.wl).

Lectura pareada: `te_wifi_5ghz.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `wlSsidIdx` | SSID (índice) | int |  |
| `ssid_id` | SSID (id) | int |  |
| `acc_pt_id` | Punto de acceso (id) | int |  |
| `wlSsid` | Nombre de la red (SSID) | text |  |
| `wlHide` | Ocultar SSID | bool |  |
| `wlAuthMode` | Seguridad | enum | `00` Abierta, `02` WEP, `34` WPA2-PSK, `44` WPA/WPA2-PSK mixto |
| `wlKeyIndex` | Clave WEP activa | int |  |
| `wlKey1` | Clave WEP 1 (XOR 0x1f) | secret |  |
| `wlKey2` | Clave WEP 2 (XOR 0x1f) | secret |  |
| `wlKey3` | Clave WEP 3 (XOR 0x1f) | secret |  |
| `wlKey4` | Clave WEP 4 (XOR 0x1f) | secret |  |
| `wlWpaPsk` | Clave WiFi (XOR 0x1f) | secret |  |
| `wlEnbl` | WiFi activada | bool |  |
| `wlChannel` | Canal | int |  |
| `wlSyncNvram` | Guardar (siempre 1) | int |  |
| `wlFltMacMode` | Filtro MAC activado | bool |  |
| `wlFltMacList` | MAC permitidas (separadas por comas) | text |  |
| `unitNum` | Radio (unidad) | int |  |
| `curWebPage` | Página de retorno | text |  |

### `te_wifi_guest.asp`

*lectura* · UI básica · riesgo bajo · página `te_wifi_guest.asp`

Página básica de la red de invitados; sus variables dan SSID, seguridad y clave (enmascarada).

### `te_wifi_guest.wl`

*escritura* · UI básica · riesgo medio · página `te_wifi_guest.asp`

Configura la red de invitados desde la UI básica: SSID, seguridad, clave (XOR 0x1f), activación y máximo de invitados.

Lectura pareada: `te_wifi_guest.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `wlSsidIdx` | SSID (índice) | int |  |
| `ssid_id` | SSID (id) | int |  |
| `acc_pt_id` | Punto de acceso (id) | int |  |
| `wlSsid` | Nombre de la red (SSID) | text |  |
| `wlAuthMode` | Seguridad | enum | `00` Abierta, `02` WEP, `34` WPA2-PSK, `44` WPA/WPA2-PSK mixto |
| `wlKeyIndex` | Clave WEP activa | int |  |
| `wlKey1` | Clave WEP 1 (XOR 0x1f) | secret |  |
| `wlKey2` | Clave WEP 2 (XOR 0x1f) | secret |  |
| `wlKey3` | Clave WEP 3 (XOR 0x1f) | secret |  |
| `wlKey4` | Clave WEP 4 (XOR 0x1f) | secret |  |
| `wlWpaPsk` | Clave WiFi (XOR 0x1f) | secret |  |
| `wlEnbl` | WiFi activada | bool |  |
| `wlMaxAssoc` | Máximo de invitados | int |  |
| `wlSyncNvram` | Guardar (siempre 1) | int |  |
| `unitNum` | Radio (unidad) | int |  |
| `curWebPage` | Página de retorno | text |  |

### `te_wifirefresh.wl`

*lectura* · UI básica · riesgo bajo · página `te_wifi.asp`

Refresca el estado de la WiFi 2,4 GHz (UI básica).

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `wlRefresh` | Refrescar (0) | int |  |

### `te_wifirefresh_5ghz.wl`

*lectura* · UI básica · riesgo bajo · página `te_wifi_5ghz.asp`

Refresca el estado de la WiFi 5 GHz (UI básica).

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `wlRefresh` | Refrescar (0) | int |  |

### `te_wifirefresh_guest.wl`

*lectura* · UI básica · riesgo bajo · página `te_wifi_guest.asp`

Refresca el estado de la red de invitados (UI básica).

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `wlRefresh` | Refrescar (0) | int |  |

## NAT

### `cbGetNatConfig`

*lectura* · UI avanzada · riesgo bajo · página `natalg.asp`

ALG (FTP, H.323, PPTP, SIP) y NAT loopback.

Elementos de la respuesta: `adm_state`, `ftp_enable`, `h323_enable`, `ip_intf_id`, `ipsec_enable`, `opr_status`, `pptp_enable`, `rtm_cfg_nat_lb_intf`, `rtm_cfg_nat_lb_intf_0i`, `rtm_cfg_nat_lb_intf_id`, `rtsp_enable`, `sip_enable`

### `cbSetNatConfig`

*escritura* · UI avanzada · riesgo bajo · página `natalg.asp`

Activa o desactiva los ALG o el NAT loopback (formularios NatAlgForm y NatLoopbackForm).

Lectura pareada: `cbGetNatConfig`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `ftp_enable` | ALG de FTP | bool |  |
| `h323_enable` | ALG de H.323 | bool |  |
| `pptp_enable` | ALG de PPTP | bool |  |
| `sip_enable` | ALG de SIP | bool |  |
| `set_nf_conntrack` | Aplicar ALG (siempre 1) | int |  |
| `adm_state` | Activado | bool |  |
| `rtm_cfg_nat_lb_intf_id` | Interfaz de loopback (id) | int |  |

### `cbGetNatIntf`

*lectura* · UI avanzada · riesgo bajo · página `natintf.asp`

Interfaces con NAT y su tipo.

Elementos de la respuesta: `adm_state`, `ip_intf_id`, `nat_intf_0i`, `nat_intf_id`, `rtm_cfg_nat_intf`, `type`

### `cbSetNatIntf`

*escritura* · UI avanzada · riesgo bajo · página `natintf.asp`

Activa o cambia el NAT de una interfaz.

Lectura pareada: `cbGetNatIntf`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `type` | Tipo | text |  |
| `nat_intf_id` | Interfaz NAT (id) | int |  |

### `cbGetNatPortMapping`

*lectura* · UI avanzada · riesgo bajo · página `natportmap.asp`

Redirecciones de puertos; con `dmz=1`, el host DMZ; con `nat_port_mapping_0i`, una redirección.

Lista: instancia por `nat_port_mapping_0i`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `dmz` | DMZ (1 = leer el host DMZ) | int |  |
| `nat_port_mapping_0i` | Redirección (índice) | int |  |

Elementos de la respuesta: `adm_state`, `desc`, `ext_l4_port_max`, `ext_l4_port_max_value`, `ext_l4_port_min`, `ext_l4_port_min_value`, `int_ip`, `int_ip_addr`, `int_l4_port_max`, `int_l4_port_max_value`, `int_l4_port_min`, `int_l4_port_min_value`, `ip_intf_all`, `ip_intf_id`, `l4_protocol`, `l4_protocol_value`, `lease_remaining`, `nat_port_mapping_0i`, `nat_port_mapping_id`, `opr_status`, `rmt_ip`, `rmt_ip_addr`, `rtm_cfg_nat_port_mapping`

### `cbSetNatPortMapping`

*escritura* · UI avanzada · riesgo bajo · página `natportmap.asp`

Crea, modifica o borra una redirección de puertos o el host DMZ.

Lectura pareada: `cbGetNatPortMapping` con `nat_port_mapping_0i={nat_port_mapping_id}`

Lista: instancia por `nat_port_mapping_id`; `del=DEL` borra

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `int_ip_addr` | IP interna | ip |  |
| `nat_port_mapping_id` | Redirección (id; -1 = nueva) | int |  |
| `ip_intf_id` | Interfaz IP | int |  |
| `desc` | Descripción | text |  |
| `l4_protocol_value` | Protocolo | enum | `6` TCP, `17` UDP |
| `ext_l4_port_min_value` | Puerto externo desde | int |  |
| `ext_l4_port_max_value` | Puerto externo hasta | int |  |
| `int_l4_port_min_value` | Puerto interno desde | int |  |
| `int_l4_port_max_value` | Puerto interno hasta | int |  |
| `rmt_ip_addr` | IP remota permitida | ip (Ip4) |  |

### `cbGetUpnpCfg`

*lectura* · UI avanzada · riesgo bajo · página `upnp.asp`

Estado de UPnP.

Elementos de la respuesta: `adm_state`, `upnp_cfg`

### `cbSetUpnpCfg`

*escritura* · UI avanzada · riesgo bajo · página `upnp.asp`

Activa o desactiva UPnP.

Lectura pareada: `cbGetUpnpCfg`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |

### `te_ppp_pm.cmd?action=retrieve`

*lectura* · UI básica · riesgo bajo · página `te_puertos.asp`

Lee las redirecciones de puertos de la UI básica (respuesta en texto).

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `type` | Tipo | text |  |
| `id` | Id | int |  |

### `te_ppp_pm.cmd?action=create`

*escritura* · UI básica · riesgo bajo · página `te_puertos.asp`

Crea una redirección de puertos desde la UI básica (campos de la regla separados por comas).

Lectura pareada: `te_ppp_pm.cmd?action=retrieve`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `inst` | Regla (campos separados por comas) | text |  |

### `te_ppp_pm.cmd?action=delete`

*escritura* · UI básica · riesgo bajo · página `te_puertos.asp`

Borra una redirección de puertos desde la UI básica.

Lectura pareada: `te_ppp_pm.cmd?action=retrieve`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `id` | Id | int |  |
| `intf` | Interfaz de entrada | text |  |

### `te_ppp_pm.cmd?action=update`

*escritura* · UI básica · riesgo bajo · página `te_puertos.asp`

Modifica una redirección de puertos desde la UI básica.

Lectura pareada: `te_ppp_pm.cmd?action=retrieve`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `inst` | Regla (campos separados por comas) | text |  |

### `te_puertos.asp`

*lectura* · UI básica · riesgo bajo · página `te_puertos.asp`

Página básica de redirección de puertos; sus variables dan la LAN y la interfaz PPP de las reglas.

### `te_puertos.cmd?action=active`

*escritura* · UI básica · riesgo bajo · página `te_puertos.asp`

Activa o desactiva una regla de puertos con el formato antiguo de la UI básica.

Lectura pareada: `te_puertos.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `portName` | Nombre de la regla | text |  |
| `portProtocol` | Protocolo | text |  |
| `wanPortRange` | Puertos externos | text |  |
| `lanPortRange` | Puertos internos | text |  |
| `portIPAddress` | IP interna | ip |  |
| `portActive` | Activa | bool |  |

### `te_puertos.cmd?action=delete`

*escritura* · UI básica · riesgo bajo · página `te_puertos.asp`

Borra una regla de puertos con el formato antiguo de la UI básica.

Lectura pareada: `te_puertos.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `portName` | Nombre de la regla | text |  |
| `portProtocol` | Protocolo | text |  |
| `wanPortRange` | Puertos externos | text |  |
| `lanPortRange` | Puertos internos | text |  |
| `portIPAddress` | IP interna | ip |  |

### `te_upnp.cmd`

*escritura* · UI básica · riesgo bajo · página `te_otras_funcionalidades.asp`

Activa o desactiva UPnP desde la UI básica.

Lectura pareada: `cbGetUpnpCfg`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `enblUpnp` | UPnP activado | bool |  |
| `curWebPage` | Página de retorno | text |  |

## Cortafuegos

### `scbidirectionalflt.cmd?action=view`

*lectura* · oculta · riesgo bajo · página `scbidirectionalflt_ls.asp`

Editor de cortafuegos que solo se enlaza desde un texto de te_otras_funcionalidades.asp. Lista las reglas IPv4/IPv6 (HTML).

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `escape` | Escapar (true) | text |  |

### `scbidirectionalflt.cmd?action=add`

*escritura* · oculta · riesgo alto · página `scbidirectionalflt_ed.asp`

Editor de cortafuegos que solo se enlaza desde un texto de te_otras_funcionalidades.asp. Añade una entrada (interfaz, versión, acción por defecto) o una regla (protocolo, origen, destino, puertos, acción).

Lectura pareada: `scbidirectionalflt.cmd?action=view`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `id` | Id | int |  |
| `escape` | Escapar (true) | text |  |
| `target` | Objetivo (entry/rule) | text |  |
| `belong` | Entrada a la que pertenece | int |  |
| `version` | Versión de IGMP | int |  |
| `enabled` | Activada | bool |  |
| `protocol` | Protocolo | text |  |
| `specificAction` | Acción | text |  |
| `rejectType` | Tipo de rechazo | text |  |
| `icmpType` | Tipo ICMP | text |  |
| `srcaddr` | IP de origen | text |  |
| `srcmask` | Máscara de origen | text |  |
| `srcportstart` | Puerto de origen desde | text |  |
| `srcportend` | Puerto de origen hasta | text |  |
| `dstaddr` | IP de destino | text |  |
| `dstmask` | Máscara de destino | text |  |
| `dstportstart` | Puerto de destino desde | text |  |
| `dstportend` | Puerto de destino hasta | text |  |
| `name` | Nombre | text |  |
| `interface` | Interfaz | text |  |
| `type` | Tipo | text |  |
| `default` | Acción por defecto | text |  |

### `scbidirectionalflt.cmd?action=modify`

*escritura* · oculta · riesgo alto · página `scbidirectionalflt_ed.asp`

Editor de cortafuegos que solo se enlaza desde un texto de te_otras_funcionalidades.asp. Modifica una entrada o regla.

Lectura pareada: `scbidirectionalflt.cmd?action=view`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `id` | Id | int |  |
| `escape` | Escapar (true) | text |  |
| `target` | Objetivo (entry/rule) | text |  |
| `belong` | Entrada a la que pertenece | int |  |
| `version` | Versión de IGMP | int |  |
| `enabled` | Activada | bool |  |
| `protocol` | Protocolo | text |  |
| `specificAction` | Acción | text |  |
| `rejectType` | Tipo de rechazo | text |  |
| `icmpType` | Tipo ICMP | text |  |
| `srcaddr` | IP de origen | text |  |
| `srcmask` | Máscara de origen | text |  |
| `srcportstart` | Puerto de origen desde | text |  |
| `srcportend` | Puerto de origen hasta | text |  |
| `dstaddr` | IP de destino | text |  |
| `dstmask` | Máscara de destino | text |  |
| `dstportstart` | Puerto de destino desde | text |  |
| `dstportend` | Puerto de destino hasta | text |  |
| `name` | Nombre | text |  |
| `interface` | Interfaz | text |  |
| `type` | Tipo | text |  |
| `default` | Acción por defecto | text |  |

### `scbidirectionalflt.cmd?action=remove`

*escritura* · oculta · riesgo alto · página `scbidirectionalflt_ls.asp`

Editor de cortafuegos que solo se enlaza desde un texto de te_otras_funcionalidades.asp. Borra una entrada o regla.

Lectura pareada: `scbidirectionalflt.cmd?action=view`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `item` | Elemento a borrar | text |  |
| `escape` | Escapar (true) | text |  |

## DNS

### `cbGetDdns`

*lectura* · UI avanzada · riesgo bajo · página `ddns.asp`

Entradas de DNS dinámico (proveedor, nombre, cuenta).

Lista: instancia por `rtm_ddns_0i`; `all=ALL` lista todas

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `all` | Todas (siempre «ALL») | text |  |
| `rtm_ddns_0i` | Entrada DDNS (índice) | int |  |

### `cbSetDdns`

*escritura* · UI avanzada · riesgo medio · página `ddns.asp`

Crea, modifica o borra una entrada de DNS dinámico.

Lectura pareada: `cbGetDdns` con `rtm_ddns_0i={rtm_ddns_id}`

Lista: instancia por `rtm_ddns_id`; `del=DEL` borra

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `provider` | Proveedor | enum | `0` No-IP, `1` Now-IP, `2` DynDNS, `3` Now-DNS |
| `hostname` | Nombre de host | text |  |
| `ip_intf_id` | Interfaz IP | int |  |
| `account` | Cuenta | secret |  |
| `password` | Contraseña | secret |  |
| `rtm_ddns_id` | Entrada DDNS (id; -1 = nueva) | int |  |

### `cbGetDnsRelay`

*lectura* · UI avanzada · riesgo bajo · página `dnsrelay.asp`

Estado del relé DNS.

Elementos de la respuesta: `dns_relay_adm_state`, `rtm_cfg_igd`

### `cbSetDnsRelay`

*escritura* · UI avanzada · riesgo medio · página `dnsrelay.asp`

Activa o desactiva el relé DNS.

Lectura pareada: `cbGetDnsRelay`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `dns_relay_adm_state` | Relé DNS activado | enum | `1` Activado, `0` Desactivado |

### `cbGetDnsRelayServer`

*lectura* · UI avanzada · riesgo bajo · página `dnsrelay.asp`

Servidores DNS del relé.

Lista: instancia por `rtm_cfg_dns_relay_id`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `rtm_cfg_dns_relay_id` | Servidor DNS (id; -1 = nuevo) | int |  |

Elementos de la respuesta: `adm_state`, `dns_addr`, `ip_addr_type`, `ip_intf_id`, `opr_status`, `rtm_cfg_dns_relay`, `rtm_cfg_dns_relay_0i`, `rtm_cfg_dns_relay_id`, `type`

### `cbSetDnsRelayServer`

*escritura* · UI avanzada · riesgo medio · página `dnsrelay.asp`

Añade, modifica o borra un servidor DNS del relé.

Lectura pareada: `cbGetDnsRelayServer` con `rtm_cfg_dns_relay_id={rtm_cfg_dns_relay_id}`

Lista: instancia por `rtm_cfg_dns_relay_id`; `del=DEL` borra

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `ip_intf_id` | Interfaz IP | int |  |
| `rtm_cfg_dns_relay_id` | Servidor DNS (id; -1 = nuevo) | int |  |
| `dns_addr` | Servidor DNS | ip (Ip4) |  |

## IPv6

### `cbGetLanIPv6Config`

*lectura* · UI avanzada · riesgo bajo · página `ipv6.asp`

IPv6 de la LAN: prefijo delegado, ULA, anuncios de router y DHCPv6.

Elementos de la respuesta: `adm_state`, `dhcpv6_mode`, `dhcpv6_svr_adm_state`, `dhcpv6_svr_opr_status`, `dhcpv6_svr_pool_id`, `eth_link_id`, `g_ipv6addr_all`, `iana_enable`, `iana_manual_addr_max`, `iana_manual_addr_min`, `ip_intf_id`, `ipv6_adm_state`, `ipv6_opr_status`, `l_ipv6addr_all`, `lan_ipv6_config`, `lease_time`, `opr_status`, `radv_adm_state`, `radv_intf_id`, `radv_m_flag`, `radv_o_flag`, `radv_opr_status`, `sc_ipv6_mask`, `sc_ipv6_parent_prefix_interface_id`, `sc_ipv6_prefix_id`, `ss_ipv6_prefix`, `ss_ipv6_prefix_adm_state`, `ss_ipv6_prefix_id`, `ss_preferred_lifetime`, `ss_valid_lifetime`

### `cbSetLanIPv6Config`

*escritura* · UI avanzada · riesgo bajo · página `ipv6.asp`

Configura IPv6 en la LAN: activación, prefijo delegado, ULA, RA y DHCPv6.

Lectura pareada: `cbGetLanIPv6Config`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `ipv6_adm_state` | IPv6 en la LAN | enum | `1` Activado, `0` Desactivado |
| `radv_adm_state` | Anuncios de router (RA) | enum | `1` Activado, `0` Desactivado |
| `sc_ipv6_parent_prefix_interface_id` | WAN que delega el prefijo | int |  |
| `ss_ipv6_prefix_adm_state` | Prefijo ULA | bool |  |
| `ss_ipv6_prefix` | Prefijo ULA | text |  |
| `ss_preferred_lifetime` | Vida preferida (s) | int |  |
| `ss_valid_lifetime` | Vida válida (s) | int |  |
| `dhcpv6_mode` | Servidor DHCPv6 | enum | `0` Desactivado, `1` Sin estado, `2` Con estado |
| `iana_manual_addr_min` | Primer id de interfaz | int |  |
| `iana_manual_addr_max` | Último id de interfaz | int |  |
| `lease_time` | Duración de la concesión (s) | int |  |
| `ip_intf_id` | Interfaz IP | int |  |
| `radv_intf_id` | Interfaz RA (id) | int |  |
| `l_ipv6_addr_id` | Dirección de enlace local (id) | int |  |
| `g_ipv6_addr_id` | Dirección global (id) | int |  |
| `sc_ipv6_prefix_id` | Prefijo delegado (id) | int |  |
| `ss_ipv6_prefix_id` | Prefijo ULA (id) | int |  |
| `dhcpv6_svr_pool_id` | Pool DHCPv6 (id) | int |  |

### `scbidirectionalfwl.cmd?action=create`

*escritura* · UI básica · riesgo alto · página `te_ipv6.asp`

Abre puertos IPv6 hacia un equipo de la LAN (UI básica).

Lectura pareada: `te_ipv6.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `newinst` | Regla nueva | text |  |
| `curWebPage` | Página de retorno | text |  |

### `scbidirectionalfwl.cmd?action=delete`

*escritura* · UI básica · riesgo alto · página `te_ipv6.asp`

Borra una apertura de puertos IPv6 (UI básica).

Lectura pareada: `te_ipv6.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `oldinst` | Regla a borrar | text |  |
| `curWebPage` | Página de retorno | text |  |

### `scbidirectionalfwl.cmd?action=save`

*escritura* · UI básica · riesgo alto · página `te_ipv6.asp`

Guarda la lista de aperturas de puertos IPv6 (UI básica).

Lectura pareada: `te_ipv6.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `inst` | Regla (campos separados por comas) | text |  |
| `curWebPage` | Página de retorno | text |  |

### `te_ipv6.asp`

*lectura* · UI básica · riesgo bajo · página `te_ipv6.asp`

Página básica de IPv6; sus variables dan prefijos, DHCPv6 y las aperturas de puertos IPv6.

### `te_ipv6.cmd?action=dhcpapply`

*escritura* · UI básica · riesgo bajo · página `te_ipv6.asp`

Configura DHCPv6 (modo y rango) desde la UI básica.

Lectura pareada: `te_ipv6.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `IPv6DHCPMode` | Modo DHCPv6 | text |  |
| `startIPv6Address` | Primera dirección IPv6 | text |  |
| `endIPv6Address` | Última dirección IPv6 | text |  |
| `curWebPage` | Página de retorno | text |  |

### `te_ipv6.cmd?action=lanapply`

*escritura* · UI básica · riesgo bajo · página `te_ipv6.asp`

Cambia el prefijo IPv6 de la LAN desde la UI básica.

Lectura pareada: `te_ipv6.asp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `globalIPv6Address` | Dirección IPv6 global | text |  |
| `globalIPv6Prefixlength` | Longitud del prefijo | int |  |
| `curWebPage` | Página de retorno | text |  |

## Multicast/IGMP

### `cbListMcastActiveGroup`

*lectura* · UI avanzada · riesgo bajo · página `igmp.asp`

Grupos multicast activos.

Elementos de la respuesta: `interface_list`, `rtm_ipv4_grp_addr`, `rtm_ipv4_grp_addr_0i`, `rtm_ipv4_grp_addr_list`

### `cbGetMcastIgmpSetting`

*lectura* · UI avanzada · riesgo bajo · página `igmp.asp`

Parámetros de IGMP (grupos máximos, intervalos de consulta).

Elementos de la respuesta: `br_id`, `eth_link_id`, `igmp_last_mbr_qry_count`, `igmp_last_mbr_qry_resp_time`, `igmp_max_group`, `igmp_qry_interval`, `igmp_qry_resp_time`, `igmp_robustness`, `ip_intf_id`, `mcast_igmp_setting`

### `cbSetMcastIgmpSetting`

*escritura* · UI avanzada · riesgo bajo · página `igmp.asp`

Configura los parámetros de IGMP.

Lectura pareada: `cbGetMcastIgmpSetting`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `igmp_max_group` | Máximo de grupos | int |  |
| `igmp_qry_interval` | Intervalo de consulta (s) | int |  |
| `ip_intf_id` | Interfaz IP | int |  |
| `eth_link_id` | Enlace Ethernet (id) | int |  |
| `br_id` | Puente (id) | int |  |

### `cbGetMcastIntf`

*lectura* · UI avanzada · riesgo bajo · página `multicastintf.asp`

Interfaces multicast y su versión de IGMP.

Elementos de la respuesta: `adm_state`, `ip_intf_id`, `rtm_cfg_ip_mcast_intf`, `rtm_ip_mcast_intf_0i`, `rtm_ip_mcast_intf_id`, `version`

### `cbSetMcastIntf`

*escritura* · UI avanzada · riesgo bajo · página `multicastintf.asp`

Activa multicast y la versión de IGMP en una interfaz.

Lectura pareada: `cbGetMcastIntf`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `version` | Versión de IGMP | int |  |
| `rtm_ip_mcast_intf_id` | Interfaz multicast (id) | int |  |

## QoS

### `cbGetAvailableIntf`

*lectura* · UI avanzada · riesgo bajo · página `classification.asp`

Interfaces disponibles para QoS y puentes (`all=ALL`).

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `all` | Todas (siempre «ALL») | text |  |

Elementos de la respuesta: `adm_state`, `mll_available_intf`, `mll_available_intf_0i`, `mll_available_intf_id`, `name`, `opr_status`

### `cbGetClassification`

*lectura* · UI avanzada · riesgo bajo · página `classification.asp`

Reglas de clasificación de QoS.

Lista: instancia por `rtm_cfg_qos_cls_0i`; `all=ALL` lista todas

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `all` | Todas (siempre «ALL») | text |  |
| `rtm_cfg_qos_cls_0i` | Regla (índice) | int |  |

Elementos de la respuesta: `adm_state`, `cls_dscp`, `cls_dscp_value`, `cls_dst_ipv4`, `cls_dst_ipv4_addr`, `cls_dst_ipv4_mask`, `cls_dst_l4_port`, `cls_dst_l4_port_value_max`, `cls_dst_l4_port_value_min`, `cls_dst_mac`, `cls_dst_mac_addr`, `cls_dst_mac_mask`, `cls_ethertype`, `cls_ethertype_value`, `cls_l4_protocol`, `cls_l4_protocol_value`, `cls_pbit`, `cls_pbit_value`, `cls_src_ipv4`, `cls_src_ipv4_addr`, `cls_src_ipv4_mask`, `cls_src_l4_port`, `cls_src_l4_port_value_max`, `cls_src_l4_port_value_min`, `cls_src_mac`, `cls_src_mac_addr`, `cls_src_mac_mask`, `cls_vid`, `cls_vid_value`, `fwd_policy`, `intf`, `mod_dscp`, `mod_dscp_by_pbit`, `mod_dscp_value`, `mod_pbit`, `mod_pbit_by_dscp`, `mod_pbit_value`, `opr_status`, `order`, `rtm_cfg_qos_cls`, `rtm_cfg_qos_cls_0i`, `rtm_cfg_qos_cls_id`, `traffic_class`

### `cbSetClassification`

*escritura* · UI avanzada · riesgo bajo · página `classification.asp`

Crea, modifica o borra una regla de clasificación de QoS (condiciones y remarcado DSCP/802.1p).

Lectura pareada: `cbGetClassification` con `rtm_cfg_qos_cls_0i={rtm_cfg_qos_cls_id}`

Lista: instancia por `rtm_cfg_qos_cls_id`; `del=DEL` borra

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `order_p` | Posición | enum | `1` Primera, `2` Según el orden, `3` Última |
| `order` | Orden | int |  |
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `traffic_class` | Cola de destino | int |  |
| `mod_dscp` | Remarcar DSCP | bool |  |
| `mod_dscp_value` | DSCP nuevo (0-63) | int |  |
| `mod_pbit` | Remarcar prioridad 802.1p | bool |  |
| `mod_pbit_value` | Prioridad nueva (0-7) | int |  |
| `intf` | Interfaz de entrada | text |  |
| `cls_ethertype` | Comprobar EtherType | bool |  |
| `cls_ethertype_value` | EtherType | text |  |
| `cls_pbit` | Comprobar prioridad 802.1p | bool |  |
| `cls_pbit_value` | Prioridad (0-7) | int |  |
| `cls_vid` | Comprobar VLAN | bool |  |
| `cls_vid_value` | VLAN (1-4094) | int |  |
| `cls_src_mac` | Comprobar MAC de origen | bool |  |
| `cls_src_mac_addr` | MAC de origen | mac |  |
| `cls_src_mac_mask` | Máscara de MAC de origen | mac |  |
| `cls_dst_mac` | Comprobar MAC de destino | bool |  |
| `cls_dst_mac_addr` | MAC de destino | mac |  |
| `cls_dst_mac_mask` | Máscara de MAC de destino | mac |  |
| `cls_dscp` | Comprobar DSCP | bool |  |
| `cls_dscp_value` | DSCP (0-63) | int |  |
| `cls_l4_protocol` | Comprobar protocolo | bool |  |
| `cls_l4_protocol_value` | Protocolo (1-255) | int |  |
| `cls_src_l4_port` | Comprobar puerto de origen | bool |  |
| `cls_src_l4_port_value_min` | Puerto de origen desde | int |  |
| `cls_src_l4_port_value_max` | Puerto de origen hasta | int |  |
| `cls_dst_l4_port` | Comprobar puerto de destino | bool |  |
| `cls_dst_l4_port_value_min` | Puerto de destino desde | int |  |
| `cls_dst_l4_port_value_max` | Puerto de destino hasta | int |  |
| `cls_src_ipv4` | Comprobar IP de origen | bool |  |
| `cls_src_ipv4_addr` | IP de origen | ip |  |
| `cls_src_ipv4_mask` | Máscara de IP de origen | ip |  |
| `cls_dst_ipv4` | Comprobar IP de destino | bool |  |
| `cls_dst_ipv4_addr` | IP de destino | ip |  |
| `cls_dst_ipv4_mask` | Máscara de IP de destino | ip |  |
| `rtm_cfg_qos_cls_id` | Regla (id; -1 = nueva) | int |  |

### `cbGetQueue`

*lectura* · UI avanzada · riesgo bajo · página `queue.asp`

Colas de QoS y cola por defecto.

Elementos de la respuesta: `adm_state`, `default_traffic_class`, `dpm_cfg_queue`, `dpm_cfg_queue_0i`, `dpm_cfg_queue_id`, `intf_name`, `opr_status`, `policy`, `priority`, `traffic_class`, `weight`

### `cbSetQueue`

*escritura* · UI avanzada · riesgo bajo · página `queue.asp`

Configura una cola de QoS.

Lectura pareada: `cbGetQueue`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `dpm_cfg_queue_id` | Cola (id) | int |  |
| `default_traffic_class` | Cola por defecto | int |  |
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |

### `cbGetTR98L2Marking`

*lectura* · UI avanzada · riesgo bajo · página `marking.asp`

Marcado de nivel 2 (VLAN y prioridad 802.1p) por interfaz.

Lista: instancia por `mll_br_marking_0i`; `all=ALL` lista todas

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `all` | Todas (siempre «ALL») | text |  |
| `mll_br_marking_0i` | Marcado (índice) | int |  |

Elementos de la respuesta: `adm_state`, `marking_br_id`, `marking_intf`, `mll_br_marking_0i`, `mll_br_marking_id`, `mll_tr098_dpm_cfg_egress_marking`, `opr_status`, `pbit_override`, `pbit_value`, `untagged`, `vid_override`, `vid_value`

### `cbSetTR98L2Marking`

*escritura* · UI avanzada · riesgo bajo · página `marking.asp`

Crea, modifica o borra un marcado de nivel 2.

Lectura pareada: `cbGetTR98L2Marking` con `mll_br_marking_0i={mll_br_marking_id}`

Lista: instancia por `mll_br_marking_id`; `del=DEL` borra

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `marking_intf` | Interfaz | text |  |
| `marking_br_id` | Puente asociado | int |  |
| `untagged` | Sin etiqueta VLAN | enum | `1` Sin etiqueta, `0` Con etiqueta |
| `vid_override` | Reescribir VLAN | bool |  |
| `vid_value` | VLAN | int |  |
| `pbit_override` | Reescribir prioridad 802.1p | bool |  |
| `pbit_value` | Prioridad 802.1p | enum | `0` 0, `1` 1, `2` 2, `3` 3, `4` 4, `5` 5, `6` 6, `7` 7 |
| `mll_br_marking_id` | Marcado (id; -1 = nuevo) | int |  |

## VoIP

### `cbGetPbxExtension`

*lectura* · UI avanzada · riesgo bajo · página `pbxstats.asp`

Extensiones SIP de la centralita interna.

### `cbGetVoiceLine`

*lectura* · UI avanzada · riesgo bajo · página `voiceadvance.asp`

Líneas de voz: número, usuario y contraseña SIP (enmascarados), servicios.

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `interwork_id` | Línea (id) | int |  |

Elementos de la respuesta: `adm_state`, `auth_password` 🔒, `auth_username` 🔒, `call_forward_on_busy`, `call_forward_on_busy_number` 🔒, `call_forward_on_no_answer`, `call_forward_on_no_answer_number` 🔒, `call_forward_unconditional`, `call_forward_unconditional_number` 🔒, `call_status`, `call_transfer`, `call_waiting`, `clt_status`, `codec_0_adm_state`, `codec_0_id`, `codec_0_order`, `codec_0_ptime`, `codec_0_type`, `codec_1_adm_state`, `codec_1_id`, `codec_1_order`, `codec_1_ptime`, `codec_1_type`, `codec_2_adm_state`, `codec_2_id`, `codec_2_order`, `codec_2_ptime`, `codec_2_type`, `codec_3_adm_state`, `codec_3_id`, `codec_3_order`, `codec_3_ptime`, `codec_3_type`, `directory_number` 🔒, `do_not_disturb`, `fxs_intf`, `interwork_id`, `mwi`, `polarity_reverse`, `register_uri` 🔒, `rx_anonymous_call_rejection`, `tr104_issue1_vp_line`, `tx_not_adm_state`, `vad_state`, `warmline_hotline`, `warmline_hotline_number` 🔒, `warmline_hotline_timeout`

### `cbSetVoiceLine`

*escritura* · UI avanzada · riesgo bajo · página `voiceadvance.asp`

Configura una línea de voz: credenciales SIP, línea directa y VAD.

Lectura pareada: `cbGetVoiceLine` con `interwork_id={interwork_id}`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `warmline_hotline_flag` | Modo línea directa | int |  |
| `warmline_hotline` | Línea directa (hotline/warmline) | bool |  |
| `warmline_hotline_number` | Número de la línea directa | secret |  |
| `warmline_hotline_timeout` | Espera de la warmline (s) | int |  |
| `interwork_id` | Línea (id) | int |  |
| `web` | Origen del formulario (BASIC/ADVANCED) | text |  |
| `auth_username` | Usuario SIP | secret |  |
| `auth_password` | Contraseña SIP | secret |  |
| `vad_state` | Detección de silencio (VAD) | enum | `1` Activado, `0` Desactivado |

### `cbGetVoicePhysical`

*lectura* · UI avanzada · riesgo bajo · página `voiceadvance.asp`

Puertos de teléfono (FXS): ganancias y paso directo de fax/módem.

Elementos de la respuesta: `adm_state`, `fax_passthrough_adm_state`, `fxs_0i`, `fxs_id`, `hook_flash_max_time`, `hook_flash_min_time`, `modem_passthrough_adm_state`, `opr_status`, `rx_gain`, `tr104_issue1_vp_fxs`, `tx_gain`

### `cbSetVoicePhysical`

*escritura* · UI avanzada · riesgo bajo · página `voiceadvance.asp`

Configura un puerto de teléfono (ganancias, fax y módem).

Lectura pareada: `cbGetVoicePhysical`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `fax_passthrough_adm_state` | Fax en paso directo | bool |  |
| `modem_passthrough_adm_state` | Módem en paso directo | bool |  |
| `tx_gain` | Ganancia de emisión (dB) | int |  |
| `rx_gain` | Ganancia de recepción (dB) | int |  |
| `fxs_id` | Puerto de teléfono (id) | int |  |

### `cbGetVoiceSip`

*lectura* · UI avanzada · riesgo bajo · página `voiceadvance.asp`

Configuración SIP: servidores proxy y de registro, dominio, tiempos y DTMF.

Elementos de la respuesta: `adm_state`, `alt_outbound_proxy_cfg` 🔒, `alt_outbound_proxy_port`, `alt_proxy_server_cfg` 🔒, `alt_proxy_server_port`, `alt_proxy_server_transport`, `alt_registrar_server_cfg` 🔒, `alt_registrar_server_port`, `alt_registrar_server_transport`, `conference_call_domain_uri` 🔒, `conference_call_remote_mixer_enable`, `digit_map_str`, `dtmf_method`, `hook_flash_method`, `invite_expires`, `ip_intf_id`, `map_id`, `min_session_expires`, `net_status`, `opr_outbound_proxy`, `outbound_proxy_cfg` 🔒, `outbound_proxy_port`, `proxy_server_cfg` 🔒, `proxy_server_port`, `proxy_server_transport`, `region`, `register_expires`, `register_min_expires`, `register_retry_interval`, `registrar_server_cfg` 🔒, `registrar_server_port`, `registrar_server_transport`, `registration_period`, `reinvite_expires`, `rtp_dscp_mark`, `session_expires`, `sip_clt_id`, `sip_dscp_mark`, `t38_enable`, `tr104_issue1_vp_sip`, `user_agent_domain` 🔒, `user_agent_port`, `user_agent_transport`

### `cbSetVoiceSip`

*escritura* · UI avanzada · riesgo bajo · página `voiceadvance.asp`

Configura SIP (servidores, puertos, tiempos, DTMF, T.38). Puede dejar sin teléfono.

Lectura pareada: `cbGetVoiceSip`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `region` | Región | text |  |
| `registration_period` | Periodo de registro (s) | int |  |
| `register_retry_interval` | Reintento de registro (s) | int |  |
| `register_expires` | Caducidad del registro (s) | int |  |
| `session_expires` | Caducidad de sesión (s) | int |  |
| `min_session_expires` | Caducidad mínima de sesión (s) | int |  |
| `sip_dscp_mark` | DSCP de SIP | int |  |
| `rtp_dscp_mark` | DSCP de RTP | int |  |
| `web` | Origen del formulario (BASIC/ADVANCED) | text |  |
| `ip_intf_id` | Interfaz IP | int |  |
| `user_agent_domain` | Dominio SIP | secret |  |
| `user_agent_port` | Puerto local SIP | int |  |
| `user_agent_transport` | Transporte local | enum | `1` UDP |
| `outbound_proxy_cfg` | Proxy de salida | secret |  |
| `outbound_proxy_port` | Puerto del proxy de salida | int |  |
| `proxy_server_cfg` | Servidor proxy | secret |  |
| `proxy_server_port` | Puerto del proxy | int |  |
| `proxy_server_transport` | Transporte del proxy | enum | `1` UDP |
| `registrar_server_cfg` | Servidor de registro | secret |  |
| `registrar_server_port` | Puerto de registro | int |  |
| `registrar_server_transport` | Transporte de registro | enum | `1` UDP |
| `digit_map_str` | Plan de marcación | text |  |
| `dtmf_method` | Método DTMF | enum | `1` InBand, `2` RFC2833, `3` SIPInfo |
| `hook_flash_method` | Método de flash | enum | `0` Ninguna, `1` SIPInfo |
| `t38_enable` | Fax T.38 | enum | `1` Activado, `0` Desactivado |

## TR-069

### `cbGetTr69Config`

*lectura* · oculta · riesgo bajo · página `cwmp.asp`

TR-069: activación, URL y credenciales del ACS, Inform periódico y credenciales de connection request (enmascaradas). Solo la usa cwmp.asp, que ningún menú enlaza.

Elementos de la respuesta: `acs_URL` 🔒, `acs_password` 🔒, `acs_username` 🔒, `connectionRequestPassword` 🔒, `connectionRequestURL` 🔒, `connectionRequestUsername` 🔒, `cwmp_data`, `enableCWMP`, `periodicInformEnable`, `periodicInformInterval`

### `cbSetTr69Config`

*escritura* · oculta · riesgo alto · página `cwmp.asp`

Configura TR-069 (ACS, credenciales, Inform). Desde cwmp.asp (oculta). Desactivarlo corta la gestión remota del operador.

Lectura pareada: `cbGetTr69Config`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `enableCWMP` | TR-069 activado | enum | `1` Activado, `0` Desactivado |
| `acs_URL` | URL del ACS | secret |  |
| `acs_username` | Usuario del ACS | secret |  |
| `acs_password` | Contraseña del ACS | secret |  |
| `periodicInformEnable` | Inform periódico | enum | `1` Activado, `0` Desactivado |
| `periodicInformInterval` | Intervalo de Inform (s) | int |  |
| `connectionRequestUsername` | Usuario de connection request | secret |  |
| `connectionRequestPassword` | Contraseña de connection request | secret |  |

## Usuarios

### `cbAddUser`

*escritura* · UI avanzada · riesgo bajo · página `user_profile.asp`

Crea un usuario de la interfaz web con su grupo (user, admin; system solo si la sesión es system).

Lectura pareada: `cbGetUserList`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `rqUser` | Usuario | secret |  |
| `rqPwd` | Contraseña | secret |  |
| `rqGroup` | Grupo (user/admin/system) | text |  |

### `cbCheckLogin`

*acción* · UI avanzada · riesgo bajo · página `login.asp`

Inicio de sesión en la UI avanzada: devuelve SessionID. Usuario y contraseña van ofuscados con XOR 0x1f.

**Bloqueada:** la usa hgu::Client para su propia sesión; invocarla desde el explorador rompería la sesión compartida del panel

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `rqUsername` | Usuario (XOR 0x1f) | secret |  |
| `rqPasswd` | Contraseña (XOR 0x1f) | secret |  |
| `rqTimeout` | Caducidad de la sesión (s) | int |  |

### `cbCheckLogout`

*acción* · UI avanzada · riesgo bajo · página `logout.asp`

Cierra la sesión de la UI avanzada.

**Bloqueada:** la usa hgu::Client para su propia sesión; invocarla desde el explorador rompería la sesión compartida del panel

### `cbCheckSession`

*lectura* · UI avanzada · riesgo bajo · página `summary.asp`

Comprueba la sesión: devuelve LoginRole y AccessMask. Todas las páginas avanzadas la llaman al cargar.

### `cbDeleteUser`

*escritura* · UI avanzada · riesgo bajo · página `user_profile.asp`

Borra usuarios de la interfaz web (lista de nombres).

Lectura pareada: `cbGetUserList`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `rqDelList` | Usuarios a borrar | secret |  |

### `cbGetUserList`

*lectura* · UI avanzada · riesgo bajo · página `user_profile.asp`

Lista los usuarios de la interfaz web con su rol. Incluye un usuario oculto del operador, con rol system, que la interfaz no deja tocar (su nombre se enmascara).

Elementos de la respuesta: `UserName` 🔒, `UserRole`

### `cbModifyPwd`

*acción* · UI avanzada · riesgo medio · página `user_profile.asp`

Cambia la contraseña del usuario con sesión iniciada (contraseña actual y nueva).

**Bloqueada:** cambia la contraseña con la que el propio panel inicia sesión: la vigilancia no podría volver a entrar ni revertir; se cambia desde el router

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `rqUser` | Usuario | secret |  |
| `rqOPwd` | Contraseña actual | secret |  |
| `rqNPwd` | Contraseña nueva | secret |  |

### `cbModifyPwdOther`

*acción* · UI avanzada · riesgo medio · página `user_profile.asp`

Cambia la contraseña y el grupo de otro usuario (ventana user_profile_modifyOthers.asp). No se puede comprobar después: acción.

**Bloqueada:** puede cambiar la contraseña del usuario con el que el propio panel inicia sesión (RP_ROUTER_USER): la vigilancia no podría volver a entrar ni revertir; se cambia desde el router

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `rqUser` | Usuario | secret |  |
| `rqNPwd` | Contraseña nueva | secret |  |
| `rqGroup` | Grupo (user/admin/system) | text |  |

### `te_acceso_router.cgi`

*acción* · UI básica · riesgo bajo · página `te_acceso_router.asp`

Inicio de sesión en la UI básica (formulario de te_acceso_router.asp): fija la cookie _httpdSessionId_.

**Bloqueada:** la usa hgu::Client para su propia sesión; invocarla desde el explorador rompería la sesión compartida del panel

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `curWebPage` | Página de retorno | text |  |
| `loginUsername` | Usuario (XOR 0x1f) | secret |  |
| `loginPassword` | Contraseña (XOR 0x1f) | secret |  |

### `te_cambio_contrasena.cgi`

*acción* · UI básica · riesgo bajo · página `te_cambio_contrasena.asp`

Cambia la contraseña de acceso desde la UI básica (actual y nueva, XOR 0x1f).

**Bloqueada:** cambia la contraseña con la que el propio panel inicia sesión: la vigilancia no podría volver a entrar ni revertir; se cambia desde el router

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `inOrgPassword` | Contraseña actual (XOR 0x1f) | secret |  |
| `changePassword` | Contraseña nueva (XOR 0x1f) | secret |  |

### `te_logout.cgi`

*acción* · UI básica · riesgo bajo · página `te_header.asp`

Cierra la sesión de la UI básica.

**Bloqueada:** la usa hgu::Client para su propia sesión; invocarla desde el explorador rompería la sesión compartida del panel

## Sistema

### `backup_restore.asp`

*acción* · UI avanzada · riesgo bajo · página `backup_restore.asp`

Página de copia y restauración de la configuración.

**Bloqueada:** al cargarse llama a cbSaveConfig con backup=1 (genera la copia con secretos)

### `cbDownloadConfig`

*acción* · UI avanzada · riesgo bajo · página `backup_restore.asp`

Descarga el fichero de copia generado por cbSaveConfig (backup=1).

**Bloqueada:** descarga la configuración completa con todas las contraseñas y claves en claro

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `fileName` | Fichero | text |  |

### `cbFWUpgradeByFTP`

*acción* · UI avanzada · riesgo alto · página `fw_upgrade.asp`

Descarga un firmware de un servidor FTP/TFTP y lo graba.

**Bloqueada:** graba un firmware: un fallo deja el router inservible

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `rqProtocol` | Protocolo | text |  |
| `rqServerIP` | Servidor | ip |  |
| `rqPort` | Puerto | int |  |
| `rqFileName` | Fichero | text |  |

### `cbGetDiagConfig`

*lectura* · UI avanzada · riesgo bajo · página `diagsw.asp`

Configuración del espejo de la WAN (copia el tráfico WAN a un puerto LAN para diagnóstico).

Elementos de la respuesta: `diagnosis_config`, `dst_lan_port`, `wan_mirror`

### `cbSetDiagConfig`

*escritura* · UI avanzada · riesgo bajo · página `diagsw.asp`

Activa o desactiva el espejo de la WAN hacia un puerto LAN.

Lectura pareada: `cbGetDiagConfig`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `wan_mirror` | Espejo de la WAN | enum | `1` Activado, `0` Desactivado |
| `dst_lan_port` | Puerto LAN de destino | enum | `1` 1, `2` 2, `4` 3, `8` 4 |

### `cbGetFWUpgradeInfo`

*lectura* · UI avanzada · riesgo bajo · página `fw_upgrade.asp`

Estado de la última actualización de firmware.

Elementos de la respuesta: `Partition`, `SWVersion`

### `cbGetSummary`

*lectura* · UI avanzada · riesgo bajo · página `summary.asp`

Resumen del equipo: modelo, versiones de firmware y hardware, tiempo encendido, carga media 1/5/15 y memoria.

Elementos de la respuesta: `AvgLoad1`, `AvgLoad15`, `AvgLoad5`, `AvgLoads`, `BootloaderVersion`, `BufferRam`, `ConfigInfo`, `Customization`, `DeviceManufacturer`, `DeviceModel`, `FreeRam`, `HWVersion`, `KernelBuild`, `ModelName`, `PrivateVersion`, `ProfileName`, `RevisionNumber`, `SWVersion`, `ShareRam`, `SysTime`, `SysTzOffset`, `SysUptime`, `TotalRam`

### `cbGetSyslog`

*lectura* · UI avanzada · riesgo bajo · página `syslog.asp`

Configuración del registro del sistema (local/remoto, severidad, servidor).

Elementos de la respuesta: `bind_interface`, `remote_ip`, `remote_port`, `remote_severity`, `syslog_enable`, `syslog_mode`, `syslog_module_info`

### `cbSetSyslog`

*acción* · UI avanzada · riesgo bajo · página `syslog.asp`

Configura el registro del sistema; con `export=EXPORT` exporta el registro (lo que syslog.asp hace al cargarse). Acción explícita.

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `export` | Exportar (siempre «EXPORT») | text |  |
| `syslog_enable` | Registro activado | enum | `1` Activado, `0` Desactivado |
| `syslog_mode` | Destino del registro | enum | `2` Local y remoto, `1` Remoto, `0` Local |
| `remote_severity` | Severidad mínima remota | enum | `0` Emergencia, `1` Alerta, `2` Crítico, `3` Error, `4` Aviso, `5` Nota, `6` Información, `7` Depuración |
| `bind_interface` | Interfaz de salida | text |  |
| `remote_ip` | Servidor syslog | ip |  |
| `remote_port` | Puerto syslog | int |  |

### `cbReboot`

*acción* · UI avanzada · riesgo alto · página `reboot.asp`

Reinicia el router. El panel vigila su vuelta hasta 5 minutos.

### `cbRestoreConfig`

*acción* · UI avanzada · riesgo alto · página `restore_default.asp`

Restaura un perfil de configuración guardado y reinicia el router.

**Bloqueada:** sustituye toda la configuración y reinicia; no hay forma de comprobarla ni de deshacerla desde el panel

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `file` | Perfil guardado | text |  |

### `cbSaveConfig`

*acción* · UI avanzada · riesgo bajo · página `save.asp`

Con `save=1` graba la configuración en flash; con `backup=1` genera el fichero de copia (lo que hace backup_restore.asp al cargarse).

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `backup` | Generar copia (1) | int |  |
| `save` | Guardar en flash (1) | int |  |

### `syslog.asp`

*acción* · UI avanzada · riesgo bajo · página `syslog.asp`

Página del registro del sistema.

**Bloqueada:** al cargarse llama a cbSetSyslog con export=EXPORT (exportación del registro)

### `upload/cfgupload.asp`

*acción* · UI avanzada · riesgo alto · página `upload/cfgupload.asp`

Recibe un fichero de configuración y lo aplica.

**Bloqueada:** sube y aplica una configuración completa; no se puede comprobar ni deshacer

### `upload/imgupgrade.asp`

*acción* · UI avanzada · riesgo alto · página `upload/imgupgrade.asp`

Recibe un firmware y lo graba.

**Bloqueada:** graba un firmware: un fallo deja el router inservible

### `restoreinfo.cgi`

*acción* · UI básica · riesgo alto · página `te_otras_funcionalidades.asp`

Restablece los valores de fábrica (enlace de te_otras_funcionalidades.asp).

**Bloqueada:** reset de fábrica: borra la configuración, incluidas las credenciales PPP y WiFi

### `te_acerca_de___.asp`

*lectura* · UI básica · riesgo bajo · página `te_acerca_de___.asp`

Página básica «Acerca de»: datos del equipo.

### `te_otras_funcionalidades.asp`

*lectura* · UI básica · riesgo bajo · página `te_otras_funcionalidades.asp`

Página básica de otras funciones (UPnP, DNS, copia y restauración, reset de fábrica, enlace al cortafuegos).

### `te_uploadinfo.asp`

*acción* · UI básica · riesgo alto · página `te_uploadinfo.asp`

Recibe un firmware desde la UI básica y lo graba.

**Bloqueada:** graba un firmware: un fallo deja el router inservible

### `te_uploadsettings.asp`

*acción* · UI básica · riesgo alto · página `te_uploadsettings.asp`

Recibe un fichero de configuración desde la UI básica y lo aplica.

**Bloqueada:** sube y aplica una configuración completa; no se puede comprobar ni deshacer

### `cbSetLocalTime`

*acción* · oculta · riesgo bajo · página `ntp.asp`

Fija la hora local del router a mano (ntp.asp, oculta). Sin lectura que la compruebe: acción.

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `SysTime` | Hora local | text |  |

### `cbGetTimeNtp`

*lectura* · oculta · riesgo bajo · página `ntp.asp`

Configuración NTP (servidores 1-5, interfaz, estado de sincronización). Solo la usa ntp.asp, que ningún menú enlaza.

Elementos de la respuesta: `adm_state`, `ever_synced`, `ip_intf_id`, `ntp_state`, `ntp_svr_1`, `ntp_svr_2`, `ntp_svr_3`, `ntp_svr_4`, `ntp_svr_5`, `rtm_time_ntp`, `timezone`

### `cbSetTimeNtp`

*escritura* · oculta · riesgo bajo · página `ntp.asp`

Configura NTP: activación, interfaz y servidores. Desde ntp.asp (oculta).

Lectura pareada: `cbGetTimeNtp`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `adm_state` | Activado | enum | `1` Activado, `0` Desactivado |
| `ip_intf_id` | Interfaz IP | int |  |
| `ntp_svr_1` | Servidor NTP 1 | text |  |
| `ntp_svr_2` | Servidor NTP 2 | text |  |
| `ntp_svr_3` | Servidor NTP 3 | text |  |
| `ntp_svr_4` | Servidor NTP 4 | text |  |
| `ntp_svr_5` | Servidor NTP 5 | text |  |

### `cbGetNTPInfo`

*lectura* · incompleta · riesgo bajo · página `system.asp`

NTP según system.asp. Restos de system.asp: la página existe pero el router responde 404 a sus llamadas.

### `cbSetNTPInfo`

*escritura* · incompleta · riesgo bajo · página `system.asp`

Configura NTP según system.asp. Restos de system.asp: la página existe pero el router responde 404 a sus llamadas.

Lectura pareada: `cbGetNTPInfo`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `rqNTPOnOff` | NTP activado | bool |  |
| `rqNTP1` | Servidor NTP 1 | text |  |
| `rqNTP2` | Servidor NTP 2 | text |  |
| `rqNTP3` | Servidor NTP 3 | text |  |
| `rqNTP4` | Servidor NTP 4 | text |  |
| `rqNTP5` | Servidor NTP 5 | text |  |

### `cbGetSystemTime`

*lectura* · incompleta · riesgo bajo · página `system.asp`

Hora del sistema según system.asp. Restos de system.asp: la página existe pero el router responde 404 a sus llamadas.

### `cbSetSystemTime`

*acción* · incompleta · riesgo bajo · página `system.asp`

Fija la hora del sistema según system.asp. Restos de system.asp: la página existe pero el router responde 404 a sus llamadas.

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `rqUTCSec` | Hora (segundos UTC) | int |  |

### `cbGetSystemTimeZone`

*lectura* · incompleta · riesgo bajo · página `system.asp`

Zona horaria y horario de verano según system.asp. Restos de system.asp: la página existe pero el router responde 404 a sus llamadas.

### `cbSetSystemTimeZone`

*escritura* · incompleta · riesgo bajo · página `system.asp`

Configura la zona horaria según system.asp. Restos de system.asp: la página existe pero el router responde 404 a sus llamadas.

Lectura pareada: `cbGetSystemTimeZone`

| Parámetro | Etiqueta | Tipo | Opciones |
|---|---|---|---|
| `rqOffset` | Desfase horario | text |  |
| `rqDstOnOff` | Horario de verano | bool |  |
| `rqMonthS` | Inicio: mes | text |  |
| `rqWeekS` | Inicio: semana | text |  |
| `rqDayS` | Inicio: día | text |  |
| `rqHourS` | Inicio: hora | text |  |
| `rqMonthE` | Fin: mes | text |  |
| `rqWeekE` | Fin: semana | text |  |
| `rqDayE` | Fin: día | text |  |
| `rqHourE` | Fin: hora | text |  |

### `cbSyncTime`

*acción* · incompleta · riesgo bajo · página `system.asp`

Fuerza la sincronización horaria según system.asp. Restos de system.asp: la página existe pero el router responde 404 a sus llamadas.

## Ocultas

### `cwmp.asp`

*lectura* · oculta · riesgo bajo · página `cwmp.asp`

Página de TR-069 (CWMP) que ningún menú enlaza: muestra y edita ACS, credenciales e Inform periódico (cbGetTr69Config/cbSetTr69Config).

### `ntp.asp`

*lectura* · oculta · riesgo bajo · página `ntp.asp`

Página de hora y NTP que ningún menú enlaza (cbGetTimeNtp, cbSetTimeNtp, cbSetLocalTime).

### `scbidirectionalflt_ed.asp`

*lectura* · oculta · riesgo bajo · página `scbidirectionalflt_ed.asp`

Editor de entradas y reglas del cortafuegos, abierto desde scbidirectionalflt_ls.asp.

### `scbidirectionalflt_ls.asp`

*lectura* · oculta · riesgo bajo · página `scbidirectionalflt_ls.asp`

Lista del cortafuegos IPv4/IPv6 completo; solo enlazada desde el texto de ayuda de te_otras_funcionalidades.asp (el bloque Firewall del menú avanzado está comentado).

### `user_profile_add.asp`

*lectura* · oculta · riesgo bajo · página `user_profile_add.asp`

Ventana emergente para crear usuarios (cbAddUser); no está en el menú, la abre un botón de user_profile.asp.

### `user_profile_modifyOthers.asp`

*lectura* · oculta · riesgo bajo · página `user_profile_modifyOthers.asp`

Ventana emergente para cambiar contraseña y grupo de otros usuarios (cbModifyPwdOther); permite elegir el grupo system si la sesión es system.

### `notsupport.asp`

*lectura* · incompleta · riesgo bajo · página `notsupport.asp`

Destino de «Online User Info», comentado en el menú (monu.asp): solo muestra «NOT support».

### `system.asp`

*lectura* · incompleta · riesgo bajo · página `system.asp`

Página antigua de hora, zona horaria y NTP; sus llamadas dan 404.

