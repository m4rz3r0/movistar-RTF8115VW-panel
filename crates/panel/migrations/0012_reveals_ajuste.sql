-- Secretos mostrados desde Ajustes y valores anteriores mostrados desde el
-- Historial (Tarea 11). Como el resto de `reveals`, sin el valor:
--   ajuste     id del ajuste (`wifi.red_5`) si se mostró desde Ajustes; `campo`
--              es entonces el id del campo (`contrasena`) y `funcion`, la
--              lectura del ajuste.
--   cambio_id  cambio cuyo valor anterior se mostró; `campo` es el parámetro
--              y `funcion`, la escritura de su paso.
ALTER TABLE reveals ADD COLUMN ajuste TEXT;
ALTER TABLE reveals ADD COLUMN cambio_id INTEGER;
