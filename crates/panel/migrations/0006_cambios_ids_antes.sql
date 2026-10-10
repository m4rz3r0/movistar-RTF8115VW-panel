-- Gestor de cambios:
--   ids_antes  al crear una instancia: ids de la lista antes de escribir
--              (JSON, lista de cadenas). La instancia creada es la que
--              aparece nueva; sin esto, una igual de antes se confundiría con
--              ella y deshacer/revertir la borraría. NULL en los demás casos.
ALTER TABLE changes ADD COLUMN ids_antes TEXT;
