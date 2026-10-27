-- Cola de cambios: un cambio confirmado mientras otro se vigila espera su
-- turno (`en_cola`) y, si no llega a aplicarse (el anterior se deshizo,
-- se quitó de la cola, el panel se reinició…), queda `cancelado` con el
-- motivo en `error`. Sus pasos, `cancelado` también.
--
-- Se reconstruyen `changes` y `cambio_pasos` (SQLite no cambia un CHECK).
-- Los pasos se apartan antes: borrar `changes` con las claves ajenas
-- activas los borraría en cascada.
CREATE TABLE cambio_pasos_copia AS SELECT * FROM cambio_pasos;
DROP TABLE cambio_pasos;

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
                            'revertido', 'fallido', 'revertido_parcial',
                            'en_cola', 'cancelado')),
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
    nuevos_cifrados, nota, wan_antes, deshecho_por, ids_antes, titulo)
SELECT id, ts, updated, user_id, funcion, instancia, riesgo, estado,
    diff, anteriores, anteriores_cifrados, error, usuario, operacion, solicitud, nuevos,
    nuevos_cifrados, nota, wan_antes, deshecho_por, ids_antes, titulo
FROM changes;

DROP TABLE changes;
ALTER TABLE changes_nueva RENAME TO changes;
CREATE INDEX changes_ts ON changes (ts);
CREATE INDEX changes_estado ON changes (estado);

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
                                    'fallido', 'revertido', 'cancelado')),
    error                       TEXT,
    PRIMARY KEY (cambio_id, orden)
) STRICT;

INSERT INTO cambio_pasos (cambio_id, orden, funcion, instancia, etiqueta, operacion, solicitud, diff,
    valores_nuevos, valores_nuevos_cifrados, valores_anteriores, valores_anteriores_cifrados,
    ids_antes, estado, error)
SELECT cambio_id, orden, funcion, instancia, etiqueta, operacion, solicitud, diff,
    valores_nuevos, valores_nuevos_cifrados, valores_anteriores, valores_anteriores_cifrados,
    ids_antes, estado, error
FROM cambio_pasos_copia;

DROP TABLE cambio_pasos_copia;
