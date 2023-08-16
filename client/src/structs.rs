use color_eyre::Result;
use serde::{Deserialize, Serialize};
use utils::{ConnectorInfo, ConnectorPort, PortType};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub connector: String,
    pub code: u64,

    pub ports: Vec<ConfigPort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPort {
    pub remote: u16,
    pub local: u16,
    pub ip: Option<String>,

    #[serde(rename = "type")]
    pub _type: Option<String>,

    #[serde(rename = "tunnelType")]
    pub tunnel_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConvertedConfig {
    pub connector: ConnectorInfo,

    pub code: u64,
    pub connector_ip: String,
    pub connector_port: u16,
}

impl Config {
    pub async fn load() -> Result<Self> {
        if std::env::var("LF_ENV").is_ok() {
            return Self::load_from_env().await;
        }

        let config_path = std::env::args()
            .nth(1)
            .unwrap_or(String::from("config.json"));
        let config_res = tokio::fs::read_to_string(config_path).await;

        match config_res {
            Ok(config) => {
                let config: Config = serde_json::from_str(&config)?;
                return Ok(config);
            }
            Err(_) => {
                let config = Config {
                    connector: String::from("server:1337"),
                    code: 123213123123123,
                    ports: vec![ConfigPort {
                        remote: 8080,
                        local: 80,
                        ip: Some(String::from("127.0.0.1")),
                        _type: Some(String::from("TCP")),
                        tunnel_type: Some(String::from("tcp")),
                    }],
                };

                let config_str = serde_json::to_string_pretty(&config)?;
                tokio::fs::write("config.json", config_str).await?;

                return Ok(config);
            }
        }
    }

    async fn load_from_env() -> Result<Self> {
        let mut config = Config::default();
        config.connector = std::env::var("LF_CONNECTOR").unwrap_or(String::from("server:1337"));
        config.code = std::env::var("LF_CODE")
            .unwrap_or(String::from("123213123123123"))
            .parse::<u64>()?;

        let mut ports: Vec<ConfigPort> = Vec::new();
        for (key, value) in std::env::vars() {
            if key.starts_with("LF_PORT") {
                let splitted_value = value
                    .split("/")
                    .nth(0)
                    .expect("SHOULDNT ERROR")
                    .split(":")
                    .collect::<Vec<&str>>();

                let _type = value.split("/").nth(1).unwrap_or("TCP");
                let tunnel_type = key.split("_").nth(2).unwrap_or("TCP");
                let mut ip = "127.0.0.1";
                let remote;
                let local;

                if splitted_value.len() == 3 {
                    ip = splitted_value[0];
                    remote = splitted_value[1].parse::<u16>()?;
                    local = splitted_value[2].parse::<u16>()?;
                } else {
                    remote = splitted_value[0].parse::<u16>()?;
                    local = splitted_value[1].parse::<u16>()?;
                }

                let port = ConfigPort {
                    remote,
                    local,
                    ip: Some(ip.to_string()),
                    _type: Some(_type.to_string()),
                    tunnel_type: Some(tunnel_type.to_string()),
                };

                ports.push(port);
            }
        }

        config.ports = ports;
        Ok(config)
    }

    pub fn convert(&self) -> Result<ConvertedConfig> {
        let mut connector_ports: Vec<ConnectorPort> = Vec::new();
        for port in self.ports.iter() {
            let _type = match port
                ._type
                .as_ref()
                .unwrap_or(&String::from("TCP"))
                .to_uppercase()
                .as_str()
            {
                "TCP" => PortType::Tcp,
                "UDP" => PortType::Udp,
                _ => {
                    return Err(color_eyre::eyre::eyre!(
                        "Invalid port type: {}",
                        port._type.as_ref().unwrap()
                    ))
                }
            };

            let tunnel_type = match port
                .tunnel_type
                .as_ref()
                .unwrap_or(&String::from("TCP"))
                .to_uppercase()
                .as_str()
            {
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
                tunnel_type,
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
