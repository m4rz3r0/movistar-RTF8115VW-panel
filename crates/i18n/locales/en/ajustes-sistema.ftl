# Settings in the System topic: router clock and mirror for diagnostics.

## Time
ajuste-sistema-hora-titulo = Router clock
ajuste-sistema-hora-desc = The router sets its clock by asking time servers on the internet.
ajuste-sistema-hora-ayuda = With the wrong time, the router's logs and any schedules that depend on it are off, and some secure connections may fail. The operator leaves a working server in place: only change it if you know another one suits you better.
ajuste-sistema-hora-activa = Set the time over the internet
ajuste-sistema-hora-activa-desc = If you turn it off, the router does not correct its clock and it drifts.
ajuste-sistema-hora-activa-recomendado = Recommended if you want the router to always have the right time, as is usual. Pros: the clock corrects itself, also after a power cut, because home routers do not usually have a clock battery. Cons: the router checks an internet time server every so often.
ajuste-sistema-hora-servidor_1 = Time server
ajuste-sistema-hora-servidor_1-desc = The name of the server asked for the time, for example hora.example. No spaces or accented letters.
ajuste-sistema-hora-servidor_2 = Backup time server
ajuste-sistema-hora-servidor_2-desc = Used if the first one does not answer. You can leave it empty.

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
