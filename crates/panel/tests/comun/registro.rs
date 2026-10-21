//! Registro (`tracing`) en memoria, para comprobar lo que se anota.

use std::sync::{Arc, Mutex, OnceLock};

/// Destino del registro en memoria.
#[derive(Clone, Default)]
pub struct Registro(Arc<Mutex<Vec<u8>>>);

impl Registro {
    pub fn texto(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

impl std::io::Write for Registro {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Registro {
    type Writer = Registro;

    fn make_writer(&'a self) -> Registro {
        self.clone()
    }
}

/// El registro de todo el binario de pruebas (todas las pruebas, en todos
/// los hilos): uno de hilo pierde eventos cuando otras pruebas corren a la
/// vez (la caché de interés de `tracing`). Que tampoco las demás pruebas
/// anoten un secreto o una contraseña es más estricto, no menos.
pub fn registro_global() -> Registro {
    static REGISTRO: OnceLock<Registro> = OnceLock::new();
    REGISTRO
        .get_or_init(|| {
            let registro = Registro::default();
            tracing::subscriber::set_global_default(
                tracing_subscriber::fmt()
                    .with_writer(registro.clone())
                    .with_max_level(tracing::Level::DEBUG)
                    .with_ansi(false)
                    .finish(),
            )
            .expect("el único registro global de estas pruebas");
            registro
        })
        .clone()
}
