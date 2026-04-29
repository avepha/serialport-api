use serde::Serialize;
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
}
