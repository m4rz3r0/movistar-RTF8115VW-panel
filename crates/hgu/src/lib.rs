//! Cliente y utilidades para el router Askey RTF8115VW (Movistar/O2).

pub mod client;
pub mod error;
pub mod fmt;
pub mod models;
pub mod read;
pub mod xml;

pub use client::{Client, ClientConfig};
pub use error::Error;
