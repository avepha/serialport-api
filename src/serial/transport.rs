use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use crate::error::Result;
use crate::serial::manager::ConnectionInfo;

pub trait SerialTransport: Clone + Send + Sync + 'static {
    fn open(&self, connection: &ConnectionInfo) -> Result<()>;
    fn close(&self, name: &str) -> Result<()>;
    fn write_frame(&self, name: &str, frame: &[u8]) -> Result<()>;
}

#[derive(Clone, Debug, Default)]
pub struct MockSerialTransport {
    open_connections: Arc<Mutex<BTreeSet<String>>>,
    closed_connections: Arc<Mutex<Vec<String>>>,
    written_frames: Arc<Mutex<BTreeMap<String, Vec<Vec<u8>>>>>,
}

impl MockSerialTransport {
    pub fn opened_names(&self) -> Vec<String> {
        self.open_connections
            .lock()
            .expect("mock serial transport open connections lock poisoned")
            .iter()
            .cloned()
            .collect()
    }

    pub fn closed_names(&self) -> Vec<String> {
        self.closed_connections
            .lock()
            .expect("mock serial transport closed connections lock poisoned")
            .clone()
    }

    pub fn written_frames(&self, name: &str) -> Vec<Vec<u8>> {
        self.written_frames
            .lock()
            .expect("mock serial transport written frames lock poisoned")
            .get(name)
            .cloned()
            .unwrap_or_default()
    }
}

impl SerialTransport for MockSerialTransport {
    fn open(&self, connection: &ConnectionInfo) -> Result<()> {
        self.open_connections
            .lock()
            .expect("mock serial transport open connections lock poisoned")
            .insert(connection.name.clone());

        Ok(())
    }

    fn close(&self, name: &str) -> Result<()> {
        self.open_connections
            .lock()
            .expect("mock serial transport open connections lock poisoned")
            .remove(name);
        self.closed_connections
            .lock()
            .expect("mock serial transport closed connections lock poisoned")
            .push(name.to_string());

        Ok(())
    }

    fn write_frame(&self, name: &str, frame: &[u8]) -> Result<()> {
        self.written_frames
            .lock()
            .expect("mock serial transport written frames lock poisoned")
            .entry(name.to_string())
            .or_default()
            .push(frame.to_vec());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::serial::manager::ConnectionInfo;

    fn connection() -> ConnectionInfo {
        ConnectionInfo {
            name: "default".to_string(),
            status: "connected",
            port: "/dev/ROBOT".to_string(),
            baud_rate: 115200,
            delimiter: "\r\n".to_string(),
        }
    }

    #[test]
    fn mock_transport_records_open_close_and_written_frames() {
        let transport = MockSerialTransport::default();
        let connection = connection();

        transport.open(&connection).unwrap();
        transport
            .write_frame("default", b"{\"topic\":\"ping\"}\r\n")
            .unwrap();
        transport.close("default").unwrap();

        assert_eq!(transport.opened_names(), Vec::<String>::new());
        assert_eq!(transport.closed_names(), vec!["default".to_string()]);
        assert_eq!(
            transport.written_frames("default"),
            vec![b"{\"topic\":\"ping\"}\r\n".to_vec()]
        );
    }
}
