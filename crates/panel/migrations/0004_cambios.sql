-- Gestor de cambios: lo necesario para retomar, revertir y deshacer.
--   usuario          quién confirmó el cambio (se conserva aunque se borre el usuario)
--   operacion        editar | crear | borrar | accion
--   solicitud        JSON de los valores enviados que no son secretos
--   nuevos           JSON de los valores nuevos de los campos cambiados, sin secretos
--   nuevos_cifrados  valores nuevos secretos, cifrados con history.key
--   nota             observaciones (verificación limitada, vigilancia retomada…)
-- `anteriores`/`anteriores_cifrados` (0001) guardan los valores previos de
-- los campos cambiados que el router mostraba.
ALTER TABLE changes ADD COLUMN usuario TEXT;
ALTER TABLE changes ADD COLUMN operacion TEXT NOT NULL DEFAULT 'editar'
    CHECK (operacion IN ('editar', 'crear', 'borrar', 'accion'));
ALTER TABLE changes ADD COLUMN solicitud TEXT NOT NULL DEFAULT '{}';
ALTER TABLE changes ADD COLUMN nuevos TEXT NOT NULL DEFAULT '{}';
ALTER TABLE changes ADD COLUMN nuevos_cifrados BLOB;
ALTER TABLE changes ADD COLUMN nota TEXT;
