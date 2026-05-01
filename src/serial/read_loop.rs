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

pub trait RealSerialLineSource: Clone + Send + Sync + 'static {
    fn drain_lines(&self, connection_name: &str, delimiter: &str) -> Result<Vec<Vec<u8>>>;
}

impl<F> RealSerialLineSource for crate::serial::real_transport::RealSerialTransport<F>
where
    F: crate::serial::real_transport::SerialPortFactory,
{
    fn drain_lines(&self, connection_name: &str, delimiter: &str) -> Result<Vec<Vec<u8>>> {
        self.drain_lines(connection_name, delimiter)
    }
}

pub fn drain_real_serial_lines<M, R>(
    manager: &M,
    read_source: &R,
    connection_name: &str,
    delimiter: &str,
) -> Result<usize>
where
    M: SerialEventRecorder,
    R: RealSerialLineSource,
{
    let lines = read_source.drain_lines(connection_name, delimiter)?;
    let processed = lines.len();

    for line in lines {
        manager.record_serial_event_for_connection(
            connection_name,
            crate::protocol::parse_line(&line),
        );
    }

    Ok(processed)
}

#[derive(Clone, Debug, Default)]
pub struct RealReadLoopStop {
    stopped: Arc<std::sync::atomic::AtomicBool>,
}

impl RealReadLoopStop {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stop(&self) {
        self.stopped
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped.load(std::sync::atomic::Ordering::SeqCst)
    }
}

pub fn spawn_real_read_loop<M, R>(
    manager: M,
    read_source: R,
    connection_name: String,
    delimiter: String,
    stop: RealReadLoopStop,
) -> tokio::task::JoinHandle<()>
where
    M: SerialEventRecorder,
    R: RealSerialLineSource,
{
    tokio::spawn(async move {
        while !stop.is_stopped() {
            match drain_real_serial_lines(&manager, &read_source, &connection_name, &delimiter) {
                Ok(0) => tokio::time::sleep(std::time::Duration::from_millis(10)).await,
                Ok(_) => tokio::task::yield_now().await,
                Err(error) => {
                    if matches!(
                        error,
                        crate::error::SerialportApiError::ConnectionNotFound(_)
                    ) {
                        break;
                    }
                    manager.record_serial_error_for_connection(&connection_name, error.to_string());
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
            }
        }
    })
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

    #[derive(Clone, Debug, Default)]
    struct FakeRealSerialReadSource {
        bytes_by_connection: Arc<Mutex<BTreeMap<String, VecDeque<u8>>>>,
        buffers_by_connection: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    }

    impl FakeRealSerialReadSource {
        fn push_bytes(&self, connection_name: impl Into<String>, bytes: &[u8]) {
            self.bytes_by_connection
                .lock()
                .expect("fake real read source byte lock poisoned")
                .entry(connection_name.into())
                .or_default()
                .extend(bytes.iter().copied());
        }

        fn buffered_bytes(&self, connection_name: &str) -> Vec<u8> {
            self.buffers_by_connection
                .lock()
                .expect("fake real read source buffer lock poisoned")
                .get(connection_name)
                .cloned()
                .unwrap_or_default()
        }
    }

    impl RealSerialLineSource for FakeRealSerialReadSource {
        fn drain_lines(&self, connection_name: &str, delimiter: &str) -> Result<Vec<Vec<u8>>> {
            let mut bytes_by_connection = self
                .bytes_by_connection
                .lock()
                .expect("fake real read source byte lock poisoned");
            let unread = bytes_by_connection
                .entry(connection_name.to_string())
                .or_default();

            let mut buffers = self
                .buffers_by_connection
                .lock()
                .expect("fake real read source buffer lock poisoned");
            let buffer = buffers.entry(connection_name.to_string()).or_default();
            buffer.extend(unread.drain(..));

            let delimiter = delimiter.as_bytes();
            let mut lines = Vec::new();
            while let Some(index) = buffer
                .windows(delimiter.len())
                .position(|window| window == delimiter)
            {
                lines.push(buffer.drain(..index + delimiter.len()).collect());
            }

            Ok(lines)
        }
    }

    #[test]
    fn real_read_source_drains_complete_delimited_lines() {
        let source = FakeRealSerialReadSource::default();
        source.push_bytes("default", b"{\"reqId\":\"1\",\"ok\":true}\r\nhello");

        let lines = source.drain_lines("default", "\r\n").unwrap();

        assert_eq!(lines, vec![b"{\"reqId\":\"1\",\"ok\":true}\r\n".to_vec()]);
        assert_eq!(source.buffered_bytes("default"), b"hello".to_vec());
    }

    #[test]
    fn real_read_lines_record_json_response_for_waited_commands() {
        use crate::serial::manager::{ConnectionManager, InMemoryConnectionManager};

        let manager = InMemoryConnectionManager::default();
        let source = FakeRealSerialReadSource::default();
        source.push_bytes("default", b"{\"reqId\":\"abc\",\"ok\":true}\r\n");

        let processed = drain_real_serial_lines(&manager, &source, "default", "\r\n").unwrap();

        assert_eq!(processed, 1);
        assert_eq!(
            manager.take_response("default", "abc").unwrap(),
            Some(serde_json::json!({"reqId":"abc","ok":true}))
        );
        assert_eq!(manager.events().unwrap()[0].event, "serial.json");
    }

    #[tokio::test]
    async fn spawned_real_read_loop_records_lines_and_stops() {
        use crate::serial::manager::InMemoryConnectionManager;

        let manager = InMemoryConnectionManager::default();
        let source = FakeRealSerialReadSource::default();
        let stop = RealReadLoopStop::new();

        let handle = spawn_real_read_loop(
            manager.clone(),
            source.clone(),
            "default".to_string(),
            "\r\n".to_string(),
            stop.clone(),
        );

        source.push_bytes("default", b"{\"reqId\":\"loop-1\",\"ok\":true}\r\n");
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            loop {
                if manager
                    .take_response("default", "loop-1")
                    .unwrap()
                    .is_some()
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();

        stop.stop();
        handle.await.unwrap();
    }

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
