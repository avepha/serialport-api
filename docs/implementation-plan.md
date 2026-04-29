# Rust Serialport API Rewrite Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Turn the current serialport prototype into a high-quality open-source Rust service for JSON-based serial communication, compatible with the useful behavior from `sg-mcu-com`.

**Architecture:** Build a reusable core library for protocol parsing and serial connection management, plus an Axum HTTP/SSE server binary. Start with testable protocol code and mockable serial abstractions before adding hardware-dependent behavior.

**Tech Stack:** Rust 2021, Tokio, Axum, Serde, Serde JSON, Thiserror, Tracing, Clap, serialport/tokio-serial, optional SQLite later.

---

## Phase 0: Repository Baseline

### Task 0.1: Create rewrite branch

**Objective:** Avoid working directly on `master`.

**Files:** none

**Steps:**

```bash
git checkout -b rewrite/axum-serial-api
```

**Verify:**

```bash
git status --short --branch
```

Expected branch:

```text
## rewrite/axum-serial-api
```

### Task 0.2: Commit planning docs

**Objective:** Preserve the spec and implementation plan as the project direction.

**Files:**
- Add: `docs/open-source-spec.md`
- Add: `docs/implementation-plan.md`

**Steps:**

```bash
git add docs/open-source-spec.md docs/implementation-plan.md
git commit -m "docs: define open source serialport api rewrite"
```

**Verify:**

```bash
git log -1 --oneline
```

---

## Phase 1: Modernize the Rust Project

### Task 1.1: Update Cargo metadata and remove old Rocket

**Objective:** Make the project build on stable Rust and prepare it for open source.

**Files:**
- Modify: `Cargo.toml`

**Change:**

Replace current package/dependency metadata with modern metadata and remove `rocket = "0.4.5"`.

Suggested initial `Cargo.toml`:

```toml
[package]
name = "serialport-api"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
authors = ["Farhan Poh-Asae <farhan.abuhazan@gmail.com>"]
license = "MIT"
description = "JSON-based serial port communication service for microcontrollers and Raspberry Pi"
repository = "https://github.com/avepha/serialport-api"
readme = "README.md"
keywords = ["serialport", "microcontroller", "raspberry-pi", "json", "api"]
categories = ["command-line-utilities", "embedded", "web-programming::http-server"]

[dependencies]
serialport = "4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
pretty_assertions = "1"
```

**Verify:**

```bash
cargo check
```

Expected: success.

### Task 1.2: Split prototype into library and binary

**Objective:** Make core behavior testable.

**Files:**
- Create: `src/lib.rs`
- Modify: `src/main.rs`

**Steps:**

Create `src/lib.rs`:

```rust
pub mod error;
pub mod protocol;
```

Replace `src/main.rs` with a minimal placeholder:

```rust
fn main() {
    println!("serialport-api: rewrite in progress");
}
```

**Verify:**

```bash
cargo check
```

Expected: success.

---

## Phase 2: Core Protocol

### Task 2.1: Add typed error module

**Objective:** Centralize errors.

**Files:**
- Create: `src/error.rs`

**Implementation:**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SerialportApiError {
    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("invalid UTF-8 serial line")]
    InvalidUtf8,

    #[error("command timed out")]
    CommandTimeout,
}

pub type Result<T> = std::result::Result<T, SerialportApiError>;
```

**Verify:**

```bash
cargo check
```

### Task 2.2: Define protocol message types

**Objective:** Represent serial input as structured events.

**Files:**
- Create: `src/protocol.rs`

**Implementation:**

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
```

**Verify:**

```bash
cargo check
```

### Task 2.3: Add protocol tests

**Objective:** Lock in compatibility with JSON-over-CRLF behavior.

**Files:**
- Modify: `src/protocol.rs`

**Add tests at bottom:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn frames_json_with_crlf() {
        let bytes = frame_json(&json!({"topic":"ping"}), "\r\n").unwrap();
        assert_eq!(bytes, br#"{"topic":"ping"}
"#.to_vec());
    }

    #[test]
    fn parses_json_line() {
        let event = parse_line(br#"{"reqId":"1","ok":true}
"#);
        assert_eq!(event, SerialEvent::Json(json!({"reqId":"1","ok":true})));
    }

    #[test]
    fn parses_log_line() {
        let event = parse_line(br#"{"method":"log","data":{"level":"info"}}
"#);
        assert!(matches!(event, SerialEvent::Log(_)));
    }

    #[test]
    fn parses_notification_line() {
        let event = parse_line(br#"{"method":"notification","data":[]}
"#);
        assert!(matches!(event, SerialEvent::Notification(_)));
    }

    #[test]
    fn parses_text_line() {
        let event = parse_line(b"hello robot\r\n");
        assert_eq!(event, SerialEvent::Text("hello robot".to_string()));
    }
}
```

**Verify:**

```bash
cargo test
```

Expected: all tests pass.

---

## Phase 3: HTTP API Skeleton

### Task 3.1: Add Axum dependencies

**Objective:** Prepare the HTTP server.

**Files:**
- Modify: `Cargo.toml`

**Add:**

```toml
axum = "0.7"
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive", "env"] }
```

**Verify:**

```bash
cargo check
```

### Task 3.2: Add health route

**Objective:** First working web API endpoint.

**Files:**
- Create: `src/api/mod.rs`
- Create: `src/api/routes.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`

**Expected endpoint:**

```text
GET /api/v1/health
```

Response:

```json
{"status":"ok","version":"0.1.0"}
```

**Verify:**

```bash
cargo run -- serve --host 127.0.0.1 --port 4002
curl http://127.0.0.1:4002/api/v1/health
```

---

## Phase 4: Serial Manager

### Task 4.1: Add port listing

**Objective:** Implement equivalent of old `GET /list`.

**Files:**
- Create: `src/serial/mod.rs`
- Create: `src/serial/manager.rs`
- Modify: `src/api/routes.rs`

**API:**

```text
GET /api/v1/ports
GET /list    # compatibility alias
```

**Verify:**

```bash
cargo test
cargo run -- serve
curl http://127.0.0.1:4002/api/v1/ports
```

### Task 4.2: Add connection lifecycle

**Objective:** Implement connect/disconnect/info.

**API:**

```text
POST /api/v1/connections
GET /api/v1/connections
DELETE /api/v1/connections/{name}
```

Compatibility aliases:

```text
GET  /info
POST /connect
POST /disconnect
```

**Verify:** use mock mode first before hardware.

---

## Phase 5: Command Sending and Events

### Task 5.1: Implement command endpoint

**Objective:** Implement equivalent of old `POST /commit`.

**API:**

```text
POST /api/v1/connections/{name}/commands
POST /commit    # compatibility alias
```

**Behavior:**

- Accept JSON payload.
- Add `reqId` if missing.
- Frame as JSON + `\r\n`.
- Write to serial connection.
- Optionally wait for matching response.

### Task 5.2: Implement event stream

**Objective:** Replace Socket.IO events with open standard SSE first.

**API:**

```text
GET /api/v1/events
```

**Events:**

- `serial.json`
- `serial.text`
- `serial.log`
- `serial.notification`
- `serial.error`

---

## Phase 6: Open Source Polish

### Task 6.1: README rewrite

**Objective:** Make the project understandable to new users.

**README sections:**

- What is serialport-api?
- Why Rust?
- Quick start
- API examples
- Raspberry Pi setup
- Compatibility with `sg-mcu-com`
- Roadmap

### Task 6.2: CI

**Objective:** Ensure every PR is checked.

**Files:**
- Create: `.github/workflows/ci.yml`

**Checks:**

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

---

## Immediate Next Action

Start with Phase 0, then implement Phase 1 and Phase 2 before touching the API server.

This keeps the rewrite grounded and testable:

1. branch
2. commit docs
3. remove Rocket
4. make stable Rust build pass
5. add protocol parser + tests
