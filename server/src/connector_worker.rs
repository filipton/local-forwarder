use color_eyre::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::RwLock,
    task::JoinHandle,
};

use crate::{channeled_channel, proxy_worker, ConnectorChannel};

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
    connector_channel: &ConnectorChannel,
    channels: &channeled_channel::ChanneledChannel<TcpStream>,
) -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:1337").await?;
    let connector_task: Arc<RwLock<Option<JoinHandle<()>>>> = Arc::new(RwLock::new(None));

    loop {
        let (mut socket, _) = listener.accept().await?;
        socket.set_nodelay(true)?;

        let channels = channels.clone();
        let connector_task = connector_task.clone();
        let connector_channel = connector_channel.clone();

        tokio::spawn(async move {
            let port = socket.read_u16().await?;
            if port == 0 {
                let code = socket.read_u128().await?;
                if code != 0 {
                    return Ok(());
                }

                if let Some(task) = connector_task.read().await.as_ref() {
                    task.abort();
                }

                let info_len = socket.read_u16().await?;
                let mut info = vec![0; info_len as usize];
                socket.read_exact(&mut info).await?;

                let info: ConnectorInfo = bincode::deserialize(&info)?;
                proxy_worker::spawn_multiple_proxy_workers(
                    channels.clone(),
                    connector_channel.clone(),
                    info.ports.iter().map(|p| p.port_worker).collect(),
                )
                .await?;

                let task = tokio::spawn(async move {
                    while let Ok(port) = connector_channel.1.recv().await {
                        if let Err(e) = socket.write_u16(port).await {
                            eprintln!("Failed to write to socket {:?}", e);
                            //let _ = channel.0.send(port);

                            return;
                        }
                    }
                });

                connector_task.write().await.replace(task);
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorInfo {
    pub ports: Vec<ConnectorPort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorPort {
    pub port_worker: u16,
    pub port_local: u16,
    pub port_type: PortType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum PortType {
    Tcp,
    Udp,
}
