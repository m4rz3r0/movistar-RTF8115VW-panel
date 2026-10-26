//! Por dónde llega este equipo al router.

use super::Client;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

impl Client {
    /// IP de este equipo con la que se llega al router: la de la interfaz
    /// por la que sale el tráfico hacia él. No envía nada (un socket UDP
    /// «conectado» solo elige la ruta). `None` sin router configurado o si
    /// no se puede saber.
    pub async fn ip_local(&self) -> Option<IpAddr> {
        let base = self
            .shared
            .state
            .lock()
            .await
            .cfg
            .as_ref()?
            .base_url
            .clone();
        let url = reqwest::Url::parse(&base).ok()?;
        let host = url.host_str()?.trim_matches(['[', ']']).to_owned();
        let puerto = url.port_or_known_default()?;
        let destino = tokio::net::lookup_host((host.as_str(), puerto))
            .await
            .ok()?
            .next()?;
        let origen: SocketAddr = if destino.is_ipv4() {
            (Ipv4Addr::UNSPECIFIED, 0).into()
        } else {
            (Ipv6Addr::UNSPECIFIED, 0).into()
        };
        let s = tokio::net::UdpSocket::bind(origen).await.ok()?;
        s.connect(destino).await.ok()?;
        s.local_addr().ok().map(|a| a.ip())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientConfig;

    /// Hacia un router en esta misma máquina se sale por la interfaz local;
    /// sin router, no se sabe.
    #[tokio::test]
    async fn ip_hacia_el_router() {
        let c = Client::new(ClientConfig::new("http://127.0.0.1:9", "u", "p"));
        assert_eq!(c.ip_local().await, Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));
        assert_eq!(Client::sin_configurar().ip_local().await, None);
    }
}
