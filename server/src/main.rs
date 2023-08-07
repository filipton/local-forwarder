use color_eyre::Result;

mod channeled_channel;
mod connector_worker;
mod proxy_worker;

pub type ConnectorChannel = (async_channel::Sender<u16>, async_channel::Receiver<u16>);

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let channels = channeled_channel::ChanneledChannel::new();
    let connector_channel = async_channel::unbounded::<u16>();

    connector_worker::spawn_connector_worker(connector_channel.clone(), channels.clone()).await?;
    proxy_worker::spawn_proxy_worker(channels.clone(), connector_channel.clone(), 80).await?;

    tokio::signal::ctrl_c().await?;
    Ok(())
}
