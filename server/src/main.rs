use crate::structs::Config;
use color_eyre::Result;

mod channeled_channel;
mod connector_worker;
mod structs;
mod tunnel;

pub type ConnectorChannel = (async_channel::Sender<u16>, async_channel::Receiver<u16>);

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let tunnel_channels = channeled_channel::ChanneledChannel::new();
    let connector_channel = async_channel::unbounded::<u16>();
    let config = Config::load().await?;

    println!("Connector code: {}", config.code);
    connector_worker::spawn_connector_worker(connector_channel, tunnel_channels, config).await?;

    tokio::signal::ctrl_c().await?;
    Ok(())
}
