-- El freno de fuerza bruta mide la espera con precisión de milisegundos: con
-- segundos enteros una espera de 1 s podía quedarse en casi nada.
DELETE FROM login_attempts;
ALTER TABLE login_attempts RENAME COLUMN last_failure TO last_failure_ms;
