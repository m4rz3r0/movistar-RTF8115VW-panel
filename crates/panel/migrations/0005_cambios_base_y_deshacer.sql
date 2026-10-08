-- Gestor de cambios:
--   wan_antes     WAN (ip2) arriba (1) o caída (0) al confirmar; NULL en un
--                 reinicio. Al retomar tras un reinicio del panel se vigila
--                 con este valor, nunca con el estado posterior al cambio.
--   deshecho_por  cambio que deshizo este (no se deshace dos veces).
ALTER TABLE changes ADD COLUMN wan_antes INTEGER CHECK (wan_antes IN (0, 1));
ALTER TABLE changes ADD COLUMN deshecho_por INTEGER REFERENCES changes (id) ON DELETE SET NULL;
