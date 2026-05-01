# Phase 9 ReqId Response Matching and `waitForResponse` Handoff

> **For Hermes / next AI coding session:** Execute this in a fresh session. Load `writing-plans`, `test-driven-development`, and `rust-axum-api-tdd` before editing. This phase should add deterministic request/response matching by `reqId` and implement the already-accepted `waitForResponse` / `timeoutMs` command request fields. Keep the scope narrow. Do not add SQLite, config files, packaging, WebSocket/Socket.IO, hardware-dependent tests, or broad API/error refactors.

**Goal:** When a client sends a command with `waitForResponse: true`, the server should write the framed command, wait for a matching inbound JSON response with the same `reqId`, and return that response or time out deterministically.

**Architecture:** Build on Phase 8's read-loop-fed manager event path. The manager should keep storing all serial events for SSE exactly as before, while additionally indexing inbound JSON events that contain a string `reqId` into a small in-memory response queue keyed by connection name and `reqId`. The HTTP command route should continue to use the manager abstraction, and should poll the in-memory response queue with Tokio time until either a match appears or `timeoutMs` elapses.

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

Latest known relevant commits after Phase 8:

```text
1ae4dbb feat: record serial read loop events
6e2d2ac feat: add mock serial read source
b69c194 docs: add phase 8 serial read loop handoff
2e14727 feat: add serial transport boundary
52a8b51 docs: add phase 7 serial transport handoff
8fc5e77 ci: add Rust verification workflow
0961b3d docs: rewrite project README
aed3c77 docs: add phase 6.1 README handoff
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
  - `src/serial/read_loop.rs`
- `cargo run -- serve --host 127.0.0.1 --port 4002` starts the Axum server.
- Current default server is still mock-backed and hardware-safe.
- Port listing uses `serialport::available_ports()`.
- Command endpoint generates or preserves `reqId`, frames JSON with the connection delimiter, writes through `SerialTransport`, and currently returns `queued` without waiting.
- Event endpoint streams recorded in-memory manager events as SSE.
- Phase 7 introduced:
  - `SerialTransport` trait with `open`, `close`, and `write_frame`.
  - `MockSerialTransport` with snapshots for opened names, closed names, and written frames.
  - `ConnectionManagerWithTransport<T>`.
  - `InMemoryConnectionManager = ConnectionManagerWithTransport<MockSerialTransport>`.
- Phase 8 introduced:
  - `src/serial/read_loop.rs`.
  - `serial::read_loop` export.
  - `SerialReadItem`.
  - `SerialReadSource`.
  - `MockSerialReadSource`.
  - `SerialEventRecorder`.
  - `drain_serial_read_items`.
  - `spawn_mock_read_loop`.
  - line bytes/read errors queued and drained per connection.
  - drained lines parsed with `protocol::parse_line`.
  - parsed input recorded into manager event storage as `serial.json`, `serial.text`, `serial.log`, `serial.notification`.
  - read errors recorded as `serial.error`.
  - SSE route coverage proving read-loop-fed manager events stream through `/api/v1/events`.
  - optional deterministic/test-safe mock background task wrapper.
- CI workflow exists at `.github/workflows/ci.yml`.
- Latest full local CI-equivalent verification after Phase 8 passed with 27 tests.
- Manual smoke test for server on `127.0.0.1:4002` passed for health, ports, connections, commands, `/commit`, events, and `/disconnect`.

Important local toolchain note:

```bash
# In this WSL environment, prefer rustup's toolchain first in PATH.
# Otherwise /usr/bin rustc/cargo 1.75 may mix with rustup clippy and cause metadata errors.
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
- Current baseline is `27 passed; 0 failed`.

---

## Phase 9 Scope

This phase should make command waiting useful without requiring real hardware. It should wire the already-existing request fields into manager response matching and route-level timeout behavior.

Do in Phase 9:

- Add an in-memory response index/queue to `ConnectionManagerWithTransport<T>`.
- Index inbound `SerialEvent::Json` values that contain a string `reqId` when the manager records serial events.
- Preserve the existing SSE event store and event names.
- Keep response matching scoped by connection name where the read-loop has a connection name.
- Keep existing `record_event(event)` behavior for older tests/callers by routing it through a reasonable default connection only if needed.
- Add a connection-aware event recording method/trait path for read-loop use, for example `record_event_for_connection(connection_name, event)` or `record_serial_event_for_connection(connection_name, event)`.
- Add deterministic manager methods to inspect/take a matched response, for example:
  - `take_response(connection_name, req_id) -> Result<Option<Value>>`
  - or `response_for_req_id(connection_name, req_id) -> Result<Option<Value>>` if it removes the matched response.
- Implement `POST /api/v1/connections/{name}/commands` wait behavior:
  - `waitForResponse: false` or omitted: preserve current `{"status":"queued","reqId":"..."}` response.
  - `waitForResponse: true`: write the command first, then wait for matching JSON response by `reqId` until timeout.
  - Success response should be `{"status":"ok","reqId":"...","response":{...}}`.
  - Timeout should return a non-2xx HTTP status and a deterministic testable error mapping. Prefer `504 Gateway Timeout` for command timeout.
- Implement the legacy `/commit` path as fire-and-forget only unless a very small backwards-compatible request shape can be supported without breaking existing alias behavior. The minimum requirement is not to break `/commit`.
- Use Tokio fake time (`#[tokio::test(start_paused = true)]`, `tokio::time::advance`) or very short deterministic timeouts in route tests. Prefer paused time where practical.
- If using Tokio paused time, add Tokio's `test-util` feature in `Cargo.toml` because the current `tokio = { version = "1", features = ["full"] }` does not guarantee `tokio::time::advance` / `start_paused` support.
- Preserve all existing route paths, response shapes for non-waiting commands, and legacy aliases.
- Use TDD: write focused failing unit/route tests before implementation.

Do **not** do in Phase 9:

- Real serial hardware reading/writing beyond the existing mockable transport boundary.
- Real persistent background read thread lifecycle decisions.
- SQLite / preset storage.
- Config file loading.
- Docker/systemd/Raspberry Pi packaging.
- WebSocket or Socket.IO support.
- Authentication.
- Large error-model response shape refactors across all routes.
- Changing SSE event body shape or wrapping existing event data with connection metadata.
- Changing generated `reqId` sequencing.
- Removing or renaming compatibility aliases (`/list`, `/connect`, `/disconnect`, `/info`, `/commit`).

If a tempting change requires connection timestamps, persistent history limits, multi-client SSE broadcast, real serial read handles, or a comprehensive JSON API error envelope, leave it for a later phase.

---

## Files Expected to Modify or Create

Expected code modifications for the implementation subagent:

- Modify: `src/serial/manager.rs` for response queue storage, response indexing, and response retrieval methods.
- Modify: `src/serial/read_loop.rs` so drained items record events with the actual connection name.
- Modify: `src/api/routes.rs` for active `waitForResponse` / `timeoutMs` handling and waited command route tests.
- Modify if using `#[tokio::test(start_paused = true)]` or `tokio::time::advance`: `Cargo.toml` to add Tokio's `test-util` feature.
- Modify only if needed: `src/error.rs` for timeout/error mapping helpers.

No new source file is required for Phase 9 unless the implementation subagent chooses to isolate a small response-matching helper module. If a new module is added, it must stay hardware-free and covered by tests.

---

## Current Code to Understand First

Read these files before editing:

```bash
cd /home/alfarie/repos/serialport-api
sed -n '1,260p' src/serial/manager.rs
sed -n '260,620p' src/serial/manager.rs
sed -n '1,260p' src/serial/read_loop.rs
sed -n '1,180p' src/serial/transport.rs
sed -n '1,180p' src/protocol.rs
sed -n '1,340p' src/api/routes.rs
sed -n '340,820p' src/api/routes.rs
sed -n '1,120p' src/error.rs
sed -n '1,90p' Cargo.toml
```

Key current facts:

- `ConnectionManager` currently has synchronous methods:

```rust
pub trait ConnectionManager: Clone + Send + Sync + 'static {
    fn connect(&self, request: ConnectionRequest) -> Result<ConnectionInfo>;
    fn connections(&self) -> Result<Vec<ConnectionInfo>>;
    fn disconnect(&self, name: &str) -> Result<String>;
    fn send_command(&self, connection_name: &str, payload: Value) -> Result<QueuedCommand>;
    fn events(&self) -> Result<Vec<SerialStreamEvent>>;
}
```

- `ConnectionManagerWithTransport<T>` currently owns:
  - `connections: Arc<Mutex<BTreeMap<String, ConnectionInfo>>>`
  - `next_req_id: Arc<Mutex<u64>>`
  - `events: Arc<Mutex<Vec<SerialStreamEvent>>>`
  - `transport: T`
- `record_event` pushes converted protocol events into the SSE event vector.
- `record_error` pushes `serial.error` into the SSE event vector.
- `SerialStreamEvent::from(SerialEvent::Json(value))` uses event name `serial.json` and data equal to the raw JSON value.
- `read_loop::drain_serial_read_items(manager, source, connection_name)` has the connection name available, parses line bytes, and currently calls `manager.record_serial_event(parse_line(...))` via the `SerialEventRecorder` trait.
- `CommandRequest` already deserializes `waitForResponse` and `timeoutMs`, but the fields are ignored:

```rust
struct CommandRequest {
    payload: Value,
    #[serde(rename = "waitForResponse", default)]
    _wait_for_response: bool,
    #[serde(rename = "timeoutMs")]
    _timeout_ms: Option<u64>,
}
```

- `CommandResponse` currently serializes only:

```json
{"status":"queued","reqId":"1"}
```

- `SerialportApiError::CommandTimeout` already exists.

---

## Recommended Design

### Response matching store

Add a small response queue to `ConnectionManagerWithTransport<T>`:

```rust
responses_by_connection_and_req_id:
    Arc<Mutex<BTreeMap<String, BTreeMap<String, VecDeque<Value>>>>>,
```

Use `VecDeque<Value>` so duplicate `reqId` values do not overwrite one another. Tests can assert first-in, first-out behavior. If you prefer a flatter key, use `(String, String)` only if it is ergonomic and keeps clippy happy.

### Connection-aware recording

Add an inherent manager method like:

```rust
pub fn record_event_for_connection(
    &self,
    connection_name: &str,
    event: crate::protocol::SerialEvent,
) {
    // 1. if event is SerialEvent::Json(value) and value.reqId is a string,
    //    push value.clone() into responses_by_connection_and_req_id[connection_name][reqId]
    // 2. preserve existing SSE storage by pushing SerialStreamEvent::from(event)
}
```

Then keep the existing method:

```rust
pub fn record_event(&self, event: crate::protocol::SerialEvent) {
    self.record_event_for_connection("default", event);
}
```

This preserves old tests/callers and gives Phase 8's read loop a connection-scoped path.

Update `SerialEventRecorder` in `src/serial/read_loop.rs` to prefer a connection-aware method:

```rust
pub trait SerialEventRecorder: Clone + Send + Sync + 'static {
    fn record_serial_event_for_connection(
        &self,
        connection_name: &str,
        event: crate::protocol::SerialEvent,
    );

    fn record_serial_error_for_connection(&self, connection_name: &str, message: String);
}
```

Then update `drain_serial_read_items` so `connection_name` is passed into the recorder. If you keep old trait methods for compatibility, have their default implementation call the new connection-aware methods with `"default"`.

Important: do not change `SerialStreamEvent` or the serialized SSE data shape in Phase 9.

### Taking matched responses

Add a manager method and trait method:

```rust
fn take_response(&self, connection_name: &str, req_id: &str) -> Result<Option<Value>>;
```

Implementation should remove and return the oldest queued response for that connection/`reqId`. Clean up empty nested maps if simple, but do not over-engineer.

### Waiting in the Axum route

Keep the manager trait synchronous. Avoid adding `async-trait` just for this phase. In `send_command`, after writing the command:

1. If `waitForResponse` is false, return the current queued response unchanged.
2. If true, poll `take_response(&name, &req_id)` until found or timeout.
3. Use `timeoutMs` when provided; choose a safe default such as `2000` ms if absent.
4. Clamp or treat `timeoutMs: 0` as an immediate timeout; document/test whichever behavior you implement.
5. Sleep with a short interval, e.g. `tokio::time::sleep(Duration::from_millis(10)).await`, so tests can use paused time.

Suggested helper in `src/api/routes.rs`:

```rust
async fn wait_for_response<C>(
    manager: &C,
    connection_name: &str,
    req_id: &str,
    timeout: std::time::Duration,
) -> crate::error::Result<Option<Value>>
where
    C: ConnectionManager,
{
    // poll manager.take_response(...), sleep, stop when timeout elapses
}
```

Alternatively, implement the wait helper in `manager.rs` if keeping route code smaller is cleaner, but keep it hardware-free and deterministic.

### Response JSON shape

Use a response enum or a struct with skipped optional fields. Preserve fire-and-forget exactly:

```json
{"status":"queued","reqId":"1"}
```

For a waited match, return:

```json
{"status":"ok","reqId":"1","response":{"reqId":"1","ok":true,"data":{}}}
```

For timeout, prefer:

- HTTP status: `504 Gateway Timeout`
- Body: any small deterministic text/body is acceptable for this phase, but if adding JSON is easy, use:

```json
{"error":"command timed out"}
```

Do not refactor every existing route's error response just to support this.

---

## Bite-Sized TDD Tasks

### Task 1: Index inbound JSON responses by connection and `reqId`

**Objective:** Prove manager event recording still stores SSE events and also stores JSON responses by connection/`reqId`.

**Files:**

- Modify: `src/serial/manager.rs`

**Step 1: Write failing tests**

Add tests near the existing manager tests:

```rust
#[test]
fn manager_indexes_json_response_by_connection_and_req_id() {
    let manager = InMemoryConnectionManager::default();

    manager.record_event_for_connection(
        "default",
        crate::protocol::SerialEvent::Json(serde_json::json!({
            "reqId": "1",
            "ok": true,
            "data": {"temperature": 28.5}
        })),
    );

    assert_eq!(
        manager.take_response("default", "1").unwrap(),
        Some(serde_json::json!({
            "reqId": "1",
            "ok": true,
            "data": {"temperature": 28.5}
        }))
    );
    assert_eq!(manager.take_response("default", "1").unwrap(), None);
    assert_eq!(
        manager.events().unwrap(),
        vec![SerialStreamEvent {
            event: "serial.json",
            data: serde_json::json!({
                "reqId": "1",
                "ok": true,
                "data": {"temperature": 28.5}
            }),
        }]
    );
}

#[test]
fn manager_does_not_match_responses_across_connections() {
    let manager = InMemoryConnectionManager::default();

    manager.record_event_for_connection(
        "default",
        crate::protocol::SerialEvent::Json(serde_json::json!({"reqId":"1","ok":true})),
    );

    assert_eq!(manager.take_response("other", "1").unwrap(), None);
    assert_eq!(
        manager.take_response("default", "1").unwrap(),
        Some(serde_json::json!({"reqId":"1","ok":true}))
    );
}
```

**Step 2: Run tests to verify RED**

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test serial::manager::tests::manager_indexes_json_response_by_connection_and_req_id -- --nocapture
cargo test serial::manager::tests::manager_does_not_match_responses_across_connections -- --nocapture
```

Expected: FAIL because `record_event_for_connection` and `take_response` do not exist yet.

**Step 3: Implement minimal response indexing**

Implement:

- new response queue field on `ConnectionManagerWithTransport<T>`.
- `record_event_for_connection`.
- `record_event` delegating to `record_event_for_connection("default", event)`.
- `take_response` as an inherent method.

Do not change `SerialStreamEvent` or current SSE event mapping.

**Step 4: Run tests to verify GREEN**

```bash
cargo test serial::manager::tests::manager_indexes_json_response_by_connection_and_req_id -- --nocapture
cargo test serial::manager::tests::manager_does_not_match_responses_across_connections -- --nocapture
```

Expected: PASS.

---

### Task 2: Preserve FIFO behavior and ignore non-response events

**Objective:** Keep matching predictable and avoid indexing logs, notifications, text, JSON without string `reqId`, or non-string `reqId`.

**Files:**

- Modify: `src/serial/manager.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn manager_returns_duplicate_req_id_responses_fifo() {
    let manager = InMemoryConnectionManager::default();

    manager.record_event_for_connection(
        "default",
        crate::protocol::SerialEvent::Json(serde_json::json!({"reqId":"1","seq":1})),
    );
    manager.record_event_for_connection(
        "default",
        crate::protocol::SerialEvent::Json(serde_json::json!({"reqId":"1","seq":2})),
    );

    assert_eq!(
        manager.take_response("default", "1").unwrap(),
        Some(serde_json::json!({"reqId":"1","seq":1}))
    );
    assert_eq!(
        manager.take_response("default", "1").unwrap(),
        Some(serde_json::json!({"reqId":"1","seq":2}))
    );
    assert_eq!(manager.take_response("default", "1").unwrap(), None);
}

#[test]
fn manager_ignores_events_without_string_req_id_for_response_matching() {
    let manager = InMemoryConnectionManager::default();

    manager.record_event_for_connection(
        "default",
        crate::protocol::SerialEvent::Text("hello".to_string()),
    );
    manager.record_event_for_connection(
        "default",
        crate::protocol::SerialEvent::Json(serde_json::json!({"ok":true})),
    );
    manager.record_event_for_connection(
        "default",
        crate::protocol::SerialEvent::Json(serde_json::json!({"reqId":1,"ok":true})),
    );
    manager.record_event_for_connection(
        "default",
        crate::protocol::SerialEvent::Log(serde_json::json!({
            "method":"log",
            "reqId":"1",
            "data":{}
        })),
    );

    assert_eq!(manager.take_response("default", "1").unwrap(), None);
}
```

**Step 2: Run tests to verify RED/GREEN**

```bash
cargo test serial::manager::tests::manager_returns_duplicate_req_id_responses_fifo -- --nocapture
cargo test serial::manager::tests::manager_ignores_events_without_string_req_id_for_response_matching -- --nocapture
```

Expected RED reason: FAIL until FIFO queueing and filtering are implemented correctly. If these already pass from Task 1's implementation, keep them as regression coverage.

**Step 3: Implement minimal fixes**

- Use `VecDeque<Value>` per key.
- Only index `SerialEvent::Json(value)` where `value.get("reqId").and_then(Value::as_str)` returns `Some`.
- Do not index `SerialEvent::Log` or `SerialEvent::Notification` even if those JSON objects contain a `reqId`; they are event classes, not command responses for Phase 9.

**Step 4: Run tests to verify GREEN**

```bash
cargo test serial::manager::tests::manager_returns_duplicate_req_id_responses_fifo -- --nocapture
cargo test serial::manager::tests::manager_ignores_events_without_string_req_id_for_response_matching -- --nocapture
```

Expected: PASS.

**Optional commit after Tasks 1-2:**

```bash
git add src/serial/manager.rs
git commit -m "feat: match serial responses by req id"
```

---

### Task 3: Pass connection names through the Phase 8 read-loop recorder

**Objective:** Prove read-loop-drained JSON responses are indexed under the actual connection name while still streaming over SSE.

**Files:**

- Modify: `src/serial/read_loop.rs`
- Modify if needed: `src/serial/manager.rs`

**Step 1: Write failing test**

Add to `src/serial/read_loop.rs` tests:

```rust
#[test]
fn drain_read_items_indexes_json_response_for_connection() {
    let manager = crate::serial::manager::InMemoryConnectionManager::default();
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
        vec![crate::serial::manager::SerialStreamEvent {
            event: "serial.json",
            data: serde_json::json!({"reqId":"42","ok":true}),
        }]
    );
}
```

**Step 2: Run test to verify RED**

```bash
cargo test serial::read_loop::tests::drain_read_items_indexes_json_response_for_connection -- --nocapture
```

Expected: FAIL if the read loop still records all events through a non-connection-aware method/default connection.

**Step 3: Implement connection-aware recorder integration**

Update `SerialEventRecorder` and its implementation for `ConnectionManagerWithTransport<T>` so `drain_serial_read_items` passes `connection_name` when recording parsed line events and read errors.

Read errors can continue to be stored only as SSE events. They do not need response matching.

**Step 4: Run test to verify GREEN**

```bash
cargo test serial::read_loop::tests::drain_read_items_indexes_json_response_for_connection -- --nocapture
```

Expected: PASS.

**Optional commit:**

```bash
git add src/serial/read_loop.rs src/serial/manager.rs
git commit -m "feat: index read loop responses by connection"
```

---

### Task 4: Preserve fire-and-forget command behavior exactly

**Objective:** Before changing the command route response type, prove existing fire-and-forget behavior remains unchanged.

**Files:**

- Modify: `src/api/routes.rs`

**Step 1: Add or keep compatibility tests**

Existing tests already cover:

- `command_route_queues_payload_for_named_connection`
- `commit_alias_queues_payload_for_default_connection`

If you change `CommandResponse` into an enum or add optional fields, run these before and after the change.

**Step 2: Run tests**

```bash
cargo test api::routes::tests::command_route_queues_payload_for_named_connection -- --nocapture
cargo test api::routes::tests::commit_alias_queues_payload_for_default_connection -- --nocapture
```

Expected: PASS before and after route refactor. The exact JSON for non-waiting commands must remain:

```json
{"status":"queued","reqId":"1"}
```

No code should be committed unless these still pass.

---

### Task 5: Implement waited command route success

**Objective:** `waitForResponse: true` returns a matching response when one arrives before timeout.

**Files:**

- Modify: `src/api/routes.rs`
- Modify if needed: `src/serial/manager.rs`
- Modify if needed: `src/error.rs`

**Step 1: Write failing route test**

Add to `src/api/routes.rs` tests. Use a client-provided `reqId` so the test can inject a matching response deterministically while the route is waiting.

```rust
#[tokio::test(start_paused = true)]
async fn command_route_waits_for_matching_response() {
    let manager = InMemoryConnectionManager::default();
    let app = router_with_state(AppState {
        port_lister: MockPortLister { ports: Vec::new() },
        connection_manager: manager.clone(),
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/connections")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    let request = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/v1/connections/default/commands")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"payload":{"reqId":"client-99","method":"query","topic":"sensor.read","data":{}},"waitForResponse":true,"timeoutMs":1000}"#,
            ))
            .unwrap(),
    );

    tokio::pin!(request);
    tokio::task::yield_now().await;

    manager.record_event_for_connection(
        "default",
        crate::protocol::SerialEvent::Json(json!({
            "reqId": "client-99",
            "ok": true,
            "data": {"temperature": 28.5}
        })),
    );
    tokio::time::advance(std::time::Duration::from_millis(10)).await;

    let response = request.await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        payload,
        json!({
            "status": "ok",
            "reqId": "client-99",
            "response": {
                "reqId": "client-99",
                "ok": true,
                "data": {"temperature": 28.5}
            }
        })
    );
}
```

If paused time hangs in this exact shape, adapt the test to spawn a task that records the event after a short sleep, then `advance` time. Keep it deterministic and fast.

**Step 2: Run test to verify RED**

```bash
cargo test api::routes::tests::command_route_waits_for_matching_response -- --nocapture
```

Expected: FAIL because `waitForResponse` is ignored and/or response matching is not wired into the route.

**Step 3: Implement minimal waited route behavior**

- Rename `CommandRequest` fields from `_wait_for_response` / `_timeout_ms` to active names.
- Add a response type that can serialize both queued and waited success shapes without adding `response: null` to fire-and-forget responses.
- Add `take_response` to `ConnectionManager` trait so the generic route can use it.
- Implement route polling with Tokio time.
- Map timeout to `StatusCode::GATEWAY_TIMEOUT`.

Avoid changing unrelated route error behavior.

**Step 4: Run test to verify GREEN**

```bash
cargo test api::routes::tests::command_route_waits_for_matching_response -- --nocapture
```

Expected: PASS.

---

### Task 6: Implement waited command timeout

**Objective:** If no matching response arrives before `timeoutMs`, the route should return a deterministic timeout status.

**Files:**

- Modify: `src/api/routes.rs`
- Modify if needed: `src/error.rs`

**Step 1: Write failing route test**

```rust
#[tokio::test(start_paused = true)]
async fn command_route_times_out_waiting_for_response() {
    let app = router_with_state(AppState {
        port_lister: MockPortLister { ports: Vec::new() },
        connection_manager: InMemoryConnectionManager::default(),
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/connections")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    let request = app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/v1/connections/default/commands")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"payload":{"reqId":"will-timeout","method":"query","topic":"sensor.read","data":{}},"waitForResponse":true,"timeoutMs":50}"#,
            ))
            .unwrap(),
    );

    tokio::pin!(request);
    tokio::time::advance(std::time::Duration::from_millis(60)).await;

    let response = request.await.unwrap();
    assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
}
```

**Step 2: Run test to verify RED**

```bash
cargo test api::routes::tests::command_route_times_out_waiting_for_response -- --nocapture
```

Expected: FAIL until timeout behavior returns `504`.

**Step 3: Implement minimal timeout mapping**

- Return `StatusCode::GATEWAY_TIMEOUT` on waited command timeout.
- If returning JSON body is easy, add `Json` with `{"error":"command timed out"}`; otherwise status-only is acceptable for this phase.
- Do not alter non-waiting command error statuses except where absolutely necessary for compilation.

**Step 4: Run test to verify GREEN**

```bash
cargo test api::routes::tests::command_route_times_out_waiting_for_response -- --nocapture
```

Expected: PASS.

**Optional commit after Tasks 4-6:**

```bash
git add src/api/routes.rs src/serial/manager.rs src/error.rs
git commit -m "feat: wait for command responses by req id"
```

---

### Task 7: Prove read-loop-fed responses satisfy waited commands

**Objective:** The command route should not require direct manager injection of responses; responses produced through Phase 8's mock read loop should satisfy waiting commands.

**Files:**

- Modify: `src/api/routes.rs`
- Modify if needed: `src/serial/read_loop.rs`

**Step 1: Write route or integration-style test**

Suggested route test:

```rust
#[tokio::test(start_paused = true)]
async fn command_route_waits_for_read_loop_recorded_response() {
    let manager = InMemoryConnectionManager::default();
    let read_source = crate::serial::read_loop::MockSerialReadSource::default();
    let app = router_with_state(AppState {
        port_lister: MockPortLister { ports: Vec::new() },
        connection_manager: manager.clone(),
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/connections")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    let request = app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/v1/connections/default/commands")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"payload":{"reqId":"loop-1","method":"query","topic":"sensor.read","data":{}},"waitForResponse":true,"timeoutMs":1000}"#,
            ))
            .unwrap(),
    );

    tokio::pin!(request);
    tokio::task::yield_now().await;

    read_source.push_line("default", b"{\"reqId\":\"loop-1\",\"ok\":true}\r\n".to_vec());
    crate::serial::read_loop::drain_serial_read_items(&manager, &read_source, "default").unwrap();
    tokio::time::advance(std::time::Duration::from_millis(10)).await;

    let response = request.await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        payload,
        json!({
            "status": "ok",
            "reqId": "loop-1",
            "response": {"reqId":"loop-1","ok":true}
        })
    );
}
```

**Step 2: Run test**

```bash
cargo test api::routes::tests::command_route_waits_for_read_loop_recorded_response -- --nocapture
```

Expected: PASS if Tasks 3 and 5 are integrated correctly. If it fails, fix only the minimal integration issue.

**Optional commit:**

```bash
git add src/api/routes.rs src/serial/read_loop.rs src/serial/manager.rs
git commit -m "test: cover waited command responses from read loop"
```

---

## Compatibility Requirements

After Phase 9, these must still work exactly as before:

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

New waited timeout behavior should work without hardware:

```bash
curl -i -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"reqId":"manual-timeout","method":"query","topic":"sensor.read","data":{}},"waitForResponse":true,"timeoutMs":50}'
```

Expected:

- HTTP status is `504 Gateway Timeout` because no manual mock read-loop injection exists in the live server.
- The server remains responsive after the timeout.

`GET /api/v1/events` must still return `content-type: text/event-stream` and stream stored manager events with the same event names and data shape as Phase 8.

---

## Manual Smoke Test Flow

Manual smoke is mostly a compatibility check because the default live server has no endpoint for injecting mock inbound serial lines. Waited success is covered by route/unit tests using `MockSerialReadSource`; live manual waited calls should time out until real/mock-device injection is added later.

Start the server:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
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
curl -i -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"reqId":"manual-timeout","method":"query","topic":"sensor.read","data":{}},"waitForResponse":true,"timeoutMs":50}'
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
- Fire-and-forget command returns `status: queued` and generated `reqId` `1` on fresh server state.
- Waited command returns `504 Gateway Timeout` in the live server because no inbound response is injected manually.
- Legacy `/commit` preserves `client-42` and returns `queued`.
- Events returns `content-type: text/event-stream`; body may be empty in the default server unless tests inject mock events.
- Disconnect returns `status: disconnected` for `default`.

Stop the server before final status/commit.

---

## Acceptance Criteria

By the end of Phase 9:

- Manager records JSON events with string `reqId` into an in-memory response queue keyed by connection name and `reqId`.
- Manager keeps storing all serial events for SSE as before.
- `take_response` or equivalent returns queued responses FIFO and removes them.
- JSON events without string `reqId` are not matched as command responses.
- Text/log/notification/error events are not matched as command responses.
- Read-loop drained JSON responses are indexed under the drained connection name, not blindly under another connection.
- `POST /api/v1/connections/{name}/commands` with `waitForResponse: false` or omitted keeps the exact current `queued` response shape.
- `POST /api/v1/connections/{name}/commands` with `waitForResponse: true` returns `status: ok`, the `reqId`, and the matched JSON response when a matching response is recorded before timeout.
- Waited command timeout returns `504 Gateway Timeout` deterministically.
- Existing legacy aliases still pass tests:
  - `/list`
  - `/connect`
  - `/disconnect`
  - `/info`
  - `/commit`
- No hardware-dependent tests are added.
- No SQLite/config/WebSocket/packaging changes are added.
- `cargo fmt --check` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `cargo test --all-features` passes.
- Manual smoke flow still works on the mock-backed default server, with live waited command timing out as expected.

Likely implementation commits:

```text
feat: match serial responses by req id
feat: wait for command responses by req id
```

If the implementation is small enough, a single commit is acceptable:

```text
feat: add waited command responses
```

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

Then manually smoke the server as described above.

After final implementation commit(s):

```bash
git status --short --branch
git log --oneline -10
```

Expected final status:

- Branch is `rewrite/axum-serial-api`.
- Working tree is clean.
- Latest commit(s) are Phase 9 request/response matching and waited command response commits.

---

## Suggested Next Phase After This

Phase 10 should likely add a usable mock-device/scripted-response mode or a real serial read/write lifecycle integration, depending on project priority after Phase 9 review.

Recommended Phase 10 title if mockability remains the priority:

```text
Phase 10: Mock Device Mode and Scripted Responses
```

Recommended Phase 10 title if hardware integration becomes the priority:

```text
Phase 10: Real Serial Read/Write Lifecycle
```

Do not start Phase 10 while implementing Phase 9.

---

## Copy/Paste Prompt for the Next Coding Session

```text
We are in /home/alfarie/repos/serialport-api on branch rewrite/axum-serial-api. Please execute docs/phase-9-reqid-response-matching-handoff.md.

Load the writing-plans, test-driven-development, and rust-axum-api-tdd skills before editing. This phase adds request/response matching by reqId and implements waitForResponse/timeoutMs for the canonical command endpoint only. First verify baseline with:

export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features

Then use TDD to add an in-memory response queue to ConnectionManagerWithTransport keyed by connection name and reqId. Index inbound SerialEvent::Json values with string reqId when manager/read-loop records serial events, while preserving existing SSE event storage and event data shapes. Add a take_response-style method and use it from the Axum command route so POST /api/v1/connections/{name}/commands with waitForResponse true writes the command, waits for the matching response until timeoutMs, returns {"status":"ok","reqId":"...","response":{...}} on match, and returns 504 Gateway Timeout on timeout. Preserve fire-and-forget command responses exactly as {"status":"queued","reqId":"..."}. Preserve /commit and all legacy aliases. Do not add real hardware tests, SQLite, config files, WebSocket/Socket.IO, Docker/systemd, or broad error response refactors.

After implementation, run cargo fmt --check, cargo clippy --all-targets --all-features -- -D warnings, cargo test --all-features, manually smoke the documented curl examples against cargo run -- serve --host 127.0.0.1 --port 4002 where practical, stop the server, and commit focused Phase 9 changes with either:

feat: match serial responses by req id
feat: wait for command responses by req id

or one small combined commit:

feat: add waited command responses
```
