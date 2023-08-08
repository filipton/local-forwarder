use crate::{channeled_channel, ConnectorChannel};
use color_eyre::Result;
use tokio::net::{TcpListener, TcpStream};

pub async fn spawn_proxy_worker(
    channels: channeled_channel::ChanneledChannel<TcpStream>,
    connector_channel: ConnectorChannel,
    port: u16,
) -> Result<()> {
    println!("Spawning proxy worker on port {}", port);
    channels.create_channel(&port).await?;

    tokio::spawn(async move {
        loop {
            if let Err(e) = proxy_worker(&channels, &connector_channel, &port).await {
                eprintln!("Proxy worker error: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    });

    Ok(())
}

async fn proxy_worker(
    channels: &channeled_channel::ChanneledChannel<TcpStream>,
    connector_channel: &ConnectorChannel,
    port: &u16,
) -> Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port.to_owned())).await?;
    let channel = channels.get_receiver(&port).await.unwrap();

    loop {
        let (mut socket, _) = listener.accept().await?;
        socket.set_nodelay(true)?;

        connector_channel.0.send(*port).await?;
        let channel = channel.clone();

        tokio::spawn(async move {
            let mut proxy_socket = channel.recv().await?;

            tokio::io::copy_bidirectional(&mut socket, &mut proxy_socket).await?;
            Ok::<(), color_eyre::Report>(())
        });
    }
}
