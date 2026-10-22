# Ajustes del tema Sistema: hora del router y espejo para diagnóstico.

## Hora
ajuste-sistema-hora-titulo = Hora del router
ajuste-sistema-hora-desc = El router pone su reloj en hora preguntando a servidores de hora de internet.
ajuste-sistema-hora-ayuda = Con la hora mal, los registros del router y los horarios que dependan de ella van desfasados, y algunas conexiones seguras pueden fallar. El operador deja puesto un servidor suyo, que funciona; el panel recomienda uno público y neutro, como hora.roa.es (la hora oficial de España) o los del proyecto NTP Pool, para que la hora no dependa del operador. Los dos ponen bien el reloj.
ajuste-sistema-hora-activa = Poner en hora por internet
ajuste-sistema-hora-activa-desc = Si lo apagas, el router no corrige su reloj y se va desajustando.
ajuste-sistema-hora-activa-recomendado = Recomendado si quieres que el router tenga siempre la hora bien, lo normal. A favor: el reloj se corrige solo, también tras un corte de luz, porque los routers de casa no suelen tener pila para el reloj. En contra: el router consulta de vez en cuando un servidor de hora de internet.
ajuste-sistema-hora-servidor_1 = Servidor de hora
ajuste-sistema-hora-servidor_1-desc = El nombre del servidor al que se pregunta la hora, por ejemplo hora.example. Sin espacios, tildes ni ñ.
ajuste-sistema-hora-servidor_1-recomendado = Recomendado si prefieres que la hora del router no dependa del operador. A favor: hora.roa.es es el servidor del Real Instituto y Observatorio de la Armada, que da la hora oficial de España, y no lo gestiona ninguna empresa. En contra: si un día no contesta, el router tarda más en ponerse en hora; pon otro de reserva.
ajuste-sistema-hora-servidor_2 = Servidor de hora de reserva
ajuste-sistema-hora-servidor_2-desc = Se usa si el primero no contesta. Puedes dejarlo vacío.
ajuste-sistema-hora-servidor_2-recomendado = Recomendado si quieres un servidor de reserva que tampoco dependa del operador. A favor: es.pool.ntp.org es un grupo público de servidores de hora en España, de voluntarios y sin empresa detrás; si el primero no contesta, el router sigue en hora. En contra: ninguno importante; también puedes dejarlo vacío.

## Diagnóstico
ajuste-sistema-diagnostico-titulo = Copia del tráfico para diagnóstico
ajuste-sistema-diagnostico-desc = Copia todo lo que entra y sale de internet a una de las tomas de cable del router, para analizarlo con un ordenador.
ajuste-sistema-diagnostico-ayuda = Sirve a técnicos para ver qué pasa por la conexión. Mientras está encendida, quien enchufe un dispositivo en esa toma puede ver todo el tráfico de tu casa hacia internet. Apágala en cuanto termines.
ajuste-sistema-diagnostico-espejo = Copia encendida
ajuste-sistema-diagnostico-espejo-desc = Si la enciendes, la toma elegida recibe una copia de todo el tráfico de internet.
ajuste-sistema-diagnostico-espejo-recomendado = Recomendado si no estás analizando la conexión con un técnico. A favor: nadie que se enchufe a esa toma puede ver lo que hace tu casa en internet. En contra: ninguno importante; enciéndela solo mientras haces un diagnóstico.
ajuste-sistema-diagnostico-puerto = Toma que recibe la copia
ajuste-sistema-diagnostico-puerto-desc = La toma de cable del router a la que se envía la copia, numerada como en la parte de atrás del router.
ajuste-sistema-diagnostico-puerto-ninguna = Ninguna
ajuste-sistema-diagnostico-puerto-ninguna-desc = La copia está apagada: ninguna toma la recibe. Para encenderla, elige una toma.
ajuste-sistema-diagnostico-puerto-puerto_1 = Toma 1
ajuste-sistema-diagnostico-puerto-puerto_1-desc = La primera toma de cable de la red de casa.
ajuste-sistema-diagnostico-puerto-puerto_2 = Toma 2
ajuste-sistema-diagnostico-puerto-puerto_2-desc = La segunda toma de cable de la red de casa.
ajuste-sistema-diagnostico-puerto-puerto_3 = Toma 3
ajuste-sistema-diagnostico-puerto-puerto_3-desc = La tercera toma de cable de la red de casa.
ajuste-sistema-diagnostico-puerto-puerto_4 = Toma 4
ajuste-sistema-diagnostico-puerto-puerto_4-desc = La cuarta toma de cable de la red de casa.
