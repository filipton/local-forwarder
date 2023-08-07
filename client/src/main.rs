use color_eyre::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let connector_addr = std::env::args()
        .nth(1)
        .ok_or_else(|| color_eyre::eyre::eyre!("Connector address not provided (first arg)"))?;
    spawn_connector_worker(connector_addr).await?;

    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn spawn_connector_worker(connector_addr: String) -> Result<()> {
    tokio::spawn(async move {
        loop {
            if let Err(e) = connector_worker(connector_addr.clone()).await {
                eprintln!("Error in connector worker: {}", e);
            }
        }
    });

    Ok(())
}

async fn connector_worker(connector_addr: String) -> Result<()> {
    let mut stream = tokio::net::TcpStream::connect(connector_addr).await?;
    stream.set_nodelay(true)?;
    loop {
        let port = stream.read_u16().await?;

        tokio::spawn(async move {
            println!("Connecting to port {}", port);

            let mut proxy_stream = tokio::net::TcpStream::connect("0.0.0.0:1338").await?;
            let mut proxy_conn = tokio::net::TcpStream::connect(("localhost", 80)).await?; // CHANGE
                                                                                           // THIS
                                                                                           // 80 TO
                                                                                           //    PORT

            proxy_stream.set_nodelay(true)?;
            proxy_stream.write_u16(port).await?;
            proxy_stream.flush().await?;

            println!("Connected to port {}", port);

            tokio::io::copy_bidirectional(&mut proxy_stream, &mut proxy_conn).await?;

            Ok::<_, color_eyre::Report>(())
        });
    }
}
