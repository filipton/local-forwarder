use std::sync::{atomic::AtomicBool, Arc};

use color_eyre::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::{channeled_channel, ConnectorChannel};

pub async fn spawn_connector_worker(
    channel: ConnectorChannel,
    channels: channeled_channel::ChanneledChannel<TcpStream>,
) -> Result<()> {
    tokio::spawn(async move {
        loop {
            if let Err(e) = connector_worker(&channel, &channels).await {
                eprintln!("Connection worker error: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    });

    Ok(())
}

async fn connector_worker(
    channel: &ConnectorChannel,
    channels: &channeled_channel::ChanneledChannel<TcpStream>,
) -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:1337").await?;
    let proxy_connected = Arc::new(AtomicBool::new(false));

    loop {
        let (mut socket, _) = listener.accept().await?;
        socket.set_nodelay(true)?;

        let channel = channel.clone();
        let channels = channels.clone();
        let proxy_connected = proxy_connected.clone();
        tokio::spawn(async move {
            let port = socket.read_u16().await.unwrap();
            if port == 0 {
                if proxy_connected.load(std::sync::atomic::Ordering::Relaxed) {
                    return Ok(());
                }
                proxy_connected.store(true, std::sync::atomic::Ordering::Relaxed);

                while let Ok(port) = channel.1.recv().await {
                    if let Err(e) = socket.write_u16(port).await {
                        eprintln!("Failed to write to socket {:?}", e);
                        let _ = channel.0.send(port);

                        proxy_connected.store(false, std::sync::atomic::Ordering::Relaxed);
                        return Ok(());
                    }
                }
            } else {
                channels
                    .get_sender(&port)
                    .await
                    .unwrap()
                    .send(socket)
                    .await?;
            }

            Ok::<(), color_eyre::Report>(())
        });
    }
}
