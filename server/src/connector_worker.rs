use crate::{
    channeled_channel,
    tunnel::{self, BUFFER_SIZE},
    ConnectorChannel,
};
use color_eyre::Result;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::RwLock,
    task::JoinHandle,
};
use udpflow::{UdpListener, UdpSocket};
use utils::{ConnectorInfo, MultiStream};

pub async fn spawn_connector_worker(
    connector_channel: ConnectorChannel,
    tunnel_channels: channeled_channel::ChanneledChannel<MultiStream>,
    connector_code: u128,
) -> Result<()> {
    let tunnel_channels_cp = tunnel_channels.clone();
    let connector_code_cp = connector_code.clone();

    tokio::spawn(async move {
        loop {
            if let Err(e) =
                connector_worker(&connector_channel, &tunnel_channels, &connector_code).await
            {
                eprintln!("Connection worker error: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    });

    tokio::spawn(async move {
        loop {
            if let Err(e) = connector_worker_udp(&tunnel_channels_cp, &connector_code_cp).await {
                eprintln!("Connection listener error: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    });

    Ok(())
}

async fn connector_worker(
    connector_channel: &ConnectorChannel,
    tunnel_channels: &channeled_channel::ChanneledChannel<MultiStream>,
    connector_code: &u128,
) -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:1337").await?;
    let connector_task: Arc<RwLock<Option<JoinHandle<()>>>> = Arc::new(RwLock::new(None));

    loop {
        let (mut socket, _) = listener.accept().await?;
        socket.set_nodelay(true)?;

        let tunnel_channels = tunnel_channels.clone();
        let connector_task = connector_task.clone();
        let connector_channel = connector_channel.clone();
        let connector_code = connector_code.clone();

        tokio::spawn(async move {
            let mut buf = [0; 18];
            socket.read_exact(&mut buf).await?;

            let port = u16::from_be_bytes(buf[0..2].try_into()?);
            let code = u128::from_be_bytes(buf[2..18].try_into()?);
            if code != connector_code {
                return Ok(());
            }

            if port == 0 {
                if let Some(task) = connector_task.read().await.as_ref() {
                    task.abort();
                }

                let info_len = socket.read_u16().await?;
                let mut info = vec![0; info_len as usize];
                socket.read_exact(&mut info).await?;

                let info: ConnectorInfo = ConnectorInfo::decode(&info)?;
                tunnel::spawn_multiple_tunnels(
                    tunnel_channels.clone(),
                    connector_channel.clone(),
                    info.ports,
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
                tunnel_channels
                    .get_sender(&port)
                    .await
                    .ok_or_else(|| {
                        color_eyre::eyre::eyre!("Could not get sender for port {}", port)
                    })?
                    .send(MultiStream::Tcp(socket))
                    .await?;
            }

            Ok::<(), color_eyre::Report>(())
        });
    }
}

async fn connector_worker_udp(
    tunnel_channels: &channeled_channel::ChanneledChannel<MultiStream>,
    connector_code: &u128,
) -> Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:1337").await?;
    let listener = UdpListener::new(socket);

    let buf = &mut [0; BUFFER_SIZE];
    loop {
        let (mut socket, _) = listener.accept(&mut buf[..]).await?;

        let tunnel_channels = tunnel_channels.clone();
        let connector_code = connector_code.clone();

        tokio::spawn(async move {
            let mut buf = [0; 18];
            socket.read_exact(&mut buf).await?;

            let port = u16::from_be_bytes(buf[0..2].try_into()?);
            let code = u128::from_be_bytes(buf[2..18].try_into()?);
            if code != connector_code {
                return Ok(());
            }

            tunnel_channels
                .get_sender(&port)
                .await
                .ok_or_else(|| color_eyre::eyre::eyre!("Could not get sender for port {}", port))?
                .send(MultiStream::UdpLocal(socket))
                .await?;

            Ok::<(), color_eyre::Report>(())
        });
    }
}
