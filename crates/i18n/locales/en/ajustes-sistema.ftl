# Settings in the System topic: router clock, mirror for diagnostics and system log.

## Time
ajuste-sistema-hora-titulo = Router clock
ajuste-sistema-hora-desc = The router sets its clock by asking time servers on the internet.
ajuste-sistema-hora-ayuda = With the wrong time, the router's logs and any schedules that depend on it are off, and some secure connections may fail. The operator leaves one of its own servers in place, which works; the panel recommends a public, neutral one, such as hora.roa.es (Spain's official time) or those of the NTP Pool project, so the time does not depend on the operator. Both set the clock correctly.
ajuste-sistema-hora-activa = Set the time over the internet
ajuste-sistema-hora-activa-desc = If you turn it off, the router does not correct its clock and it drifts.
ajuste-sistema-hora-activa-recomendado = Recommended if you want the router to always have the right time, as is usual. Pros: the clock corrects itself, also after a power cut, because home routers do not usually have a clock battery. Cons: the router checks an internet time server every so often.
ajuste-sistema-hora-servidor_1 = Time server
ajuste-sistema-hora-servidor_1-desc = The name of the server asked for the time, for example hora.example. No spaces or accented letters.
ajuste-sistema-hora-servidor_1-recomendado = Recommended if you would rather the router's time did not depend on your operator. Pros: hora.roa.es is the server of the Spanish Navy's Royal Institute and Observatory, which keeps Spain's official time, and no company runs it. Cons: if it ever does not answer, the router takes longer to set its time; add a backup server.
ajuste-sistema-hora-servidor_2 = Backup time server
ajuste-sistema-hora-servidor_2-desc = Used if the first one does not answer. You can leave it empty.
ajuste-sistema-hora-servidor_2-recomendado = Recommended if you want a backup server that does not depend on the operator either. Pros: es.pool.ntp.org is a public group of time servers in Spain, run by volunteers with no company behind it; if the first one does not answer, the router keeps the right time. Cons: none significant; you can also leave it empty.

## Diagnostics
ajuste-sistema-diagnostico-titulo = Traffic copy for diagnostics
ajuste-sistema-diagnostico-desc = Copies everything that goes to and from the internet to one of the router's cable sockets, so it can be analysed with a computer.
ajuste-sistema-diagnostico-ayuda = Technicians use it to see what goes through the connection. While it is on, whoever plugs a device into that socket can see all your home's internet traffic. Turn it off as soon as you are done.
ajuste-sistema-diagnostico-espejo = Copy on
ajuste-sistema-diagnostico-espejo-desc = If you turn it on, the chosen socket receives a copy of all internet traffic.
ajuste-sistema-diagnostico-espejo-recomendado = Recommended if you are not analysing the connection with a technician. Pros: nobody who plugs into that socket can see what your home does on the internet. Cons: none significant; turn it on only while you run a diagnosis.
ajuste-sistema-diagnostico-puerto = Socket that receives the copy
ajuste-sistema-diagnostico-puerto-desc = The router's cable socket the copy is sent to, numbered as on the back of the router.
ajuste-sistema-diagnostico-puerto-ninguna = None
ajuste-sistema-diagnostico-puerto-ninguna-desc = The copy is off: no socket receives it. To turn it on, choose a socket.
ajuste-sistema-diagnostico-puerto-puerto_1 = Socket 1
ajuste-sistema-diagnostico-puerto-puerto_1-desc = The first cable socket of the home network.
ajuste-sistema-diagnostico-puerto-puerto_2 = Socket 2
ajuste-sistema-diagnostico-puerto-puerto_2-desc = The second cable socket of the home network.
ajuste-sistema-diagnostico-puerto-puerto_3 = Socket 3
ajuste-sistema-diagnostico-puerto-puerto_3-desc = The third cable socket of the home network.
ajuste-sistema-diagnostico-puerto-puerto_4 = Socket 4
ajuste-sistema-diagnostico-puerto-puerto_4-desc = The fourth cable socket of the home network.

## System log
ajuste-sistema-registro-titulo = System log
ajuste-sistema-registro-desc = The router writes down what it does in a log. Besides keeping it, it can send it to another computer.
ajuste-sistema-registro-ayuda = The log tells when the router restarts and what fails, and may tell which devices connect. If you send it to another computer, that computer receives all of it, and it travels unencrypted on the way. With sending off, the log stays on the router. The panel never exports the log.
ajuste-sistema-registro-envio = Send the log to another computer
ajuste-sistema-registro-envio-desc = If you turn it on, the router keeps its log and also sends it to the server you enter.
ajuste-sistema-registro-envio-recomendado = Recommended if you do not have your own server to collect logs, which is normal at home. Pros: the log, which may tell which devices in your home connect and when, does not leave the router unencrypted towards another computer. Cons: if you use your own server for logs, it stops receiving the router's.
ajuste-sistema-registro-servidor = Server that receives the log
ajuste-sistema-registro-servidor-desc = The IP address of the computer the log is sent to, for example 192.168.1.60. It is needed to turn sending on.
ajuste-sistema-registro-puerto = Server port
ajuste-sistema-registro-puerto-desc = The port on which that computer receives the log, from 1 to 65535. The usual one is 514.
