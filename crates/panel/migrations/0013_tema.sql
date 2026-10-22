-- Tema de la interfaz de cada usuario del panel (ver `crate::tema`):
-- «automatico», «oscuro» o «claro». NULL es el de por defecto (automático).
ALTER TABLE users ADD COLUMN tema TEXT
    CHECK (tema IN ('automatico', 'oscuro', 'claro'));
