use color_eyre::Result;

mod channeled_channel;
mod connector_worker;
mod proxy_worker;
mod structs;

pub type ConnectorChannel = (async_channel::Sender<u16>, async_channel::Receiver<u16>);

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let channels = channeled_channel::ChanneledChannel::new();
    let connector_channel = async_channel::unbounded::<u16>();

    connector_worker::spawn_connector_worker(connector_channel.clone(), channels.clone()).await?;

    tokio::signal::ctrl_c().await?;
    Ok(())
}
