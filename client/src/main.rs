use color_eyre::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const CONNECTOR_PORT: u16 = 1337;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let connector_ip = std::env::args()
        .nth(1)
        .ok_or_else(|| color_eyre::eyre::eyre!("Connector address not provided (first arg)"))?;
    spawn_connector_worker(connector_ip).await?;

    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn spawn_connector_worker(connector_ip: String) -> Result<()> {
    tokio::spawn(async move {
        loop {
            if let Err(e) = connector_worker(connector_ip.clone()).await {
                eprintln!("Error in connector worker: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    });

    Ok(())
}

async fn connector_worker(connector_ip: String) -> Result<()> {
    let mut stream = tokio::net::TcpStream::connect((connector_ip.clone(), CONNECTOR_PORT)).await?;
    stream.set_nodelay(true)?;
    stream.write_u16(0).await?;
    stream.flush().await?;

    loop {
        let connector_ip = connector_ip.clone();
        let port = stream.read_u16().await?;

        tokio::spawn(async move {
            let mut proxy_stream =
                tokio::net::TcpStream::connect((connector_ip, CONNECTOR_PORT)).await?;
            let mut proxy_conn = tokio::net::TcpStream::connect(("localhost", port)).await?;

            proxy_stream.set_nodelay(true)?;
            proxy_stream.write_u16(port).await?;
            proxy_stream.flush().await?;

            tokio::io::copy_bidirectional(&mut proxy_stream, &mut proxy_conn).await?;
            Ok::<_, color_eyre::Report>(())
        });
    }
}
