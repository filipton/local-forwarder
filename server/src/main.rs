use color_eyre::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

mod channeled_channel;

type ConnectorChannel = (async_channel::Sender<u16>, async_channel::Receiver<u16>);

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let channels = channeled_channel::ChanneledChannel::<TcpStream>::new();
    let connector_channel = async_channel::unbounded::<u16>();

    spawn_connector_worker(connector_channel.clone()).await?;
    spawn_proxy_connector_worker(channels.clone()).await?;
    spawn_proxy_worker(channels.clone(), connector_channel.clone(), 80).await?;

    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn spawn_proxy_worker(
    channels: channeled_channel::ChanneledChannel<TcpStream>,
    connector_channel: ConnectorChannel,
    port: u16,
) -> Result<()> {
    println!("Spawning proxy worker on port {}", port);
    channels.create_channel(port).await?;

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

        let connector_channel = connector_channel.clone();
        let port = port.to_owned();
        let channel = channel.clone();

        tokio::spawn(async move {
            connector_channel.0.send(port).await?;

            let mut proxy_socket = channel.recv().await?;
            tokio::io::copy_bidirectional(&mut socket, &mut proxy_socket).await?;

            Ok::<(), color_eyre::Report>(())
        });
    }
}

async fn spawn_connector_worker(channel: ConnectorChannel) -> Result<()> {
    tokio::spawn(async move {
        loop {
            if let Err(e) = connector_worker(&channel).await {
                eprintln!("Connection worker error: {:?}", e);
            }
        }
    });

    Ok(())
}

async fn connector_worker(channel: &ConnectorChannel) -> Result<()> {
    let listener = TcpListener::bind("0.0.0.0:1337").await?;
    let mut last_task: Option<JoinHandle<()>> = None;

    loop {
        let (mut socket, _) = listener.accept().await?;
        socket.set_nodelay(true)?;

        if let Some(task) = last_task.take() {
            task.abort();
        }

        let channel = channel.clone();
        last_task = Some(tokio::spawn(async move {
            while let Ok(port) = channel.1.recv().await {
                if let Err(e) = socket.write_u16(port).await {
                    eprintln!("Failed to write to socket {:?}", e);
                    let _ = channel.0.send(port);

                    break;
                }
            }
        }));
    }
}

async fn spawn_proxy_connector_worker(
    channels: channeled_channel::ChanneledChannel<TcpStream>,
) -> Result<()> {
    tokio::spawn(async move {
        loop {
            if let Err(e) = proxy_connector_worker(&channels).await {
                eprintln!("Proxy connector worker error: {:?}", e);
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
