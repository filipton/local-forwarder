use std::{sync::Arc, time::Duration};

use crate::{
    channeled_channel,
    connector_worker::{ConnectorPort, PortType},
    ConnectorChannel,
};
use color_eyre::Result;
use lazy_static::lazy_static;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::RwLock,
    task::JoinHandle,
};
use udp_stream::UdpListener;

const UDP_BUFFER_SIZE: usize = 17480; // 17kb
const UDP_TIMEOUT: u64 = 10 * 1000; // 10sec

lazy_static! {
    pub static ref PROXY_TASKS: Arc<RwLock<Vec<JoinHandle<()>>>> =
        Arc::new(RwLock::new(Vec::new()));
}

pub async fn spawn_multiple_proxy_workers(
    channels: channeled_channel::ChanneledChannel<TcpStream>,
    connector_channel: ConnectorChannel,
    ports: Vec<ConnectorPort>,
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
    port: ConnectorPort,
) -> Result<()> {
    println!(
        "Spawning proxy worker on port {} ({:?})",
        port.port_worker, port.port_type
    );
    channels.create_channel(&port.port_worker).await?;

    let task = tokio::spawn(async move {
        loop {
            let res = match port.port_type {
                PortType::Tcp => {
                    proxy_worker_tcp(&channels, &connector_channel, &port.port_worker).await
                }
                PortType::Udp => {
                    proxy_worker_udp(&channels, &connector_channel, &port.port_worker).await
                }
            };

            if let Err(e) = res {
                eprintln!("Proxy worker error: {:?}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    });

    PROXY_TASKS.write().await.push(task);
    Ok(())
}

async fn proxy_worker_tcp(
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

async fn proxy_worker_udp(
    channels: &channeled_channel::ChanneledChannel<TcpStream>,
    connector_channel: &ConnectorChannel,
    port: &u16,
) -> Result<()> {
    let timeout = Duration::from_millis(UDP_TIMEOUT);
    let listener = UdpListener::bind(format!("0.0.0.0:{}", port).parse()?).await?;

    let channel = channels.get_receiver(&port).await.unwrap();
    loop {
        let (mut stream, _) = listener.accept().await?;
        println!("UDP connection accepted {:?}", stream.peer_addr()?);

        connector_channel.0.send(*port).await?;
        let channel = channel.clone();
        let res = tokio::spawn(async move {
            tokio::select! {
                Ok(mut proxy_socket) = channel.recv() => {
                    let mut local_buf = vec![0u8; UDP_BUFFER_SIZE];
                    let mut remote_buf = vec![0u8; UDP_BUFFER_SIZE];

                    loop {
                        tokio::select! {
                            res = tokio::time::timeout(timeout, stream.read(&mut local_buf)) => {
                                if res.is_err() {
                                    stream.shutdown();
                                    proxy_socket.shutdown().await?;

                                    println!("UDP connection timed out {:?}", stream.peer_addr()?);
                                    break;
                                }
                                let n = res??;
                                println!("UDP connection read {} bytes", n);

                                proxy_socket.write_all(&local_buf[..n]).await?;
                            }
                            res = tokio::time::timeout(timeout, proxy_socket.read(&mut remote_buf)) => {
                                if res.is_err() {
                                    stream.shutdown();
                                    proxy_socket.shutdown().await?;

                                    println!("2UDP connection timed out {:?}", stream.peer_addr()?);
                                    break;
                                }
                                let n = res??;
                                if n == 0 {
                                    tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
                                }
                                //println!("2UDP connection read {} bytes", n);

                                stream.write_all(&remote_buf[..n]).await?;
                            }
                        }
                    }
                },
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                    eprintln!("Proxy worker timed out");
                }
            }

            Ok::<(), color_eyre::Report>(())
        });
    }
}
