use bincode::{Decode, Encode};
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};
use udp_stream::UdpStream;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub connector: String,
    pub code: u128,

    pub ports: Vec<ConfigPort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPort {
    pub remote: u16,
    pub local: u16,
    pub ip: Option<String>,

    #[serde(rename = "type")]
    pub _type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConvertedConfig {
    pub connector: ConnectorInfo,
    pub code: u128,
    pub connector_ip: String,
    pub connector_port: u16,
}

impl Config {
    pub async fn load() -> Result<Self> {
        let config_res = tokio::fs::read_to_string("config.json").await;
        match config_res {
            Ok(config) => {
                let config: Config = serde_json::from_str(&config)?;
                return Ok(config);
            }
            Err(_) => {
                let config = Config {
                    connector: String::from("server:1337"),
                    code: 123213123123123,
                    ports: vec![],
                };

                let config_str = serde_json::to_string_pretty(&config)?;
                tokio::fs::write("config.json", config_str).await?;

                return Ok(config);
            }
        }
    }

    pub fn convert(&self) -> Result<ConvertedConfig> {
        let mut connector_ports: Vec<ConnectorPort> = Vec::new();
        for port in self.ports.iter() {
            let _type = match port._type.as_ref().unwrap().to_uppercase().as_str() {
                "TCP" => PortType::Tcp,
                "UDP" => PortType::Udp,
                _ => {
                    return Err(color_eyre::eyre::eyre!(
                        "Invalid port type: {}",
                        port._type.as_ref().unwrap()
                    ))
                }
            };

            connector_ports.push(ConnectorPort {
                port_remote: port.remote,
                port_local: port.local,
                local_ip: port.ip.clone().unwrap_or(String::from("127.0.0.1")),
                port_type: _type,
            });
        }

        let connector_splitted = self.connector.split(":").collect::<Vec<&str>>();
        let connector_ip = connector_splitted[0].to_string();
        let connector_port = connector_splitted
            .get(1)
            .unwrap_or(&"1337")
            .parse::<u16>()?;

        let converted_config = ConvertedConfig {
            connector: ConnectorInfo {
                ports: connector_ports,
            },
            code: self.code,
            connector_ip,
            connector_port,
        };

        Ok(converted_config)
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
