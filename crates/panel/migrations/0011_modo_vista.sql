-- Modo de visualización de cada usuario del panel (ver `crate::vista`):
-- «texto», «grafica» o «mixto». NULL es el de por defecto (mixto).
ALTER TABLE users ADD COLUMN modo_vista TEXT
    CHECK (modo_vista IN ('texto', 'grafica', 'mixto'));
