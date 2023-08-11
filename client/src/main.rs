use color_eyre::Result;
use std::{time::Duration, u128};
use structs::{ConnectorInfo, ConnectorPort, PortType};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use udp_stream::UdpStream;

mod structs;

const CONNECTOR_PORT: u16 = 1337;

const UDP_BUFFER_SIZE: usize = 65536;
const UDP_TIMEOUT: u64 = 10 * 1000;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let connector_ip = std::env::args()
        .nth(1)
        .ok_or_else(|| color_eyre::eyre::eyre!("Connector address not provided (first arg)"))?;
    let connector_code = u128::from_str_radix(
        &std::env::args()
            .nth(2)
            .ok_or_else(|| color_eyre::eyre::eyre!("Connector code not provided (second arg)"))?,
        10,
    )?;

    spawn_connector_worker(connector_ip, connector_code).await?;

    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn spawn_connector_worker(connector_ip: String, connector_code: u128) -> Result<()> {
    tokio::spawn(async move {
        loop {
            if let Err(e) = connector_worker(&connector_ip, &connector_code).await {
                eprintln!("Error in connector worker: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    });

    Ok(())
}

async fn connector_worker(connector_ip: &String, connector_code: &u128) -> Result<()> {
    let mut stream = tokio::net::TcpStream::connect((connector_ip.clone(), CONNECTOR_PORT)).await?;
    stream.set_nodelay(true)?;
    stream.write_u16(0).await?;
    stream.write_u128(*connector_code).await?;

    let info = ConnectorInfo {
        ports: vec![
            ConnectorPort {
                port_worker: 80,
                port_local: 80,
                port_type: PortType::Tcp,
            },
            ConnectorPort {
                port_worker: 81,
                port_local: 5173,
                port_type: PortType::Tcp,
            },
            ConnectorPort {
                port_worker: 82,
                port_local: 8888,
                port_type: PortType::Udp,
            },
        ],
    };
    let encoded_data = bincode::encode_to_vec(&info, bincode::config::standard())?;

    stream.write_u16(encoded_data.len() as u16).await?;
    stream.write_all(&encoded_data).await?;
    stream.flush().await?;

    loop {
        let connector_ip = connector_ip.clone();
        let port = stream.read_u16().await?;
        let connector_code = connector_code.clone();

        let local_port = match info.ports.iter().find(|p| p.port_worker == port).cloned() {
            Some(p) => p,
            None => {
                eprintln!("Unknown port: {}", port);
                continue;
            }
        };

        tokio::spawn(async move {
            let mut proxy_stream =
                tokio::net::TcpStream::connect((connector_ip, CONNECTOR_PORT)).await?;

            proxy_stream.set_nodelay(true)?;
            proxy_stream.write_u16(port).await?;
            proxy_stream.write_u128(connector_code).await?;
            proxy_stream.flush().await?;

            if local_port.port_type == PortType::Tcp {
                proxy_tcp(proxy_stream, "127.0.0.1", local_port.port_local).await?;
            } else if local_port.port_type == PortType::Udp {
                proxy_udp(proxy_stream, "127.0.0.1", local_port.port_local).await?;
            }

            Ok::<_, color_eyre::Report>(())
        });
    }
}

async fn proxy_tcp(mut proxy_stream: TcpStream, ip: &str, local_port: u16) -> Result<()> {
    let mut remote = tokio::net::TcpStream::connect((ip, local_port)).await?;

    tokio::io::copy_bidirectional(&mut proxy_stream, &mut remote).await?;
    Ok(())
}

async fn proxy_udp(mut proxy_stream: TcpStream, ip: &str, local_port: u16) -> Result<()> {
    let mut remote = UdpStream::connect(format!("{}:{}", ip, local_port).parse()?).await?;

    let mut local_buf = vec![0u8; UDP_BUFFER_SIZE];
    let mut remote_buf = vec![0u8; UDP_BUFFER_SIZE];

    let timeout = Duration::from_millis(UDP_TIMEOUT);
    loop {
        tokio::select! {
            res = tokio::time::timeout(timeout, proxy_stream.read(&mut local_buf))=> {
                if res.is_err() {
                    remote.shutdown();
                    proxy_stream.shutdown().await?;
                    break;
                }

                let n = res??;
                remote.write_all(&local_buf[..n]).await?;
            }
            res = tokio::time::timeout(timeout, remote.read(&mut remote_buf)) => {
                if res.is_err() {
                    remote.shutdown();
                    proxy_stream.shutdown().await?;
                    break;
                }

                let n = res??;
                proxy_stream.write_all(&remote_buf[..n]).await?;
            }
        }
    }
    Ok(())
}
