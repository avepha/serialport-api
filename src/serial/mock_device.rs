use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Clone, Debug, Default)]
pub struct MockDeviceResponder {
    script: MockResponseScript,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct MockResponseScript {
    #[serde(default)]
    pub responses: Vec<MockResponseRule>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MockResponseRule {
    pub topic: String,
    pub response: Value,
}

impl MockResponseScript {
    pub fn from_json_str(input: &str) -> serde_json::Result<Self> {
        serde_json::from_str(input)
    }

    pub fn responses(&self) -> &[MockResponseRule] {
        &self.responses
    }

    fn response_for_topic(&self, topic: &str, req_id: &str) -> Option<Value> {
        let rule = self.responses.iter().find(|rule| rule.topic == topic)?;
        let mut response = rule.response.clone();
        let object = response.as_object_mut()?;
        if !matches!(object.get("reqId"), Some(Value::String(_))) {
            object.insert("reqId".to_string(), Value::String(req_id.to_string()));
        }
        Some(response)
    }
}

impl MockDeviceResponder {
    pub fn from_script(script: MockResponseScript) -> Self {
        Self { script }
    }

    pub fn response_for_frame(&self, frame: &[u8], delimiter: &str) -> Option<Value> {
        let body = trim_frame_delimiter(frame, delimiter);
        let command: Value = serde_json::from_slice(body).ok()?;
        let object = command.as_object()?;
        let req_id = object.get("reqId")?.as_str()?;
        let topic = object.get("topic").and_then(Value::as_str);

        if let Some(topic) = topic {
            if let Some(scripted) = self.script.response_for_topic(topic, req_id) {
                return Some(scripted);
            }
        }

        Some(default_ack_response(req_id, topic))
    }
}

fn trim_frame_delimiter<'a>(frame: &'a [u8], delimiter: &str) -> &'a [u8] {
    frame
        .strip_suffix(delimiter.as_bytes())
        .or_else(|| frame.strip_suffix(b"\r\n"))
        .or_else(|| frame.strip_suffix(b"\n"))
        .or_else(|| frame.strip_suffix(b"\r"))
        .unwrap_or(frame)
}

fn default_ack_response(req_id: &str, topic: Option<&str>) -> Value {
    match topic {
        Some(topic) => json!({
            "reqId": req_id,
            "ok": true,
            "data": {"mock": true, "topic": topic}
        }),
        None => json!({
            "reqId": req_id,
            "ok": true,
            "data": {"mock": true, "topic": null}
        }),
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn default_mock_device_acks_command_with_same_req_id() {
        let responder = MockDeviceResponder::default();

        let response = responder
            .response_for_frame(
                b"{\"reqId\":\"1\",\"method\":\"query\",\"topic\":\"sensor.read\",\"data\":{}}\r\n",
                "\r\n",
            )
            .unwrap();

        assert_eq!(
            response,
            serde_json::json!({
                "reqId": "1",
                "ok": true,
                "data": {"mock": true, "topic": "sensor.read"}
            })
        );
    }

    #[test]
    fn default_mock_device_ignores_frames_without_string_req_id() {
        let responder = MockDeviceResponder::default();

        assert_eq!(
            responder.response_for_frame(b"{\"topic\":\"sensor.read\"}\r\n", "\r\n"),
            None
        );
        assert_eq!(
            responder.response_for_frame(b"{\"reqId\":1,\"topic\":\"sensor.read\"}\r\n", "\r\n"),
            None
        );
    }

    #[test]
    fn scripted_mock_device_matches_response_by_topic_and_injects_req_id() {
        let responder = MockDeviceResponder::from_script(MockResponseScript {
            responses: vec![MockResponseRule {
                topic: "sensor.read".to_string(),
                response: serde_json::json!({"ok": true, "data": {"temperature": 28.5}}),
            }],
        });

        let response = responder
            .response_for_frame(
                b"{\"reqId\":\"client-99\",\"method\":\"query\",\"topic\":\"sensor.read\",\"data\":{}}\r\n",
                "\r\n",
            )
            .unwrap();

        assert_eq!(
            response,
            serde_json::json!({
                "reqId": "client-99",
                "ok": true,
                "data": {"temperature": 28.5}
            })
        );
    }

    #[test]
    fn scripted_mock_device_falls_back_to_default_ack_for_unknown_topic() {
        let responder = MockDeviceResponder::from_script(MockResponseScript {
            responses: vec![MockResponseRule {
                topic: "sensor.read".to_string(),
                response: serde_json::json!({"ok": true}),
            }],
        });

        let response = responder
            .response_for_frame(
                b"{\"reqId\":\"2\",\"topic\":\"unknown.topic\",\"data\":{}}\r\n",
                "\r\n",
            )
            .unwrap();

        assert_eq!(
            response,
            serde_json::json!({
                "reqId": "2",
                "ok": true,
                "data": {"mock": true, "topic": "unknown.topic"}
            })
        );
    }
}
