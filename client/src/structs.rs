use color_eyre::Result;
use serde::{Deserialize, Serialize};
use utils::{ConnectorInfo, ConnectorPort, PortType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub connector: String,
    pub code: u128,

    pub ports: Vec<ConfigPort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPort {
    pub remote: u16,
    pub local: u16,
    pub ip: Option<String>,

    #[serde(rename = "type")]
    pub _type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConvertedConfig {
    pub connector: ConnectorInfo,
    pub code: u128,
    pub connector_ip: String,
    pub connector_port: u16,
}

impl Config {
    pub async fn load() -> Result<Self> {
        let config_res = tokio::fs::read_to_string("config.json").await;
        match config_res {
            Ok(config) => {
                let config: Config = serde_json::from_str(&config)?;
                return Ok(config);
            }
            Err(_) => {
                let config = Config {
                    connector: String::from("server:1337"),
                    code: 123213123123123,
                    ports: vec![],
                };

                let config_str = serde_json::to_string_pretty(&config)?;
                tokio::fs::write("config.json", config_str).await?;

                return Ok(config);
            }
        }
    }

    pub fn convert(&self) -> Result<ConvertedConfig> {
        let mut connector_ports: Vec<ConnectorPort> = Vec::new();
        for port in self.ports.iter() {
            let _type = match port._type.as_ref().unwrap().to_uppercase().as_str() {
                "TCP" => PortType::Tcp,
                "UDP" => PortType::Udp,
                _ => {
                    return Err(color_eyre::eyre::eyre!(
                        "Invalid port type: {}",
                        port._type.as_ref().unwrap()
                    ))
                }
            };

            connector_ports.push(ConnectorPort {
                port_remote: port.remote,
                port_local: port.local,
                local_ip: port.ip.clone().unwrap_or(String::from("127.0.0.1")),
                port_type: _type,
            });
        }

        let connector_splitted = self.connector.split(":").collect::<Vec<&str>>();
        let connector_ip = connector_splitted[0].to_string();
        let connector_port = connector_splitted
            .get(1)
            .unwrap_or(&"1337")
            .parse::<u16>()?;

        let converted_config = ConvertedConfig {
            connector: ConnectorInfo {
                ports: connector_ports,
            },
            code: self.code,
            connector_ip,
            connector_port,
        };

        Ok(converted_config)
    }
}
