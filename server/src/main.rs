use color_eyre::Result;
use std::{fs::Permissions, os::unix::prelude::PermissionsExt, path::Path};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

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
    let connector_code = get_or_generate_code().await?;

    println!("Connector code: {}", connector_code);
    connector_worker::spawn_connector_worker(connector_channel, tunnel_channels, connector_code)
        .await?;

    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn get_or_generate_code() -> Result<u128> {
    if Path::new("/etc/local-forwarder").exists() {
        let file = File::open("/etc/local-forwarder/code").await;

        match file {
            Ok(mut file) => {
                let mut code = vec![0; 16];

                file.read_exact(&mut code).await?;
                Ok(u128::from_be_bytes(code[..].try_into()?))
            }
            Err(_) => {
                let code = rand::random::<u128>();

                let mut file = File::create("/etc/local-forwarder/code").await?;
                tokio::fs::set_permissions(
                    "/etc/local-forwarder/code",
                    Permissions::from_mode(0o600),
                )
                .await?;

                file.write_all(&code.to_be_bytes()).await?;
                Ok(code)
            }
        }
    } else {
        tokio::fs::create_dir_all("/etc/local-forwarder").await?;

        let code = rand::random::<u128>();

        let mut file = File::create("/etc/local-forwarder/code").await?;
        tokio::fs::set_permissions("/etc/local-forwarder/code", Permissions::from_mode(0o600))
            .await?;

        file.write_all(&code.to_be_bytes()).await?;
        Ok(code)
    }
}
