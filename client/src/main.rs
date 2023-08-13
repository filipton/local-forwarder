use color_eyre::Result;
use structs::{Config, ConvertedConfig};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UdpSocket,
};
use udpflow::UdpStreamRemote;
use utils::{MultiStream, PortType};

mod structs;

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

async fn proxy_tcp(tunnel: MultiStream, ip: &str, local_port: u16) -> Result<()> {
    let local = tokio::net::TcpStream::connect((ip, local_port)).await?;
    local.set_nodelay(true)?;

    tunnel.copy_bidirectional(local).await?;
    Ok(())
}

async fn proxy_udp(tunnel: MultiStream, ip: &str, local_port: u16) -> Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    let local = UdpStreamRemote::new(socket, format!("{}:{}", ip, local_port).parse()?);

    tunnel.copy_bidirectional(local).await?;
    Ok(())
}
