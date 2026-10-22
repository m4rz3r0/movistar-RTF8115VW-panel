# Ajustes del tema Red local: dispositivos, IP fijas, reparto de direcciones e IPv6.

## Dispositivos conectados
ajuste-red_local-dispositivos-titulo = Dispositivos conectados
ajuste-red_local-dispositivos-desc = Los dispositivos que el router conoce en tu red de casa: cómo se llaman, su dirección y por dónde se conectan. Con «Fijar esta IP», un dispositivo recibe siempre la misma dirección.
ajuste-red_local-dispositivos-ayuda = Ver esta lista no cambia nada en el router. «Fijar esta IP» guarda la dirección que tiene ahora el dispositivo como su IP fija (la verás en «IP fijas»); antes de aplicarlo te enseña el cambio. Desde aquí no se puede bloquear un dispositivo ni ponerle control parental: el router no ofrece al panel una forma segura de hacerlo, y se está investigando.
ajuste-red_local-dispositivos-col-ip = Dirección IP
ajuste-red_local-dispositivos-col-mac = Dirección física (MAC)
ajuste-red_local-dispositivos-col-conexion = Conexión
ajuste-red_local-dispositivos-col-ip_fija = IP fija
ajuste-red_local-dispositivos-sin-nombre = Sin nombre
ajuste-red_local-dispositivos-cable = Por cable
ajuste-red_local-dispositivos-wifi24 = WiFi 2,4 GHz
ajuste-red_local-dispositivos-wifi5 = WiFi 5 GHz
ajuste-red_local-dispositivos-invitados = WiFi de invitados
ajuste-red_local-dispositivos-desconocida = No se sabe
ajuste-red_local-dispositivos-desconectado = No conectado ahora
ajuste-red_local-dispositivos-fijar = Fijar esta IP
ajuste-red_local-dispositivos-fijar-de = Fijar esta IP: { $ip } ({ $equipo })
ajuste-red_local-dispositivos-ya-fija = Ya tiene IP fija: { $ip }
ajuste-red_local-dispositivos-no-fijable = Esta IP no se puede fijar
ajuste-red_local-dispositivos-fijar-desconectado = Conéctalo para fijar su IP
ajuste-red_local-dispositivos-vacia = El router todavía no conoce ningún dispositivo en tu red.

## IP fijas
ajuste-red_local-reservas-titulo = IP fijas
ajuste-red_local-reservas-desc = Dispositivos que reciben siempre la misma dirección IP en tu red, como una impresora o una consola a la que abres puertos.
ajuste-red_local-reservas-ayuda = El router reconoce cada dispositivo por su dirección física (MAC) y le da siempre la IP que elijas. El cambio vale la próxima vez que el dispositivo pida dirección: si lo desconectas y lo vuelves a conectar, la toma enseguida. Algunos móviles y ordenadores pueden cambiar su dirección física de vez en cuando (la opción suele llamarse «dirección privada», «dirección aleatoria» o «MAC aleatoria»). Si al dispositivo no le llega su IP fija, en sus ajustes WiFi de esta red pon esa opción en «fija» o desactívala.
ajuste-red_local-reservas-mac = Dirección física (MAC)
ajuste-red_local-reservas-mac-desc = El identificador del dispositivo: seis pares de cifras y letras separados por dos puntos, como 02:00:00:00:00:01.
ajuste-red_local-reservas-ip = Dirección IP
ajuste-red_local-reservas-ip-desc = La dirección que tendrá siempre el dispositivo, dentro de tu red (por ejemplo 192.168.1.60) y que no use ya otro dispositivo.
ajuste-red_local-reservas-activa = Activa
ajuste-red_local-reservas-activa-desc = Si la desactivas, la reserva se guarda pero el dispositivo vuelve a recibir una dirección cualquiera.
ajuste-red_local-reservas-activa-recomendado = Recomendado si quieres que el dispositivo tenga ya su IP fija. A favor: la recibe la próxima vez que se conecte. En contra: ninguno importante; desactívala solo si quieres guardarla para más adelante sin usarla.
ajuste-red_local-reservas-col-equipo = Dispositivo
ajuste-red_local-reservas-col-acciones = Qué hacer
ajuste-red_local-reservas-col-mac = Dirección física (MAC)
ajuste-red_local-reservas-col-ip = Dirección IP
ajuste-red_local-reservas-col-activa = Activa
ajuste-red_local-reservas-anadir = Añadir IP fija
ajuste-red_local-reservas-nueva = Nueva IP fija
ajuste-red_local-reservas-cambiar = Cambiar la IP fija
ajuste-red_local-reservas-editar-de = Editar la IP fija de { $equipo }
ajuste-red_local-reservas-borrar = Borrar
ajuste-red_local-reservas-borrar-de = Borrar la IP fija de { $equipo }
ajuste-red_local-reservas-vacia = Todavía no hay ninguna IP fija. Añádela aquí o con «Fijar esta IP» en la lista de dispositivos.
ajuste-red_local-reservas-error-fuera = Esa IP no es de tu red. Tiene que ser de la misma red que el router ({ $router }).
ajuste-red_local-reservas-error-router = Esa es la IP del router: elige otra.
ajuste-red_local-reservas-error-ip-repetida = Esa IP ya es la IP fija de otro dispositivo: elige otra.
ajuste-red_local-reservas-error-mac-repetida = Ese dispositivo ya tiene una IP fija: cámbiala en su fila en vez de añadir otra.
ajuste-red_local-reservas-error-difusion = Esa IP es de toda la red (la primera y la última no se pueden dar a un dispositivo): elige otra.
ajuste-red_local-reservas-error-cambiada = Esa IP fija ha cambiado desde que abriste la página (quizá desde otra ventana). Recarga la página y prueba de nuevo.

## Reparto de direcciones (DHCP)
ajuste-red_local-dhcp-titulo = Reparto de direcciones (DHCP)
ajuste-red_local-dhcp-desc = El router da automáticamente una dirección IP a cada dispositivo que se conecta a tu red.
ajuste-red_local-dhcp-ayuda = Cambiar el rango o la duración afecta a todos los dispositivos de casa cuando renueven su dirección. Si lo apagas, los dispositivos nuevos no podrán conectarse a menos que les pongas la dirección a mano. Las IP fijas deben quedar dentro de tu red.
ajuste-red_local-dhcp-activo = Reparto automático
ajuste-red_local-dhcp-activo-desc = Si lo apagas, el router deja de dar direcciones y cada dispositivo tiene que configurarse a mano.
ajuste-red_local-dhcp-activo-recomendado = Recomendado si quieres que tus dispositivos se conecten solos, como en casi todas las casas. A favor: cada móvil, ordenador o televisor recibe su dirección sin que hagas nada. En contra: ninguno importante para la mayoría; solo se apaga si otro dispositivo de tu red ya reparte las direcciones.
ajuste-red_local-dhcp-inicio = Primera dirección
ajuste-red_local-dhcp-inicio-desc = La primera dirección que el router puede repartir (por ejemplo 192.168.1.100).
ajuste-red_local-dhcp-fin = Última dirección
ajuste-red_local-dhcp-fin-desc = La última dirección que el router puede repartir. Entre la primera y la última tiene que haber sitio para todos tus dispositivos.
ajuste-red_local-dhcp-concesion = Duración de cada dirección
ajuste-red_local-dhcp-concesion-desc = Cuánto tiempo guarda cada dispositivo su dirección antes de pedir que se la renueven.
ajuste-red_local-dhcp-concesion-hora = 1 hora
ajuste-red_local-dhcp-concesion-hora-desc = Las direcciones se liberan pronto, útil si pasan muchos dispositivos distintos; a cambio, los dispositivos las renuevan más a menudo.
ajuste-red_local-dhcp-concesion-doce_horas = 12 horas
ajuste-red_local-dhcp-concesion-doce_horas-desc = Un término medio: cada dispositivo conserva su dirección medio día.
ajuste-red_local-dhcp-concesion-dia = 1 día
ajuste-red_local-dhcp-concesion-dia-desc = Los dispositivos conservan su dirección todo el día; las que dejan de usarse tardan un día en liberarse.
ajuste-red_local-dhcp-concesion-semana = 1 semana
ajuste-red_local-dhcp-concesion-semana-desc = Las direcciones casi no cambian, pero las de dispositivos que ya no vienen tardan en liberarse.
ajuste-red_local-dhcp-concesion-recomendado = Recomendado si en tu casa se conectan casi siempre los mismos dispositivos y alguna visita. A favor: cada dispositivo conserva su dirección y las de los que ya no vienen se liberan en medio día. En contra: si pasan muchos dispositivos distintos, una hora libera antes las direcciones.

## IPv6 en casa
ajuste-red_local-ipv6-titulo = IPv6 en casa
ajuste-red_local-ipv6-desc = Da también direcciones IPv6, la versión nueva de las direcciones de internet, a los dispositivos de tu red.
ajuste-red_local-ipv6-ayuda = Solo sirve si tu conexión a internet tiene IPv6. Al activarlo, los dispositivos empiezan a usar IPv6 donde se pueda; si tu conexión no lo tiene, los dispositivos no ganan nada. Si algún servicio deja de ir tras activarlo, vuelve a desactivarlo.
ajuste-red_local-ipv6-ipv6 = IPv6 en la red local
ajuste-red_local-ipv6-ipv6-desc = Si lo activas, el router da direcciones IPv6 a tus dispositivos además de las de siempre.
ajuste-red_local-ipv6-anuncios = Anuncios del router
ajuste-red_local-ipv6-anuncios-desc = El router avisa a los dispositivos de que hay IPv6 para que se configuren solos. Sin estos avisos, casi ninguno usará IPv6.
ajuste-red_local-ipv6-anuncios-recomendado = Recomendado si tienes IPv6 encendido en la red de casa. A favor: los dispositivos se configuran IPv6 solos y encuentran por dónde salir a internet con IPv6. En contra: ninguno importante para la mayoría; si no usas IPv6, no hacen nada.
