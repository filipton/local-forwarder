use crate::channeled_channel;
use color_eyre::Result;
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
};

pub async fn spawn_proxy_connector_worker(
    channels: channeled_channel::ChanneledChannel<TcpStream>,
) -> Result<()> {
    tokio::spawn(async move {
        loop {
            if let Err(e) = proxy_connector_worker(&channels).await {
                eprintln!("Proxy connector worker error: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    });

    Ok(())
}

async fn proxy_connector_worker(
    channels: &channeled_channel::ChanneledChannel<TcpStream>,
) -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:1338").await?;
    loop {
        let (mut socket, _) = listener.accept().await?;
        socket.set_nodelay(true)?;

        let channels = channels.clone();
        tokio::spawn(async move {
            let port = socket.read_u16().await?;
            channels
                .get_sender(&port)
                .await
                .unwrap()
                .send(socket)
                .await?;

            Ok::<(), color_eyre::Report>(())
        });
    }
}
