use std::io::Write;

use color_eyre::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

mod channeled_channel;

type ConnectorChannel = (
    crossbeam_channel::Sender<u16>,
    crossbeam_channel::Receiver<u16>,
);

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let channels = channeled_channel::ChanneledChannel::<TcpStream>::new();
    let connector_channel = crossbeam_channel::unbounded::<u16>();

    spawn_connector_worker(connector_channel.1.clone())?;
    spawn_connector_worker2(channels.clone()).await?;
    spawn_proxy_worker(&channels, connector_channel.clone(), 8070).await?;

    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?
        .recv()
        .await;
    Ok(())
}

async fn spawn_proxy_worker(
    channels: &channeled_channel::ChanneledChannel<TcpStream>,
    connector_channel: ConnectorChannel,
    port: u16,
) -> Result<()> {
    println!("Spawning proxy worker on port {}", port);
    channels.create_channel(port).await?;
    let rx = channels.get_receiver(&port).await.unwrap();

    tokio::spawn(async move {
        loop {
            if let Err(e) = proxy_worker(rx.clone(), &connector_channel, &port).await {
                eprintln!("proxy_worker error: {:?}", e);
            }
        }
    });

    Ok(())
}

async fn proxy_worker(
    channel: crossbeam_channel::Receiver<TcpStream>,
    connector_channel: &ConnectorChannel,
    port: &u16,
) -> Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port.to_owned())).await?;

    loop {
        let (mut socket, _) = listener.accept().await?;
        socket.set_nodelay(true)?;

        let connector_channel = connector_channel.clone();
        let port = port.to_owned();
        let channel = channel.clone();

        tokio::spawn(async move {
            connector_channel.0.send(port)?;

            loop {
                if let Ok(mut proxy_socket) = channel.try_recv() {
                    tokio::io::copy_bidirectional(&mut socket, &mut proxy_socket).await?;
                    return Ok::<(), color_eyre::Report>(());
                }

                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
        });
    }
}

fn spawn_connector_worker(channel: crossbeam_channel::Receiver<u16>) -> Result<()> {
    std::thread::spawn(move || loop {
        if let Err(e) = connector_worker(&channel) {
            eprintln!("connector_worker error: {:?}", e);
        }
    });

    Ok(())
}

fn connector_worker(channel: &crossbeam_channel::Receiver<u16>) -> Result<()> {
    let listener = std::net::TcpListener::bind("0.0.0.0:1337")?;
    loop {
        let (mut socket, _) = listener.accept()?;
        socket.set_nodelay(true)?;

        while let Ok(port) = channel.recv() {
            if let Err(e) = socket.write_all(&port.to_be_bytes()) {
                eprintln!("Failed to write to socket {:?}", e);

                break;
            }
        }
    }
}

async fn spawn_connector_worker2(
    channels: channeled_channel::ChanneledChannel<TcpStream>,
) -> Result<()> {
    tokio::spawn(async move {
        loop {
            if let Err(e) = connector_worker2(&channels).await {
                eprintln!("connector_worker error: {:?}", e);
            }
        }
    });

    Ok(())
}

async fn connector_worker2(
    channels: &channeled_channel::ChanneledChannel<TcpStream>,
) -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:1338").await?;
    loop {
        let (mut socket, _) = listener.accept().await?;
        socket.set_nodelay(true)?;

        let channels = channels.clone();
        tokio::spawn(async move {
            let port = socket.read_u16().await?;
            println!("Waiting for port");
            channels.get_sender(&port).await.unwrap().send(socket)?;
            println!("Waiting for port2");

            Ok::<(), color_eyre::Report>(())
        });
    }
}
