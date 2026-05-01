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
            return Ok(Vec::new());
        }

        let handle = self
            .handles
            .lock()
            .expect("real serial handles lock poisoned")
            .get(connection_name)
            .cloned()
            .ok_or_else(|| SerialportApiError::ConnectionNotFound(connection_name.to_string()))?;

        let mut newly_read = Vec::new();
        loop {
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
    stops_by_connection: Arc<Mutex<BTreeMap<String, RealReadLoopStop>>>,
}

impl<F> Clone for RealSerialConnectionManager<F>
where
    F: SerialPortFactory,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            stops_by_connection: self.stops_by_connection.clone(),
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
            stops_by_connection: Arc::default(),
        }
    }

    pub fn inner(&self) -> ConnectionManagerWithTransport<RealSerialTransport<F>> {
        self.inner.clone()
    }
}

impl<F> ConnectionManager for RealSerialConnectionManager<F>
where
    F: SerialPortFactory,
{
    fn connect(&self, request: ConnectionRequest) -> Result<ConnectionInfo> {
        let connection = self.inner.connect(request)?;
        let stop = RealReadLoopStop::new();
        self.stops_by_connection
            .lock()
            .expect("real read-loop stop registry lock poisoned")
            .insert(connection.name.clone(), stop.clone());

        spawn_real_read_loop(
            self.inner.clone(),
            self.inner.transport(),
            connection.name.clone(),
            connection.delimiter.clone(),
            stop,
        );

        Ok(connection)
    }

    fn connections(&self) -> Result<Vec<ConnectionInfo>> {
        self.inner.connections()
    }

    fn disconnect(&self, name: &str) -> Result<String> {
        if let Some(stop) = self
            .stops_by_connection
            .lock()
            .expect("real read-loop stop registry lock poisoned")
            .remove(name)
        {
            stop.stop();
        }
        self.inner.disconnect(name)
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
}
