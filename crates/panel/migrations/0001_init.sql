-- Esquema inicial del panel. Las marcas de tiempo son segundos Unix (UTC).

-- Usuario administrador del panel (alta con `panel user add`).
CREATE TABLE users (
    id            INTEGER PRIMARY KEY,
    username      TEXT    NOT NULL UNIQUE,
    password_hash TEXT    NOT NULL, -- Argon2id en formato PHC
    created       INTEGER NOT NULL
) STRICT;

-- Sesiones del panel: id aleatorio de 256 bits (base64url).
CREATE TABLE sessions (
    id        TEXT    PRIMARY KEY,
    user_id   INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created   INTEGER NOT NULL,
    last_seen INTEGER NOT NULL
) STRICT;
CREATE INDEX sessions_user_id ON sessions (user_id);

-- Freno de fuerza bruta: fallos seguidos por IP de cliente y por usuario.
CREATE TABLE login_attempts (
    kind         TEXT    NOT NULL CHECK (kind IN ('ip', 'user')),
    key          TEXT    NOT NULL, -- IP o nombre de usuario
    failures     INTEGER NOT NULL,
    last_failure INTEGER NOT NULL,
    PRIMARY KEY (kind, key)
) STRICT;

-- Historial de cambios aplicados al router (gestor de cambios).
CREATE TABLE changes (
    id                  INTEGER PRIMARY KEY,
    ts                  INTEGER NOT NULL,
    updated             INTEGER NOT NULL,
    user_id             INTEGER REFERENCES users (id) ON DELETE SET NULL,
    funcion             TEXT    NOT NULL,
    instancia           TEXT,
    riesgo              TEXT    NOT NULL CHECK (riesgo IN ('bajo', 'medio', 'alto')),
    estado              TEXT    NOT NULL CHECK (estado IN (
                            'preparado', 'aplicando', 'vigilando',
                            'aplicado', 'revertido', 'fallido')),
    diff                TEXT    NOT NULL, -- JSON, secretos enmascarados
    anteriores          TEXT    NOT NULL, -- JSON de valores previos sin secretos
    anteriores_cifrados BLOB,             -- valores previos secretos, cifrados con history.key
    error               TEXT
) STRICT;
CREATE INDEX changes_ts ON changes (ts);
CREATE INDEX changes_estado ON changes (estado);

-- Registro de cada "Revelar" de un valor secreto.
CREATE TABLE reveals (
    id        INTEGER PRIMARY KEY,
    ts        INTEGER NOT NULL,
    user_id   INTEGER REFERENCES users (id) ON DELETE SET NULL,
    funcion   TEXT    NOT NULL,
    campo     TEXT    NOT NULL,
    instancia TEXT
) STRICT;
CREATE INDEX reveals_ts ON reveals (ts);

-- Series temporales del estado: crudo (24 h), 1 min (7 d) y 15 min (30 d).
CREATE TABLE samples_raw (
    ts           INTEGER PRIMARY KEY,
    rx_bps       REAL,
    tx_bps       REAL,
    load1        REAL    NOT NULL,
    ram_used_pct REAL    NOT NULL,
    clients      INTEGER,
    wan_up       INTEGER NOT NULL CHECK (wan_up IN (0, 1))
) STRICT;

CREATE TABLE samples_1m (
    ts           INTEGER PRIMARY KEY,
    rx_bps       REAL,
    tx_bps       REAL,
    load1        REAL    NOT NULL,
    ram_used_pct REAL    NOT NULL,
    clients      INTEGER,
    wan_up       INTEGER NOT NULL CHECK (wan_up IN (0, 1))
) STRICT;

CREATE TABLE samples_15m (
    ts           INTEGER PRIMARY KEY,
    rx_bps       REAL,
    tx_bps       REAL,
    load1        REAL    NOT NULL,
    ram_used_pct REAL    NOT NULL,
    clients      INTEGER,
    wan_up       INTEGER NOT NULL CHECK (wan_up IN (0, 1))
) STRICT;

-- Cortes de la WAN principal (ip2); `hasta` es NULL mientras sigue caída.
CREATE TABLE wan_events (
    id    INTEGER PRIMARY KEY,
    desde INTEGER NOT NULL,
    hasta INTEGER
) STRICT;
CREATE INDEX wan_events_desde ON wan_events (desde);
