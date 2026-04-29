# Phase 4.2 Connection Lifecycle Handoff

> **For Hermes:** Execute this in a fresh session. Load `test-driven-development` before coding. Follow RED-GREEN-REFACTOR: write each test first, run it and confirm it fails for the expected reason, then implement the minimum code. If you revise this plan, also load `writing-plans`.

**Goal:** Add a mockable/testable connection lifecycle foundation for `serialport-api` without implementing serial I/O or command sending yet.

**Architecture:** Keep HTTP routes in `src/api/routes.rs`, serial domain types in `src/serial/manager.rs`, and reusable serial module exports in `src/serial/mod.rs`. Introduce shared application state only as needed to support both port listing and connection lifecycle routes. The next step should be an in-memory/mock connection registry that records requested connections; do not open physical serial ports yet.

**Tech Stack:** Rust 2021, Axum 0.7, Tokio 1, Clap 4, Serde, existing protocol and serial manager modules.

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

Relevant completed commits:

```text
cd16800 feat: add axum health endpoint
88c9649 docs: add phase 3 handoff plan
6b3a2f9 feat: add serial protocol parsing foundation
7085601 chore: modernize Rust project foundation
fc270c0 docs: define open source serialport api rewrite
```

Phase 4.1 is complete when this handoff is committed:

- `src/serial/mod.rs` exists.
- `src/serial/manager.rs` contains:
  - `PortInfo`
  - `SerialPortLister`
  - `SystemPortLister`
  - `list_ports`
- `src/api/routes.rs` exposes:
  - `GET /api/v1/health`
  - `GET /api/v1/ports`
- `GET /api/v1/ports` returns:

```json
{"ports":[]}
```

on systems with no detected serial ports.

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

This phase is still a foundation phase. Do **not** implement serial hardware behavior yet.

Do not add:

- physical serial port opening
- background serial read loops
- command sending
- `POST /api/v1/connections/{name}/commands`
- `POST /commit`
- SSE events
- Socket.IO compatibility
- SQLite/preset storage
- hardware-dependent tests

Those belong to later phases.

---

## Acceptance Criteria

By the end of this phase:

- `cargo fmt --check` passes.
- `cargo check` passes.
- `cargo test` passes.
- Existing endpoints still work:
  - `GET /api/v1/health`
  - `GET /api/v1/ports`
- New connection lifecycle endpoints work against an in-memory/mock manager:

```text
POST /api/v1/connections
GET /api/v1/connections
DELETE /api/v1/connections/{name}
```

- Suggested manual flow:

```bash
cargo run -- serve --host 127.0.0.1 --port 4002
curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\\r\\n"}'
curl -s http://127.0.0.1:4002/api/v1/connections
curl -s -X DELETE http://127.0.0.1:4002/api/v1/connections/default
```

Expected response shapes:

```json
{"status":"connected","connection":{"name":"default","status":"connected","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\r\n"}}
```

```json
{"connections":[{"name":"default","status":"connected","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\r\n"}]}
```

```json
{"status":"disconnected","name":"default"}
```

- Work is committed with a conventional commit message, suggested:

```text
feat: add mock connection lifecycle endpoints
```

---

## Task 1: Add Connection Domain Types and In-Memory Manager

**Objective:** Define connection request/state types and a mockable in-memory manager without opening serial hardware.

**Files:**

- Modify: `src/serial/manager.rs`
- Test: `src/serial/manager.rs`

**Step 1: Write failing manager tests**

Add tests for:

1. `connect` records a connection.
2. `connections` lists recorded connections.
3. `disconnect` removes the named connection.

Suggested test behavior:

```rust
#[test]
fn in_memory_connection_manager_records_connections() {
    let manager = InMemoryConnectionManager::default();

    let connection = manager
        .connect(ConnectionRequest {
            name: "default".to_string(),
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: 115200,
            delimiter: "\r\n".to_string(),
        })
        .unwrap();

    assert_eq!(connection.name, "default");
    assert_eq!(connection.status, "connected");
    assert_eq!(manager.connections().unwrap(), vec![connection]);
}
```

**Step 2: Run the manager test and confirm RED**

Run:

```bash
cargo test serial::manager::tests::in_memory_connection_manager_records_connections -- --nocapture
```

Expected: FAIL because connection lifecycle types/functions are not defined.

**Step 3: Implement minimal manager**

Add only the minimal types needed:

- `ConnectionRequest`
- `ConnectionInfo`
- `ConnectionManager` trait, if useful
- `InMemoryConnectionManager`

Use an in-memory `Arc<Mutex<...>>` or similar cloneable state. Avoid serial hardware access.

Important serde naming:

- Rust field can be `baud_rate`.
- JSON field must be `baudRate`.
- Use `#[serde(rename = "baudRate")]`.

**Step 4: Run the manager tests and confirm GREEN**

Run:

```bash
cargo test serial::manager::tests::in_memory_connection_manager -- --nocapture
```

Expected: PASS.

---

## Task 2: Add HTTP Tests for Connection Lifecycle

**Objective:** Define route behavior before route implementation.

**Files:**

- Modify: `src/api/routes.rs`

**Step 1: Write failing route tests**

Add tests for:

- `POST /api/v1/connections`
- `GET /api/v1/connections`
- `DELETE /api/v1/connections/{name}`

The route tests should use the in-memory/mock manager and must not require serial hardware.

**Step 2: Run the specific route tests and confirm RED**

Run:

```bash
cargo test api::routes::tests::connection_lifecycle_routes_manage_mock_connections -- --nocapture
```

Expected: FAIL because the routes are not implemented.

---

## Task 3: Implement Minimal Connection Routes

**Objective:** Make the route tests pass with the smallest Axum implementation.

**Files:**

- Modify: `src/api/routes.rs`

**Implementation notes:**

- You may need an `AppState` struct instead of storing only the port lister as router state.
- Preserve the existing ability to inject a mock `SerialPortLister` for `/api/v1/ports` tests.
- Do not break `router()` used by `src/main.rs`.
- Keep response JSON stable and simple.

Suggested state shape:

```rust
#[derive(Clone)]
pub struct AppState<L, C> {
    port_lister: L,
    connection_manager: C,
}
```

`router()` can construct:

```rust
router_with_state(AppState {
    port_lister: SystemPortLister,
    connection_manager: InMemoryConnectionManager::default(),
})
```

**Step 2: Run the route tests and confirm GREEN**

Run:

```bash
cargo test api::routes::tests::connection_lifecycle_routes_manage_mock_connections -- --nocapture
```

Expected: PASS.

---

## Task 4: Full Verification and Manual Smoke Test

Run:

```bash
cargo fmt
cargo fmt --check
cargo check
cargo test
git status --short --branch
```

Start the server:

```bash
cargo run -- serve --host 127.0.0.1 --port 4002
```

From another shell:

```bash
curl -s http://127.0.0.1:4002/api/v1/health
curl -s http://127.0.0.1:4002/api/v1/ports
curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\\r\\n"}'
curl -s http://127.0.0.1:4002/api/v1/connections
curl -s -X DELETE http://127.0.0.1:4002/api/v1/connections/default
```

Stop the server afterward.

---

## Task 5: Commit

```bash
git add src/api/routes.rs src/serial/manager.rs src/serial/mod.rs src/lib.rs
git commit -m "feat: add mock connection lifecycle endpoints"
```

Final verification:

```bash
git status --short --branch
git log --oneline -5
```

---

## Full Prompt for the Next Coding Session

Copy/paste this into a fresh session:

```text
We are working on the Rust rewrite of the serialport-api repo.

Repository path:

/home/alfarie/repos/serialport-api

Use the existing branch:

rewrite/axum-serial-api

Important: avoid context pollution from previous implementation sessions. Start by reading the handoff document and then execute Phase 4.2 exactly from it.

Read this file first:

docs/phase-4.2-handoff-connection-lifecycle.md

Also refer to these existing docs if needed:

docs/open-source-spec.md
docs/implementation-plan.md

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
  - JSON + CRLF framing implemented
  - JSON line parsing implemented
  - plain text parsing implemented
  - method detection for "log" and "notification" implemented
  - reqId is preserved
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

Your task is to execute Phase 4.2: Connection Lifecycle Foundation.

Goal:

Add mock/in-memory connection lifecycle endpoints without opening serial hardware yet:

- POST /api/v1/connections
- GET /api/v1/connections
- DELETE /api/v1/connections/{name}

Acceptance criteria:

- Use test-driven development.
- Write failing manager/route tests first.
- Run them and confirm they fail for the expected reason.
- Implement only the minimal in-memory/mock connection manager and Axum routes.
- Keep HTTP layer separate from serial domain logic.
- Do not implement serial hardware behavior yet.
- Do not implement command sending, SSE, Socket.IO compatibility, SQLite, or Phase 5.
- cargo fmt --check passes.
- cargo check passes.
- cargo test passes.
- cargo run -- serve --host 127.0.0.1 --port 4002 starts an HTTP server.
- Existing curl checks still work:
  - curl -s http://127.0.0.1:4002/api/v1/health
  - curl -s http://127.0.0.1:4002/api/v1/ports
- New manual lifecycle flow works:
  - POST /api/v1/connections creates an in-memory connection
  - GET /api/v1/connections lists it
  - DELETE /api/v1/connections/default removes it

Expected files to modify:

- Modify: src/serial/manager.rs
- Modify: src/api/routes.rs
- Modify if needed: src/serial/mod.rs
- Modify if needed: src/lib.rs

Expected commit message:

feat: add mock connection lifecycle endpoints

Before starting, verify the baseline:

cd /home/alfarie/repos/serialport-api
git status --short --branch
cargo fmt --check
cargo check
cargo test

Then follow docs/phase-4.2-handoff-connection-lifecycle.md task by task.

When finished, report:

- files changed
- tests run
- manual curl results
- commit hash
- current git status
```
