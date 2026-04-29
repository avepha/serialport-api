# Phase 7 Real Serial Transport Handoff

> **For Hermes / next AI coding session:** Execute this in a fresh session. Load `writing-plans` and `rust-axum-api-tdd` before editing. This phase starts runtime work again, but it must stay narrow: add a testable serial transport foundation and route command writes through that abstraction. Do not add background read loops, waited responses, config files, SQLite, packaging, or WebSocket/Socket.IO in this phase.

**Goal:** Introduce a mockable serial transport boundary so connection lifecycle and command writes can later use physical serial ports without making route/unit tests hardware-dependent.

**Architecture:** Keep the current Axum routes and public HTTP API shape stable. Move low-level open/close/write behavior behind a `SerialTransport` trait in the `serial` module, keep the default server mock-backed unless this phase explicitly wires a safe real transport, and preserve the current in-memory connection registry, generated/preserved `reqId` behavior, JSON + delimiter framing, and SSE event storage.

**Tech Stack:** Rust 2021, Axum 0.7, Tokio 1, Serde/Serde JSON, Thiserror, Tracing, `serialport`, test-first Rust unit tests and Axum route tests.

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
- `cargo run -- serve --host 127.0.0.1 --port 4002` starts the Axum server.
- Current server is intentionally mock/in-memory for connections, command writes, and events.
- Port listing uses `serialport::available_ports()`.
- Command endpoint generates or preserves `reqId`, frames JSON with the connection delimiter, stores written frames in memory, and returns `queued`.
- Event endpoint streams recorded mock events as SSE.
- CI workflow exists at `.github/workflows/ci.yml`.
- Latest full local CI-equivalent verification before this handoff passed with 19 tests.

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

---

## Phase 7 Scope

This phase should create the foundation for real serial I/O while preserving safe, deterministic tests.

Do in Phase 7:

- Add a low-level `SerialTransport` trait for open/close/write behavior.
- Add a mock transport implementation that records opened/closed connections and written frames.
- Refactor the existing in-memory manager to use that transport for connect/disconnect/send command behavior.
- Preserve the current default mock-backed server behavior unless the real transport is explicitly injected in a safe way.
- Add a real `serialport`-backed transport type only if it can be isolated and tested without hardware.
- Keep all existing HTTP responses and legacy aliases compatible.
- Use TDD: write focused failing unit tests before implementation.

Do **not** do in Phase 7:

- background serial read loops
- spawning threads/tasks for serial readers
- SSE events from real hardware
- command `waitForResponse` behavior
- request/response matching by `reqId`
- command timeout handling beyond preserving accepted request fields
- SQLite / preset storage
- config file loading
- GitHub Actions changes, unless fixing a discovered CI issue
- Docker/systemd/Raspberry Pi packaging
- WebSocket or Socket.IO support
- hardware-dependent tests
- API response shape changes

If a tempting change requires real hardware or asynchronous read-loop design, leave it for Phase 8 or later.

---

## Current Code to Understand First

Read these files before editing:

```bash
sed -n '1,260p' src/serial/manager.rs
sed -n '260,520p' src/serial/manager.rs
sed -n '1,260p' src/api/routes.rs
sed -n '1,140p' src/protocol.rs
sed -n '1,120p' src/error.rs
sed -n '1,80p' src/serial/mod.rs
sed -n '1,80p' Cargo.toml
```

Key current facts:

- `ConnectionManager` is the trait used by Axum routes.
- `InMemoryConnectionManager` currently owns:
  - `connections: Arc<Mutex<BTreeMap<String, ConnectionInfo>>>`
  - `next_req_id: Arc<Mutex<u64>>`
  - `written_frames: Arc<Mutex<BTreeMap<String, Vec<Vec<u8>>>>>`
  - `events: Arc<Mutex<Vec<SerialStreamEvent>>>`
- `send_command` currently:
  - verifies the named connection exists
  - requires the payload to be a JSON object
  - generates `reqId` when missing
  - preserves `reqId` when present
  - frames via `crate::protocol::frame_json(&payload, &connection.delimiter)`
  - records the frame in `written_frames`
  - returns `QueuedCommand { req_id }`
- Route tests assert exact JSON response shapes. Do not break them.

---

## Recommended Design

### New module

Create:

```text
src/serial/transport.rs
```

Expose it from:

```rust
// src/serial/mod.rs
pub mod manager;
pub mod transport;
```

### Transport trait

Recommended shape:

```rust
use crate::error::Result;
use crate::serial::manager::ConnectionInfo;

pub trait SerialTransport: Clone + Send + Sync + 'static {
    fn open(&self, connection: &ConnectionInfo) -> Result<()>;
    fn close(&self, name: &str) -> Result<()>;
    fn write_frame(&self, name: &str, frame: &[u8]) -> Result<()>;
}
```

Why this shape:

- It keeps low-level serial I/O separate from HTTP routes.
- It gives tests a clean mock boundary.
- It lets the manager own protocol-level behavior: `reqId`, framing, connection registry.
- It leaves read loops for Phase 8.

### Mock transport

Recommended first implementation:

```rust
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Default)]
pub struct MockSerialTransport {
    open_connections: Arc<Mutex<BTreeSet<String>>>,
    written_frames: Arc<Mutex<BTreeMap<String, Vec<Vec<u8>>>>>,
    closed_connections: Arc<Mutex<Vec<String>>>,
}
```

Minimum behavior:

- `open(&ConnectionInfo)` records the connection name.
- `close(name)` records the close and removes the name from opened connections.
- `write_frame(name, frame)` records `frame.to_vec()` under `name`.
- Test-only helpers can expose snapshots:
  - `opened_names()`
  - `closed_names()`
  - `written_frames(name)`

Keep helper methods either public if harmless or `#[cfg(test)]` if only needed in tests.

### Optional real transport skeleton

If adding a real type in this phase, keep it minimal and isolated:

```rust
#[derive(Clone, Default)]
pub struct SystemSerialTransport;
```

Acceptable Phase 7 behavior options:

1. **Preferred safest option:** add `SystemSerialTransport` type but do not wire it into the default server yet. Implementing it can be deferred to a later phase if it complicates the design.
2. **Acceptable if simple:** implement `open` by using `serialport::new(&connection.port, connection.baud_rate).open()?`, then close immediately/drop it, and keep `write_frame` returning a clear `SerialportApiError` variant like `SerialWriteNotConnected` until persistent handles are designed.
3. **Only if clean:** store opened port handles in an internal `Arc<Mutex<BTreeMap<String, Box<dyn serialport::SerialPort>>>>` and write frames in `write_frame`.

Avoid a large real-transport implementation if it forces complicated trait object, `Send`, lifetime, or blocking behavior decisions. The important deliverable is the tested boundary.

### Manager refactor direction

A clean option is to make the manager generic and keep the public default name mock-backed:

```rust
#[derive(Clone, Debug)]
pub struct ConnectionManagerWithTransport<T> {
    connections: Arc<Mutex<BTreeMap<String, ConnectionInfo>>>,
    next_req_id: Arc<Mutex<u64>>,
    events: Arc<Mutex<Vec<SerialStreamEvent>>>,
    transport: T,
}

pub type InMemoryConnectionManager = ConnectionManagerWithTransport<MockSerialTransport>;
```

Then implement:

```rust
impl<T> ConnectionManagerWithTransport<T>
where
    T: SerialTransport,
{
    pub fn new(transport: T) -> Self { ... }

    pub fn transport(&self) -> T {
        self.transport.clone()
    }
}

impl Default for InMemoryConnectionManager {
    fn default() -> Self {
        ConnectionManagerWithTransport::new(MockSerialTransport::default())
    }
}
```

Important: if a type alias plus `impl Default for InMemoryConnectionManager` causes Rust coherence/type-alias issues, use a concrete wrapper instead:

```rust
#[derive(Clone, Debug, Default)]
pub struct InMemoryConnectionManager {
    inner: ConnectionManagerWithTransport<MockSerialTransport>,
}
```

Choose the simpler compiling approach.

### Preserve compatibility

After refactor, these route examples must still work exactly as before:

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

---

## Bite-Sized TDD Tasks

### Task 1: Add a mock serial transport module

**Objective:** Create the transport trait and mock implementation without touching routes.

**Files:**

- Create: `src/serial/transport.rs`
- Modify: `src/serial/mod.rs`

**Step 1: Write failing tests**

In `src/serial/transport.rs`, add tests like:

```rust
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
        transport.write_frame("default", b"{\"topic\":\"ping\"}\r\n").unwrap();
        transport.close("default").unwrap();

        assert_eq!(transport.opened_names(), Vec::<String>::new());
        assert_eq!(transport.closed_names(), vec!["default".to_string()]);
        assert_eq!(
            transport.written_frames("default"),
            vec![b"{\"topic\":\"ping\"}\r\n".to_vec()]
        );
    }
}
```

**Step 2: Run test to verify RED**

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test serial::transport::tests::mock_transport_records_open_close_and_written_frames -- --nocapture
```

Expected: FAIL because `transport` module/types do not exist yet.

**Step 3: Implement minimal transport module**

Create `src/serial/transport.rs` with the trait, `MockSerialTransport`, helper methods, and tests.

Modify `src/serial/mod.rs`:

```rust
pub mod manager;
pub mod transport;
```

**Step 4: Run test to verify GREEN**

```bash
cargo test serial::transport::tests::mock_transport_records_open_close_and_written_frames -- --nocapture
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src/serial/mod.rs src/serial/transport.rs
git commit -m "feat: add mock serial transport boundary"
```

---

### Task 2: Route manager connection lifecycle through transport

**Objective:** Make connect/disconnect call the transport while preserving existing API behavior.

**Files:**

- Modify: `src/serial/manager.rs`
- Test: `src/serial/manager.rs`

**Step 1: Write failing tests**

Add or update a manager test that proves `connect` opens and `disconnect` closes through the mock transport.

Suggested test shape:

```rust
#[test]
fn connection_manager_opens_and_closes_transport_connections() {
    let transport = MockSerialTransport::default();
    let manager = ConnectionManagerWithTransport::new(transport.clone());

    manager
        .connect(ConnectionRequest {
            name: "default".to_string(),
            port: "/dev/ROBOT".to_string(),
            baud_rate: 115200,
            delimiter: "\r\n".to_string(),
        })
        .unwrap();

    assert_eq!(transport.opened_names(), vec!["default".to_string()]);

    manager.disconnect("default").unwrap();

    assert_eq!(transport.opened_names(), Vec::<String>::new());
    assert_eq!(transport.closed_names(), vec!["default".to_string()]);
}
```

**Step 2: Run test to verify RED**

```bash
cargo test serial::manager::tests::connection_manager_opens_and_closes_transport_connections -- --nocapture
```

Expected: FAIL because manager is not transport-backed yet.

**Step 3: Refactor manager minimally**

Recommended approach:

- Import `MockSerialTransport` and `SerialTransport`.
- Add a generic manager or wrapper as described above.
- Keep the existing public `InMemoryConnectionManager::default()` behavior mock-backed.
- In `connect`, create `ConnectionInfo`, call `self.transport.open(&connection)?`, then store it in `connections`.
- In `disconnect`, remove the connection registry entry and call `self.transport.close(name)?`.

Be careful about ordering:

- On connect, prefer calling `transport.open` before inserting into the registry so failed open does not leave a false connected record.
- On disconnect, it is acceptable to call `transport.close` even if the registry did not contain the connection, preserving current idempotent behavior.

**Step 4: Run narrow tests**

```bash
cargo test serial::manager::tests::connection_manager_opens_and_closes_transport_connections -- --nocapture
cargo test serial::manager::tests::in_memory_connection_manager_records_connections -- --nocapture
cargo test serial::manager::tests::in_memory_connection_manager_removes_disconnected_connections -- --nocapture
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src/serial/manager.rs src/serial/transport.rs
git commit -m "feat: route connection lifecycle through serial transport"
```

---

### Task 3: Route command writes through transport

**Objective:** Make `send_command` write framed bytes through the transport boundary while preserving generated/preserved `reqId` behavior.

**Files:**

- Modify: `src/serial/manager.rs`
- Test: `src/serial/manager.rs`

**Step 1: Write/update failing tests**

Update existing command tests to assert frames through the mock transport instead of manager-owned `written_frames`.

Suggested generated-`reqId` assertion:

```rust
#[test]
fn connection_manager_writes_framed_command_through_transport_with_generated_req_id() {
    let transport = MockSerialTransport::default();
    let manager = ConnectionManagerWithTransport::new(transport.clone());

    manager
        .connect(ConnectionRequest {
            name: "default".to_string(),
            port: "/dev/ROBOT".to_string(),
            baud_rate: 115200,
            delimiter: "\r\n".to_string(),
        })
        .unwrap();

    let queued = manager
        .send_command(
            "default",
            serde_json::json!({
                "method": "query",
                "topic": "sensor.read",
                "data": {}
            }),
        )
        .unwrap();

    assert_eq!(queued.req_id, "1");
    let frames = transport.written_frames("default");
    assert_eq!(frames.len(), 1);
    assert!(frames[0].ends_with(b"\r\n"));
    let body = &frames[0][..frames[0].len() - 2];
    let payload: serde_json::Value = serde_json::from_slice(body).unwrap();
    assert_eq!(
        payload,
        serde_json::json!({
            "reqId": "1",
            "method": "query",
            "topic": "sensor.read",
            "data": {}
        })
    );
}
```

Keep the existing preserved-`reqId` test behavior too.

**Step 2: Run tests to verify RED if not implemented**

```bash
cargo test serial::manager::tests::connection_manager_writes_framed_command_through_transport_with_generated_req_id -- --nocapture
```

Expected: FAIL until `send_command` calls `transport.write_frame`.

**Step 3: Implement minimal manager write change**

In `send_command`, after framing:

```rust
let frame = crate::protocol::frame_json(&payload, &connection.delimiter)?;
self.transport.write_frame(connection_name, &frame)?;
```

Then return `QueuedCommand { req_id }` as before.

Remove the manager-owned `written_frames` map if all tests have moved to `MockSerialTransport`. If keeping a compatibility helper temporarily is simpler, avoid duplicating frame storage long-term.

**Step 4: Run narrow tests**

```bash
cargo test serial::manager::tests::in_memory_connection_manager_records_framed_command_with_generated_req_id -- --nocapture
cargo test serial::manager::tests::in_memory_connection_manager_preserves_existing_req_id -- --nocapture
cargo test api::routes::tests::command_route_queues_payload_for_named_connection -- --nocapture
cargo test api::routes::tests::commit_alias_queues_payload_for_default_connection -- --nocapture
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src/serial/manager.rs src/serial/transport.rs
git commit -m "feat: write commands through serial transport"
```

---

### Task 4: Preserve route behavior and manual smoke tests

**Objective:** Prove HTTP behavior stayed compatible after the transport refactor.

**Files:**

- Modify only if needed: `src/api/routes.rs`
- Test: `src/api/routes.rs`

**Step 1: Run route tests**

```bash
cargo test api::routes::tests::connection_lifecycle_routes_manage_mock_connections -- --nocapture
cargo test api::routes::tests::legacy_alias_routes_share_connection_state -- --nocapture
cargo test api::routes::tests::command_route_queues_payload_for_named_connection -- --nocapture
cargo test api::routes::tests::commit_alias_queues_payload_for_default_connection -- --nocapture
cargo test api::routes::tests::events_route_streams_recorded_serial_events_as_sse -- --nocapture
```

Expected: all PASS.

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
- Events returns `content-type: text/event-stream`; body may be empty.
- Disconnect returns `status: disconnected` for `default`.

Stop the server before final status/commit.

**Step 4: Commit any route compatibility fixes**

Only if route/API changes were needed:

```bash
git add src/api/routes.rs src/serial/manager.rs src/serial/transport.rs
git commit -m "fix: preserve API behavior after transport refactor"
```

---

## Acceptance Criteria

By the end of Phase 7:

- `src/serial/transport.rs` exists.
- `src/serial/mod.rs` exports `transport`.
- A `SerialTransport` trait exists with open/close/write-frame responsibilities or an equivalent narrow transport boundary.
- A mock transport records open, close, and written frame behavior without hardware.
- The connection manager uses the transport for connection lifecycle and command writes.
- Current API behavior is preserved:
  - `POST /api/v1/connections` response shape unchanged.
  - `GET /api/v1/connections` response shape unchanged.
  - `DELETE /api/v1/connections/:name` response shape unchanged.
  - `POST /api/v1/connections/:name/commands` returns `{"status":"queued","reqId":"..."}` as before.
  - `POST /commit` preserves client-provided `reqId`.
- No hardware-dependent tests are added.
- No background serial read loop is added.
- No waited responses or timeout matching are added.
- `cargo fmt --check` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `cargo test --all-features` passes.
- Manual smoke flow still works on the mock-backed default server.
- Commits are created with focused messages, likely:
  - `feat: add mock serial transport boundary`
  - `feat: route connection lifecycle through serial transport`
  - `feat: write commands through serial transport`

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

Then manually smoke the server as described in Task 4.

After final commit(s):

```bash
git status --short --branch
git log --oneline -8
```

Expected final status:

- Branch is `rewrite/axum-serial-api`.
- Working tree is clean.
- Latest commit(s) are Phase 7 transport commits.

---

## Suggested Next Phase After This

Phase 8 should add the background serial read loop that feeds parsed `protocol::parse_line` results into SSE events.

Phase 8 should still be test-first and should avoid waited responses until Phase 9.

Suggested Phase 8 title:

```text
Phase 8: Background Serial Read Loop and SSE Event Feed
```

---

## Copy/Paste Prompt for the Next Coding Session

```text
We are in /home/alfarie/repos/serialport-api on branch rewrite/axum-serial-api. Please execute docs/phase-7-real-serial-transport-handoff.md.

Load the writing-plans and rust-axum-api-tdd skills before editing. This phase adds a testable serial transport foundation only. First verify baseline with:

export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features

Then use TDD to create src/serial/transport.rs with a SerialTransport trait and MockSerialTransport, export it from src/serial/mod.rs, and refactor src/serial/manager.rs so connection lifecycle and send_command use the transport for open/close/write_frame. Preserve all current HTTP API and legacy alias behavior. Do not add background serial read loops, waited responses, reqId matching, timeouts, config files, SQLite, Docker/systemd, WebSocket/Socket.IO, or hardware-dependent tests in this phase.

After implementation, run cargo fmt --check, cargo clippy --all-targets --all-features -- -D warnings, cargo test --all-features, manually smoke the documented curl examples against cargo run -- serve --host 127.0.0.1 --port 4002 where practical, stop the server, and commit focused Phase 7 changes.
```
