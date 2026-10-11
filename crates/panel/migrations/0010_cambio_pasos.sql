-- Gestor de cambios por grupos: cada cambio (`changes`) es un grupo de
-- pasos (`cambio_pasos`), cada uno la escritura de una función.
--
-- `changes` se reconstruye (SQLite no cambia un CHECK) para admitir el estado
-- `revertido_parcial` y guardar el título del grupo:
--   titulo  clave i18n del resumen del grupo (NULL: el de su primer paso)
-- Sus columnas funcion/instancia/operacion/diff/anteriores/nuevos… quedan con
-- los datos del primer paso escrito; lo que vale para retomar, revertir y
-- deshacer está en `cambio_pasos`.
CREATE TABLE changes_nueva (
    id                  INTEGER PRIMARY KEY,
    ts                  INTEGER NOT NULL,
    updated             INTEGER NOT NULL,
    user_id             INTEGER REFERENCES users (id) ON DELETE SET NULL,
    funcion             TEXT    NOT NULL,
    instancia           TEXT,
    riesgo              TEXT    NOT NULL CHECK (riesgo IN ('bajo', 'medio', 'alto')),
    estado              TEXT    NOT NULL CHECK (estado IN (
                            'preparado', 'aplicando', 'vigilando', 'aplicado',
                            'revertido', 'fallido', 'revertido_parcial')),
    diff                TEXT    NOT NULL,
    anteriores          TEXT    NOT NULL,
    anteriores_cifrados BLOB,
    error               TEXT,
    usuario             TEXT,
    operacion           TEXT    NOT NULL DEFAULT 'editar'
                            CHECK (operacion IN ('editar', 'crear', 'borrar', 'accion')),
    solicitud           TEXT    NOT NULL DEFAULT '{}',
    nuevos              TEXT    NOT NULL DEFAULT '{}',
    nuevos_cifrados     BLOB,
    nota                TEXT,
    wan_antes           INTEGER CHECK (wan_antes IN (0, 1)),
    -- Se refiere a sí misma: al renombrarla pasa a `changes`. (Con
    -- `REFERENCES changes`, borrar la tabla vieja vaciaría el enlace.)
    deshecho_por        INTEGER REFERENCES changes_nueva (id) ON DELETE SET NULL,
    ids_antes           TEXT,
    titulo              TEXT
) STRICT;

INSERT INTO changes_nueva (id, ts, updated, user_id, funcion, instancia, riesgo, estado,
    diff, anteriores, anteriores_cifrados, error, usuario, operacion, solicitud, nuevos,
    nuevos_cifrados, nota, wan_antes, deshecho_por, ids_antes)
SELECT id, ts, updated, user_id, funcion, instancia, riesgo, estado,
    diff, anteriores, anteriores_cifrados, error, usuario, operacion, solicitud, nuevos,
    nuevos_cifrados, nota, wan_antes, deshecho_por, ids_antes
FROM changes;

DROP TABLE changes;
ALTER TABLE changes_nueva RENAME TO changes;
CREATE INDEX changes_ts ON changes (ts);
CREATE INDEX changes_estado ON changes (estado);

-- Pasos de un grupo, en orden (`orden` desde 0).
--   etiqueta              clave i18n del resumen del paso (NULL: la función)
--   operacion             editar | crear | borrar | accion
--   solicitud             JSON de los valores enviados que no son secretos
--   diff                  JSON del diff, secretos enmascarados
--   valores_nuevos        JSON (UTF-8) de los valores nuevos no secretos
--   valores_anteriores    JSON (UTF-8) de los valores previos no secretos
--   *_cifrados            los secretos, cifrados con history.key
--   ids_antes             al crear: ids de la lista antes de escribir (JSON)
--   estado                pendiente (aún no se escribió) | omitido (sin
--                         cambios o desmarcado) | aplicando (escrito o a
--                         punto, sin verificar) | aplicado | fallido (no se
--                         aplicó, o no se pudo revertir sin saber si se
--                         aplicó) | revertido
--   error                 motivo del fallo del paso o de su reversión
CREATE TABLE cambio_pasos (
    cambio_id                   INTEGER NOT NULL REFERENCES changes (id) ON DELETE CASCADE,
    orden                       INTEGER NOT NULL,
    funcion                     TEXT    NOT NULL,
    instancia                   TEXT,
    etiqueta                    TEXT,
    operacion                   TEXT    NOT NULL DEFAULT 'editar'
                                    CHECK (operacion IN ('editar', 'crear', 'borrar', 'accion')),
    solicitud                   TEXT    NOT NULL DEFAULT '{}',
    diff                        TEXT    NOT NULL DEFAULT '[]',
    valores_nuevos              BLOB    NOT NULL,
    valores_nuevos_cifrados     BLOB,
    valores_anteriores          BLOB    NOT NULL,
    valores_anteriores_cifrados BLOB,
    ids_antes                   TEXT,
    estado                      TEXT    NOT NULL CHECK (estado IN (
                                    'pendiente', 'omitido', 'aplicando', 'aplicado',
                                    'fallido', 'revertido')),
    error                       TEXT,
    PRIMARY KEY (cambio_id, orden)
) STRICT;

-- Los cambios de antes: un grupo de un paso con sus mismos datos.
INSERT INTO cambio_pasos (cambio_id, orden, funcion, instancia, operacion, solicitud, diff,
    valores_nuevos, valores_nuevos_cifrados, valores_anteriores, valores_anteriores_cifrados,
    ids_antes, estado, error)
SELECT id, 0, funcion, instancia, operacion, solicitud, diff,
    CAST(nuevos AS BLOB), nuevos_cifrados, CAST(anteriores AS BLOB), anteriores_cifrados,
    ids_antes,
    CASE estado
        WHEN 'preparado' THEN 'pendiente'
        WHEN 'vigilando' THEN 'aplicado'
        ELSE estado
    END,
    NULL
FROM changes;
