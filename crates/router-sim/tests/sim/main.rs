//! Pruebas del router simulado y de las fixtures saneadas.

use hgu::{Client, ClientConfig, Error, xml::XmlNode};
use router_sim::{Sim, SimConfig, SimHandle, sanitize};
use std::{collections::HashSet, net::Ipv4Addr, path::PathBuf, sync::LazyLock, time::Duration};

mod comun;
use comun::*;

mod control;
mod escrituras;
mod fixtures;
mod lecturas;
mod listas;
mod reservas;
mod sesion;
