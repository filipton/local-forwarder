use bincode::{Decode, Encode};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use udp_stream::UdpStream;

#[derive(Debug, Clone, Encode, Decode)]
pub struct ConnectorInfo {
    pub ports: Vec<ConnectorPort>,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ConnectorPort {
    pub port_remote: u16,
    pub port_local: u16,
    pub local_ip: String,
    pub port_type: PortType,
}

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
#[allow(dead_code)]
pub enum PortType {
    Tcp,
    Udp,
}

pub enum MultiStream {
    Tcp(TcpStream),
    Udp(UdpStream),
}

impl AsyncWrite for MultiStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.get_mut() {
            MultiStream::Tcp(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
            MultiStream::Udp(stream) => std::pin::Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            MultiStream::Tcp(stream) => std::pin::Pin::new(stream).poll_flush(cx),
            MultiStream::Udp(stream) => std::pin::Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            MultiStream::Tcp(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
            MultiStream::Udp(stream) => std::pin::Pin::new(stream).poll_shutdown(cx),
        }
    }
}

impl AsyncRead for MultiStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            MultiStream::Tcp(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
            MultiStream::Udp(stream) => std::pin::Pin::new(stream).poll_read(cx, buf),
        }
    }
}
