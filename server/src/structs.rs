use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorInfo {
    pub ports: Vec<ConnectorPort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorPort {
    pub port_worker: u16,
    pub port_local: u16,
    pub port_type: PortType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(dead_code)]
pub enum PortType {
    Tcp,
    Udp,
}
