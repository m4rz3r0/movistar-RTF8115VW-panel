-- Datos de la instancia del panel.
--   router_credenciales  una sola fila: URL, usuario y contraseña del router
--                        (cifrada con history.key: nonce + texto cifrado);
--                        actualizado en segundos Unix
--   ajustes_instancia    parámetros de la instancia, clave → valor JSON
CREATE TABLE router_credenciales (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    url         TEXT NOT NULL,
    usuario     TEXT NOT NULL,
    contrasena  BLOB NOT NULL,
    actualizado INTEGER NOT NULL
);

CREATE TABLE ajustes_instancia (
    clave TEXT PRIMARY KEY,
    valor TEXT NOT NULL
);
