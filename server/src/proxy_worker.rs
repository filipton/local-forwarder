use std::sync::Arc;

use crate::{channeled_channel, ConnectorChannel};
use color_eyre::Result;
use lazy_static::lazy_static;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::RwLock,
    task::JoinHandle,
};

lazy_static! {
    pub static ref PROXY_TASKS: Arc<RwLock<Vec<JoinHandle<()>>>> =
        Arc::new(RwLock::new(Vec::new()));
}

pub async fn spawn_multiple_proxy_workers(
    channels: channeled_channel::ChanneledChannel<TcpStream>,
    connector_channel: ConnectorChannel,
    ports: Vec<u16>,
) -> Result<()> {
    for task in PROXY_TASKS.write().await.drain(..) {
        task.abort();
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    for port in ports {
        spawn_proxy_worker(channels.clone(), connector_channel.clone(), port).await?;
    }

    Ok(())
}

pub async fn spawn_proxy_worker(
    channels: channeled_channel::ChanneledChannel<TcpStream>,
    connector_channel: ConnectorChannel,
    port: u16,
) -> Result<()> {
    println!("Spawning proxy worker on port {}", port);
    channels.create_channel(&port).await?;

    let task = tokio::spawn(async move {
        loop {
            if let Err(e) = proxy_worker(&channels, &connector_channel, &port).await {
                eprintln!("Proxy worker error: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    });

    PROXY_TASKS.write().await.push(task);
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
            tokio::select! {
                Ok(mut proxy_socket) = channel.recv() => {
                    tokio::io::copy_bidirectional(&mut socket, &mut proxy_socket).await?;
                },
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                    eprintln!("Proxy worker timed out");
                }
            }

            Ok::<(), color_eyre::Report>(())
        });
    }
}
