use color_eyre::Result;
use std::time::Duration;
use structs::{Config, ConvertedConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use udp_stream::UdpStream;
use utils::{MultiStream, PortType};

mod structs;

const BUFFER_SIZE: usize = 65536;
const UDP_TIMEOUT: u64 = 10 * 1000;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let config = Config::load().await?.convert()?;
    spawn_connector_worker(config).await?;

    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn spawn_connector_worker(config: ConvertedConfig) -> Result<()> {
    tokio::spawn(async move {
        loop {
            if let Err(e) = connector_worker(&config).await {
                eprintln!("Error in connector worker: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    });

    Ok(())
}

async fn connector_worker(config: &ConvertedConfig) -> Result<()> {
    let mut stream = tokio::net::TcpStream::connect((
        config.connector_ip.clone(),
        config.connector_port.clone(),
    ))
    .await?;
    stream.set_nodelay(true)?;

    let mut bytes = vec![];
    bytes.write_u16(0).await?;
    bytes.write_u128(config.code).await?;
    stream.write_all(&bytes).await?;

    let encoded_data = config.connector.encode()?;
    stream.write_u16(encoded_data.len() as u16).await?;
    stream.write_all(&encoded_data).await?;
    stream.flush().await?;

    loop {
        let config = config.clone();

        let port = stream.read_u16().await?;
        let local_port = match config
            .connector
            .ports
            .iter()
            .find(|p| p.port_remote == port)
            .cloned()
        {
            Some(p) => p,
            None => {
                eprintln!("Unknown port: {}", port);
                continue;
            }
        };

        tokio::spawn(async move {
            let tunnel = MultiStream::connect_and_setup(
                &config.connector_ip,
                config.connector_port,
                local_port.tunnel_type,
                port,
                config.code,
            )
            .await?;

            if local_port.port_type == PortType::Tcp {
                proxy_tcp(tunnel, &local_port.local_ip, local_port.port_local).await?;
            } else if local_port.port_type == PortType::Udp {
                proxy_udp(tunnel, &local_port.local_ip, local_port.port_local).await?;
            }

            Ok::<_, color_eyre::Report>(())
        });
    }
}

async fn proxy_tcp(mut tunnel: MultiStream, ip: &str, local_port: u16) -> Result<()> {
    let mut local = tokio::net::TcpStream::connect((ip, local_port)).await?;
    local.set_nodelay(true)?;

    tokio::io::copy_bidirectional(&mut tunnel, &mut local).await?;
    Ok(())
}

async fn proxy_udp(mut tunnel: MultiStream, ip: &str, local_port: u16) -> Result<()> {
    let mut local = UdpStream::connect(format!("{}:{}", ip, local_port).parse()?).await?;

    let mut local_buf = vec![0u8; BUFFER_SIZE];
    let mut remote_buf = vec![0u8; BUFFER_SIZE];

    let timeout = Duration::from_millis(UDP_TIMEOUT);
    loop {
        tokio::select! {
            res = tokio::time::timeout(timeout, tunnel.read(&mut local_buf))=> {
                if res.is_err() {
                    local.shutdown();
                    tunnel.shutdown().await?;
                    break;
                }

                let n = res??;
                local.write_all(&local_buf[..n]).await?;
            }
            res = tokio::time::timeout(timeout, local.read(&mut remote_buf)) => {
                if res.is_err() {
                    local.shutdown();
                    tunnel.shutdown().await?;
                    break;
                }

                let n = res??;
                tunnel.write_all(&remote_buf[..n]).await?;
            }
        }
    }
    Ok(())
}
