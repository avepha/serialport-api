# Phase 10 Mock Device Mode and Scripted Responses Handoff

> **For Hermes / next AI coding session:** Execute this in a fresh session. Load `writing-plans`, `test-driven-development`, and `rust-axum-api-tdd` before editing. This phase should add a hardware-free mock-device response mode so waited commands can succeed in a live server without real serial hardware. Keep the scope narrow. Do not add real serial hardware lifecycle, SQLite, broad config-file support, Docker/systemd packaging, WebSocket/Socket.IO, authentication, or a full API error-envelope refactor.

**Goal:** Add an explicit mock-device mode that can automatically synthesize inbound serial JSON responses for written commands, including optional topic-keyed scripted responses. This should make `POST /api/v1/connections/{name}/commands` with `waitForResponse: true` return a successful response in manual smoke tests when the server is started in mock-device mode.

**Architecture:** Build on Phase 9's manager-owned request/response queue. Keep the existing default server behavior compatible and mock-backed. Add a small pure mock-device responder that accepts a framed outbound JSON command, extracts `reqId` and command metadata, produces a JSON response event, and lets the existing manager `record_event_for_connection` path store both SSE events and waitable responses. Prefer an opt-in server flag, for example `cargo run -- serve --mock-device`, so the current no-auto-response behavior remains available for compatibility tests.

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

Latest known relevant commits after Phase 9:

```text
bf2d64a feat: add waited command responses
2231156 docs: add phase 9 reqid response matching handoff
1ae4dbb feat: record serial read loop events
6e2d2ac feat: add mock serial read source
b69c194 docs: add phase 8 serial read loop handoff
2e14727 feat: add serial transport boundary
52a8b51 docs: add phase 7 serial transport handoff
8fc5e77 ci: add Rust verification workflow
0961b3d docs: rewrite project README
```

Independent Phase 9 review status:

- Verdict: **APPROVED**.
- No critical or important issues.
- Verification passed: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features` with 35 tests passing.
- Manual smoke passed, including a waited live command timeout returning `504 Gateway Timeout`.

Completed functionality as of this handoff:

- Axum server starts with `cargo run -- serve --host 127.0.0.1 --port 4002`.
- Default server remains hardware-safe and mock-backed.
- Port listing uses `serialport::available_ports()`.
- Named connection lifecycle exists for canonical routes and legacy aliases.
- Commands generate or preserve string `reqId`, frame JSON with the connection delimiter, and write through `SerialTransport`.
- Phase 7 added the `SerialTransport` trait and `MockSerialTransport`.
- Phase 8 added `src/serial/read_loop.rs`, `MockSerialReadSource`, read-loop draining, and manager event recording.
- Phase 9 added response matching by connection name and string `reqId`, FIFO response queues, `take_response`, and active `waitForResponse` / `timeoutMs` handling.
- Waited command success is covered by tests, but the default live server has no manual inbound-message injection path. A live waited command currently times out unless tests directly inject a response.
- SSE route/event names and legacy aliases are preserved.

Important local toolchain note:

```bash
# In this WSL environment, prefer rustup's toolchain first in PATH.
# Otherwise /usr/bin rustc/cargo can mix with rustup cargo-clippy and cause metadata errors.
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
- Current baseline is 35 tests passing.

---

## Why Phase 10 Is Mock Device Mode

The original open-source spec requires mock mode with echo/scripted responses so the project is testable without hardware. Phases 7-9 created the required foundations:

- a mockable serial transport boundary;
- read-loop/event recording and response matching;
- waited command success/timeout behavior.

The remaining gap is usability: a developer can manually prove timeout behavior, but cannot manually prove waited success against the live server without real hardware or test-only injection. A narrow mock-device mode closes that gap and creates a better platform for future real-serial lifecycle work.

Recommended Phase 11 after this: real serial read/write lifecycle wiring, using the mock-device tests from Phase 10 as compatibility coverage.

---

## Phase 10 Scope

Do in Phase 10:

- Add a pure mock-device response module, likely `src/serial/mock_device.rs`.
- Add a default mock responder that turns a written framed JSON command into a JSON response containing the same string `reqId`.
- Add optional topic-keyed scripted responses loaded from one explicit JSON file passed at server start, for example `--mock-script ./mock-responses.json`.
- Add an opt-in server flag, for example `--mock-device`, that enables automatic mock responses in the live server.
- Preserve the default `serve` behavior unless `--mock-device` is set.
- Reuse existing manager event recording so mock responses appear in both:
  - `take_response(connection, reqId)` for waited commands;
  - `GET /api/v1/events` as `serial.json` events with the same SSE shape as before.
- Keep legacy `/commit` fire-and-forget behavior compatible. It may trigger a mock response event when mock-device mode is enabled, but the HTTP response must remain `{"status":"queued","reqId":"..."}`.
- Add route/unit tests that prove waited commands can succeed through the same path used by the live mock-device server.
- Add documentation comments or README notes only if needed to explain the new flags and script format.

Do **not** do in Phase 10:

- Real serial hardware opening, blocking reads, or hardware-dependent tests.
- Switching the default server to real serial mode.
- SQLite / preset storage.
- Broad config-file discovery such as `serialport-api.toml`, `~/.config/...`, or `/etc/...`.
- Docker, systemd, Raspberry Pi install docs, release binaries, or packaging.
- WebSocket or Socket.IO.
- Authentication.
- Large error response refactors across existing routes.
- Changing SSE event names or wrapping event data with new metadata.
- Changing generated `reqId` sequencing.
- Removing or renaming compatibility aliases (`/list`, `/connect`, `/disconnect`, `/info`, `/commit`).

If a tempting change requires a production-grade scripting language, real serial port ownership, multi-client live SSE broadcast, persistent config, or a comprehensive API error model, leave it for a later phase.

---

## Expected Files to Modify or Create

Expected implementation changes:

- Create: `src/serial/mock_device.rs`
  - Pure responder types/functions.
  - Default response behavior.
  - Script loading/parsing if kept outside `main.rs`.
  - Unit tests.
- Modify: `src/serial/mod.rs`
  - Export `mock_device`.
- Modify: `src/serial/manager.rs`
  - Add an optional mock responder hook to `ConnectionManagerWithTransport<T>` or add a small manager wrapper that can record mock responses after successful writes.
  - Preserve default `InMemoryConnectionManager::default()` behavior unless the new constructor is explicitly used.
- Modify: `src/api/routes.rs`
  - Add a public constructor or state constructor if `main.rs` needs to inject a mock-device-enabled manager.
  - Add route tests that use the new mock-device-enabled manager and prove waited success.
- Modify: `src/main.rs`
  - Add CLI flags such as `--mock-device` and `--mock-script <path>`.
  - Start the router with the mock-device-enabled manager only when requested.
- Modify if necessary: `src/error.rs`
  - Add narrow script parse/load error mapping only if needed.
- Modify if useful: `README.md`
  - Add a short mock-device quick-start and script format example.

No dependency additions should be necessary. If the implementation adds a dependency, justify it in the commit message/body or avoid it.

---

## Current Code to Understand First

Read these files before editing:

```bash
cd /home/alfarie/repos/serialport-api
sed -n '1,260p' src/serial/manager.rs
sed -n '260,760p' src/serial/manager.rs
sed -n '1,220p' src/serial/transport.rs
sed -n '1,260p' src/serial/read_loop.rs
sed -n '1,180p' src/protocol.rs
sed -n '1,360p' src/api/routes.rs
sed -n '360,1040p' src/api/routes.rs
sed -n '1,120p' src/main.rs
sed -n '1,120p' src/error.rs
sed -n '1,90p' Cargo.toml
```

Key current facts:

- `ConnectionManagerWithTransport<T>` owns the connection registry, request id counter, SSE event vector, response queues, and transport.
- `send_command` writes framed bytes through the transport and returns `QueuedCommand { req_id }`.
- `record_event_for_connection(connection_name, SerialEvent::Json(value))` indexes JSON values with string `reqId` for waited responses and also records SSE events.
- The route helper waits by polling `ConnectionManager::take_response`.
- `router()` currently constructs `InMemoryConnectionManager::default()` internally.
- `AppState` fields are private; `router_with_state` is public but `main.rs` may need an ergonomic public constructor or a dedicated router constructor to inject a custom manager.

---

## Recommended Design

### 1. Pure responder module

Create `src/serial/mock_device.rs` with a small, hardware-free responder. Keep this module independent from Axum.

Suggested concepts:

```rust
#[derive(Clone, Debug, Default)]
pub struct MockDeviceResponder {
    script: MockResponseScript,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct MockResponseScript {
    #[serde(default)]
    responses: Vec<MockResponseRule>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MockResponseRule {
    topic: String,
    response: serde_json::Value,
}
```

Recommended script JSON format:

```json
{
  "responses": [
    {
      "topic": "sensor.read",
      "response": {"ok": true, "data": {"temperature": 28.5}}
    },
    {
      "topic": "motor.fail",
      "response": {"ok": false, "error": "mock motor failure"}
    }
  ]
}
```

Behavior:

- Parse the outbound frame with the connection delimiter or reuse the protocol line parser carefully.
- Only synthesize responses for JSON object commands with a string `reqId`.
- Match scripts by `payload.topic` string.
- If a matching script response is found, clone it and inject the command `reqId` into the response object unless the scripted response already contains a string `reqId`.
- If no script matches, return a default ack response such as:

```json
{"reqId":"...","ok":true,"data":{"mock":true,"topic":"sensor.read"}}
```

- If the command has no topic, use `null` or omit `topic` inside `data`; keep the exact shape deterministic in tests.
- If the outbound frame is invalid JSON or lacks a string `reqId`, return `None` and do not record an event.

### 2. Manager hook for mock responses

Prefer adding an optional responder hook to `ConnectionManagerWithTransport<T>` rather than adding mock-device behavior to `MockSerialTransport`. Reason: the manager already has the correct `record_event_for_connection` method and response queues. The transport should remain a write/open/close abstraction.

Suggested shape:

```rust
pub trait MockCommandResponder: Clone + Send + Sync + 'static {
    fn response_for_frame(
        &self,
        frame: &[u8],
        delimiter: &str,
    ) -> Option<serde_json::Value>;
}
```

Implementation idea:

- Add an optional responder field to `ConnectionManagerWithTransport<T>`.
- Keep `new(transport)` and `Default` with no responder.
- Add a constructor such as `with_mock_responder(transport, responder)`.
- In `send_command`, after `transport.write_frame` succeeds, call responder if present.
- If responder returns a JSON value, call `record_event_for_connection(connection_name, SerialEvent::Json(response))`.
- This lets Phase 9's existing waited-route polling find the response without special route code.

If a generic optional responder makes the type too complex, a narrow wrapper type is acceptable, but keep the routes generic over the existing `ConnectionManager` trait.

### 3. CLI wiring

Extend `ServeArgs` in `src/main.rs`:

```text
--mock-device
--mock-script <PATH>
```

Recommended behavior:

- `cargo run -- serve --host 127.0.0.1 --port 4002` keeps current no-auto-response behavior.
- `cargo run -- serve --host 127.0.0.1 --port 4002 --mock-device` enables default ack responses.
- `cargo run -- serve --host 127.0.0.1 --port 4002 --mock-device --mock-script ./mock-responses.json` enables topic-keyed scripted responses.
- If `--mock-script` is provided without `--mock-device`, either enable mock-device mode automatically or return a clear startup error. Prefer enabling automatically if it keeps CLI simple; document/test the chosen behavior if practical.

### 4. Route construction

If `main.rs` needs to inject a manager, add a public constructor in `src/api/routes.rs`, for example:

```rust
impl<L, C> AppState<L, C> {
    pub fn new(port_lister: L, connection_manager: C) -> Self {
        Self { port_lister, connection_manager }
    }
}
```

Then `main.rs` can call `router_with_state(AppState::new(SystemPortLister, manager))`.

Do not make route tests brittle by depending on CLI parsing when a direct state constructor is simpler.

---

## Bite-Sized TDD Tasks

### Task 1: Add default mock-device response generation

**Objective:** Prove a framed command with a string `reqId` produces a deterministic JSON response with the same `reqId`.

**Files:**

- Create: `src/serial/mock_device.rs`
- Modify: `src/serial/mod.rs`

**Step 1: Write failing tests**

Add tests like:

```rust
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
```

**Step 2: Run tests to verify RED**

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test serial::mock_device::tests::default_mock_device_acks_command_with_same_req_id -- --nocapture
cargo test serial::mock_device::tests::default_mock_device_ignores_frames_without_string_req_id -- --nocapture
```

Expected RED reason: FAIL because `serial::mock_device` and `MockDeviceResponder` do not exist.

**Step 3: Implement minimal GREEN**

- Create the module.
- Parse and trim the delimiter deterministically.
- Parse JSON object frames with `serde_json`.
- Return default ack only for string `reqId`.

**Step 4: Run tests to verify GREEN**

```bash
cargo test serial::mock_device::tests::default_mock_device_acks_command_with_same_req_id -- --nocapture
cargo test serial::mock_device::tests::default_mock_device_ignores_frames_without_string_req_id -- --nocapture
```

Expected: PASS.

---

### Task 2: Add topic-keyed scripted responses

**Objective:** Prove a script can override the default ack for a command topic while preserving/injecting `reqId`.

**Files:**

- Modify: `src/serial/mock_device.rs`

**Step 1: Write failing tests**

```rust
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
```

**Step 2: Run tests to verify RED/GREEN**

```bash
cargo test serial::mock_device::tests::scripted_mock_device_matches_response_by_topic_and_injects_req_id -- --nocapture
cargo test serial::mock_device::tests::scripted_mock_device_falls_back_to_default_ack_for_unknown_topic -- --nocapture
```

Expected RED reason: FAIL until script structs and topic matching exist. If Task 1 implementation already made part of this pass, keep the tests as regression coverage.

**Step 3: Implement minimal script support**

- Add `MockResponseScript` and `MockResponseRule` with `Deserialize`.
- Add `MockDeviceResponder::from_script`.
- Match the first rule with an exact topic string.
- Inject `reqId` into object responses if missing. If the scripted response is not an object, either wrap/reject it; prefer requiring object responses and returning default ack for invalid script entries.

**Step 4: Run tests to verify GREEN**

```bash
cargo test serial::mock_device::tests::scripted_mock_device_matches_response_by_topic_and_injects_req_id -- --nocapture
cargo test serial::mock_device::tests::scripted_mock_device_falls_back_to_default_ack_for_unknown_topic -- --nocapture
```

Expected: PASS.

---

### Task 3: Wire mock responses through manager event/response recording

**Objective:** Prove `send_command` can synthesize a mock response into the existing manager queue and SSE events when a responder-enabled manager is used, while default managers remain unchanged.

**Files:**

- Modify: `src/serial/manager.rs`
- Modify if needed: `src/serial/mock_device.rs`

**Step 1: Write failing manager tests**

```rust
#[test]
fn manager_with_mock_responder_records_response_after_send_command() {
    let transport = crate::serial::transport::MockSerialTransport::default();
    let manager = ConnectionManagerWithTransport::with_mock_responder(
        transport,
        crate::serial::mock_device::MockDeviceResponder::default(),
    );

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
            serde_json::json!({"reqId":"mock-1","method":"query","topic":"sensor.read","data":{}}),
        )
        .unwrap();

    assert_eq!(queued.req_id, "mock-1");
    assert_eq!(
        manager.take_response("default", "mock-1").unwrap(),
        Some(serde_json::json!({
            "reqId": "mock-1",
            "ok": true,
            "data": {"mock": true, "topic": "sensor.read"}
        }))
    );
    assert_eq!(manager.events().unwrap()[0].event, "serial.json");
}

#[test]
fn default_manager_does_not_auto_record_mock_response() {
    let manager = InMemoryConnectionManager::default();

    manager
        .connect(ConnectionRequest {
            name: "default".to_string(),
            port: "/dev/ROBOT".to_string(),
            baud_rate: 115200,
            delimiter: "\r\n".to_string(),
        })
        .unwrap();

    manager
        .send_command(
            "default",
            serde_json::json!({"reqId":"no-auto","topic":"sensor.read","data":{}}),
        )
        .unwrap();

    assert_eq!(manager.take_response("default", "no-auto").unwrap(), None);
    assert_eq!(manager.events().unwrap(), vec![]);
}
```

Adapt names if the implementation uses a wrapper instead of `with_mock_responder`.

**Step 2: Run tests to verify RED**

```bash
cargo test serial::manager::tests::manager_with_mock_responder_records_response_after_send_command -- --nocapture
cargo test serial::manager::tests::default_manager_does_not_auto_record_mock_response -- --nocapture
```

Expected RED reason: FAIL because the manager has no responder-enabled constructor/hook yet.

**Step 3: Implement minimal manager wiring**

- Add a responder hook without changing default manager behavior.
- Call the hook only after a successful `transport.write_frame`.
- Record returned responses with `record_event_for_connection` so Phase 9 matching and SSE storage are reused.

**Step 4: Run tests to verify GREEN**

```bash
cargo test serial::manager::tests::manager_with_mock_responder_records_response_after_send_command -- --nocapture
cargo test serial::manager::tests::default_manager_does_not_auto_record_mock_response -- --nocapture
```

Expected: PASS.

Optional commit after Tasks 1-3:

```bash
git add src/serial/mock_device.rs src/serial/mod.rs src/serial/manager.rs
git commit -m "feat: add mock device response generator"
```

---

### Task 4: Prove waited route success through mock-device manager

**Objective:** The canonical command endpoint should return waited success using the same manager hook the live server will use.

**Files:**

- Modify: `src/api/routes.rs`
- Modify if needed: `src/serial/manager.rs`

**Step 1: Write failing route test**

```rust
#[tokio::test]
async fn command_route_waits_for_mock_device_response() {
    let manager = ConnectionManagerWithTransport::with_mock_responder(
        crate::serial::transport::MockSerialTransport::default(),
        crate::serial::mock_device::MockDeviceResponder::default(),
    );
    let app = router_with_state(AppState::new(
        MockPortLister { ports: Vec::new() },
        manager,
    ));

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

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/connections/default/commands")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"payload":{"reqId":"mock-route-1","method":"query","topic":"sensor.read","data":{}},"waitForResponse":true,"timeoutMs":1000}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        payload,
        json!({
            "status": "ok",
            "reqId": "mock-route-1",
            "response": {
                "reqId": "mock-route-1",
                "ok": true,
                "data": {"mock": true, "topic": "sensor.read"}
            }
        })
    );
}
```

If `AppState::new` does not exist yet, add it as a small public constructor.

**Step 2: Run test to verify RED**

```bash
cargo test api::routes::tests::command_route_waits_for_mock_device_response -- --nocapture
```

Expected RED reason: FAIL until routes can use the responder-enabled manager and waited route sees the generated response.

**Step 3: Implement minimal route/state constructor support**

- Add `AppState::new` if needed.
- Keep `router()` and existing route behavior unchanged.
- Avoid route-specific mock-device logic; prefer manager-level behavior.

**Step 4: Run test to verify GREEN**

```bash
cargo test api::routes::tests::command_route_waits_for_mock_device_response -- --nocapture
```

Expected: PASS.

---

### Task 5: Add CLI mock-device mode and script loading

**Objective:** Starting the live server with `--mock-device` should enable successful waited commands without hardware.

**Files:**

- Modify: `src/main.rs`
- Modify if needed: `src/api/routes.rs`
- Modify if needed: `src/serial/mock_device.rs`
- Modify if needed: `README.md`

**Step 1: Write failing tests where practical**

If CLI parsing is kept in `src/main.rs`, add small unit-testable helpers rather than trying to spawn a server in unit tests. For example:

```rust
#[test]
fn mock_script_loads_from_json_file_shape() {
    let script = serialport_api::serial::mock_device::MockResponseScript::from_json_str(
        r#"{"responses":[{"topic":"sensor.read","response":{"ok":true,"data":{"temperature":28.5}}}]}"#,
    )
    .unwrap();

    assert_eq!(script.responses().len(), 1);
}
```

Or test CLI parsing if the `Cli` types are test-accessible:

```rust
#[test]
fn serve_cli_accepts_mock_device_and_mock_script() {
    let cli = Cli::parse_from([
        "serialport-api",
        "serve",
        "--mock-device",
        "--mock-script",
        "mock-responses.json",
    ]);

    // assert parsed fields
}
```

**Step 2: Run focused tests to verify RED**

```bash
cargo test mock_script_loads_from_json_file_shape -- --nocapture
# or, if testing CLI parsing:
cargo test serve_cli_accepts_mock_device_and_mock_script -- --nocapture
```

Expected RED reason: FAIL until script loading and/or CLI flags exist.

**Step 3: Implement minimal CLI/server wiring**

- Add `mock_device: bool` and `mock_script: Option<PathBuf>` to `ServeArgs`.
- Load `MockResponseScript` from the provided JSON file when present.
- Build a responder-enabled manager when `--mock-device` is true or a script path is provided.
- Use the existing default router path when neither flag is provided.
- Log whether mock-device mode is enabled, but do not require logging assertions in tests.

**Step 4: Run focused tests to verify GREEN**

```bash
cargo test mock_script_loads_from_json_file_shape -- --nocapture
# or the CLI parsing test name used above
```

Expected: PASS.

Optional commit after Tasks 4-5:

```bash
git add src/api/routes.rs src/main.rs src/serial/mock_device.rs README.md
git commit -m "feat: add mock device server mode"
```

---

### Task 6: Preserve existing timeout and compatibility behavior

**Objective:** Ensure the new opt-in mock-device mode does not alter default command timeout behavior or legacy aliases.

**Files:**

- Modify: tests in `src/api/routes.rs` only if coverage is missing.

**Step 1: Run existing tests before full verification**

```bash
cargo test api::routes::tests::command_route_times_out_waiting_for_response -- --nocapture
cargo test api::routes::tests::command_route_queues_payload_for_named_connection -- --nocapture
cargo test api::routes::tests::commit_alias_queues_payload_for_default_connection -- --nocapture
```

Expected: PASS. The default manager should still produce a `504 Gateway Timeout` for waited commands with no injected response.

**Step 2: Add regression test only if needed**

If existing tests do not cover default no-auto-response behavior after the new constructor changes, add a route test that uses `InMemoryConnectionManager::default()` and asserts waited timeout still returns `504`.

**Step 3: Run full route tests**

```bash
cargo test api::routes::tests -- --nocapture
```

Expected: PASS.

---

## Script File Format

If Phase 10 implements `--mock-script`, use this exact starting format unless there is a compelling reason to simplify further:

```json
{
  "responses": [
    {
      "topic": "sensor.read",
      "response": {
        "ok": true,
        "data": {"temperature": 28.5}
      }
    },
    {
      "topic": "motor.fail",
      "response": {
        "ok": false,
        "error": "mock motor failure"
      }
    }
  ]
}
```

Rules:

- `responses` defaults to an empty list if omitted.
- `topic` must match the outbound command's string `topic` exactly.
- `response` should be a JSON object.
- The responder injects the outbound command's string `reqId` into `response.reqId` if `reqId` is absent.
- Unknown topics use the default ack response.
- Invalid script files should fail server startup clearly rather than panic.

---

## Compatibility Requirements

These existing calls must still work with the default server:

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

Default server waited timeout should remain:

```bash
curl -i -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"reqId":"manual-timeout","method":"query","topic":"sensor.read","data":{}},"waitForResponse":true,"timeoutMs":50}'
```

Expected:

- HTTP status is `504 Gateway Timeout`.
- Body remains deterministic, currently `{"error":"command timed out"}`.

New mock-device waited success should work only when enabled:

```bash
cargo run -- serve --host 127.0.0.1 --port 4002 --mock-device
```

Then:

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}'

curl -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"reqId":"manual-mock-1","method":"query","topic":"sensor.read","data":{}},"waitForResponse":true,"timeoutMs":1000}'
```

Expected response:

```json
{"status":"ok","reqId":"manual-mock-1","response":{"reqId":"manual-mock-1","ok":true,"data":{"mock":true,"topic":"sensor.read"}}}
```

Legacy `/commit` should still return fire-and-forget even in mock-device mode:

```bash
curl -s -X POST http://127.0.0.1:4002/commit \
  -H 'content-type: application/json' \
  -d '{"reqId":"client-42","method":"query","topic":"sensor.read","data":{}}'
```

Expected HTTP response:

```json
{"status":"queued","reqId":"client-42"}
```

It is acceptable if this also records a `serial.json` mock response event internally when mock-device mode is enabled.

---

## Manual Smoke Test Flow

### 1. Default server compatibility smoke

Start default server:

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
curl -s -X POST http://127.0.0.1:4002/disconnect \
  -H 'content-type: application/json' \
  -d '{"name":"default"}'
```

Expected:

- Health returns `{"status":"ok","version":"0.1.0"}`.
- Ports returns a JSON object with `ports` array.
- Fire-and-forget command returns `queued`.
- Waited command still returns `504 Gateway Timeout` on default server.
- `/commit` returns `queued`.
- Disconnect works.

Stop the server.

### 2. Mock-device success smoke

Start server with mock-device mode:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -- serve --host 127.0.0.1 --port 4002 --mock-device
```

In another terminal:

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}'
curl -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"reqId":"manual-mock-1","method":"query","topic":"sensor.read","data":{}},"waitForResponse":true,"timeoutMs":1000}'
curl -i -s --max-time 2 http://127.0.0.1:4002/api/v1/events
```

Expected:

- Waited command returns `status: ok`, `reqId: manual-mock-1`, and a response object with matching `reqId`.
- Events stream contains or can stream a `serial.json` event for the mock response with unchanged SSE event naming.

Stop the server.

### 3. Scripted response smoke

Create a temporary script file:

```bash
cat > /tmp/serialport-api-mock-responses.json <<'JSON'
{
  "responses": [
    {
      "topic": "sensor.read",
      "response": {"ok": true, "data": {"temperature": 28.5}}
    }
  ]
}
JSON
```

Start server:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -- serve --host 127.0.0.1 --port 4002 --mock-device --mock-script /tmp/serialport-api-mock-responses.json
```

In another terminal:

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}'
curl -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"reqId":"script-1","method":"query","topic":"sensor.read","data":{}},"waitForResponse":true,"timeoutMs":1000}'
```

Expected response:

```json
{"status":"ok","reqId":"script-1","response":{"reqId":"script-1","ok":true,"data":{"temperature":28.5}}}
```

Stop the server before final status/commit.

---

## Acceptance Criteria

By the end of Phase 10:

- `src/serial/mock_device.rs` exists and is exported from `src/serial/mod.rs`.
- Default mock responder returns deterministic ack responses for valid framed JSON commands with string `reqId`.
- Frames without string `reqId` do not produce waitable mock responses.
- Topic-keyed scripted responses are supported through a small explicit JSON file format, or a documented narrower alternative if implementation constraints require it.
- Scripted responses preserve/inject the outbound command `reqId`.
- A responder-enabled manager records mock responses through `record_event_for_connection`, so Phase 9 response queues and SSE event storage are reused.
- `InMemoryConnectionManager::default()` and `router()` default behavior remain compatible and do not auto-respond.
- `serve --mock-device` enables manual waited command success without hardware.
- `serve --mock-device --mock-script <path>` enables deterministic topic-keyed scripted responses.
- Fire-and-forget command responses remain exactly `{"status":"queued","reqId":"..."}`.
- `/commit` remains fire-and-forget and returns the same queued shape.
- Legacy aliases still pass tests.
- No hardware-dependent tests are added.
- No SQLite/config-discovery/WebSocket/Socket.IO/packaging changes are added.
- `cargo fmt --check` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `cargo test --all-features` passes.
- Manual smoke proves both default timeout behavior and opt-in mock-device waited success.

Likely implementation commits:

```text
feat: add mock device response generator
feat: add mock device server mode
```

If the implementation is small enough, a single commit is acceptable:

```text
feat: add mock device scripted responses
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
- Latest commit(s) are Phase 10 mock-device response commits.

---

## Copy/Paste Prompt for the Next Coding Session

```text
We are in /home/alfarie/repos/serialport-api on branch rewrite/axum-serial-api. Please execute docs/phase-10-mock-device-scripted-responses-handoff.md.

Load the writing-plans, test-driven-development, and rust-axum-api-tdd skills before editing. This phase adds an opt-in mock-device mode and topic-keyed scripted responses so waited commands can succeed in a live server without real serial hardware. First verify baseline with:

export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features

Use TDD. Add a pure `src/serial/mock_device.rs` responder that converts framed outbound JSON commands with string `reqId` into deterministic JSON responses with the same `reqId`. Add a small script format keyed by command `topic`, inject/preserve `reqId`, and fall back to a default mock ack for unknown topics. Wire an opt-in responder-enabled manager so, after a successful command write, the mock response is recorded via `record_event_for_connection`; this must reuse Phase 9 response queues and SSE storage. Add CLI flags such as `serve --mock-device` and `--mock-script <path>` so manual waited commands can return `{"status":"ok","reqId":"...","response":{...}}` without hardware. Preserve default server behavior, fire-and-forget responses, `/commit`, all legacy aliases, generated reqId sequencing, and SSE event names/shapes. Do not add real serial hardware lifecycle, SQLite, broad config files, WebSocket/Socket.IO, Docker/systemd, authentication, or broad error-model refactors.

After implementation, run cargo fmt --check, cargo clippy --all-targets --all-features -- -D warnings, cargo test --all-features, manually smoke the documented default timeout flow and mock-device success/script flows against cargo run -- serve --host 127.0.0.1 --port 4002, stop the server, and commit focused Phase 10 changes with either:

feat: add mock device response generator
feat: add mock device server mode

or one small combined commit:

feat: add mock device scripted responses
```
