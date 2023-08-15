use color_eyre::Result;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};
use udpflow::{UdpSocket, UdpStreamLocal, UdpStreamRemote};

pub const BUFFER_SIZE: usize = 65536;

impl ConnectorInfo {
    pub fn encode(&self) -> Result<Vec<u8>> {
        println!("ConnectorInfo: {:?}", self);
        println!("ConnectorInfo: {:?}", serde_json::to_vec(self)?);
        println!("ConnectorInfo: {:?}", serde_json::to_string(self)?);
        Ok(serde_json::to_vec(self)?)
    }

    pub fn decode(data: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(data)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorInfo {
    pub ports: Vec<ConnectorPort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorPort {
    pub port_remote: u16,
    pub port_local: u16,
    pub local_ip: String,

    pub port_type: PortType,
    pub tunnel_type: PortType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(dead_code)]
pub enum PortType {
    Tcp,
    Udp,
}

pub enum MultiStream {
    Tcp(TcpStream),
    UdpLocal(UdpStreamLocal),
    UdpRemote(UdpStreamRemote),
}

impl MultiStream {
    pub async fn connect_and_setup(
        connector_ip: &str,
        connector_port: u16,
        port_type: PortType,
        port: u16,
        code: u64,
    ) -> Result<Self> {
        let mut bytes: Vec<u8> = vec![];
        bytes.write_u16(port).await?;
        bytes.write_u64(code).await?;

        match port_type {
            PortType::Tcp => {
                let mut stream =
                    TcpStream::connect(format!("{}:{}", connector_ip, connector_port)).await?;

                stream.set_nodelay(true)?;
                stream.write_all(&bytes).await?;
                stream.flush().await?;

                Ok(MultiStream::Tcp(stream))
            }
            PortType::Udp => {
                let socket = UdpSocket::bind("0.0.0.0:0").await?;
                let mut stream = UdpStreamRemote::new(
                    socket,
                    format!("{}:{}", connector_ip, connector_port).parse()?,
                );
                stream.write_all(&bytes).await?;

                Ok(MultiStream::UdpRemote(stream))
            }
        }
    }

    pub async fn copy_bidirectional<T>(self, s2: T) -> Result<()>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        match self {
            MultiStream::Tcp(s) => Self::inner_copy_bidirectional(s, s2).await?,
            MultiStream::UdpLocal(s) => Self::inner_copy_bidirectional(s, s2).await?,
            MultiStream::UdpRemote(s) => Self::inner_copy_bidirectional(s, s2).await?,
        }

        Ok(())
    }

    async fn inner_copy_bidirectional<T1, T2>(mut stream1: T1, mut stream2: T2) -> Result<()>
    where
        T1: AsyncRead + AsyncWrite + Unpin,
        T2: AsyncRead + AsyncWrite + Unpin,
    {
        let local_buf = &mut [0u8; BUFFER_SIZE];
        let remote_buf = &mut [0u8; BUFFER_SIZE];

        loop {
            tokio::select! {
                res = stream1.read(&mut local_buf[..])=> {
                    if res.is_err() {
                        stream1.shutdown().await?;
                        stream2.shutdown().await?;
                        break;
                    }

                    let n = res?;
                    if n == 0 {
                        break;
                    }

                    stream2.write_all(&local_buf[..n]).await?;
                }
                res = stream2.read(&mut remote_buf[..]) => {
                    if res.is_err() {
                        stream1.shutdown().await?;
                        stream2.shutdown().await?;
                        break;
                    }

                    let n = res?;
                    if n == 0 {
                        break;
                    }

                    stream1.write_all(&remote_buf[..n]).await?;
                }
            }
        }

        Ok(())
    }
}
