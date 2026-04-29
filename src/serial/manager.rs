use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serialport::{SerialPortInfo, SerialPortType};

use crate::error::Result;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PortInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub port_type: String,
    pub manufacturer: Option<String>,
    pub serial_number: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct SystemPortLister;

#[derive(Clone, Debug, Deserialize)]
pub struct ConnectionRequest {
    pub name: String,
    pub port: String,
    #[serde(rename = "baudRate")]
    pub baud_rate: u32,
    pub delimiter: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConnectionInfo {
    pub name: String,
    pub status: &'static str,
    pub port: String,
    #[serde(rename = "baudRate")]
    pub baud_rate: u32,
    pub delimiter: String,
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryConnectionManager {
    connections: Arc<Mutex<BTreeMap<String, ConnectionInfo>>>,
}

pub trait ConnectionManager: Clone + Send + Sync + 'static {
    fn connect(&self, request: ConnectionRequest) -> Result<ConnectionInfo>;
    fn connections(&self) -> Result<Vec<ConnectionInfo>>;
    fn disconnect(&self, name: &str) -> Result<String>;
}

impl ConnectionManager for InMemoryConnectionManager {
    fn connect(&self, request: ConnectionRequest) -> Result<ConnectionInfo> {
        let connection = ConnectionInfo {
            name: request.name,
            status: "connected",
            port: request.port,
            baud_rate: request.baud_rate,
            delimiter: request.delimiter,
        };

        self.connections
            .lock()
            .expect("in-memory connection registry lock poisoned")
            .insert(connection.name.clone(), connection.clone());

        Ok(connection)
    }

    fn connections(&self) -> Result<Vec<ConnectionInfo>> {
        Ok(self
            .connections
            .lock()
            .expect("in-memory connection registry lock poisoned")
            .values()
            .cloned()
            .collect())
    }

    fn disconnect(&self, name: &str) -> Result<String> {
        self.connections
            .lock()
            .expect("in-memory connection registry lock poisoned")
            .remove(name);

        Ok(name.to_string())
    }
}

pub trait SerialPortLister: Clone + Send + Sync + 'static {
    fn available_ports(&self) -> Result<Vec<PortInfo>>;
}

impl SerialPortLister for SystemPortLister {
    fn available_ports(&self) -> Result<Vec<PortInfo>> {
        serialport::available_ports()?
            .into_iter()
            .map(PortInfo::try_from)
            .collect()
    }
}

impl TryFrom<SerialPortInfo> for PortInfo {
    type Error = crate::error::SerialportApiError;

    fn try_from(port: SerialPortInfo) -> Result<Self> {
        let (port_type, manufacturer, serial_number) = match port.port_type {
            SerialPortType::UsbPort(info) => ("usb", info.manufacturer, info.serial_number),
            SerialPortType::BluetoothPort => ("bluetooth", None, None),
            SerialPortType::PciPort => ("pci", None, None),
            SerialPortType::Unknown => ("unknown", None, None),
        };

        Ok(Self {
            name: port.port_name,
            port_type: port_type.to_string(),
            manufacturer,
            serial_number,
        })
    }
}

pub fn list_ports<L>(lister: &L) -> Result<Vec<PortInfo>>
where
    L: SerialPortLister,
{
    lister.available_ports()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::error::Result;

    #[derive(Clone)]
    struct MockPortLister {
        ports: Vec<PortInfo>,
    }

    impl SerialPortLister for MockPortLister {
        fn available_ports(&self) -> Result<Vec<PortInfo>> {
            Ok(self.ports.clone())
        }
    }

    #[test]
    fn list_ports_returns_available_serial_ports() {
        let lister = MockPortLister {
            ports: vec![PortInfo {
                name: "/dev/ttyUSB0".to_string(),
                port_type: "usb".to_string(),
                manufacturer: Some("FTDI".to_string()),
                serial_number: Some("ABC123".to_string()),
            }],
        };

        let ports = list_ports(&lister).unwrap();

        assert_eq!(
            ports,
            vec![PortInfo {
                name: "/dev/ttyUSB0".to_string(),
                port_type: "usb".to_string(),
                manufacturer: Some("FTDI".to_string()),
                serial_number: Some("ABC123".to_string()),
            }]
        );
    }

    #[test]
    fn in_memory_connection_manager_records_connections() {
        let manager = InMemoryConnectionManager::default();

        let connection = manager
            .connect(ConnectionRequest {
                name: "default".to_string(),
                port: "/dev/ttyUSB0".to_string(),
                baud_rate: 115200,
                delimiter: "\r\n".to_string(),
            })
            .unwrap();

        assert_eq!(connection.name, "default");
        assert_eq!(connection.status, "connected");
        assert_eq!(manager.connections().unwrap(), vec![connection]);
    }

    #[test]
    fn in_memory_connection_manager_removes_disconnected_connections() {
        let manager = InMemoryConnectionManager::default();

        manager
            .connect(ConnectionRequest {
                name: "default".to_string(),
                port: "/dev/ttyUSB0".to_string(),
                baud_rate: 115200,
                delimiter: "\r\n".to_string(),
            })
            .unwrap();

        let disconnected_name = manager.disconnect("default").unwrap();

        assert_eq!(disconnected_name, "default");
        assert_eq!(manager.connections().unwrap(), Vec::<ConnectionInfo>::new());
    }
}
