use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::error::Result;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SerialReadItem {
    Line(Vec<u8>),
    Error(String),
}

pub trait SerialReadSource: Clone + Send + Sync + 'static {
    fn drain_items(&self, connection_name: &str) -> Result<Vec<SerialReadItem>>;
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
}
