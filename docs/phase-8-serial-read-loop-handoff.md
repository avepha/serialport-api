# Phase 8 Background Serial Read Loop and SSE Event Feed Handoff

> **For Hermes / next AI coding session:** Execute this in a fresh session. Load `writing-plans`, `test-driven-development`, and `rust-axum-api-tdd` before editing. This phase should add a testable background serial read-loop foundation that feeds parsed serial input into the existing SSE event store. Keep the scope narrow. Do not add waited command responses, reqId matching, SQLite, config files, packaging, or WebSocket/Socket.IO in this phase.

**Goal:** Add a hardware-free, testable background serial read-loop path so incoming serial lines can be parsed with the existing protocol parser and emitted through the existing `/api/v1/events` SSE endpoint.

**Architecture:** Keep HTTP routes and response shapes stable. Extend the Phase 7 transport/manager boundary with a narrow read-loop abstraction that can be driven by mock incoming lines in tests. The manager should remain responsible for connection registry state and event storage; the read loop should parse incoming bytes with `protocol::parse_line` and record `SerialStreamEvent`s through the manager.

**Tech Stack:** Rust 2021, Axum 0.7, Tokio 1, Serde/Serde JSON, Thiserror, Tracing, `serialport`, `tokio-stream`, test-first Rust unit tests and Axum route tests.

---

## Current Repository State

Repository path:

```bash
/home/alfarie/repos/serialport-api
```

Expected branch:

```bash
rewrite/axum-serial-api
```

Latest known relevant commits:

```text
2e14727 feat: add serial transport boundary
52a8b51 docs: add phase 7 serial transport handoff
8fc5e77 ci: add Rust verification workflow
0961b3d docs: rewrite project README
aed3c77 docs: add phase 6.1 README handoff
6109494 feat: add serial event stream endpoint
abb9a8c feat: add mock command endpoint
34d7aec feat: add legacy connection aliases
8f66424 feat: add mock connection lifecycle endpoints
6edb02d feat: add serial protocol parsing foundation
```

Completed functionality as of this handoff:

- Rust 2021 crate metadata exists in `Cargo.toml`.
- Library/binary split exists:
  - `src/lib.rs`
  - `src/main.rs`
  - `src/protocol.rs`
  - `src/error.rs`
  - `src/api/routes.rs`
  - `src/serial/manager.rs`
  - `src/serial/transport.rs`
- `cargo run -- serve --host 127.0.0.1 --port 4002` starts the Axum server.
- Current default server is still mock-backed and hardware-safe.
- Port listing uses `serialport::available_ports()`.
- Command endpoint generates or preserves `reqId`, frames JSON with the connection delimiter, writes through `SerialTransport`, and returns `queued`.
- Event endpoint streams recorded in-memory manager events as SSE.
- Phase 7 introduced:
  - `SerialTransport` trait with `open`, `close`, and `write_frame`.
  - `MockSerialTransport` with snapshots for opened names, closed names, and written frames.
  - `ConnectionManagerWithTransport<T>`.
  - `InMemoryConnectionManager = ConnectionManagerWithTransport<MockSerialTransport>`.
- CI workflow exists at `.github/workflows/ci.yml`.
- Latest full local CI-equivalent verification before this handoff passed with 22 tests.

Important local toolchain note:

```bash
# In this WSL environment, prefer rustup's toolchain first in PATH.
# Otherwise /usr/bin rustc/cargo 1.75 may mix with rustup clippy 1.95 and cause E0786 metadata errors.
export PATH="$HOME/.cargo/bin:$PATH"
```

Baseline verification before starting:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Expected:

- Branch is `rewrite/axum-serial-api`.
- Working tree is clean.
- All cargo commands pass.
- Current baseline is `22 passed; 0 failed`.

---

## Phase 8 Scope

This phase should create the foundation for receiving serial input and publishing it as SSE events while preserving deterministic, hardware-free tests.

Do in Phase 8:

- Add a testable read-loop abstraction for incoming serial lines.
- Add a mock input/read source that can feed byte lines without real hardware.
- Parse incoming lines with `crate::protocol::parse_line`.
- Record parsed events through the existing manager event store:
  - `serial.json`
  - `serial.text`
  - `serial.log`
  - `serial.notification`
  - `serial.error` for read-loop errors if modeled in this phase.
- Start read-loop behavior when a connection is opened, but only in a mock/test-safe way unless a real reader can be isolated cleanly.
- Preserve the current HTTP API and legacy alias behavior exactly.
- Preserve command write behavior from Phase 7.
- Use TDD: write focused failing unit tests before implementation.

Do **not** do in Phase 8:

- command `waitForResponse` behavior
- request/response matching by `reqId`
- command timeout handling beyond preserving accepted request fields
- persistent response queues
- SQLite / preset storage
- config file loading
- GitHub Actions changes, unless fixing a discovered CI issue
- Docker/systemd/Raspberry Pi packaging
- WebSocket or Socket.IO support
- hardware-dependent tests
- blocking route handlers waiting for serial data
- API response shape changes

If a tempting change requires request matching, command waiting, or a real hardware lifecycle policy, leave it for Phase 9 or later.

---

## Current Code to Understand First

Read these files before editing:

```bash
sed -n '1,220p' src/serial/transport.rs
sed -n '1,280p' src/serial/manager.rs
sed -n '280,580p' src/serial/manager.rs
sed -n '1,180p' src/protocol.rs
sed -n '1,280p' src/api/routes.rs
sed -n '520,720p' src/api/routes.rs
sed -n '1,120p' src/error.rs
sed -n '1,80p' src/serial/mod.rs
sed -n '1,90p' Cargo.toml
```

Key current facts:

- `SerialTransport` currently only covers connection lifecycle and command writes:

```rust
pub trait SerialTransport: Clone + Send + Sync + 'static {
    fn open(&self, connection: &ConnectionInfo) -> Result<()>;
    fn close(&self, name: &str) -> Result<()>;
    fn write_frame(&self, name: &str, frame: &[u8]) -> Result<()>;
}
```

- `MockSerialTransport` records opened names, closed names, and written frames.
- `ConnectionManagerWithTransport<T>` currently owns:
  - `connections: Arc<Mutex<BTreeMap<String, ConnectionInfo>>>`
  - `next_req_id: Arc<Mutex<u64>>`
  - `events: Arc<Mutex<Vec<SerialStreamEvent>>>`
  - `transport: T`
- `record_event` and `record_error` already exist on `ConnectionManagerWithTransport<T>`.
- `events()` returns the stored `SerialStreamEvent`s.
- `protocol::parse_line` already handles:
  - JSON values as `SerialEvent::Json`
  - non-JSON UTF-8 as `SerialEvent::Text`
  - invalid UTF-8 as lossy text
  - JSON with `method: "log"` as `SerialEvent::Log`
  - JSON with `method: "notification"` as `SerialEvent::Notification`
- `/api/v1/events` already converts stored manager events to SSE.

---

## Recommended Design

### Preferred narrow design

Add a separate reader/read-loop module rather than bloating `SerialTransport` immediately. This keeps Phase 7's transport simple and avoids making every writer transport implement async/background read behavior too soon.

Create:

```text
src/serial/read_loop.rs
```

Expose it from:

```rust
// src/serial/mod.rs
pub mod manager;
pub mod read_loop;
pub mod transport;
```

### Read event type

Use an explicit input item type so tests can model successful lines and read errors without hardware:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SerialReadItem {
    Line(Vec<u8>),
    Error(String),
}
```

### Mock read source

A minimal mock source can just own a queue of items:

```rust
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Default)]
pub struct MockSerialReadSource {
    items_by_connection: Arc<Mutex<BTreeMap<String, VecDeque<SerialReadItem>>>>,
}
```

Suggested methods:

```rust
impl MockSerialReadSource {
    pub fn push_line(&self, connection_name: impl Into<String>, line: impl Into<Vec<u8>>) {
        // store SerialReadItem::Line
    }

    pub fn push_error(&self, connection_name: impl Into<String>, message: impl Into<String>) {
        // store SerialReadItem::Error
    }

    pub fn drain_items(&self, connection_name: &str) -> Vec<SerialReadItem> {
        // remove and return all queued items for the connection
    }
}
```

This is intentionally not a real serial reader yet. It gives the manager/read loop a hardware-free seam.

### Read-loop processor

Start with a synchronous, deterministic processor that drains currently queued mock items and records events. This can later be replaced or wrapped by a background Tokio task.

Recommended function shape:

```rust
pub fn drain_serial_read_items<M, R>(
    manager: &M,
    read_source: &R,
    connection_name: &str,
) -> crate::error::Result<usize>
where
    M: SerialEventRecorder,
    R: SerialReadSource,
{
    // drain items, parse line items, record events/errors, return number processed
}
```

To keep this generic and testable, add tiny traits if needed:

```rust
pub trait SerialReadSource: Clone + Send + Sync + 'static {
    fn drain_items(&self, connection_name: &str) -> crate::error::Result<Vec<SerialReadItem>>;
}

pub trait SerialEventRecorder: Clone + Send + Sync + 'static {
    fn record_serial_event(&self, event: crate::protocol::SerialEvent);
    fn record_serial_error(&self, message: String);
}
```

Then implement `SerialEventRecorder` for `ConnectionManagerWithTransport<T>` by delegating to existing `record_event` / `record_error`.

Alternative acceptable approach:

- Add a method directly on `ConnectionManagerWithTransport<T>` such as:

```rust
pub fn drain_read_source<R>(&self, read_source: &R, connection_name: &str) -> Result<usize>
where
    R: SerialReadSource,
{
    ...
}
```

This is simpler and acceptable if it stays small and testable.

### Background task boundary

Only after the deterministic drain processor is tested, add an optional task starter. Keep it small and mockable:

```rust
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
```

However, if adding a spawned task complicates tests or lifetime bounds, defer actual spawning and complete Phase 8 with the deterministic drain processor plus manager integration tests. The important deliverable is the event-feed path from incoming lines to SSE.

### Real serial read skeleton guidance

Do **not** require real hardware in Phase 8.

If adding a real type, keep it isolated and optional:

```rust
#[derive(Clone, Debug, Default)]
pub struct SystemSerialReadSource;
```

Acceptable Phase 8 behavior:

1. **Preferred safest option:** define `SystemSerialReadSource` but leave real reading unimplemented/deferred.
2. **Acceptable if simple:** return a clear error from `drain_items` like `SerialReadUnsupported` until persistent handles and blocking read policy are designed.
3. **Avoid:** opening real serial ports in tests, spawning permanent blocking read threads, or coupling real port handles into HTTP route tests.

If a real read source needs new error variants, add only the minimal variants needed. Avoid broad error refactors.

---

## Compatibility Requirements

After Phase 8, these must still work exactly as before:

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}'
```

```json
{"status":"connected","connection":{"name":"default","status":"connected","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}}
```

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"method":"query","topic":"sensor.read","data":{}},"waitForResponse":false}'
```

```json
{"status":"queued","reqId":"1"}
```

```bash
curl -s -X POST http://127.0.0.1:4002/commit \
  -H 'content-type: application/json' \
  -d '{"reqId":"client-42","method":"query","topic":"sensor.read","data":{}}'
```

```json
{"status":"queued","reqId":"client-42"}
```

`GET /api/v1/events` must still return `content-type: text/event-stream` and stream events already stored on the manager.

---

## Bite-Sized TDD Tasks

### Task 1: Add a mock serial read source module

**Objective:** Create a hardware-free read source that can queue and drain incoming serial line items.

**Files:**

- Create: `src/serial/read_loop.rs`
- Modify: `src/serial/mod.rs`

**Step 1: Write failing tests**

In `src/serial/read_loop.rs`, start with tests like:

```rust
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
```

**Step 2: Run test to verify RED**

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test serial::read_loop::tests::mock_read_source_drains_lines_and_errors_for_named_connection -- --nocapture
```

Expected: FAIL because `read_loop` module/types do not exist yet.

**Step 3: Implement minimal read source**

Implement:

- `SerialReadItem`
- `SerialReadSource` trait
- `MockSerialReadSource`
- `push_line`
- `push_error`
- `drain_items`

Modify `src/serial/mod.rs`:

```rust
pub mod manager;
pub mod read_loop;
pub mod transport;
```

**Step 4: Run test to verify GREEN**

```bash
cargo test serial::read_loop::tests::mock_read_source_drains_lines_and_errors_for_named_connection -- --nocapture
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src/serial/mod.rs src/serial/read_loop.rs
git commit -m "feat: add mock serial read source"
```

---

### Task 2: Parse drained read items into recorded manager events

**Objective:** Prove incoming mock serial lines become manager `SerialStreamEvent`s using `protocol::parse_line`.

**Files:**

- Modify: `src/serial/read_loop.rs`
- Modify if needed: `src/serial/manager.rs`

**Step 1: Write failing test**

Add a test like:

```rust
#[test]
fn drain_read_items_records_parsed_events_on_manager() {
    let manager = crate::serial::manager::InMemoryConnectionManager::default();
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
            crate::serial::manager::SerialStreamEvent {
                event: "serial.json",
                data: serde_json::json!({"reqId":"1","ok":true}),
            },
            crate::serial::manager::SerialStreamEvent {
                event: "serial.text",
                data: serde_json::json!("hello robot"),
            },
            crate::serial::manager::SerialStreamEvent {
                event: "serial.log",
                data: serde_json::json!({"method":"log","data":{"level":"info"}}),
            },
            crate::serial::manager::SerialStreamEvent {
                event: "serial.notification",
                data: serde_json::json!({"method":"notification","data":[]}),
            },
        ]
    );
}
```

**Step 2: Run test to verify RED**

```bash
cargo test serial::read_loop::tests::drain_read_items_records_parsed_events_on_manager -- --nocapture
```

Expected: FAIL because `drain_serial_read_items` or event recording integration does not exist yet.

**Step 3: Implement minimal drain processor**

Implement `drain_serial_read_items` so that:

- It drains items for the named connection.
- For `SerialReadItem::Line(line)`, it calls `crate::protocol::parse_line(&line)`.
- It records parsed events on the manager.
- It returns the number of processed items.

If needed, add a trait:

```rust
pub trait SerialEventRecorder: Clone + Send + Sync + 'static {
    fn record_serial_event(&self, event: crate::protocol::SerialEvent);
    fn record_serial_error(&self, message: String);
}
```

And implement it for `ConnectionManagerWithTransport<T>` where `T: SerialTransport`.

**Step 4: Run test to verify GREEN**

```bash
cargo test serial::read_loop::tests::drain_read_items_records_parsed_events_on_manager -- --nocapture
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src/serial/read_loop.rs src/serial/manager.rs
git commit -m "feat: record parsed serial read events"
```

---

### Task 3: Record read errors as `serial.error` events

**Objective:** Ensure read-loop errors are visible via the same event stream without crashing the processor.

**Files:**

- Modify: `src/serial/read_loop.rs`
- Modify if needed: `src/serial/manager.rs`

**Step 1: Write failing test**

```rust
#[test]
fn drain_read_items_records_errors_as_serial_error_events() {
    let manager = crate::serial::manager::InMemoryConnectionManager::default();
    let source = MockSerialReadSource::default();

    source.push_error("default", "serial read failed");

    let processed = drain_serial_read_items(&manager, &source, "default").unwrap();

    assert_eq!(processed, 1);
    assert_eq!(
        manager.events().unwrap(),
        vec![crate::serial::manager::SerialStreamEvent {
            event: "serial.error",
            data: serde_json::json!("serial read failed"),
        }]
    );
}
```

**Step 2: Run test to verify RED**

```bash
cargo test serial::read_loop::tests::drain_read_items_records_errors_as_serial_error_events -- --nocapture
```

Expected: FAIL until error items are recorded.

**Step 3: Implement minimal error recording**

Handle `SerialReadItem::Error(message)` by recording a `serial.error` event using `record_error` / `record_serial_error`.

**Step 4: Run test to verify GREEN**

```bash
cargo test serial::read_loop::tests::drain_read_items_records_errors_as_serial_error_events -- --nocapture
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src/serial/read_loop.rs src/serial/manager.rs
git commit -m "feat: record serial read errors"
```

---

### Task 4: Prove read-loop-fed events are streamed by the existing SSE route

**Objective:** Verify the existing `/api/v1/events` route streams events produced from drained serial input.

**Files:**

- Modify only if needed: `src/api/routes.rs`
- Test: `src/api/routes.rs`

**Step 1: Write failing or compatibility test**

Add a route test that uses the manager and read source, drains mock lines into the manager, then calls `/api/v1/events`:

```rust
#[tokio::test]
async fn events_route_streams_read_loop_recorded_events_as_sse() {
    let manager = InMemoryConnectionManager::default();
    let source = crate::serial::read_loop::MockSerialReadSource::default();

    source.push_line("default", b"{\"reqId\":\"1\",\"ok\":true}\r\n".to_vec());
    source.push_line("default", b"hello robot\n".to_vec());
    crate::serial::read_loop::drain_serial_read_items(&manager, &source, "default").unwrap();

    let response = router_with_state(AppState {
        port_lister: MockPortLister { ports: Vec::new() },
        connection_manager: manager,
    })
    .oneshot(
        Request::builder()
            .uri("/api/v1/events")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = std::str::from_utf8(&body).unwrap();

    assert!(body.contains("event: serial.json"));
    assert!(body.contains("data: {\"ok\":true,\"reqId\":\"1\"}"));
    assert!(body.contains("event: serial.text"));
    assert!(body.contains("data: \"hello robot\""));
}
```

**Step 2: Run test to verify RED if route integration is missing**

```bash
cargo test api::routes::tests::events_route_streams_read_loop_recorded_events_as_sse -- --nocapture
```

Expected: PASS if Task 2 already integrated through the existing manager event store. If it fails, fix only the minimal integration issue.

**Step 3: Keep route behavior unchanged**

Do not change route paths or response shapes. The existing events route should be enough because it already streams manager events.

**Step 4: Run route tests**

```bash
cargo test api::routes::tests::events_route_streams_recorded_serial_events_as_sse -- --nocapture
cargo test api::routes::tests::events_route_streams_read_loop_recorded_events_as_sse -- --nocapture
```

Expected: PASS.

**Step 5: Commit**

Only if route tests or imports changed:

```bash
git add src/api/routes.rs src/serial/read_loop.rs
git commit -m "test: cover read-loop events over sse"
```

---

### Task 5: Optional small background task wrapper

**Objective:** Add a minimal async spawn helper only if it stays deterministic and simple.

**Files:**

- Modify: `src/serial/read_loop.rs`

**Important:** This task is optional. Skip it if it starts forcing complicated lifecycle/cancellation decisions.

**Step 1: Write failing async test**

```rust
#[tokio::test]
async fn spawned_mock_read_loop_drains_items_into_manager_events() {
    let manager = crate::serial::manager::InMemoryConnectionManager::default();
    let source = MockSerialReadSource::default();
    source.push_line("default", b"hello robot\n".to_vec());

    let handle = spawn_mock_read_loop(manager.clone(), source, "default".to_string());
    handle.await.unwrap();

    assert_eq!(
        manager.events().unwrap(),
        vec![crate::serial::manager::SerialStreamEvent {
            event: "serial.text",
            data: serde_json::json!("hello robot"),
        }]
    );
}
```

**Step 2: Run test to verify RED**

```bash
cargo test serial::read_loop::tests::spawned_mock_read_loop_drains_items_into_manager_events -- --nocapture
```

Expected: FAIL until spawn helper exists.

**Step 3: Implement minimal spawn helper**

```rust
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
```

**Step 4: Run test to verify GREEN**

```bash
cargo test serial::read_loop::tests::spawned_mock_read_loop_drains_items_into_manager_events -- --nocapture
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src/serial/read_loop.rs
git commit -m "feat: add mock serial read loop task"
```

---

### Task 6: Preserve route behavior and manual smoke tests

**Objective:** Prove HTTP behavior stayed compatible after adding the read-loop foundation.

**Files:**

- Modify only if needed: `src/api/routes.rs`

**Step 1: Run route tests**

```bash
cargo test api::routes::tests::connection_lifecycle_routes_manage_mock_connections -- --nocapture
cargo test api::routes::tests::legacy_alias_routes_share_connection_state -- --nocapture
cargo test api::routes::tests::command_route_queues_payload_for_named_connection -- --nocapture
cargo test api::routes::tests::commit_alias_queues_payload_for_default_connection -- --nocapture
cargo test api::routes::tests::events_route_streams_recorded_serial_events_as_sse -- --nocapture
cargo test api::routes::tests::events_route_streams_read_loop_recorded_events_as_sse -- --nocapture
```

Expected: all PASS. If the last test was not added because no route changes were needed, run the first five.

**Step 2: Run full local CI-equivalent checks**

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Expected: PASS.

**Step 3: Run live smoke test**

Start the server:

```bash
cargo run -- serve --host 127.0.0.1 --port 4002
```

In another terminal:

```bash
curl -s http://127.0.0.1:4002/api/v1/health
curl -s http://127.0.0.1:4002/api/v1/ports
curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}'
curl -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"method":"query","topic":"sensor.read","data":{}},"waitForResponse":false}'
curl -s -X POST http://127.0.0.1:4002/commit \
  -H 'content-type: application/json' \
  -d '{"reqId":"client-42","method":"query","topic":"sensor.read","data":{}}'
curl -i -s --max-time 2 http://127.0.0.1:4002/api/v1/events
curl -s -X POST http://127.0.0.1:4002/disconnect \
  -H 'content-type: application/json' \
  -d '{"name":"default"}'
```

Expected smoke notes:

- Health returns `{"status":"ok","version":"0.1.0"}`.
- Ports returns a JSON object with `ports` array; it may be empty.
- Connect returns `status: connected` with the same field shape as before.
- Command returns `status: queued` and generated `reqId` `1` on fresh server state.
- Legacy `/commit` preserves `client-42`.
- Events returns `content-type: text/event-stream`; body may be empty in the default server unless mock read events are injected by tests.
- Disconnect returns `status: disconnected` for `default`.

Stop the server before final status/commit.

---

## Acceptance Criteria

By the end of Phase 8:

- `src/serial/read_loop.rs` exists.
- `src/serial/mod.rs` exports `read_loop`.
- A mock serial read source can queue and drain line/error items by connection name.
- Incoming byte lines are parsed with `protocol::parse_line`.
- Parsed lines are recorded into manager events as:
  - `serial.json`
  - `serial.text`
  - `serial.log`
  - `serial.notification`
- Read errors, if modeled, are recorded as `serial.error` events.
- Existing `/api/v1/events` SSE route streams read-loop-recorded events.
- Existing command write behavior from Phase 7 remains intact.
- Current API behavior is preserved:
  - `POST /api/v1/connections` response shape unchanged.
  - `GET /api/v1/connections` response shape unchanged.
  - `DELETE /api/v1/connections/:name` response shape unchanged.
  - `POST /api/v1/connections/:name/commands` returns `{"status":"queued","reqId":"..."}` as before.
  - `POST /commit` preserves client-provided `reqId`.
- No hardware-dependent tests are added.
- No waited responses or timeout matching are added.
- `cargo fmt --check` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `cargo test --all-features` passes.
- Manual smoke flow still works on the mock-backed default server.
- Commits are focused, likely:
  - `feat: add mock serial read source`
  - `feat: record parsed serial read events`
  - `feat: record serial read errors`
  - optional `test: cover read-loop events over sse`
  - optional `feat: add mock serial read loop task`

---

## Full Verification Command Set

Use this before final status:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Then manually smoke the server as described in Task 6.

After final commit(s):

```bash
git status --short --branch
git log --oneline -10
```

Expected final status:

- Branch is `rewrite/axum-serial-api`.
- Working tree is clean.
- Latest commit(s) are Phase 8 read-loop/event-feed commits.

---

## Suggested Next Phase After This

Phase 9 should add request/response matching by `reqId` and implement `waitForResponse` behavior using the read-loop-fed event stream.

Phase 9 should still be test-first and should keep timeouts deterministic in unit tests.

Suggested Phase 9 title:

```text
Phase 9: ReqId Response Matching and waitForResponse
```

---

## Copy/Paste Prompt for the Next Coding Session

```text
We are in /home/alfarie/repos/serialport-api on branch rewrite/axum-serial-api. Please execute docs/phase-8-serial-read-loop-handoff.md.

Load the writing-plans, test-driven-development, and rust-axum-api-tdd skills before editing. This phase adds a testable background serial read-loop/event-feed foundation only. First verify baseline with:

export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features

Then use TDD to create src/serial/read_loop.rs with a mock serial read source, drain incoming line/error items by connection name, parse lines with protocol::parse_line, and record parsed events on the existing manager event store so /api/v1/events streams them as SSE. Preserve all current HTTP API and legacy alias behavior. Do not add waitForResponse behavior, reqId response matching, timeouts, config files, SQLite, Docker/systemd, WebSocket/Socket.IO, or hardware-dependent tests in this phase.

After implementation, run cargo fmt --check, cargo clippy --all-targets --all-features -- -D warnings, cargo test --all-features, manually smoke the documented curl examples against cargo run -- serve --host 127.0.0.1 --port 4002 where practical, stop the server, and commit focused Phase 8 changes.
```
