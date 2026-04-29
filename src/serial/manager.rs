use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serialport::{SerialPortInfo, SerialPortType};

use crate::error::{Result, SerialportApiError};

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct QueuedCommand {
    #[serde(rename = "reqId")]
    pub req_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SerialStreamEvent {
    pub event: &'static str,
    pub data: Value,
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryConnectionManager {
    connections: Arc<Mutex<BTreeMap<String, ConnectionInfo>>>,
    next_req_id: Arc<Mutex<u64>>,
    written_frames: Arc<Mutex<BTreeMap<String, Vec<Vec<u8>>>>>,
    events: Arc<Mutex<Vec<SerialStreamEvent>>>,
}

pub trait ConnectionManager: Clone + Send + Sync + 'static {
    fn connect(&self, request: ConnectionRequest) -> Result<ConnectionInfo>;
    fn connections(&self) -> Result<Vec<ConnectionInfo>>;
    fn disconnect(&self, name: &str) -> Result<String>;
    fn send_command(&self, connection_name: &str, payload: Value) -> Result<QueuedCommand>;
    fn events(&self) -> Result<Vec<SerialStreamEvent>>;
}

impl InMemoryConnectionManager {
    #[cfg(test)]
    pub fn written_frames(&self, name: &str) -> Vec<Vec<u8>> {
        self.written_frames
            .lock()
            .expect("in-memory written frames lock poisoned")
            .get(name)
            .cloned()
            .unwrap_or_default()
    }

    pub fn record_event(&self, event: crate::protocol::SerialEvent) {
        let event = SerialStreamEvent::from(event);
        self.events
            .lock()
            .expect("in-memory serial events lock poisoned")
            .push(event);
    }

    pub fn record_error(&self, message: impl Into<String>) {
        self.events
            .lock()
            .expect("in-memory serial events lock poisoned")
            .push(SerialStreamEvent {
                event: "serial.error",
                data: Value::String(message.into()),
            });
    }
}

impl From<crate::protocol::SerialEvent> for SerialStreamEvent {
    fn from(event: crate::protocol::SerialEvent) -> Self {
        match event {
            crate::protocol::SerialEvent::Json(data) => Self {
                event: "serial.json",
                data,
            },
            crate::protocol::SerialEvent::Text(text) => Self {
                event: "serial.text",
                data: Value::String(text),
            },
            crate::protocol::SerialEvent::Log(data) => Self {
                event: "serial.log",
                data,
            },
            crate::protocol::SerialEvent::Notification(data) => Self {
                event: "serial.notification",
                data,
            },
        }
    }
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

    fn send_command(&self, connection_name: &str, mut payload: Value) -> Result<QueuedCommand> {
        let connection = self
            .connections
            .lock()
            .expect("in-memory connection registry lock poisoned")
            .get(connection_name)
            .cloned()
            .ok_or_else(|| SerialportApiError::ConnectionNotFound(connection_name.to_string()))?;

        let object = payload
            .as_object_mut()
            .ok_or(SerialportApiError::InvalidCommandPayload)?;

        let req_id = match object.get("reqId").and_then(Value::as_str) {
            Some(req_id) => req_id.to_string(),
            None => {
                let mut next_req_id = self
                    .next_req_id
                    .lock()
                    .expect("in-memory request id counter lock poisoned");
                *next_req_id += 1;
                let req_id = next_req_id.to_string();
                object.insert("reqId".to_string(), Value::String(req_id.clone()));
                req_id
            }
        };

        let frame = crate::protocol::frame_json(&payload, &connection.delimiter)?;
        self.written_frames
            .lock()
            .expect("in-memory written frames lock poisoned")
            .entry(connection_name.to_string())
            .or_default()
            .push(frame);

        Ok(QueuedCommand { req_id })
    }

    fn events(&self) -> Result<Vec<SerialStreamEvent>> {
        Ok(self
            .events
            .lock()
            .expect("in-memory serial events lock poisoned")
            .clone())
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

    #[test]
    fn in_memory_connection_manager_records_framed_command_with_generated_req_id() {
        let manager = InMemoryConnectionManager::default();

        manager
            .connect(ConnectionRequest {
                name: "default".to_string(),
                port: "/dev/ttyUSB0".to_string(),
                baud_rate: 115200,
                delimiter: "\r\n".to_string(),
            })
            .unwrap();

        let queued = manager
            .send_command(
                "default",
                serde_json::json!({
                    "method": "query",
                    "topic": "sensor.read",
                    "data": {}
                }),
            )
            .unwrap();

        assert_eq!(queued.req_id, "1");
        let frames = manager.written_frames("default");
        assert_eq!(frames.len(), 1);
        assert!(frames[0].ends_with(b"\r\n"));
        let body = &frames[0][..frames[0].len() - 2];
        let payload: serde_json::Value = serde_json::from_slice(body).unwrap();
        assert_eq!(
            payload,
            serde_json::json!({
                "reqId": "1",
                "method": "query",
                "topic": "sensor.read",
                "data": {}
            })
        );
    }

    #[test]
    fn in_memory_connection_manager_preserves_existing_req_id() {
        let manager = InMemoryConnectionManager::default();

        manager
            .connect(ConnectionRequest {
                name: "default".to_string(),
                port: "/dev/ttyUSB0".to_string(),
                baud_rate: 115200,
                delimiter: "\n".to_string(),
            })
            .unwrap();

        let queued = manager
            .send_command(
                "default",
                serde_json::json!({
                    "reqId": "client-42",
                    "method": "mutation",
                    "topic": "led.set",
                    "data": {"on": true}
                }),
            )
            .unwrap();

        assert_eq!(queued.req_id, "client-42");
        let frames = manager.written_frames("default");
        assert_eq!(frames.len(), 1);
        assert!(frames[0].ends_with(b"\n"));
        let body = &frames[0][..frames[0].len() - 1];
        let payload: serde_json::Value = serde_json::from_slice(body).unwrap();
        assert_eq!(
            payload,
            serde_json::json!({
                "reqId": "client-42",
                "method": "mutation",
                "topic": "led.set",
                "data": {"on": true}
            })
        );
    }

    #[test]
    fn in_memory_connection_manager_records_serial_events_for_streaming() {
        let manager = InMemoryConnectionManager::default();

        manager.record_event(crate::protocol::SerialEvent::Json(serde_json::json!({
            "reqId": "1",
            "ok": true
        })));
        manager.record_event(crate::protocol::SerialEvent::Text(
            "hello robot".to_string(),
        ));
        manager.record_event(crate::protocol::SerialEvent::Log(serde_json::json!({
            "method": "log",
            "data": {"level": "info"}
        })));
        manager.record_event(crate::protocol::SerialEvent::Notification(
            serde_json::json!({
                "method": "notification",
                "data": []
            }),
        ));
        manager.record_error("serial read failed");

        assert_eq!(
            manager.events().unwrap(),
            vec![
                SerialStreamEvent {
                    event: "serial.json",
                    data: serde_json::json!({"reqId":"1","ok":true}),
                },
                SerialStreamEvent {
                    event: "serial.text",
                    data: serde_json::json!("hello robot"),
                },
                SerialStreamEvent {
                    event: "serial.log",
                    data: serde_json::json!({"method":"log","data":{"level":"info"}}),
                },
                SerialStreamEvent {
                    event: "serial.notification",
                    data: serde_json::json!({"method":"notification","data":[]}),
                },
                SerialStreamEvent {
                    event: "serial.error",
                    data: serde_json::json!("serial read failed"),
                },
            ]
        );
    }
}
