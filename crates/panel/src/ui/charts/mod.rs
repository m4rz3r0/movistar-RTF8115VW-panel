//! Gráficos SVG propios (sin librerías JS).
//!
//! Funciones puras que se prueban sin navegador (`escalas`, `tiempo`,
//! `geometria`) y componentes ([`SerieTemporal`], [`FranjaDisponibilidad`],
//! [`Minigrafica`] e [`Indicador`]).
//!
//! Criterios: una sola escala Y por gráfico y desde cero (son magnitudes),
//! líneas de 2 px, rejilla y ejes en filetes discretos, huecos sin datos como
//! cortes de línea (nunca se interpola), cruceta con lectura de todas las
//! series al pasar el puntero o tocar, y una tabla con los mismos datos para
//! quien no pueda ver el gráfico. Los colores de las series salen de las clases
//! `tono-1`/`tono-2` de la hoja de estilos (sin estilos en línea por la CSP);
//! los textos nunca llevan el color de la serie.
//!
//! Las horas se pintan en hora peninsular española, calculada aquí mismo para
//! que el servidor y el navegador generen el mismo HTML al hidratar.

mod escalas;
mod franja;
mod geometria;
mod indicador;
mod serie;
mod tiempo;

pub use escalas::*;
pub use franja::*;
pub use geometria::*;
pub use indicador::*;
pub use serie::*;
pub use tiempo::*;
