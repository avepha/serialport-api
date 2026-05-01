# Phase 16 WebSocket Events Handoff

> **For Hermes / next AI implementation session:** Execute this in a fresh session. Load `writing-plans`, `test-driven-development`, and `rust-axum-api-tdd` before editing. This phase should add native WebSocket serial-event streaming to the existing Axum `serialport-api` service while preserving the current HTTP/SSE API. Keep tests hardware-free. Do **not** implement Socket.IO protocol compatibility unless it is deliberately scoped as a separate, fully tested protocol feature.

**Goal:** Add a native WebSocket endpoint for serial events, likely `GET /api/v1/events/ws`, so browser and CLI clients can receive the same recorded serial event objects currently exposed by `GET /api/v1/events` (SSE). Reuse the existing manager event snapshot behavior first; do not redesign the event store or introduce live broadcast fan-out unless it remains small, well tested, and backward-compatible.

**Inferred next phase:** Phase 16 is **native WebSocket event streaming support**. Repository evidence supports this ordering:

- Phase 15 completed Docker and release packaging (`d972b68 chore: add Docker and release packaging`).
- `README.md` now lists `WebSocket or Socket.IO support` and `ARM/Raspberry Pi release binary automation` as remaining planned work, while Docker/release packaging is implemented.
- `README.md` later says WebSocket or Socket.IO compatibility is future browser-client work.
- Existing `src/api/routes.rs` exposes `GET /api/v1/events` as SSE and route tests already seed recorded serial events and assert serialized event content.
- `src/serial/manager.rs` stores serial events as `SerialStreamEvent { event, data }` and exposes them through the `ConnectionManager::events()` snapshot method.
- `Cargo.toml` uses `axum = "0.7"` without the `ws` feature enabled; native Axum WebSocket support will require dependency/feature changes.
- `docs/open-source-spec.md` states SSE was implemented first and WebSocket can be added later for browser clients; Socket.IO was an old compatibility concept, not an already-implemented protocol.

---

## Strict Orchestration Input Schema

The implementation agent should accept this handoff plus the repository as its complete input. No hidden context is required.

```json
{
  "agent_role": "implementation",
  "phase": "Phase 16",
  "repository": "/home/alfarie/repos/serialport-api",
  "branch": "rewrite/axum-serial-api",
  "base_commit_expected": "d972b68 chore: add Docker and release packaging",
  "toolchain_env": {
    "PATH_prefix": "$HOME/.cargo/bin"
  },
  "scope": "Add native WebSocket event endpoint for existing serial event snapshots, route tests, README/API docs refresh, and hardware-free verification",
  "non_goals": [
    "Socket.IO protocol compatibility or Engine.IO handshake semantics",
    "Removing, renaming, or changing GET /api/v1/events SSE behavior",
    "Changing command, connection, preset, config, Docker, systemd, or release behavior",
    "Authentication, TLS, CORS, reverse proxy, browser UI, or frontend client",
    "Hardware-required automated tests",
    "Large event-store redesign or required live broadcast architecture",
    "Pushing commits or tags"
  ]
}
```

### Required Preconditions

Before editing, run:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
git log --oneline -6
```

Expected:

- Branch is `rewrite/axum-serial-api`.
- Working tree is clean.
- Recent history includes `d972b68 chore: add Docker and release packaging` or a descendant of it.

If the working tree is not clean before Phase 16 edits, stop and report instead of modifying files.

---

## Strict Orchestration Output Schema

The implementation agent's final response should use this JSON shape:

```json
{
  "agent_role": "implementation",
  "phase": "Phase 16",
  "summary": [
    "Added a native WebSocket event endpoint at /api/v1/events/ws.",
    "Preserved existing SSE event behavior and added hardware-free WebSocket route tests."
  ],
  "files_changed": [
    "Cargo.toml",
    "Cargo.lock",
    "README.md",
    "src/api/routes.rs"
  ],
  "verification": {
    "commands_run": [
      "cargo fmt --check",
      "cargo clippy --all-targets --all-features -- -D warnings",
      "cargo test --all-features",
      "manual WebSocket smoke check listed in this handoff if feasible"
    ],
    "status": "passed"
  },
  "commit": "<sha or null>",
  "approval_status": "ready_for_review|blocked",
  "issues": []
}
```

If blocked, set `commit` to `null`, `approval_status` to `blocked`, and list exact blockers.

---

## Current Repository State to Understand First

Repository path:

```bash
/home/alfarie/repos/serialport-api
```

Expected branch:

```text
rewrite/axum-serial-api
```

Known latest completed Phase 15 commit:

```text
d972b68 chore: add Docker and release packaging
```

Important current behavior after Phase 15:

- `cargo run -- serve --host 127.0.0.1 --port 4002` starts the Axum HTTP server in mock/in-memory mode.
- Default startup is hardware-free and does not open physical serial ports.
- `serve --real-serial` opts into opening/writing/reading OS serial ports.
- `serve --preset-db <PATH>` opts into SQLite-backed preset persistence.
- Optional config file loading exists via `serve --config <PATH>` and auto-discovered `./serialport-api.toml`.
- The API exposes health, ports, connections, commands, SSE events, legacy aliases, and preset CRUD routes.
- Docker, Docker Compose example, Raspberry Pi/systemd docs, and tag-triggered release workflow exist.
- Event storage is currently a snapshot vector behind `ConnectionManager::events()`, not a live broadcast channel.
- Existing SSE event names are:
  - `serial.json`
  - `serial.text`
  - `serial.log`
  - `serial.notification`
  - `serial.error`

Important local toolchain note:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Use that before all `cargo` commands in this WSL environment to avoid Rust toolchain mismatch.

---

## Phase 16 Scope

Do in Phase 16:

- Add a native WebSocket endpoint:

```text
GET /api/v1/events/ws
```

- Use Axum WebSocket support if possible. With the current dependency set, this likely means changing `Cargo.toml` from `axum = "0.7"` to `axum = { version = "0.7", features = ["ws"] }`.
- Serialize each existing `SerialStreamEvent` snapshot into one WebSocket text message.
- Keep message schema stable and simple:

```json
{"event":"serial.json","data":{"reqId":"1","ok":true}}
```

- Preserve the current SSE endpoint exactly:

```text
GET /api/v1/events
```

- Add route tests proving the WebSocket endpoint returns seeded existing events using the same event names and JSON data as SSE.
- Keep all automated tests hardware-free by seeding `InMemoryConnectionManager` events directly or through the existing mock read-loop helpers.
- Update `README.md` only enough to document the WebSocket endpoint, examples, and roadmap/status.
- Optionally update `docs/open-source-spec.md` if needed to keep API docs accurate. Keep this concise.
- Add any minimal test-only dependency needed for WebSocket route tests. Prefer reliable Rust test clients such as `tokio-tungstenite` if Axum/tower cannot easily exercise WebSocket upgrades in-process.

Out of scope / do **not** do in Phase 16:

- Do not implement Socket.IO/Engine.IO protocol compatibility unless a separate deliberate scope is created. A native WebSocket endpoint is **not** Socket.IO-compatible.
- Do not remove, rename, or alter `GET /api/v1/events` SSE response formatting, content type, or tests.
- Do not change route shapes for health, ports, connections, commands, presets, or legacy aliases.
- Do not add authentication, TLS, CORS, reverse-proxy config, firewall automation, or browser UI.
- Do not change Docker, systemd, release workflows, config precedence, or SQLite persistence unless a tiny docs note is required.
- Do not require physical serial hardware in tests or CI.
- Do not make real serial mode the default.
- Do not add streaming command input over WebSocket in this phase; this endpoint is events-only.
- Do not promise Socket.IO client compatibility in docs.

---

## Expected Files to Inspect Before Editing

Read these first:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"

# Repository docs and roadmap
sed -n '1,540p' README.md
sed -n '1,220p' docs/phase-15-release-docker-handoff.md
sed -n '1,380p' docs/open-source-spec.md
sed -n '440,570p' docs/implementation-plan.md

# Current routes, event storage, server startup, and dependencies
sed -n '1,260p' src/api/routes.rs
sed -n '1000,1160p' src/api/routes.rs
sed -n '1,340p' src/serial/manager.rs
sed -n '1,260p' src/serial/read_loop.rs
sed -n '1,140p' src/main.rs
sed -n '1,120p' Cargo.toml
```

Use `read_file`/`search_files` equivalents if operating through tools that prohibit shell readers.

---

## Expected Files to Modify or Create

Required:

- Modify: `Cargo.toml`
  - Enable Axum WebSocket support, likely `features = ["ws"]`.
  - Add a minimal dev-dependency only if needed for WebSocket tests.

- Modify: `Cargo.lock`
  - Expected if dependency features or test dependencies change.

- Modify: `src/api/routes.rs`
  - Add route wiring for `GET /api/v1/events/ws`.
  - Add WebSocket handler and helper function(s) to serialize/send event snapshots.
  - Add hardware-free route tests.

- Modify: `README.md`
  - Add a concise WebSocket events section or subsection near the existing SSE events documentation.
  - Mark native WebSocket support as implemented after the code lands.
  - Keep Socket.IO listed as not implemented/future compatibility unless actually implemented.

Optional only if justified:

- Modify: `docs/open-source-spec.md`
  - Refresh statements that say WebSocket can be added later once this phase implements native WebSocket.
  - Make clear Socket.IO protocol compatibility remains separate.

Files not expected to change:

- `src/serial/**` unless a tiny trait/helper change is required for testability.
- `src/main.rs` unless router construction must change; it should normally not.
- `src/config.rs`.
- `src/storage/**`.
- Dockerfile, `.dockerignore`, `.github/workflows/release.yml`, examples, systemd docs.

If any unexpected file changes become necessary, document the reason explicitly in the final output and keep the change minimal.

---

## Required API Contract

### Endpoint

```text
GET /api/v1/events/ws
```

### Protocol

- Native WebSocket upgrade endpoint.
- Server sends text frames containing JSON objects.
- Initial Phase 16 behavior should be snapshot-based: on connection, send one text frame per event currently returned by `connection_manager.events()`, then close normally or leave the connection open only if doing so is deliberate and tested.
- If the implementation chooses to keep the socket open after sending snapshots, it must not busy-loop and tests must remain deterministic.
- Live broadcast of future events is optional and should be deferred unless it can be implemented cleanly with a tested event channel without changing existing manager semantics.

### Message schema

Each text message must be exactly this logical JSON shape:

```json
{
  "event": "serial.json",
  "data": {
    "reqId": "1",
    "ok": true
  }
}
```

Field details:

- `event`: string; one of the existing serial event names.
- `data`: any JSON value currently stored in `SerialStreamEvent::data`.

Examples:

```json
{"event":"serial.json","data":{"reqId":"1","ok":true}}
```

```json
{"event":"serial.text","data":"hello robot"}
```

```json
{"event":"serial.error","data":"read failed"}
```

### Error behavior

- If event snapshot retrieval fails, the handler should fail the upgrade with HTTP `500` when possible, consistent with the existing SSE route's `StatusCode::INTERNAL_SERVER_ERROR` mapping.
- If a client disconnects while frames are being sent, stop sending and do not panic.
- Serialization should not fail for normal `serde_json::Value`; if it does, close the socket gracefully or return a server error before upgrade where possible.

### Socket.IO compatibility statement

Native WebSocket support does **not** imply Socket.IO compatibility. Socket.IO clients expect an Engine.IO handshake/framing protocol and should not be documented as compatible with `/api/v1/events/ws`.

---

## Suggested Implementation Shape

One acceptable route shape in `src/api/routes.rs`:

```rust
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
```

Add route near the SSE endpoint:

```rust
.route("/api/v1/events/ws", get(events_ws::<L, C>))
```

Handler sketch:

```rust
async fn events_ws<L, C>(
    State(state): State<AppState<L, C>>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, StatusCode>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    let events = state
        .connection_manager
        .events()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(ws.on_upgrade(move |socket| send_event_snapshot(socket, events)))
}

async fn send_event_snapshot(mut socket: WebSocket, events: Vec<SerialStreamEvent>) {
    for serial_event in events {
        let payload = serde_json::json!({
            "event": serial_event.event,
            "data": serial_event.data,
        });

        match serde_json::to_string(&payload) {
            Ok(text) => {
                if socket.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let _ = socket.close().await;
}
```

The implementation agent may choose a different equivalent structure if it satisfies tests, clippy, and the API contract.

---

## Acceptance Criteria

Phase 16 is complete when all of these are true:

1. `GET /api/v1/events/ws` exists and performs a native WebSocket upgrade.
2. A client connecting to `/api/v1/events/ws` receives seeded existing serial events as JSON text frames in snapshot order.
3. The WebSocket message shape is `{ "event": <string>, "data": <json> }`.
4. Existing SSE endpoint `GET /api/v1/events` still passes all current tests and remains documented.
5. Existing command, connection, ports, health, presets, legacy alias, config, Docker, and release tests still pass.
6. Tests are hardware-free and deterministic.
7. README accurately documents the new endpoint and clearly states Socket.IO compatibility is not implemented unless a later phase adds it.
8. `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` pass with `PATH="$HOME/.cargo/bin:$PATH"`.
9. The implementation is committed with a conventional commit message.

---

## Bite-Sized TDD Tasks

### Task 16.1: Establish baseline and RED for missing WebSocket route

RED/check first:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
git log --oneline -6
grep -R "events/ws\|WebSocket\|Socket.IO" -n README.md Cargo.toml src/api/routes.rs docs || true
cargo test --all-features
```

Expected finding before implementation:

- No `/api/v1/events/ws` route exists.
- `Cargo.toml` does not enable Axum `ws` support.
- README lists WebSocket or Socket.IO as planned/future.
- Existing tests pass before changes.

GREEN:

- Decide exact dependency/test-client strategy.
- Keep the initial failing test focused on WebSocket receiving seeded events.

### Task 16.2: Add a failing hardware-free WebSocket route test

Add a test in `src/api/routes.rs` that:

- Seeds an `InMemoryConnectionManager` with:
  - `SerialEvent::Json(json!({"reqId":"1","ok":true}))`
  - `SerialEvent::Text("hello robot".to_string())`
- Starts the router in a way that supports WebSocket upgrade testing.
- Connects a WebSocket client to `/api/v1/events/ws`.
- Reads two text frames.
- Asserts parsed JSON values equal:

```json
{"event":"serial.json","data":{"reqId":"1","ok":true}}
```

```json
{"event":"serial.text","data":"hello robot"}
```

Expected RED reason:

- Route and/or Axum WebSocket support do not exist yet.

Testing notes:

- If `tower::ServiceExt::oneshot` cannot exercise an upgrade cleanly, spawn the Axum router on a local ephemeral `TcpListener` inside the test and connect with `tokio_tungstenite::connect_async`.
- Use port `0` / `listener.local_addr()` rather than a fixed port.
- Ensure the spawned server task is aborted or ends cleanly after the test.

### Task 16.3: Enable WebSocket dependencies/features

Update dependencies minimally:

```toml
axum = { version = "0.7", features = ["ws"] }
```

Add a dev-dependency only if needed for integration-style WebSocket route tests, for example:

```toml
tokio-tungstenite = "0.24"
futures-util = "0.3"
```

The implementation agent should choose versions compatible with the resolved dependency graph. Let Cargo update `Cargo.lock` normally.

Validate partial build:

```bash
cargo check --all-targets --all-features
```

### Task 16.4: Implement `/api/v1/events/ws`

GREEN:

- Add route wiring in `router_with_state`.
- Add handler and send helper.
- Reuse `connection_manager.events()` rather than duplicating event parsing logic.
- Serialize events as JSON text frames.
- Handle client disconnect without panic.

Validate:

```bash
cargo test --all-features events_ws
```

If the exact test name differs, run the targeted route-test module or full suite.

### Task 16.5: Prove SSE is unchanged

Run existing SSE-focused tests:

```bash
cargo test --all-features events_route_streams
```

Expected:

- Existing SSE tests still pass without changing their expected content.

### Task 16.6: README/API docs update

Update `README.md`:

- Add implemented feature bullet for native WebSocket serial event endpoint.
- Keep Socket.IO compatibility separate and not implemented.
- Add a subsection near `GET /api/v1/events`:

```text
GET /api/v1/events/ws
```

- Document message examples and a manual smoke command.
- Keep roadmap accurate: native WebSocket implemented; Socket.IO compatibility and/or ARM Pi binaries can remain future work if still true.

Optional `docs/open-source-spec.md` update:

- If it still says WebSocket can be added later, update it to say native WebSocket is available and Socket.IO protocol compatibility remains future/deferred.

Docs validation:

```bash
grep -R "/api/v1/events/ws\|Socket.IO" -n README.md docs/open-source-spec.md docs/phase-16-websocket-events-handoff.md
```

### Task 16.7: Full verification and commit

Run:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
git status --short
git diff --stat
```

If all pass, commit:

```bash
git add Cargo.toml Cargo.lock README.md src/api/routes.rs docs/open-source-spec.md
git commit -m "feat: add WebSocket event stream"
```

Only include `docs/open-source-spec.md` in `git add` if it changed.

Do not push.

---

## Verification Commands

Required final verification:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
git status --short --branch
```

Recommended targeted checks while developing:

```bash
cargo check --all-targets --all-features
cargo test --all-features events_route_streams
cargo test --all-features events_ws
grep -R "/api/v1/events/ws\|Socket.IO" -n README.md docs/open-source-spec.md src/api/routes.rs Cargo.toml
```

If Docker packaging was not touched, Docker build/run verification is not required for this phase. It is acceptable to skip Docker to keep Phase 16 focused, but the Rust suite must pass.

---

## Manual Smoke Checks

After automated tests pass, manual smoke is optional but recommended if a WebSocket CLI is available.

Start the server:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -- serve --host 127.0.0.1 --port 4002
```

In another terminal, verify existing HTTP endpoints still work:

```bash
curl -s http://127.0.0.1:4002/api/v1/health
curl -i -s http://127.0.0.1:4002/api/v1/events
```

Connect to the WebSocket endpoint with a tool such as `websocat` or `wscat`:

```bash
websocat ws://127.0.0.1:4002/api/v1/events/ws
```

or:

```bash
npx wscat -c ws://127.0.0.1:4002/api/v1/events/ws
```

Expected notes:

- A fresh server may have no recorded events, so the WebSocket may close or stay quiet depending on the implemented snapshot behavior.
- Route tests must seed events and prove non-empty message behavior.
- Do not require real serial hardware for manual smoke.

Optional mock-device smoke if an implementation wants observable events without hardware:

1. Start with `--mock-device` or `--mock-script` following current README docs.
2. Create a mock connection.
3. Send a command that produces a mock response.
4. Connect to `/api/v1/events/ws` and verify JSON text frames contain `serial.json` events.

---

## Risks and Mitigations

- **Axum WebSocket feature mismatch:** `axum = "0.7"` currently lacks `ws` feature. Mitigate by enabling only the `ws` feature and letting Cargo resolve compatible transitive dependencies.
- **Testing WebSocket upgrades in-process:** Tower `oneshot` may not be enough. Mitigate by spawning the router on an ephemeral local TCP listener in the test and using a real WebSocket client dev-dependency.
- **Confusing native WebSocket with Socket.IO:** Mitigate through README wording and tests/docs that call this a native WebSocket endpoint only.
- **Event snapshot vs live stream expectations:** Current manager only exposes snapshots. Mitigate by documenting snapshot semantics. Defer live broadcast unless implemented cleanly.
- **Breaking SSE formatting:** Mitigate by preserving existing SSE handler and running existing SSE tests unchanged.
- **Open socket test hangs:** Mitigate by closing after snapshot messages or making tests read exact expected frames with timeouts.
- **Over-broad dependency additions:** Mitigate by adding only Axum `ws` and minimal dev dependencies.

---

## Commit Guidance

Implementation commit message should be conventional, for example:

```bash
git commit -m "feat: add WebSocket event stream"
```

Commit only the files needed for Phase 16. Do not push.

Before committing, inspect:

```bash
git diff --stat
git diff -- Cargo.toml src/api/routes.rs README.md docs/open-source-spec.md
```

Expected changed files are limited to:

- `Cargo.toml`
- `Cargo.lock`
- `src/api/routes.rs`
- `README.md`
- optionally `docs/open-source-spec.md`

---

## Implementation Agent Short Instruction

Add native WebSocket event streaming at `GET /api/v1/events/ws` using existing serial event snapshots, keep SSE unchanged, write hardware-free WebSocket route tests, update README accurately, run full Rust verification, commit, and do not push.
