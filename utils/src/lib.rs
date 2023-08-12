use bincode::{Decode, Encode};
use color_eyre::Result;
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};
use udp_stream::UdpStream;

impl ConnectorInfo {
    pub fn encode(&self) -> Result<Vec<u8>> {
        Ok(bincode::encode_to_vec(self, bincode::config::standard())?)
    }

    pub fn decode(data: &[u8]) -> Result<Self> {
        Ok(bincode::decode_from_slice(data, bincode::config::standard())?.0)
    }
}

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

impl MultiStream {
    pub async fn connect_and_setup(
        connector_ip: &str,
        connector_port: u16,
        port_type: PortType,
        port: u16,
        code: u128,
    ) -> Result<Self> {
        match port_type {
            PortType::Tcp => {
                let mut stream =
                    TcpStream::connect(format!("{}:{}", connector_ip, connector_port)).await?;
                stream.set_nodelay(true)?;
                stream.write_u16(port).await?;
                stream.write_u128(code).await?;
                stream.flush().await?;

                Ok(MultiStream::Tcp(stream))
            }
            PortType::Udp => {
                let mut stream =
                    UdpStream::connect(format!("{}:{}", connector_ip, connector_port).parse()?)
                        .await?;
                stream.write_u16(port).await?;
                stream.write_u128(code).await?;
                stream.flush().await?;

                Ok(MultiStream::Udp(stream))
            }
        }
    }
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
