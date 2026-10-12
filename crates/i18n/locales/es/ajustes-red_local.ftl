# Ajustes del tema Red local: aparatos, IP fijas, reparto de direcciones e IPv6.

## Aparatos conectados
ajuste-red_local-dispositivos-titulo = Aparatos conectados
ajuste-red_local-dispositivos-desc = Los aparatos que el router conoce en tu red de casa, con su dirección y si están conectados ahora.
ajuste-red_local-dispositivos-ayuda = Esta lista solo se consulta: no cambia nada en el router. Te sirve para saber qué hay conectado y para dar una IP fija a un aparato.

## IP fijas
ajuste-red_local-reservas-titulo = IP fijas
ajuste-red_local-reservas-desc = Aparatos que reciben siempre la misma dirección IP en tu red, como una impresora o una consola a la que abres puertos.
ajuste-red_local-reservas-ayuda = El router reconoce cada aparato por su dirección física (MAC) y le da siempre la IP que elijas. El cambio vale la próxima vez que el aparato pida dirección: si lo desconectas y lo vuelves a conectar, la toma enseguida. Algunos móviles y ordenadores pueden cambiar su dirección física de vez en cuando (la opción suele llamarse «dirección privada», «dirección aleatoria» o «MAC aleatoria»). Si al aparato no le llega su IP fija, en sus ajustes WiFi de esta red pon esa opción en «fija» o desactívala.
ajuste-red_local-reservas-mac = Dirección física (MAC)
ajuste-red_local-reservas-mac-desc = El identificador del aparato: seis pares de cifras y letras separados por dos puntos, como 02:00:00:00:00:01.
ajuste-red_local-reservas-ip = Dirección IP
ajuste-red_local-reservas-ip-desc = La dirección que tendrá siempre el aparato, dentro de tu red (por ejemplo 192.168.1.60) y que no use ya otro aparato.
ajuste-red_local-reservas-activa = Activa
ajuste-red_local-reservas-activa-desc = Si la desactivas, la reserva se guarda pero el aparato vuelve a recibir una dirección cualquiera.

## Reparto de direcciones (DHCP)
ajuste-red_local-dhcp-titulo = Reparto de direcciones (DHCP)
ajuste-red_local-dhcp-desc = El router da automáticamente una dirección IP a cada aparato que se conecta a tu red.
ajuste-red_local-dhcp-ayuda = Cambiar el rango o la duración afecta a todos los aparatos de casa cuando renueven su dirección. Si lo apagas, los aparatos nuevos no podrán conectarse a menos que les pongas la dirección a mano. Las IP fijas deben quedar dentro de tu red.
ajuste-red_local-dhcp-activo = Reparto automático
ajuste-red_local-dhcp-activo-desc = Si lo apagas, el router deja de dar direcciones y cada aparato tiene que configurarse a mano.
ajuste-red_local-dhcp-activo-recomendado = Recomendado si quieres que tus aparatos se conecten solos, como en casi todas las casas. A favor: cada móvil, ordenador o televisor recibe su dirección sin que hagas nada. En contra: ninguno importante para la mayoría; solo se apaga si otro aparato de tu red ya reparte las direcciones.
ajuste-red_local-dhcp-inicio = Primera dirección
ajuste-red_local-dhcp-inicio-desc = La primera dirección que el router puede repartir (por ejemplo 192.168.1.100).
ajuste-red_local-dhcp-fin = Última dirección
ajuste-red_local-dhcp-fin-desc = La última dirección que el router puede repartir. Entre la primera y la última tiene que haber sitio para todos tus aparatos.
ajuste-red_local-dhcp-concesion = Duración de cada dirección
ajuste-red_local-dhcp-concesion-desc = Cuánto tiempo guarda cada aparato su dirección antes de pedir que se la renueven.
ajuste-red_local-dhcp-concesion-hora = 1 hora
ajuste-red_local-dhcp-concesion-hora-desc = Las direcciones se liberan pronto, útil si pasan muchos aparatos distintos; a cambio, los aparatos las renuevan más a menudo.
ajuste-red_local-dhcp-concesion-doce_horas = 12 horas
ajuste-red_local-dhcp-concesion-doce_horas-desc = Un término medio: cada aparato conserva su dirección medio día.
ajuste-red_local-dhcp-concesion-dia = 1 día
ajuste-red_local-dhcp-concesion-dia-desc = Los aparatos conservan su dirección todo el día; las que dejan de usarse tardan un día en liberarse.
ajuste-red_local-dhcp-concesion-semana = 1 semana
ajuste-red_local-dhcp-concesion-semana-desc = Las direcciones casi no cambian, pero las de aparatos que ya no vienen tardan en liberarse.
ajuste-red_local-dhcp-concesion-recomendado = Recomendado si en tu casa se conectan casi siempre los mismos aparatos y alguna visita. A favor: cada aparato conserva su dirección y las de los que ya no vienen se liberan en medio día. En contra: si pasan muchos aparatos distintos, una hora libera antes las direcciones.

## IPv6 en casa
ajuste-red_local-ipv6-titulo = IPv6 en casa
ajuste-red_local-ipv6-desc = Da también direcciones IPv6, la versión nueva de las direcciones de internet, a los aparatos de tu red.
ajuste-red_local-ipv6-ayuda = Solo sirve si tu conexión a internet tiene IPv6. Al activarlo, los aparatos empiezan a usar IPv6 donde se pueda; si tu conexión no lo tiene, los aparatos no ganan nada. Si algún servicio deja de ir tras activarlo, vuelve a desactivarlo.
ajuste-red_local-ipv6-ipv6 = IPv6 en la red local
ajuste-red_local-ipv6-ipv6-desc = Si lo activas, el router da direcciones IPv6 a tus aparatos además de las de siempre.
ajuste-red_local-ipv6-anuncios = Anuncios del router
ajuste-red_local-ipv6-anuncios-desc = El router avisa a los aparatos de que hay IPv6 para que se configuren solos. Sin estos avisos, casi ninguno usará IPv6.
ajuste-red_local-ipv6-anuncios-recomendado = Recomendado si tienes IPv6 encendido en la red de casa. A favor: los aparatos se configuran IPv6 solos y encuentran por dónde salir a internet con IPv6. En contra: ninguno importante para la mayoría; si no usas IPv6, no hacen nada.
