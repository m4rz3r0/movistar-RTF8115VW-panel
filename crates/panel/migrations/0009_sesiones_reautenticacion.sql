-- Ventana de reautenticación de cada sesión: hasta cuándo (segundos Unix)
-- no hay que volver a escribir la contraseña del panel para las operaciones
-- delicadas de «Ajustes del panel». NULL: hay que escribirla.
ALTER TABLE sessions ADD COLUMN reautenticado_hasta INTEGER;
