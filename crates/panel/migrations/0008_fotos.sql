-- Fotos de la configuración del router.
--   momento    segundos Unix
--   tipo       'partida' (lectura completa del asistente de instalación) o
--              'manual'
--   contenido  JSON {función → XML de su respuesta}, cifrado con la clave
--              maestra (nonce + texto cifrado)
CREATE TABLE fotos (
    id        INTEGER PRIMARY KEY,
    momento   INTEGER NOT NULL,
    tipo      TEXT NOT NULL,
    contenido BLOB NOT NULL
);
