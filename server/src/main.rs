use color_eyre::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

mod channeled_channel;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let channels = channeled_channel::ChanneledChannel::<TcpStream>::new();

    spawn_connector_worker(channels.clone()).await?;

    Ok(())
}

async fn spawn_connector_worker(
    channels: channeled_channel::ChanneledChannel<TcpStream>,
) -> Result<()> {
    tokio::spawn(async move {
        loop {
            if let Err(e) = connector_worker().await {
                eprintln!("connector_worker error: {:?}", e);
            }
        }
    });

    Ok(())
}

async fn connector_worker() -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:1337").await?;
    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 1024];
            loop {
                let n = socket.read(&mut buf).await.unwrap();
                if n == 0 {
                    return;
                }
                socket.write_all(&buf[0..n]).await.unwrap();
            }
        });
    }
}
