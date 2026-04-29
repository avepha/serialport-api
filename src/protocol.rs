use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RequestMethod {
    Query,
    Mutation,
    Log,
    Notification,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Command {
    #[serde(rename = "reqId", skip_serializing_if = "Option::is_none")]
    pub req_id: Option<String>,
    pub method: Option<String>,
    pub topic: Option<String>,
    #[serde(default)]
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SerialEvent {
    Json(Value),
    Text(String),
    Log(Value),
    Notification(Value),
}

pub fn frame_json(value: &Value, delimiter: &str) -> Result<Vec<u8>> {
    let mut encoded = serde_json::to_vec(value)?;
    encoded.extend_from_slice(delimiter.as_bytes());
    Ok(encoded)
}

pub fn parse_line(line: &[u8]) -> SerialEvent {
    let trimmed = trim_line_delimiter(line);
    let text = match std::str::from_utf8(trimmed) {
        Ok(text) => text,
        Err(_) => return SerialEvent::Text(String::from_utf8_lossy(trimmed).to_string()),
    };

    match serde_json::from_str::<Value>(text) {
        Ok(value) => match value.get("method").and_then(Value::as_str) {
            Some("log") => SerialEvent::Log(value),
            Some("notification") => SerialEvent::Notification(value),
            _ => SerialEvent::Json(value),
        },
        Err(_) => SerialEvent::Text(text.to_string()),
    }
}

fn trim_line_delimiter(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r\n")
        .or_else(|| line.strip_suffix(b"\n"))
        .or_else(|| line.strip_suffix(b"\r"))
        .unwrap_or(line)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;

    #[test]
    fn frames_json_with_crlf() {
        let bytes = frame_json(&json!({"topic":"ping"}), "\r\n").unwrap();
        assert_eq!(bytes, b"{\"topic\":\"ping\"}\r\n".to_vec());
    }

    #[test]
    fn parses_json_line_and_preserves_req_id() {
        let event = parse_line(
            br#"{"reqId":"1","ok":true}
"#,
        );
        assert_eq!(event, SerialEvent::Json(json!({"reqId":"1","ok":true})));
    }

    #[test]
    fn parses_log_line() {
        let event = parse_line(
            br#"{"method":"log","data":{"level":"info"}}
"#,
        );
        assert!(matches!(event, SerialEvent::Log(_)));
    }

    #[test]
    fn parses_notification_line() {
        let event = parse_line(
            br#"{"method":"notification","data":[]}
"#,
        );
        assert!(matches!(event, SerialEvent::Notification(_)));
    }

    #[test]
    fn parses_text_line() {
        let event = parse_line(b"hello robot\r\n");
        assert_eq!(event, SerialEvent::Text("hello robot".to_string()));
    }

    #[test]
    fn parses_non_utf8_as_lossy_text() {
        let event = parse_line(&[0xff, b'o', b'k', b'\r', b'\n']);
        assert_eq!(event, SerialEvent::Text("�ok".to_string()));
    }
}
