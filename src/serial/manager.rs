use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serialport::{SerialPortInfo, SerialPortType};

use crate::error::{Result, SerialportApiError};
use crate::serial::mock_device::MockDeviceResponder;
use crate::serial::transport::{MockSerialTransport, SerialTransport};

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

type ResponseQueue = BTreeMap<String, BTreeMap<String, VecDeque<Value>>>;

#[derive(Clone, Debug)]
pub struct ConnectionManagerWithTransport<T> {
    connections: Arc<Mutex<BTreeMap<String, ConnectionInfo>>>,
    next_req_id: Arc<Mutex<u64>>,
    events: Arc<Mutex<Vec<SerialStreamEvent>>>,
    responses_by_connection_and_req_id: Arc<Mutex<ResponseQueue>>,
    transport: T,
    mock_responder: Option<MockDeviceResponder>,
}

pub type InMemoryConnectionManager = ConnectionManagerWithTransport<MockSerialTransport>;

pub trait ConnectionManager: Clone + Send + Sync + 'static {
    fn connect(&self, request: ConnectionRequest) -> Result<ConnectionInfo>;
    fn connections(&self) -> Result<Vec<ConnectionInfo>>;
    fn disconnect(&self, name: &str) -> Result<String>;
    fn send_command(&self, connection_name: &str, payload: Value) -> Result<QueuedCommand>;
    fn take_response(&self, connection_name: &str, req_id: &str) -> Result<Option<Value>>;
    fn events(&self) -> Result<Vec<SerialStreamEvent>>;
}

impl<T> ConnectionManagerWithTransport<T>
where
    T: SerialTransport,
{
    pub fn new(transport: T) -> Self {
        Self {
            connections: Arc::default(),
            next_req_id: Arc::default(),
            events: Arc::default(),
            responses_by_connection_and_req_id: Arc::default(),
            transport,
            mock_responder: None,
        }
    }

    pub fn with_mock_responder(transport: T, responder: MockDeviceResponder) -> Self {
        Self {
            connections: Arc::default(),
            next_req_id: Arc::default(),
            events: Arc::default(),
            responses_by_connection_and_req_id: Arc::default(),
            transport,
            mock_responder: Some(responder),
        }
    }

    pub fn transport(&self) -> T {
        self.transport.clone()
    }

    pub fn record_event(&self, event: crate::protocol::SerialEvent) {
        self.record_event_for_connection("default", event);
    }

    pub fn record_event_for_connection(
        &self,
        connection_name: &str,
        event: crate::protocol::SerialEvent,
    ) {
        if let crate::protocol::SerialEvent::Json(value) = &event {
            if let Some(req_id) = value.get("reqId").and_then(Value::as_str) {
                self.responses_by_connection_and_req_id
                    .lock()
                    .expect("in-memory response queue lock poisoned")
                    .entry(connection_name.to_string())
                    .or_default()
                    .entry(req_id.to_string())
                    .or_default()
                    .push_back(value.clone());
            }
        }

        let event = SerialStreamEvent::from(event);
        self.events
            .lock()
            .expect("in-memory serial events lock poisoned")
            .push(event);
    }

    pub fn take_response(&self, connection_name: &str, req_id: &str) -> Result<Option<Value>> {
        let mut responses = self
            .responses_by_connection_and_req_id
            .lock()
            .expect("in-memory response queue lock poisoned");

        let Some(responses_by_req_id) = responses.get_mut(connection_name) else {
            return Ok(None);
        };
        let Some(queue) = responses_by_req_id.get_mut(req_id) else {
            return Ok(None);
        };

        let response = queue.pop_front();
        if queue.is_empty() {
            responses_by_req_id.remove(req_id);
        }
        if responses_by_req_id.is_empty() {
            responses.remove(connection_name);
        }

        Ok(response)
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

impl<T> Default for ConnectionManagerWithTransport<T>
where
    T: SerialTransport + Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl ConnectionManagerWithTransport<MockSerialTransport> {
    #[cfg(test)]
    pub fn written_frames(&self, name: &str) -> Vec<Vec<u8>> {
        self.transport.written_frames(name)
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

impl<T> ConnectionManager for ConnectionManagerWithTransport<T>
where
    T: SerialTransport,
{
    fn connect(&self, request: ConnectionRequest) -> Result<ConnectionInfo> {
        let connection = ConnectionInfo {
            name: request.name,
            status: "connected",
            port: request.port,
            baud_rate: request.baud_rate,
            delimiter: request.delimiter,
        };

        self.transport.open(&connection)?;

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
        self.transport.close(name)?;

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
        self.transport.write_frame(connection_name, &frame)?;

        if let Some(responder) = &self.mock_responder {
            if let Some(response) = responder.response_for_frame(&frame, &connection.delimiter) {
                self.record_event_for_connection(
                    connection_name,
                    crate::protocol::SerialEvent::Json(response),
                );
            }
        }

        Ok(QueuedCommand { req_id })
    }

    fn take_response(&self, connection_name: &str, req_id: &str) -> Result<Option<Value>> {
        Self::take_response(self, connection_name, req_id)
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
    use crate::serial::transport::MockSerialTransport;

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
    fn connection_manager_opens_and_closes_transport_connections() {
        let transport = MockSerialTransport::default();
        let manager = ConnectionManagerWithTransport::new(transport.clone());

        manager
            .connect(ConnectionRequest {
                name: "default".to_string(),
                port: "/dev/ROBOT".to_string(),
                baud_rate: 115200,
                delimiter: "\r\n".to_string(),
            })
            .unwrap();

        assert_eq!(transport.opened_names(), vec!["default".to_string()]);

        manager.disconnect("default").unwrap();

        assert_eq!(transport.opened_names(), Vec::<String>::new());
        assert_eq!(transport.closed_names(), vec!["default".to_string()]);
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
    fn connection_manager_writes_framed_command_through_transport_with_generated_req_id() {
        let transport = MockSerialTransport::default();
        let manager = ConnectionManagerWithTransport::new(transport.clone());

        manager
            .connect(ConnectionRequest {
                name: "default".to_string(),
                port: "/dev/ROBOT".to_string(),
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
        let frames = transport.written_frames("default");
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

    #[test]
    fn manager_indexes_json_response_by_connection_and_req_id() {
        let manager = InMemoryConnectionManager::default();

        manager.record_event_for_connection(
            "default",
            crate::protocol::SerialEvent::Json(serde_json::json!({
                "reqId": "1",
                "ok": true,
                "data": {"temperature": 28.5}
            })),
        );

        assert_eq!(
            manager.take_response("default", "1").unwrap(),
            Some(serde_json::json!({
                "reqId": "1",
                "ok": true,
                "data": {"temperature": 28.5}
            }))
        );
        assert_eq!(manager.take_response("default", "1").unwrap(), None);
        assert_eq!(
            manager.events().unwrap(),
            vec![SerialStreamEvent {
                event: "serial.json",
                data: serde_json::json!({
                    "reqId": "1",
                    "ok": true,
                    "data": {"temperature": 28.5}
                }),
            }]
        );
    }

    #[test]
    fn manager_does_not_match_responses_across_connections() {
        let manager = InMemoryConnectionManager::default();

        manager.record_event_for_connection(
            "default",
            crate::protocol::SerialEvent::Json(serde_json::json!({"reqId":"1","ok":true})),
        );

        assert_eq!(manager.take_response("other", "1").unwrap(), None);
        assert_eq!(
            manager.take_response("default", "1").unwrap(),
            Some(serde_json::json!({"reqId":"1","ok":true}))
        );
    }

    #[test]
    fn manager_returns_duplicate_req_id_responses_fifo() {
        let manager = InMemoryConnectionManager::default();

        manager.record_event_for_connection(
            "default",
            crate::protocol::SerialEvent::Json(serde_json::json!({"reqId":"1","seq":1})),
        );
        manager.record_event_for_connection(
            "default",
            crate::protocol::SerialEvent::Json(serde_json::json!({"reqId":"1","seq":2})),
        );

        assert_eq!(
            manager.take_response("default", "1").unwrap(),
            Some(serde_json::json!({"reqId":"1","seq":1}))
        );
        assert_eq!(
            manager.take_response("default", "1").unwrap(),
            Some(serde_json::json!({"reqId":"1","seq":2}))
        );
        assert_eq!(manager.take_response("default", "1").unwrap(), None);
    }

    #[test]
    fn manager_ignores_events_without_string_req_id_for_response_matching() {
        let manager = InMemoryConnectionManager::default();

        manager.record_event_for_connection(
            "default",
            crate::protocol::SerialEvent::Text("hello".to_string()),
        );
        manager.record_event_for_connection(
            "default",
            crate::protocol::SerialEvent::Json(serde_json::json!({"ok":true})),
        );
        manager.record_event_for_connection(
            "default",
            crate::protocol::SerialEvent::Json(serde_json::json!({"reqId":1,"ok":true})),
        );
        manager.record_event_for_connection(
            "default",
            crate::protocol::SerialEvent::Log(serde_json::json!({
                "method":"log",
                "reqId":"1",
                "data":{}
            })),
        );

        assert_eq!(manager.take_response("default", "1").unwrap(), None);
    }

    #[test]
    fn manager_with_mock_responder_records_response_after_send_command() {
        let transport = crate::serial::transport::MockSerialTransport::default();
        let manager = ConnectionManagerWithTransport::with_mock_responder(
            transport,
            crate::serial::mock_device::MockDeviceResponder::default(),
        );

        manager
            .connect(ConnectionRequest {
                name: "default".to_string(),
                port: "/dev/ROBOT".to_string(),
                baud_rate: 115200,
                delimiter: "\r\n".to_string(),
            })
            .unwrap();

        let queued = manager
            .send_command(
                "default",
                serde_json::json!({"reqId":"mock-1","method":"query","topic":"sensor.read","data":{}}),
            )
            .unwrap();

        assert_eq!(queued.req_id, "mock-1");
        assert_eq!(
            manager.take_response("default", "mock-1").unwrap(),
            Some(serde_json::json!({
                "reqId": "mock-1",
                "ok": true,
                "data": {"mock": true, "topic": "sensor.read"}
            }))
        );
        assert_eq!(manager.events().unwrap()[0].event, "serial.json");
    }

    #[test]
    fn default_manager_does_not_auto_record_mock_response() {
        let manager = InMemoryConnectionManager::default();

        manager
            .connect(ConnectionRequest {
                name: "default".to_string(),
                port: "/dev/ROBOT".to_string(),
                baud_rate: 115200,
                delimiter: "\r\n".to_string(),
            })
            .unwrap();

        manager
            .send_command(
                "default",
                serde_json::json!({"reqId":"no-auto","topic":"sensor.read","data":{}}),
            )
            .unwrap();

        assert_eq!(manager.take_response("default", "no-auto").unwrap(), None);
        assert_eq!(manager.events().unwrap(), vec![]);
    }
}
