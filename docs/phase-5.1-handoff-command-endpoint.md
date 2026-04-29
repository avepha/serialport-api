# Phase 5.1 Command Endpoint Handoff

> **For Hermes / next AI coding session:** Execute this in a fresh session. Load `test-driven-development` and `rust-axum-api-tdd` before coding. Follow strict RED-GREEN-REFACTOR: write each failing test first, run it and confirm it fails for the expected reason, then implement the minimum code. If you revise this plan, also load `writing-plans`.

**Goal:** Add a mock-backed command sending endpoint and legacy `/commit` alias without opening physical serial ports yet.

**Architecture:** Keep HTTP route wiring in `src/api/routes.rs`. Add only the smallest serial manager behavior required to record framed outbound command bytes against the existing in-memory connection state. Reuse `crate::protocol::frame_json` for JSON + delimiter framing, and keep real serial I/O, read loops, response waiting, and SSE for later phases.

**Tech Stack:** Rust 2021, Axum 0.7, Tokio 1, Serde, Serde JSON, existing `protocol` and `serial::manager` modules.

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
34d7aec feat: add legacy connection aliases
9e5c58e docs: add phase 4.3 handoff plan
8f66424 feat: add mock connection lifecycle endpoints
1289a7a docs: add phase 4.2 handoff plan
6edb02d feat: add serial port listing endpoint
cd16800 feat: add axum health endpoint
6b3a2f9 feat: add serial protocol parsing foundation
```

Relevant completed work:

- Phase 2: `src/protocol.rs` has `frame_json(value, delimiter)` and inbound line parsing.
- Phase 4.1: `GET /api/v1/ports` and port lister abstraction exist.
- Phase 4.2: in-memory connection lifecycle exists:
  - `POST /api/v1/connections`
  - `GET /api/v1/connections`
  - `DELETE /api/v1/connections/:name`
- Phase 4.3: legacy aliases exist and share the same `AppState`:
  - `GET /list`
  - `GET /info`
  - `POST /connect`
  - `POST /disconnect`

Baseline verification before starting:

```bash
cd /home/alfarie/repos/serialport-api
git status --short --branch
cargo fmt --check
cargo check
cargo test
```

Expected: clean branch and all commands pass.

---

## Do Not Do Yet

Do **not** add:

- physical serial port opening
- `tokio-serial` or async serial transport
- background serial read loops
- waiting for matching responses from serial hardware
- SSE events or WebSocket / Socket.IO compatibility
- SQLite / preset storage
- hardware-dependent tests
- config file support
- real `/dev/ttyUSB0` writes

Those belong to later phases. This phase is only a mock-backed outbound command API that proves request/response shapes, `reqId` handling, framing, and shared state.

---

## API Scope for This Phase

Add canonical endpoint:

```text
POST /api/v1/connections/:name/commands
```

Add legacy compatibility alias:

```text
POST /commit
```

### Canonical request shape

```json
{
  "payload": {
    "method": "query",
    "topic": "sensor.read",
    "data": {}
  },
  "waitForResponse": false,
  "timeoutMs": 2000
}
```

For this phase:

- `payload` must be a JSON object.
- If `payload.reqId` is missing, generate one.
- If `payload.reqId` is present, preserve it.
- `waitForResponse` and `timeoutMs` may be accepted for compatibility but should not implement waiting yet.
- Return `queued`, not a waited response.

### Canonical response shape

```json
{"status":"queued","reqId":"1"}
```

### Legacy `/commit` request shape

Use the old/simple shape where the posted JSON body is the command payload itself:

```json
{
  "method": "query",
  "topic": "sensor.read",
  "data": {}
}
```

For this phase, `/commit` should send to the connection named `default`.

### Legacy `/commit` response shape

```json
{"status":"queued","reqId":"1"}
```

If the test connects `default` first, `/commit` must use that connection's delimiter when framing.

---

## Suggested Design

The exact implementation may vary, but keep it minimal and testable.

### `src/serial/manager.rs`

Extend the existing `ConnectionManager` trait with one outbound command method:

```rust
fn send_command(&self, connection_name: &str, payload: serde_json::Value) -> Result<QueuedCommand>;
```

Add a response/model type:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct QueuedCommand {
    #[serde(rename = "reqId")]
    pub req_id: String,
}
```

For test visibility, the in-memory manager may also expose recorded writes:

```rust
#[cfg(test)]
pub fn written_frames(&self, name: &str) -> Vec<Vec<u8>> { ... }
```

Recommended in-memory behavior:

- Store existing `ConnectionInfo` records as today.
- Add a sequence counter, e.g. `Arc<Mutex<u64>>`, for generated `reqId` values.
- Add a recorded-write map, e.g. `Arc<Mutex<BTreeMap<String, Vec<Vec<u8>>>>>`.
- `send_command` should:
  1. Look up `connection_name`.
  2. If no such connection exists, return an error.
  3. Ensure the payload is a JSON object.
  4. Preserve existing string `reqId`, or insert the next generated string ID if missing.
  5. Frame the updated payload using `crate::protocol::frame_json(&payload, &connection.delimiter)`.
  6. Store framed bytes under that connection name.
  7. Return `QueuedCommand { req_id }`.

Do not add real hardware I/O.

### `src/error.rs`

If needed, add small typed errors only for behavior in this phase, for example:

```rust
#[error("connection not found: {0}")]
ConnectionNotFound(String),

#[error("command payload must be a JSON object")]
InvalidCommandPayload,
```

Do not design the full final error model yet.

### `src/api/routes.rs`

Add request/response types in the routes module unless a type clearly belongs in `serial::manager`.

Suggested request type:

```rust
#[derive(Debug, Deserialize)]
struct CommandRequest {
    payload: serde_json::Value,
    #[serde(rename = "waitForResponse", default)]
    wait_for_response: bool,
    #[serde(rename = "timeoutMs")]
    timeout_ms: Option<u64>,
}
```

It is okay if `wait_for_response` and `timeout_ms` are unused in Phase 5.1; prefix local bindings with `_` or destructure carefully to avoid warnings.

Suggested response type:

```rust
#[derive(Debug, Serialize)]
struct CommandResponse {
    status: &'static str,
    #[serde(rename = "reqId")]
    req_id: String,
}
```

Add routes:

```rust
.route(
    "/api/v1/connections/:name/commands",
    post(send_command::<L, C>),
)
.route("/commit", post(commit_alias::<L, C>))
```

Because this project uses Axum 0.7, keep path captures as `:name`, not `{name}`.

---

## Acceptance Criteria

By the end of this phase:

- `cargo fmt --check` passes.
- `cargo check` passes.
- `cargo test` passes.
- Existing canonical endpoints still work:
  - `GET /api/v1/health`
  - `GET /api/v1/ports`
  - `POST /api/v1/connections`
  - `GET /api/v1/connections`
  - `DELETE /api/v1/connections/:name`
- Existing legacy aliases still work:
  - `GET /list`
  - `GET /info`
  - `POST /connect`
  - `POST /disconnect`
- New canonical command endpoint works against in-memory/mock state:
  - `POST /api/v1/connections/:name/commands`
- New legacy alias works against the same default connection state:
  - `POST /commit`
- Command payloads are framed with the connected connection's delimiter.
- Existing `reqId` values are preserved.
- Missing `reqId` values are generated.
- No physical serial port is opened or written.

---

## Task 1: Add failing serial manager command tests

**Objective:** Define in-memory command queuing and framing before implementation.

**Files:**

- Modify: `src/serial/manager.rs`
- Existing helper: `src/protocol.rs::frame_json`

**Step 1: Write failing tests**

Add tests in `src/serial/manager.rs` under the existing `#[cfg(test)] mod tests`.

Suggested test names:

```rust
#[test]
fn in_memory_connection_manager_records_framed_command_with_generated_req_id() {
    let manager = InMemoryConnectionManager::default();

    manager
        .connect(ConnectionRequest {
            name: "default".to_string(),
            port: "/dev/ttyUSB0".to_string(),
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
    assert_eq!(
        manager.written_frames("default"),
        vec![br#"{"data":{},"method":"query","reqId":"1","topic":"sensor.read"}
"#.to_vec()]
    );
}

#[test]
fn in_memory_connection_manager_preserves_existing_req_id() {
    let manager = InMemoryConnectionManager::default();

    manager
        .connect(ConnectionRequest {
            name: "default".to_string(),
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: 115200,
            delimiter: "\n".to_string(),
        })
        .unwrap();

    let queued = manager
        .send_command(
            "default",
            serde_json::json!({
                "reqId": "client-42",
                "method": "mutation",
                "topic": "led.set",
                "data": {"on": true}
            }),
        )
        .unwrap();

    assert_eq!(queued.req_id, "client-42");
    assert_eq!(
        manager.written_frames("default"),
        vec![br#"{"data":{"on":true},"method":"mutation","reqId":"client-42","topic":"led.set"}
"#.to_vec()]
    );
}
```

If `serde_json::json!` object key ordering makes byte assertions brittle, either:

1. Build the payload using `serde_json::Map` in the insertion order expected by the test; or
2. Assert that the frame ends with the delimiter and parse the frame body back to `serde_json::Value` before comparing.

Prefer option 2 if formatting differs.

**Step 2: Run tests and confirm RED**

Run:

```bash
cargo test serial::manager::tests::in_memory_connection_manager_records_framed_command_with_generated_req_id -- --nocapture
cargo test serial::manager::tests::in_memory_connection_manager_preserves_existing_req_id -- --nocapture
```

Expected: FAIL because `send_command`, `QueuedCommand`, and/or `written_frames` do not exist yet.

**Step 3: Implement minimal serial manager behavior**

Modify only what is needed in `src/serial/manager.rs` and, if necessary, `src/error.rs`.

Implementation notes:

- Add `send_command` to `ConnectionManager`.
- Extend `InMemoryConnectionManager` fields.
- Reuse `crate::protocol::frame_json`.
- Store frames in memory for tests/manual mock behavior.
- Do not open hardware.

**Step 4: Re-run narrow tests and confirm GREEN**

Run:

```bash
cargo test serial::manager::tests::in_memory_connection_manager_records_framed_command_with_generated_req_id -- --nocapture
cargo test serial::manager::tests::in_memory_connection_manager_preserves_existing_req_id -- --nocapture
```

Expected: PASS.

---

## Task 2: Add failing canonical route test

**Objective:** Define `POST /api/v1/connections/:name/commands` route behavior before implementation.

**Files:**

- Modify: `src/api/routes.rs`

**Step 1: Write failing route test**

Add a route test in `src/api/routes.rs`.

Suggested test name:

```rust
api::routes::tests::command_route_queues_payload_for_named_connection
```

Test flow:

1. Build `app` with `router_with_state(AppState { port_lister: MockPortLister { ports: Vec::new() }, connection_manager: InMemoryConnectionManager::default() })`.
2. `POST /api/v1/connections` to create `default` with delimiter `\r\n`.
3. `POST /api/v1/connections/default/commands` with:

```json
{
  "payload": {
    "method": "query",
    "topic": "sensor.read",
    "data": {}
  },
  "waitForResponse": false,
  "timeoutMs": 2000
}
```

4. Assert status `200 OK`.
5. Assert JSON response:

```json
{"status":"queued","reqId":"1"}
```

**Step 2: Run test and confirm RED**

Run:

```bash
cargo test api::routes::tests::command_route_queues_payload_for_named_connection -- --nocapture
```

Expected: FAIL because the command route is not implemented yet, likely `404 Not Found` or missing handler.

**Step 3: Implement minimal canonical route wiring**

In `src/api/routes.rs`:

- Import `serde_json::Value` if useful.
- Add `CommandRequest` and `CommandResponse` types.
- Add route:

```rust
.route(
    "/api/v1/connections/:name/commands",
    post(send_command::<L, C>),
)
```

- Add handler:

```rust
async fn send_command<L, C>(
    State(state): State<AppState<L, C>>,
    Path(name): Path<String>,
    Json(request): Json<CommandRequest>,
) -> Result<Json<CommandResponse>, StatusCode>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    let queued = state
        .connection_manager
        .send_command(&name, request.payload)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(CommandResponse {
        status: "queued",
        req_id: queued.req_id,
    }))
}
```

Adjust for exact type names chosen in Task 1.

**Step 4: Re-run route test and confirm GREEN**

Run:

```bash
cargo test api::routes::tests::command_route_queues_payload_for_named_connection -- --nocapture
```

Expected: PASS.

---

## Task 3: Add failing legacy `/commit` route test

**Objective:** Define old `/commit` compatibility behavior backed by the same `AppState` and default connection.

**Files:**

- Modify: `src/api/routes.rs`

**Step 1: Write failing route test**

Suggested test name:

```rust
api::routes::tests::commit_alias_queues_payload_for_default_connection
```

Test flow:

1. Build `app` with `router_with_state` and `InMemoryConnectionManager::default()`.
2. Create default connection using legacy alias `POST /connect` or canonical `POST /api/v1/connections`.
3. `POST /commit` with raw command payload:

```json
{
  "reqId": "client-42",
  "method": "mutation",
  "topic": "led.set",
  "data": {"on": true}
}
```

4. Assert status `200 OK`.
5. Assert JSON response:

```json
{"status":"queued","reqId":"client-42"}
```

**Step 2: Run test and confirm RED**

Run:

```bash
cargo test api::routes::tests::commit_alias_queues_payload_for_default_connection -- --nocapture
```

Expected: FAIL because `/commit` is not implemented yet, likely `404 Not Found`.

**Step 3: Implement minimal `/commit` alias**

In `src/api/routes.rs`:

- Add route:

```rust
.route("/commit", post(commit_alias::<L, C>))
```

- Add handler that sends to `default`:

```rust
async fn commit_alias<L, C>(
    State(state): State<AppState<L, C>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<CommandResponse>, StatusCode>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    let queued = state
        .connection_manager
        .send_command("default", payload)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(CommandResponse {
        status: "queued",
        req_id: queued.req_id,
    }))
}
```

**Step 4: Re-run legacy test and confirm GREEN**

Run:

```bash
cargo test api::routes::tests::commit_alias_queues_payload_for_default_connection -- --nocapture
```

Expected: PASS.

---

## Task 4: Full verification and manual smoke test

**Objective:** Prove all existing and new behavior works through the running server.

Run full Rust checks:

```bash
cargo fmt
cargo fmt --check
cargo check
cargo test
```

Expected: all pass.

Start server:

```bash
cargo run -- serve --host 127.0.0.1 --port 4002
```

From another shell, run:

```bash
curl -s http://127.0.0.1:4002/api/v1/health
curl -s http://127.0.0.1:4002/api/v1/ports

curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\r\n"}'

curl -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"method":"query","topic":"sensor.read","data":{}},"waitForResponse":false,"timeoutMs":2000}'

curl -s -X POST http://127.0.0.1:4002/commit \
  -H 'content-type: application/json' \
  -d '{"reqId":"client-42","method":"mutation","topic":"led.set","data":{"on":true}}'

curl -s http://127.0.0.1:4002/info

curl -s -X POST http://127.0.0.1:4002/disconnect \
  -H 'content-type: application/json' \
  -d '{"name":"default"}'
```

Expected response shapes:

```json
{"status":"ok","version":"0.1.0"}
```

```json
{"ports":[]}
```

```json
{"status":"connected","connection":{"name":"default","status":"connected","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\r\n"}}
```

```json
{"status":"queued","reqId":"1"}
```

```json
{"status":"queued","reqId":"client-42"}
```

```json
{"connections":[{"name":"default","status":"connected","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\r\n"}]}
```

```json
{"status":"disconnected","name":"default"}
```

Stop the server afterward.

---

## Task 5: Commit

Inspect changes:

```bash
git status --short --branch
git diff -- src/api/routes.rs src/serial/manager.rs src/error.rs src/protocol.rs
```

Stage only intended files. Likely:

```bash
git add src/api/routes.rs src/serial/manager.rs src/error.rs
```

If `src/protocol.rs` was modified for a small helper, include it too:

```bash
git add src/protocol.rs
```

Commit:

```bash
git commit -m "feat: add mock command endpoint"
```

Final verification:

```bash
git status --short --branch
git log --oneline -5
```

---

## Expected Files to Modify

Likely code files:

- Modify: `src/serial/manager.rs`
- Modify: `src/api/routes.rs`
- Modify: `src/error.rs` only if needed for small typed errors
- Modify: `src/protocol.rs` only if a tiny helper is needed

Do not modify docs unless the plan is wrong. If you update/fix this handoff doc, include it in a separate docs commit.

---

## Expected Commit Message

```text
feat: add mock command endpoint
```

If the handoff doc is updated separately:

```text
docs: update phase 5.1 command endpoint handoff
```

---

## Full Prompt for the Next Coding Session

Copy/paste this into a fresh AI coding session:

```text
We are working on the Rust rewrite of the serialport-api repo.

Repository path:

/home/alfarie/repos/serialport-api

Use the existing branch:

rewrite/axum-serial-api

Important: avoid context pollution from previous implementation sessions. Start by reading the handoff document and then execute Phase 5.1 exactly from it.

Read this file first:

docs/phase-5.1-handoff-command-endpoint.md

Also refer to these existing docs if needed:

docs/open-source-spec.md
docs/implementation-plan.md
docs/phase-4.3-handoff-compatibility-aliases.md

Current completed work:

- Phase 0 is done:
  - branch created
  - planning docs committed

- Phase 1 is done:
  - Rocket removed
  - Rust edition upgraded to 2021
  - project renamed to serialport-api
  - stable Rust build passes

- Phase 2 is done:
  - protocol/error modules added
  - JSON + delimiter framing implemented in src/protocol.rs
  - JSON line parsing implemented
  - plain text parsing implemented
  - method detection for "log" and "notification" implemented
  - reqId is preserved in parsed JSON events
  - protocol tests pass

- Phase 3 is done:
  - Axum added
  - GET /api/v1/health implemented
  - serve CLI command implemented
  - manual curl verification passes

- Phase 4.1 is done:
  - serial manager foundation started
  - mockable port listing abstraction added
  - GET /api/v1/ports implemented
  - manual curl verification passes

- Phase 4.2 is done:
  - in-memory connection lifecycle manager added
  - POST /api/v1/connections implemented
  - GET /api/v1/connections implemented
  - DELETE /api/v1/connections/:name implemented
  - manual curl verification passes

- Phase 4.3 is done:
  - GET /list implemented
  - GET /info implemented
  - POST /connect implemented
  - POST /disconnect implemented
  - all aliases share the same AppState as canonical routes
  - latest code commit: 34d7aec feat: add legacy connection aliases

Your task is to execute Phase 5.1: Mock-backed Command Endpoint.

Goal:

Add outbound command endpoints backed by the in-memory connection manager only:

- POST /api/v1/connections/:name/commands
- POST /commit compatibility alias that sends to the "default" connection

Acceptance criteria:

- Use test-driven development.
- Write failing serial manager command tests first.
- Run them and confirm they fail for the expected reason.
- Implement only minimal in-memory command recording/framing behavior.
- Write failing canonical route test first.
- Run it and confirm it fails for the expected reason.
- Implement only minimal canonical route wiring.
- Write failing /commit alias route test first.
- Run it and confirm it fails for the expected reason.
- Implement only minimal /commit alias route wiring.
- Use crate::protocol::frame_json for JSON + delimiter framing.
- Generate reqId when missing.
- Preserve reqId when present.
- Return queued response shape: {"status":"queued","reqId":"..."}
- Keep HTTP layer separate from serial domain logic.
- Do not implement physical serial port opening/writing yet.
- Do not implement background read loops, response waiting, SSE, Socket.IO, SQLite, config files, or presets.
- cargo fmt --check passes.
- cargo check passes.
- cargo test passes.
- cargo run -- serve --host 127.0.0.1 --port 4002 starts an HTTP server.
- Existing canonical endpoints still work.
- Existing legacy aliases still work.
- New manual command flow works through both /api/v1/connections/:name/commands and /commit.

Expected files to modify:

- Modify: src/serial/manager.rs
- Modify: src/api/routes.rs
- Modify: src/error.rs only if a small typed error is needed
- Modify: src/protocol.rs only if a tiny helper is needed

Expected commit message:

feat: add mock command endpoint

Before starting, verify the baseline:

cd /home/alfarie/repos/serialport-api
git status --short --branch
cargo fmt --check
cargo check
cargo test

Then follow docs/phase-5.1-handoff-command-endpoint.md task by task.

Required TDD sequence:

1. Write failing serial manager command tests first.
2. Run:

cargo test serial::manager::tests::in_memory_connection_manager_records_framed_command_with_generated_req_id -- --nocapture
cargo test serial::manager::tests::in_memory_connection_manager_preserves_existing_req_id -- --nocapture

Expected: FAIL because send_command / recorded frames do not exist yet.

3. Implement the minimal in-memory command behavior.

4. Re-run those tests.

Expected: PASS.

5. Write failing canonical route test:

api::routes::tests::command_route_queues_payload_for_named_connection

6. Run:

cargo test api::routes::tests::command_route_queues_payload_for_named_connection -- --nocapture

Expected: FAIL because command route is not implemented yet, likely 404 Not Found.

7. Implement minimal route wiring for POST /api/v1/connections/:name/commands.

8. Re-run the canonical route test.

Expected: PASS.

9. Write failing legacy alias route test:

api::routes::tests::commit_alias_queues_payload_for_default_connection

10. Run:

cargo test api::routes::tests::commit_alias_queues_payload_for_default_connection -- --nocapture

Expected: FAIL because /commit is not implemented yet, likely 404 Not Found.

11. Implement minimal /commit alias wiring.

12. Re-run the alias route test.

Expected: PASS.

13. Run full verification:

cargo fmt
cargo fmt --check
cargo check
cargo test

14. Run manual verification:

cargo run -- serve --host 127.0.0.1 --port 4002

From another shell:

curl -s http://127.0.0.1:4002/api/v1/health
curl -s http://127.0.0.1:4002/api/v1/ports

curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\r\n"}'

curl -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"method":"query","topic":"sensor.read","data":{}},"waitForResponse":false,"timeoutMs":2000}'

curl -s -X POST http://127.0.0.1:4002/commit \
  -H 'content-type: application/json' \
  -d '{"reqId":"client-42","method":"mutation","topic":"led.set","data":{"on":true}}'

curl -s http://127.0.0.1:4002/info

curl -s -X POST http://127.0.0.1:4002/disconnect \
  -H 'content-type: application/json' \
  -d '{"name":"default"}'

Stop the server afterward.

15. Commit:

git add src/api/routes.rs src/serial/manager.rs src/error.rs src/protocol.rs
git commit -m "feat: add mock command endpoint"

Only stage files that actually changed. If src/error.rs or src/protocol.rs were not modified, do not stage them.

If you update or fix the handoff doc, include it in a separate docs commit.

When finished, report:

- files changed
- tests run
- manual curl results
- commit hash
- current git status
```
