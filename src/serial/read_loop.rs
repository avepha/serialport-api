use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::error::Result;
use crate::serial::manager::ConnectionManagerWithTransport;
use crate::serial::transport::SerialTransport;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SerialReadItem {
    Line(Vec<u8>),
    Error(String),
}

pub trait SerialReadSource: Clone + Send + Sync + 'static {
    fn drain_items(&self, connection_name: &str) -> Result<Vec<SerialReadItem>>;
}

pub trait SerialEventRecorder: Clone + Send + Sync + 'static {
    fn record_serial_event_for_connection(
        &self,
        connection_name: &str,
        event: crate::protocol::SerialEvent,
    );

    fn record_serial_error_for_connection(&self, connection_name: &str, message: String);

    fn record_serial_event(&self, event: crate::protocol::SerialEvent) {
        self.record_serial_event_for_connection("default", event);
    }

    fn record_serial_error(&self, message: String) {
        self.record_serial_error_for_connection("default", message);
    }
}

#[derive(Clone, Debug, Default)]
pub struct MockSerialReadSource {
    items_by_connection: Arc<Mutex<BTreeMap<String, VecDeque<SerialReadItem>>>>,
}

impl MockSerialReadSource {
    pub fn push_line(&self, connection_name: impl Into<String>, line: impl Into<Vec<u8>>) {
        self.push_item(connection_name, SerialReadItem::Line(line.into()));
    }

    pub fn push_error(&self, connection_name: impl Into<String>, message: impl Into<String>) {
        self.push_item(connection_name, SerialReadItem::Error(message.into()));
    }

    fn push_item(&self, connection_name: impl Into<String>, item: SerialReadItem) {
        self.items_by_connection
            .lock()
            .expect("mock serial read source lock poisoned")
            .entry(connection_name.into())
            .or_default()
            .push_back(item);
    }
}

impl SerialReadSource for MockSerialReadSource {
    fn drain_items(&self, connection_name: &str) -> Result<Vec<SerialReadItem>> {
        Ok(self
            .items_by_connection
            .lock()
            .expect("mock serial read source lock poisoned")
            .remove(connection_name)
            .map(|items| items.into())
            .unwrap_or_default())
    }
}

impl<T> SerialEventRecorder for ConnectionManagerWithTransport<T>
where
    T: SerialTransport,
{
    fn record_serial_event_for_connection(
        &self,
        connection_name: &str,
        event: crate::protocol::SerialEvent,
    ) {
        self.record_event_for_connection(connection_name, event);
    }

    fn record_serial_error_for_connection(&self, _connection_name: &str, message: String) {
        self.record_error(message);
    }
}

pub fn drain_serial_read_items<M, R>(
    manager: &M,
    read_source: &R,
    connection_name: &str,
) -> Result<usize>
where
    M: SerialEventRecorder,
    R: SerialReadSource,
{
    let items = read_source.drain_items(connection_name)?;
    let processed = items.len();

    for item in items {
        match item {
            SerialReadItem::Line(line) => {
                manager.record_serial_event_for_connection(
                    connection_name,
                    crate::protocol::parse_line(&line),
                );
            }
            SerialReadItem::Error(message) => {
                manager.record_serial_error_for_connection(connection_name, message);
            }
        }
    }

    Ok(processed)
}

pub fn spawn_mock_read_loop<M, R>(
    manager: M,
    read_source: R,
    connection_name: String,
) -> tokio::task::JoinHandle<()>
where
    M: SerialEventRecorder,
    R: SerialReadSource,
{
    tokio::spawn(async move {
        let _ = drain_serial_read_items(&manager, &read_source, &connection_name);
    })
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn mock_read_source_drains_lines_and_errors_for_named_connection() {
        let source = MockSerialReadSource::default();

        source.push_line("default", b"{\"reqId\":\"1\",\"ok\":true}\r\n".to_vec());
        source.push_error("default", "serial read failed");
        source.push_line("other", b"ignored\n".to_vec());

        assert_eq!(
            source.drain_items("default").unwrap(),
            vec![
                SerialReadItem::Line(b"{\"reqId\":\"1\",\"ok\":true}\r\n".to_vec()),
                SerialReadItem::Error("serial read failed".to_string()),
            ]
        );
        assert_eq!(source.drain_items("default").unwrap(), Vec::new());
        assert_eq!(
            source.drain_items("other").unwrap(),
            vec![SerialReadItem::Line(b"ignored\n".to_vec())]
        );
    }

    #[test]
    fn drain_read_items_records_parsed_events_on_manager() {
        use crate::serial::manager::{
            ConnectionManager, InMemoryConnectionManager, SerialStreamEvent,
        };

        let manager = InMemoryConnectionManager::default();
        let source = MockSerialReadSource::default();

        source.push_line("default", b"{\"reqId\":\"1\",\"ok\":true}\r\n".to_vec());
        source.push_line("default", b"hello robot\n".to_vec());
        source.push_line(
            "default",
            b"{\"method\":\"log\",\"data\":{\"level\":\"info\"}}\n".to_vec(),
        );
        source.push_line(
            "default",
            b"{\"method\":\"notification\",\"data\":[]}\n".to_vec(),
        );

        let processed = drain_serial_read_items(&manager, &source, "default").unwrap();

        assert_eq!(processed, 4);
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
            ]
        );
    }

    #[test]
    fn drain_read_items_records_errors_as_serial_error_events() {
        use crate::serial::manager::{
            ConnectionManager, InMemoryConnectionManager, SerialStreamEvent,
        };

        let manager = InMemoryConnectionManager::default();
        let source = MockSerialReadSource::default();

        source.push_error("default", "serial read failed");

        let processed = drain_serial_read_items(&manager, &source, "default").unwrap();

        assert_eq!(processed, 1);
        assert_eq!(
            manager.events().unwrap(),
            vec![SerialStreamEvent {
                event: "serial.error",
                data: serde_json::json!("serial read failed"),
            }]
        );
    }

    #[test]
    fn drain_read_items_indexes_json_response_for_connection() {
        use crate::serial::manager::{
            ConnectionManager, InMemoryConnectionManager, SerialStreamEvent,
        };

        let manager = InMemoryConnectionManager::default();
        let source = MockSerialReadSource::default();

        source.push_line("robot", b"{\"reqId\":\"42\",\"ok\":true}\r\n".to_vec());

        let processed = drain_serial_read_items(&manager, &source, "robot").unwrap();

        assert_eq!(processed, 1);
        assert_eq!(manager.take_response("default", "42").unwrap(), None);
        assert_eq!(
            manager.take_response("robot", "42").unwrap(),
            Some(serde_json::json!({"reqId":"42","ok":true}))
        );
        assert_eq!(
            manager.events().unwrap(),
            vec![SerialStreamEvent {
                event: "serial.json",
                data: serde_json::json!({"reqId":"42","ok":true}),
            }]
        );
    }

    #[tokio::test]
    async fn spawned_mock_read_loop_drains_items_into_manager_events() {
        use crate::serial::manager::{
            ConnectionManager, InMemoryConnectionManager, SerialStreamEvent,
        };

        let manager = InMemoryConnectionManager::default();
        let source = MockSerialReadSource::default();
        source.push_line("default", b"hello robot\n".to_vec());

        let handle = spawn_mock_read_loop(manager.clone(), source, "default".to_string());
        handle.await.unwrap();

        assert_eq!(
            manager.events().unwrap(),
            vec![SerialStreamEvent {
                event: "serial.text",
                data: serde_json::json!("hello robot"),
            }]
        );
    }
}
