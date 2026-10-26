//! Conexiones TCP del simulador que se pueden «cortar» en silencio, como las
//! de un router que rehace su red o su servidor web (p. ej. al cambiar el
//! QoS): lo que llegue después por una conexión de antes no se lee nunca ni
//! se responde, sin cerrarla. Las conexiones nuevas van con normalidad.

use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpListener, TcpStream},
};

/// Escucha que numera cada conexión con la «época» en que se aceptó.
pub(crate) struct Escucha {
    pub(crate) tcp: TcpListener,
    /// Época actual: [`cortar`] la sube.
    pub(crate) epoca: Arc<AtomicU64>,
}

/// Corta en silencio las conexiones abiertas hasta ahora.
pub(crate) fn cortar(epoca: &AtomicU64) {
    epoca.fetch_add(1, Ordering::SeqCst);
}

impl axum::serve::Listener for Escucha {
    type Io = Conexion;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Conexion, SocketAddr) {
        loop {
            match self.tcp.accept().await {
                Ok((tcp, addr)) => {
                    let conexion = Conexion {
                        tcp,
                        desde: self.epoca.load(Ordering::SeqCst),
                        epoca: self.epoca.clone(),
                    };
                    return (conexion, addr);
                }
                // Como `axum::serve` con un `TcpListener`: un error al
                // aceptar (demasiados ficheros abiertos…) no para el servidor.
                Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        }
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.tcp.local_addr()
    }
}

/// Una conexión aceptada. Si la época subió desde que se aceptó, no lee
/// nada más (queda pendiente para siempre, sin cerrarse): lo que el cliente
/// mande por ella no tiene respuesta. Lo que ya se estaba respondiendo
/// termina de escribirse.
pub(crate) struct Conexion {
    tcp: TcpStream,
    desde: u64,
    epoca: Arc<AtomicU64>,
}

impl Conexion {
    fn cortada(&self) -> bool {
        self.epoca.load(Ordering::SeqCst) > self.desde
    }
}

impl AsyncRead for Conexion {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.cortada() {
            return Poll::Pending;
        }
        Pin::new(&mut self.tcp).poll_read(cx, buf)
    }
}

impl AsyncWrite for Conexion {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.tcp).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.tcp).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.tcp).poll_shutdown(cx)
    }
}
