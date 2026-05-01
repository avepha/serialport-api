use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::error::{Result, SerialportApiError};
use crate::serial::manager::{
    ConnectionInfo, ConnectionManager, ConnectionManagerWithTransport, ConnectionRequest,
    QueuedCommand, SerialStreamEvent,
};
use crate::serial::read_loop::{spawn_real_read_loop, RealReadLoopStop};
use crate::serial::transport::SerialTransport;

pub const DEFAULT_SERIAL_TIMEOUT_MS: u64 = 50;
const MAX_BYTES_PER_DRAIN: usize = 8192;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerialOpenSettings {
    pub port: String,
    pub baud_rate: u32,
    pub timeout_ms: u64,
}

impl SerialOpenSettings {
    pub fn from_connection(connection: &ConnectionInfo) -> Self {
        Self {
            port: connection.port.clone(),
            baud_rate: connection.baud_rate,
            timeout_ms: DEFAULT_SERIAL_TIMEOUT_MS,
        }
    }
}

pub trait SerialPortHandle: Send + 'static {
    fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()>;
    fn flush(&mut self) -> std::io::Result<()>;
    fn read_byte(&mut self) -> std::io::Result<Option<u8>>;
}

pub trait SerialPortFactory: Clone + Send + Sync + 'static {
    type Handle: SerialPortHandle;

    fn open(&self, connection: &ConnectionInfo) -> Result<Self::Handle>;
}

type SharedHandle<H> = Arc<Mutex<H>>;
type SharedHandles<H> = Arc<Mutex<BTreeMap<String, SharedHandle<H>>>>;

#[derive(Debug)]
pub struct RealSerialTransport<F>
where
    F: SerialPortFactory,
{
    factory: F,
    handles: SharedHandles<F::Handle>,
    read_buffers: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

impl<F> Clone for RealSerialTransport<F>
where
    F: SerialPortFactory,
{
    fn clone(&self) -> Self {
        Self {
            factory: self.factory.clone(),
            handles: self.handles.clone(),
            read_buffers: self.read_buffers.clone(),
        }
    }
}

impl<F> RealSerialTransport<F>
where
    F: SerialPortFactory,
{
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            handles: Arc::default(),
            read_buffers: Arc::default(),
        }
    }

    pub fn is_open(&self, name: &str) -> bool {
        self.handles
            .lock()
            .expect("real serial handles lock poisoned")
            .contains_key(name)
    }

    pub fn drain_lines(&self, connection_name: &str, delimiter: &str) -> Result<Vec<Vec<u8>>> {
        if delimiter.is_empty() {
            return Err(SerialportApiError::InvalidConnectionRequest(
                "delimiter must not be empty".to_string(),
            ));
        }

        let handle = self
            .handles
            .lock()
            .expect("real serial handles lock poisoned")
            .get(connection_name)
            .cloned()
            .ok_or_else(|| SerialportApiError::ConnectionNotFound(connection_name.to_string()))?;

        let mut newly_read = Vec::new();
        for _ in 0..MAX_BYTES_PER_DRAIN {
            let read = handle
                .lock()
                .expect("real serial handle lock poisoned")
                .read_byte()?;
            match read {
                Some(byte) => newly_read.push(byte),
                None => break,
            }
        }

        let mut buffers = self
            .read_buffers
            .lock()
            .expect("real serial read buffer lock poisoned");
        let buffer = buffers.entry(connection_name.to_string()).or_default();
        buffer.extend(newly_read);

        let delimiter = delimiter.as_bytes();
        let mut lines = Vec::new();
        while let Some(end_index) = find_subslice(buffer, delimiter) {
            let line_end = end_index + delimiter.len();
            lines.push(buffer.drain(..line_end).collect());
        }

        Ok(lines)
    }
}

impl<F> SerialTransport for RealSerialTransport<F>
where
    F: SerialPortFactory,
{
    fn open(&self, connection: &ConnectionInfo) -> Result<()> {
        if connection.delimiter.is_empty() {
            return Err(SerialportApiError::InvalidConnectionRequest(
                "delimiter must not be empty".to_string(),
            ));
        }

        let handle = self.factory.open(connection)?;
        self.handles
            .lock()
            .expect("real serial handles lock poisoned")
            .insert(connection.name.clone(), Arc::new(Mutex::new(handle)));
        self.read_buffers
            .lock()
            .expect("real serial read buffer lock poisoned")
            .remove(&connection.name);
        Ok(())
    }

    fn close(&self, name: &str) -> Result<()> {
        self.handles
            .lock()
            .expect("real serial handles lock poisoned")
            .remove(name);
        self.read_buffers
            .lock()
            .expect("real serial read buffer lock poisoned")
            .remove(name);
        Ok(())
    }

    fn write_frame(&self, name: &str, frame: &[u8]) -> Result<()> {
        let handle = self
            .handles
            .lock()
            .expect("real serial handles lock poisoned")
            .get(name)
            .cloned()
            .ok_or_else(|| SerialportApiError::ConnectionNotFound(name.to_string()))?;

        let mut handle = handle.lock().expect("real serial handle lock poisoned");
        handle.write_all(frame)?;
        handle.flush()?;
        Ok(())
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[derive(Clone, Debug, Default)]
pub struct SystemSerialPortFactory;

impl SerialPortFactory for SystemSerialPortFactory {
    type Handle = SystemSerialPortHandle;

    fn open(&self, connection: &ConnectionInfo) -> Result<Self::Handle> {
        let settings = SerialOpenSettings::from_connection(connection);
        let port = serialport::new(settings.port, settings.baud_rate)
            .timeout(Duration::from_millis(settings.timeout_ms))
            .open()?;
        Ok(SystemSerialPortHandle { port })
    }
}

pub struct SystemSerialPortHandle {
    port: Box<dyn serialport::SerialPort>,
}

impl std::fmt::Debug for SystemSerialPortHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SystemSerialPortHandle")
            .finish_non_exhaustive()
    }
}

impl SerialPortHandle for SystemSerialPortHandle {
    fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        Write::write_all(&mut self.port, bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Write::flush(&mut self.port)
    }

    fn read_byte(&mut self) -> std::io::Result<Option<u8>> {
        let mut byte = [0_u8; 1];
        match Read::read(&mut self.port, &mut byte) {
            Ok(0) => Ok(None),
            Ok(_) => Ok(Some(byte[0])),
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => Ok(None),
            Err(error) => Err(error),
        }
    }
}

pub type SystemRealSerialTransport = RealSerialTransport<SystemSerialPortFactory>;

impl Default for SystemRealSerialTransport {
    fn default() -> Self {
        Self::new(SystemSerialPortFactory)
    }
}

pub struct RealSerialConnectionManager<F>
where
    F: SerialPortFactory,
{
    inner: ConnectionManagerWithTransport<RealSerialTransport<F>>,
    loops_by_connection: Arc<Mutex<BTreeMap<String, RealReadLoopTask>>>,
}

struct RealReadLoopTask {
    stop: RealReadLoopStop,
    handle: std::thread::JoinHandle<()>,
}

impl<F> Clone for RealSerialConnectionManager<F>
where
    F: SerialPortFactory,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            loops_by_connection: self.loops_by_connection.clone(),
        }
    }
}

impl<F> RealSerialConnectionManager<F>
where
    F: SerialPortFactory,
{
    pub fn new(transport: RealSerialTransport<F>) -> Self {
        Self {
            inner: ConnectionManagerWithTransport::new(transport),
            loops_by_connection: Arc::default(),
        }
    }

    pub fn inner(&self) -> ConnectionManagerWithTransport<RealSerialTransport<F>> {
        self.inner.clone()
    }

    fn take_read_loop_for(&self, name: &str) -> Option<RealReadLoopTask> {
        self.loops_by_connection
            .lock()
            .expect("real read-loop registry lock poisoned")
            .remove(name)
    }
}

impl<F> ConnectionManager for RealSerialConnectionManager<F>
where
    F: SerialPortFactory,
{
    fn connect(&self, request: ConnectionRequest) -> Result<ConnectionInfo> {
        if request.delimiter.is_empty() {
            return Err(SerialportApiError::InvalidConnectionRequest(
                "delimiter must not be empty".to_string(),
            ));
        }

        if let Some(loop_task) = self.take_read_loop_for(&request.name) {
            loop_task.stop.stop();
            let _ = self.inner.disconnect(&request.name);
            let _ = loop_task.handle.join();
        } else {
            let _ = self.inner.disconnect(&request.name);
        }

        let connection = self.inner.connect(request)?;
        let stop = RealReadLoopStop::new();
        let handle = spawn_real_read_loop(
            self.inner.clone(),
            self.inner.transport(),
            connection.name.clone(),
            connection.delimiter.clone(),
            stop.clone(),
        );

        self.loops_by_connection
            .lock()
            .expect("real read-loop registry lock poisoned")
            .insert(connection.name.clone(), RealReadLoopTask { stop, handle });

        Ok(connection)
    }

    fn connections(&self) -> Result<Vec<ConnectionInfo>> {
        self.inner.connections()
    }

    fn disconnect(&self, name: &str) -> Result<String> {
        let loop_task = self
            .loops_by_connection
            .lock()
            .expect("real read-loop registry lock poisoned")
            .remove(name);

        if let Some(loop_task) = loop_task {
            loop_task.stop.stop();
            let result = self.inner.disconnect(name);
            let _ = loop_task.handle.join();
            result
        } else {
            self.inner.disconnect(name)
        }
    }

    fn send_command(
        &self,
        connection_name: &str,
        payload: serde_json::Value,
    ) -> Result<QueuedCommand> {
        self.inner.send_command(connection_name, payload)
    }

    fn take_response(
        &self,
        connection_name: &str,
        req_id: &str,
    ) -> Result<Option<serde_json::Value>> {
        self.inner.take_response(connection_name, req_id)
    }

    fn events(&self) -> Result<Vec<SerialStreamEvent>> {
        self.inner.events()
    }
}

pub type SystemRealSerialConnectionManager = RealSerialConnectionManager<SystemSerialPortFactory>;

impl Default for SystemRealSerialConnectionManager {
    fn default() -> Self {
        Self::new(SystemRealSerialTransport::default())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use pretty_assertions::assert_eq;

    use super::*;
    use crate::error::SerialportApiError;
    use crate::serial::manager::ConnectionManager;

    #[derive(Clone, Default)]
    struct FakeSerialPortFactory {
        state: Arc<Mutex<FakeFactoryState>>,
    }

    #[derive(Default)]
    struct FakeFactoryState {
        opened_ports: Vec<(String, u32)>,
        handles_by_connection: BTreeMap<String, FakeSerialPortHandle>,
    }

    #[derive(Clone, Default)]
    struct FakeSerialPortHandle {
        written: Arc<Mutex<Vec<u8>>>,
        flush_count: Arc<Mutex<usize>>,
        readable: Arc<Mutex<VecDeque<u8>>>,
    }

    impl FakeSerialPortFactory {
        fn opened_ports(&self) -> Vec<(String, u32)> {
            self.state
                .lock()
                .expect("fake factory state lock poisoned")
                .opened_ports
                .clone()
        }

        fn written_for(&self, name: &str) -> Vec<u8> {
            self.state
                .lock()
                .expect("fake factory state lock poisoned")
                .handles_by_connection
                .get(name)
                .expect("expected fake handle")
                .written
                .lock()
                .expect("fake written lock poisoned")
                .clone()
        }

        fn flush_count_for(&self, name: &str) -> usize {
            *self
                .state
                .lock()
                .expect("fake factory state lock poisoned")
                .handles_by_connection
                .get(name)
                .expect("expected fake handle")
                .flush_count
                .lock()
                .expect("fake flush lock poisoned")
        }

        fn push_bytes(&self, name: &str, bytes: &[u8]) {
            let state = self.state.lock().expect("fake factory state lock poisoned");
            let mut readable = state
                .handles_by_connection
                .get(name)
                .expect("expected fake handle")
                .readable
                .lock()
                .expect("fake readable lock poisoned");
            readable.extend(bytes.iter().copied());
        }
    }

    impl SerialPortFactory for FakeSerialPortFactory {
        type Handle = FakeSerialPortHandle;

        fn open(&self, connection: &ConnectionInfo) -> Result<Self::Handle> {
            let handle = FakeSerialPortHandle::default();
            let mut state = self.state.lock().expect("fake factory state lock poisoned");
            state
                .opened_ports
                .push((connection.port.clone(), connection.baud_rate));
            state
                .handles_by_connection
                .insert(connection.name.clone(), handle.clone());
            Ok(handle)
        }
    }

    impl SerialPortHandle for FakeSerialPortHandle {
        fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
            self.written
                .lock()
                .expect("fake written lock poisoned")
                .extend_from_slice(bytes);
            Ok(())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            *self.flush_count.lock().expect("fake flush lock poisoned") += 1;
            Ok(())
        }

        fn read_byte(&mut self) -> std::io::Result<Option<u8>> {
            Ok(self
                .readable
                .lock()
                .expect("fake readable lock poisoned")
                .pop_front())
        }
    }

    fn connection() -> ConnectionInfo {
        ConnectionInfo {
            name: "default".to_string(),
            status: "connected",
            port: "/dev/ttyTEST0".to_string(),
            baud_rate: 115200,
            delimiter: "\r\n".to_string(),
        }
    }

    #[test]
    fn real_transport_opens_writes_flushes_and_closes_named_connection() {
        let factory = FakeSerialPortFactory::default();
        let transport = RealSerialTransport::new(factory.clone());
        let connection = connection();

        transport.open(&connection).unwrap();
        transport
            .write_frame("default", b"{\"reqId\":\"1\"}\r\n")
            .unwrap();
        transport.close("default").unwrap();

        assert_eq!(
            factory.opened_ports(),
            vec![("/dev/ttyTEST0".to_string(), 115200)]
        );
        assert_eq!(
            factory.written_for("default"),
            b"{\"reqId\":\"1\"}\r\n".to_vec()
        );
        assert_eq!(factory.flush_count_for("default"), 1);
        assert!(!transport.is_open("default"));
    }

    #[test]
    fn real_transport_write_missing_connection_returns_connection_not_found() {
        let transport = RealSerialTransport::new(FakeSerialPortFactory::default());

        let error = transport.write_frame("missing", b"{}").unwrap_err();

        assert!(matches!(error, SerialportApiError::ConnectionNotFound(name) if name == "missing"));
    }

    #[test]
    fn serial_open_settings_are_derived_from_connection_info() {
        let connection = ConnectionInfo {
            name: "robot".to_string(),
            status: "connected",
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: 345600,
            delimiter: "\n".to_string(),
        };

        let settings = SerialOpenSettings::from_connection(&connection);

        assert_eq!(settings.port, "/dev/ttyUSB0");
        assert_eq!(settings.baud_rate, 345600);
        assert!(settings.timeout_ms > 0);
    }

    #[test]
    fn real_transport_drains_complete_delimited_lines_and_keeps_partial_bytes() {
        let factory = FakeSerialPortFactory::default();
        let transport = RealSerialTransport::new(factory.clone());
        transport.open(&connection()).unwrap();
        factory.push_bytes("default", b"{\"ok\":true}\r\nhello");

        let lines = transport.drain_lines("default", "\r\n").unwrap();

        assert_eq!(lines, vec![b"{\"ok\":true}\r\n".to_vec()]);
        factory.push_bytes("default", b" robot\r\n");
        assert_eq!(
            transport.drain_lines("default", "\r\n").unwrap(),
            vec![b"hello robot\r\n".to_vec()]
        );
    }

    #[test]
    fn real_transport_rejects_empty_delimiter() {
        let transport = RealSerialTransport::new(FakeSerialPortFactory::default());
        let mut connection = connection();
        connection.delimiter.clear();

        let error = transport.open(&connection).unwrap_err();

        assert!(matches!(
            error,
            SerialportApiError::InvalidConnectionRequest(message)
                if message.contains("delimiter")
        ));
    }

    #[tokio::test]
    async fn real_manager_writes_reads_and_satisfies_waited_response_with_fake_handle() {
        let factory = FakeSerialPortFactory::default();
        let manager = RealSerialConnectionManager::new(RealSerialTransport::new(factory.clone()));

        manager
            .connect(ConnectionRequest {
                name: "default".to_string(),
                port: "/dev/ttyTEST0".to_string(),
                baud_rate: 115200,
                delimiter: "\r\n".to_string(),
            })
            .unwrap();

        let queued = manager
            .send_command(
                "default",
                serde_json::json!({"reqId":"fake-route-1","method":"query","topic":"ping","data":{}}),
            )
            .unwrap();
        assert_eq!(queued.req_id, "fake-route-1");
        let written = factory.written_for("default");
        assert!(written.ends_with(b"\r\n"));
        let payload: serde_json::Value =
            serde_json::from_slice(&written[..written.len() - 2]).unwrap();
        assert_eq!(payload["reqId"], "fake-route-1");
        assert_eq!(payload["topic"], "ping");
        assert_eq!(factory.flush_count_for("default"), 1);

        factory.push_bytes("default", b"{\"reqId\":\"fake-route-1\",\"ok\":true}\r\n");
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            loop {
                if manager
                    .take_response("default", "fake-route-1")
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

        manager.disconnect("default").unwrap();
    }

    #[test]
    fn real_manager_reconnect_replaces_single_read_loop_and_disconnect_joins_it() {
        let factory = FakeSerialPortFactory::default();
        let manager = RealSerialConnectionManager::new(RealSerialTransport::new(factory.clone()));

        manager
            .connect(ConnectionRequest {
                name: "default".to_string(),
                port: "/dev/ttyTEST0".to_string(),
                baud_rate: 115200,
                delimiter: "\r\n".to_string(),
            })
            .unwrap();
        assert_eq!(manager.loops_by_connection.lock().unwrap().len(), 1);

        manager
            .connect(ConnectionRequest {
                name: "default".to_string(),
                port: "/dev/ttyTEST1".to_string(),
                baud_rate: 57600,
                delimiter: "\n".to_string(),
            })
            .unwrap();
        assert_eq!(manager.loops_by_connection.lock().unwrap().len(), 1);
        assert_eq!(
            factory.opened_ports(),
            vec![
                ("/dev/ttyTEST0".to_string(), 115200),
                ("/dev/ttyTEST1".to_string(), 57600),
            ]
        );

        manager.disconnect("default").unwrap();

        assert_eq!(manager.loops_by_connection.lock().unwrap().len(), 0);
        assert!(!manager.inner().transport().is_open("default"));
    }
}
