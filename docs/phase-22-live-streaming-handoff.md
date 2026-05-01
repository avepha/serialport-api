# Phase 22 Live Streaming Polish Handoff

> **For Hermes / next AI implementation session:** Execute this in a fresh implementation session. Load `writing-plans`, `test-driven-development`, and `rust-axum-api-tdd` before editing. This phase upgrades the existing serial event endpoints from finite snapshots to true long-lived broadcast streams while preserving API paths and event schemas. Keep the change conservative, hardware-free in tests, and independent from authentication, new serial protocol semantics, or dashboard packaging.

**Goal:** Add explicit manager support and tests for long-lived live event fan-out so connected SSE, native WebSocket, and Socket.IO-compatible clients receive serial events recorded after they connect, not only a snapshot of events that existed before connection.

**Background:** Phase 21 added the React dashboard and release bundle. Its Events panel currently uses `EventSource('/api/v1/events')`, but `src/api/routes.rs` implements `/api/v1/events`, `/api/v1/events/ws`, and `/socket.io/?EIO=4&transport=websocket` as finite snapshots by calling `connection_manager.events()` once and then ending/closing. Phase 22 should make these endpoints genuinely live without changing route names or serial semantics.

---

## Strict Orchestration Input Schema

The implementation agent should accept this document plus the repository as its complete input. No hidden context or user choices are required.

```json
{
  "agent_role": "implementation",
  "phase": "Phase 22",
  "repository": "/home/alfarie/repos/serialport-api",
  "branch": "rewrite/axum-serial-api",
  "base_commit_expected": "699007f feat: add React dashboard release bundle",
  "required_artifact_to_read": "docs/phase-22-live-streaming-handoff.md",
  "toolchain_env": {
    "PATH_prefix": "$HOME/.cargo/bin",
    "node_version": "20",
    "package_manager": "pnpm"
  },
  "scope": "Add true long-lived broadcast updates for existing serial event endpoints while preserving public API shape and hardware-free tests",
  "non_goals": [
    "Authentication, accounts, sessions, authorization, CORS policy design, TLS, or secrets",
    "New serial command semantics, new serial protocol framing, new default baud/delimiter behavior, or hardware-only behavior",
    "Hardware-dependent automated tests or tests requiring physical serial devices",
    "Changing existing route paths for SSE, native WebSocket, Socket.IO compatibility, commands, connections, presets, dashboard, or legacy aliases",
    "Changing existing serial event names or payload JSON shapes",
    "Durable event persistence, replay from disk, database-backed streams, clustering, or multi-process fan-out",
    "Full Socket.IO feature parity, long-polling transport, rooms, acknowledgements, binary packets, namespaces beyond default, or client-to-server Socket.IO commands",
    "Dashboard redesign, new frontend dependencies, browser E2E test framework, CI/release packaging changes unrelated to this stream behavior",
    "Pushing commits or tags"
  ]
}
```

---

## Strict Orchestration Output Schema

The implementation agent's final response must use this JSON shape:

```json
{
  "agent_role": "implementation",
  "phase": "Phase 22",
  "summary": [
    "Added long-lived broadcast support for serial events in the connection manager.",
    "Updated SSE, native WebSocket, Socket.IO-compatible endpoint tests to prove post-subscription events are delivered."
  ],
  "files_changed": [
    "Cargo.toml",
    "Cargo.lock",
    "README.md",
    "src/api/routes.rs",
    "src/serial/manager.rs",
    "web/src/App.tsx",
    "web/src/api.ts"
  ],
  "verification": {
    "commands_run": [
      "cargo fmt --check",
      "cargo check",
      "cargo clippy --all-targets --all-features -- -D warnings",
      "cargo test --all-features",
      "cd web && pnpm typecheck",
      "cd web && pnpm build",
      "git diff --check"
    ],
    "status": "passed|blocked"
  },
  "commit": "<sha or null>",
  "approval_status": "ready_for_review|blocked|deferred",
  "issues": []
}
```

If blocked, set `commit` to `null`, `approval_status` to `blocked`, and list exact blockers. If implementation is intentionally deferred after investigation, update only docs that record the deferral, run applicable lightweight checks, and commit with a `docs:` message.

---

## Required Preconditions

Before editing, run:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
git log --oneline -8
```

Expected:

- Branch is `rewrite/axum-serial-api`.
- Working tree is clean.
- Recent history includes `699007f feat: add React dashboard release bundle` or a descendant of it.

If the working tree is not clean before Phase 22 edits, stop and report instead of modifying files.

---

## Current Repository Context Observed During Planning

Repository path:

```text
/home/alfarie/repos/serialport-api
```

Expected branch:

```text
rewrite/axum-serial-api
```

Latest completed phase commit at planning time:

```text
699007f feat: add React dashboard release bundle
```

Important current behavior:

- `src/api/routes.rs` exposes:
  - `GET /api/v1/events` as SSE.
  - `GET /api/v1/events/ws` as native WebSocket.
  - `GET /socket.io/?EIO=4&transport=websocket` as a small Engine.IO/Socket.IO compatibility shim.
- All three event endpoints currently call `ConnectionManager::events()` and stream/send only the current snapshot.
- SSE uses `tokio_stream::iter(...)`, so the HTTP response finishes after existing events are emitted.
- Native WebSocket uses `send_event_snapshot(...)`, sends existing events as JSON text frames shaped as `{ "event": <string>, "data": <json> }`, and closes.
- Socket.IO compatibility uses `send_socket_io_event_snapshot(...)`, emits Engine.IO open packet, Socket.IO connect packet, existing events as `42[...]` frames, then closes.
- `ConnectionManager` currently has methods:
  - `connect`
  - `connections`
  - `disconnect`
  - `send_command`
  - `take_response`
  - `events`
- `ConnectionManagerWithTransport<T>` stores events in `Arc<Mutex<Vec<SerialStreamEvent>>>` and records events via `record_event_for_connection` / `record_error`.
- `SerialStreamEvent` has stable fields:
  - `event: &'static str`
  - `data: serde_json::Value`
- Event names currently used by the manager are:
  - `serial.json`
  - `serial.text`
  - `serial.log`
  - `serial.notification`
  - `serial.error`
- `src/serial/read_loop.rs` records parsed serial events through `SerialEventRecorder`, which is already implemented for `ConnectionManagerWithTransport<T>`.
- `src/serial/real_transport.rs` wraps `ConnectionManagerWithTransport<RealSerialTransport<F>>` in `RealSerialConnectionManager<F>` and delegates `events()` to the inner manager.
- `web/src/App.tsx` currently opens `new EventSource('/api/v1/events')`, appends known serial event names, and treats `onerror` as `snapshot ended or reconnecting` when not closed.
- `web/src/api.ts` contains typed HTTP helpers only; there is no dedicated event stream helper yet.
- Existing tests include hardware-free route tests for snapshot SSE, native WebSocket snapshots, Socket.IO snapshots, real serial behavior using fake handles, and read-loop behavior using fake/mock sources.

---

## Phase 22 Scope

Do in Phase 22:

- Add live broadcast support inside the serial manager abstraction.
- Preserve all public route paths and existing event payload shapes.
- Make `GET /api/v1/events` long-lived and capable of delivering events recorded after the HTTP request starts.
- Make `GET /api/v1/events/ws` long-lived and capable of delivering events recorded after the WebSocket connects.
- Make `GET /socket.io/?EIO=4&transport=websocket` long-lived for `EIO=4&transport=websocket` clients and capable of delivering events recorded after the Socket.IO-compatible connection is established.
- Keep initial snapshot/replay behavior where possible: a new subscriber may receive already-recorded events first, then live events.
- Add deterministic hardware-free tests proving post-subscription delivery for SSE, native WebSocket, and Socket.IO compatibility.
- Keep the React dashboard API shape stable; at most update copy/status handling so the Events panel no longer describes the stream as snapshot-only.
- Update README/docs so users understand event endpoints are live streams with snapshot replay.

The preferred behavior is **snapshot plus live tail**:

1. A client connects.
2. The server sends events already present in the manager's in-memory event store.
3. The server remains connected.
4. Events recorded later through `record_event_for_connection` or `record_error` are broadcast to all current subscribers.
5. A client disconnecting must not break other subscribers.

---

## Non-goals / Explicitly Out of Scope

Do **not** do these in Phase 22:

- Do not add auth, accounts, tokens, sessions, roles, HTTPS/TLS, or CORS policy design.
- Do not add new serial command semantics or change how payloads are framed, written, parsed, or matched to `reqId`.
- Do not require physical serial devices in automated tests.
- Do not change default server mode or make real serial the default.
- Do not introduce durable event persistence, SQLite event tables, event log compaction, or replay cursors.
- Do not implement Last-Event-ID, resumable streams, per-client offsets, or persistent delivery guarantees.
- Do not implement full Socket.IO: no long-polling transport, rooms, namespaces beyond default, acknowledgements, binary, compression tuning, middleware, or command submission over Socket.IO.
- Do not replace existing endpoints with new paths. Additive helpers are allowed only if internal or clearly documented as optional.
- Do not alter command, connection, preset, dashboard static-serving, Docker, release, systemd, or config behavior except docs/copy directly related to live streams.
- Do not add a browser E2E framework or new frontend state-management library.

---

## Stable API and Wire Schemas

### SSE endpoint

Path:

```text
GET /api/v1/events
```

Request:

```http
GET /api/v1/events HTTP/1.1
Accept: text/event-stream
```

Response headers must include:

```http
HTTP/1.1 200 OK
content-type: text/event-stream
```

Each serial event must keep the current SSE shape:

```text
event: <serial event name>
data: <JSON-encoded event data>

```

Examples:

```text
event: serial.json
data: {"ok":true,"reqId":"1"}

```

```text
event: serial.text
data: "hello robot"

```

Required event names:

```json
[
  "serial.json",
  "serial.text",
  "serial.log",
  "serial.notification",
  "serial.error"
]
```

SSE stream lifecycle:

- Must not end immediately after emitting a snapshot.
- Must stay open while the client is connected.
- Must deliver later events without requiring polling or reconnect.
- It is acceptable to emit periodic keep-alive comments if this is useful, but do not require clients to interpret them.

### Native WebSocket endpoint

Path:

```text
GET /api/v1/events/ws
```

Handshake:

```http
GET /api/v1/events/ws HTTP/1.1
Upgrade: websocket
Connection: Upgrade
```

Server-to-client text frame schema must remain:

```json
{
  "event": "serial.json|serial.text|serial.log|serial.notification|serial.error",
  "data": {}
}
```

Examples:

```json
{"event":"serial.json","data":{"reqId":"1","ok":true}}
```

```json
{"event":"serial.text","data":"hello robot"}
```

Native WebSocket stream lifecycle:

- Must send existing snapshot events if present.
- Must stay open after snapshot events.
- Must send later broadcast events as they are recorded.
- May ignore client text/binary messages or close on unsupported client input; document whichever behavior is implemented.
- Must stop work cleanly when the client disconnects.

### Socket.IO-compatible endpoint

Path:

```text
GET /socket.io/?EIO=4&transport=websocket
```

Supported query:

```json
{
  "EIO": "4",
  "transport": "websocket"
}
```

Unsupported query behavior must remain deterministic:

- Requests with `EIO` other than `4` should return `400 Bad Request` before WebSocket upgrade.
- Requests with `transport` other than `websocket` should return `400 Bad Request` before WebSocket upgrade.

Initial server frames must keep the Phase 17 compatibility shape:

1. Engine.IO open packet:

```text
0{"sid":"serialport-api-<number>","upgrades":[],"pingInterval":25000,"pingTimeout":20000,"maxPayload":1000000}
```

2. Socket.IO connect packet:

```text
40
```

Serial event packet schema must remain:

```text
42["<serial event name>",<event data JSON>]
```

Examples:

```text
42["serial.json",{"reqId":"1","ok":true}]
```

```text
42["serial.text","hello robot"]
```

Socket.IO-compatible stream lifecycle:

- Must send open and connect frames first.
- Must send existing snapshot events if present.
- Must stay open after snapshot events.
- Must send later broadcast events using the same `42[...]` schema.
- May keep current minimal compatibility and does not need to implement client-side ping/pong beyond what existing tests require unless needed to keep clients stable. If Engine.IO ping/pong is added, keep it minimal and hardware-free tested.

### Manager-level live event schema

`SerialStreamEvent` should remain the public in-process serial event object:

```rust
pub struct SerialStreamEvent {
    pub event: &'static str,
    pub data: serde_json::Value,
}
```

The manager should expose a subscription mechanism that yields `SerialStreamEvent` values and supports multiple concurrent subscribers. Preferred additive trait method:

```rust
fn subscribe_events(&self) -> Result<tokio::sync::broadcast::Receiver<SerialStreamEvent>>;
```

If using `tokio::sync::broadcast`, then `SerialStreamEvent` must remain `Clone` (already true) and broadcast lag handling must be explicit. Acceptable lag policy for this phase:

- On `RecvError::Lagged(_)`, record/emit a `serial.error` stream item or skip lagged items and continue.
- Do not panic on lag.
- Do not block serial recording indefinitely because a client is slow.

---

## Recommended Implementation Design

Prefer a small additive broadcast layer in `ConnectionManagerWithTransport<T>`:

```rust
use tokio::sync::broadcast;

pub struct ConnectionManagerWithTransport<T> {
    connections: Arc<Mutex<BTreeMap<String, ConnectionInfo>>>,
    next_req_id: Arc<Mutex<u64>>,
    events: Arc<Mutex<Vec<SerialStreamEvent>>>,
    event_tx: broadcast::Sender<SerialStreamEvent>,
    responses_by_connection_and_req_id: Arc<Mutex<ResponseQueue>>,
    transport: T,
    mock_responder: Option<MockDeviceResponder>,
}
```

Recommended channel capacity:

```rust
const EVENT_BROADCAST_CAPACITY: usize = 1024;
```

Recording behavior:

1. Convert `crate::protocol::SerialEvent` to `SerialStreamEvent` exactly as today.
2. Push the event into the existing in-memory `events` vector so snapshot behavior remains intact.
3. Send the event to `event_tx`.
4. Ignore `send` errors caused by there being no active receivers; this is not an application error.
5. Keep response indexing by `reqId` unchanged.

Subscription behavior:

- `ConnectionManager::events()` remains unchanged and returns the snapshot vector.
- New `ConnectionManager::subscribe_events()` returns a live receiver for future events.
- Route handlers can combine `events()` snapshot with `subscribe_events()` tail.
- `RealSerialConnectionManager<F>` delegates `subscribe_events()` to its inner manager.

Suggested helper functions in `src/api/routes.rs`:

- `serial_event_to_sse_event(serial_event: SerialStreamEvent) -> Result<Event, axum::Error>`
- `serial_event_to_ws_text(serial_event: SerialStreamEvent) -> Option<String>`
- `serial_event_to_socket_io_frame(serial_event: SerialStreamEvent) -> Option<String>`
- `live_event_stream(snapshot: Vec<SerialStreamEvent>, receiver: broadcast::Receiver<SerialStreamEvent>) -> impl Stream<Item = SerialStreamEvent>`

The exact helper names may vary, but keep logic small and testable.

Important subscription ordering guidance:

- To avoid missing events between snapshot and subscription, subscribe first, then read snapshot:

```rust
let receiver = state.connection_manager.subscribe_events()?;
let snapshot = state.connection_manager.events()?;
```

- This can duplicate an event if one is recorded at the exact boundary and appears in both snapshot and receiver. For Phase 22 this is acceptable unless easily avoided without adding IDs/cursors.
- Do not add global event IDs or durable offsets solely to solve boundary duplicates.

---

## Exact Files to Inspect Before Editing

Read these before changes:

```text
Cargo.toml
README.md
docs/phase-21-web-dashboard-handoff.md
src/api/routes.rs
src/serial/manager.rs
src/serial/read_loop.rs
src/serial/real_transport.rs
src/serial/transport.rs
src/protocol.rs
src/main.rs
web/src/App.tsx
web/src/api.ts
```

---

## Expected Files to Modify

Likely required:

```text
src/serial/manager.rs
src/serial/real_transport.rs
src/api/routes.rs
README.md
web/src/App.tsx
```

Possibly required:

```text
Cargo.toml
Cargo.lock
web/src/api.ts
docs/open-source-spec.md
docs/implementation-plan.md
```

Avoid modifying unless genuinely needed:

```text
.github/workflows/ci.yml
.github/workflows/release.yml
Dockerfile
src/storage/*
src/config.rs
```

---

## TDD Task List

Implement in this order.

### Task 1: Manager broadcast tests first

Objective: prove the manager can deliver events recorded after subscription.

Files:

- Modify tests in `src/serial/manager.rs`.
- Then implement in `src/serial/manager.rs`.
- Modify `src/serial/real_transport.rs` only after the trait changes require delegation.

Add tests similar to:

- `manager_broadcasts_events_recorded_after_subscription`
- `manager_allows_multiple_live_event_subscribers`
- `manager_snapshot_events_still_return_recorded_history`
- `real_serial_connection_manager_delegates_event_subscription` if practical without physical serial hardware.

Behavior to assert:

- A subscriber created before `record_event_for_connection(...)` receives that event.
- Two subscribers both receive the same later event.
- `events()` still returns the stored snapshot in the same shape as before.
- No receiver is required for `record_event_for_connection(...)` to succeed.

Implementation notes:

- Add `subscribe_events` to `ConnectionManager` as an additive trait method.
- Because trait implementations are local, update all implementations in this repo.
- Use `tokio::sync::broadcast`; no new dependency should be needed because `tokio = { features = ["full"] }` already exists.

Verification:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test serial::manager::tests::manager_broadcasts_events_recorded_after_subscription -- --nocapture
cargo test serial::manager -- --nocapture
```

### Task 2: SSE live stream route tests first

Objective: prove `/api/v1/events` remains open and emits an event recorded after request start.

Files:

- Modify tests in `src/api/routes.rs`.
- Then modify SSE route implementation in `src/api/routes.rs`.

Recommended test shape:

- Build an app with an `InMemoryConnectionManager` clone.
- Start an SSE request with `app.oneshot(...)` in a spawned task or consume the response body stream directly.
- Ensure the response status is `200 OK` and content type is `text/event-stream`.
- Record a new event on the manager after the response starts.
- Read from the body stream with a short `tokio::time::timeout`.
- Assert the body chunk contains `event: serial.json` and matching `data:`.

Keep the existing snapshot SSE test or adapt it so it still proves snapshot replay.

Implementation notes:

- Use `tokio_stream::wrappers::BroadcastStream` only if enabling `tokio-stream`'s `sync` feature is acceptable. Current `Cargo.toml` has `tokio-stream = "0.1"` without feature flags.
- To avoid dependency feature churn, a small `async_stream` dependency may be considered, but prefer avoiding new dependencies if straightforward.
- A manual stream using `futures_util::stream` or `tokio_stream` utilities is acceptable if it stays readable.
- If adding a dependency, justify it in the final summary and verify `Cargo.lock`.

Verification:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test events_route_replays_snapshot_and_streams_live_serial_events -- --nocapture
```

### Task 3: Native WebSocket live route tests first

Objective: prove `/api/v1/events/ws` sends post-connection events and no longer closes immediately after the snapshot.

Files:

- Modify tests in `src/api/routes.rs`.
- Then modify WebSocket sender implementation in `src/api/routes.rs`.

Recommended test shape:

- Start a test server on `127.0.0.1:0` as current WebSocket tests do.
- Connect with `tokio_tungstenite`.
- Record a manager event after connection establishment.
- Await the next text frame with a timeout.
- Assert JSON equals:

```json
{"event":"serial.json","data":{"reqId":"live-1","ok":true}}
```

- Assert the socket is not closed immediately before the live event arrives.

Implementation notes:

- Change `send_event_snapshot` into a live loop that:
  - sends snapshot events first,
  - awaits broadcast receiver events,
  - sends each as a text frame,
  - exits on send error/client disconnect/receiver closed.
- Consider splitting client receive handling with `tokio::select!` if needed to notice disconnects. A simpler send-loop that exits when `socket.send(...)` fails is acceptable for MVP.

Verification:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test events_ws_streams_live_serial_events_after_connection -- --nocapture
```

### Task 4: Socket.IO live route tests first

Objective: prove `/socket.io/?EIO=4&transport=websocket` sends post-connection events in the existing `42[...]` schema.

Files:

- Modify tests in `src/api/routes.rs`.
- Then modify Socket.IO sender implementation in `src/api/routes.rs`.

Recommended test shape:

- Start a test server as current Socket.IO tests do.
- Connect to `ws://{addr}/socket.io/?EIO=4&transport=websocket`.
- Assert first frame starts with `0` and contains open payload.
- Assert second frame is `40`.
- Record a manager event after the connection is established.
- Await a `42[...]` frame with timeout.
- Assert payload equals:

```json
["serial.json", {"reqId":"socket-live-1", "ok":true}]
```

- Keep unsupported query param tests unchanged.

Implementation notes:

- Rename `send_socket_io_event_snapshot` to a live-aware name or keep the old name only if behavior is clear from comments/tests.
- Do not add Socket.IO command submission.
- Do not add long-polling.

Verification:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test socket_io_streams_live_serial_events_after_connection -- --nocapture
```

### Task 5: Dashboard/frontend copy polish

Objective: keep the dashboard stable and make labels accurate for live streams.

Files:

- Modify `web/src/App.tsx` if needed.
- Modify `web/src/api.ts` only if extracting a small typed event helper is valuable.

Required behavior:

- Continue using `EventSource('/api/v1/events')`.
- Do not change the public API base path.
- Update copy such as `SSE snapshot and stream from /api/v1/events.` to accurately describe live streaming, e.g. `Live SSE stream from /api/v1/events with current snapshot replay.`
- Keep graceful handling for reconnect/error status.
- Do not add frontend dependencies.

Verification:

```bash
cd web
pnpm typecheck
pnpm build
```

### Task 6: Documentation updates

Objective: document live event semantics and limitations.

Files:

- Modify `README.md`.
- Modify `docs/open-source-spec.md` or `docs/implementation-plan.md` only if they still state event streams are snapshot-only or planned.

Documentation must state:

- `/api/v1/events` is a live SSE stream that replays currently recorded in-memory events and then tails new events.
- `/api/v1/events/ws` is a native WebSocket live stream using `{ "event": ..., "data": ... }` frames.
- `/socket.io/?EIO=4&transport=websocket` is a minimal live Socket.IO-compatible stream using Engine.IO v4 WebSocket transport only.
- Event history is in-memory only and not durable across process restarts.
- Tests are hardware-free; real serial remains opt-in.

---

## Acceptance Criteria

Phase 22 is complete only when all of these are true:

1. `ConnectionManager` exposes a live event subscription mechanism while preserving `events()` snapshot behavior.
2. `ConnectionManagerWithTransport<T>` broadcasts every event recorded through `record_event_for_connection` and every error recorded through `record_error` to all active subscribers.
3. Broadcast send failure due to no subscribers is ignored safely and does not make serial event recording fail.
4. `RealSerialConnectionManager<F>` supports the same live subscription behavior by delegation.
5. `GET /api/v1/events` returns `200 OK`, `content-type: text/event-stream`, replays existing events, stays open, and emits events recorded after the request starts.
6. `GET /api/v1/events/ws` keeps the existing JSON text frame schema, replays existing events, stays open, and emits events recorded after WebSocket connection.
7. `GET /socket.io/?EIO=4&transport=websocket` keeps the existing Engine.IO open frame, Socket.IO connect frame, and `42[...]` serial event frame schema; it stays open and emits events recorded after connection.
8. Unsupported Socket.IO query parameters still return deterministic `400 Bad Request` behavior as current tests require.
9. Existing command, connection, preset, dashboard static asset, health, legacy alias, and serial read-loop tests still pass.
10. No automated test requires physical serial hardware.
11. Frontend behavior still typechecks/builds and continues to consume `/api/v1/events` with `EventSource`.
12. README/docs describe live streaming accurately and note in-memory/non-durable limitations.
13. Full verification commands pass.
14. Final diff contains no unrelated auth, serial semantics, release, Docker, or CI changes.

---

## Full Verification Commands

Run from repository root unless noted:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"

cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features

cd web
pnpm typecheck
pnpm build
cd ..

git diff --check
```

Focused commands useful during development:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test serial::manager -- --nocapture
cargo test events_route -- --nocapture
cargo test events_ws -- --nocapture
cargo test socket_io -- --nocapture
```

Optional manual smoke test after `web` build:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -- serve --host 127.0.0.1 --port 4002
```

In another terminal:

```bash
curl -N http://127.0.0.1:4002/api/v1/events
```

Then use the dashboard or API to create a mock connection and send commands. The SSE `curl -N` session should remain open and print later serial events when the server records them.

---

## Commit Guidance for Implementation Agent

Use a single focused implementation commit unless the orchestration environment requests smaller commits.

Expected implementation commit message:

```text
feat: add live serial event streaming
```

Before committing, inspect:

```bash
git status --short
git diff --stat
git diff --check
```

Expected changed files are limited to the files listed in **Expected Files to Modify** plus `Cargo.lock` if dependencies/features changed.

Do not commit:

- Generated `web/dist/` changes unless the repository already tracks them and the diff is expected.
- Secrets, local `.env` files, machine-specific paths, or serial hardware logs.
- Unrelated CI/release/Docker changes.

---

## Copy/Paste Prompt for Hands-Off Implementation Agent

You are implementing Phase 22 in `/home/alfarie/repos/serialport-api` on branch `rewrite/axum-serial-api`. Use `docs/phase-22-live-streaming-handoff.md` as the source of truth and complete the phase end-to-end without asking for clarification unless there is a hard safety blocker.

Required outcome: preserve existing event endpoint paths and wire schemas while making `/api/v1/events`, `/api/v1/events/ws`, and `/socket.io/?EIO=4&transport=websocket` true long-lived streams that replay current in-memory event snapshots and then broadcast newly recorded serial events to connected clients. Add manager-level subscription support, hardware-free tests, documentation updates, and minimal dashboard copy polish if needed.

Operate in this order:

1. Confirm repository root, branch, recent history, and clean working tree.
2. Read the full Phase 22 handoff and inspect referenced files.
3. Write failing manager broadcast tests first, then implement manager subscription support.
4. Write failing SSE live route tests first, then implement long-lived SSE stream behavior.
5. Write failing native WebSocket live route tests first, then implement long-lived WebSocket event behavior.
6. Write failing Socket.IO-compatible live route tests first, then implement long-lived Socket.IO event behavior.
7. Update dashboard copy/helpers only as needed; keep `EventSource('/api/v1/events')`.
8. Update README/docs.
9. Run full verification: `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features`, `cd web && pnpm typecheck && pnpm build`, and `git diff --check`.
10. Commit with `feat: add live serial event streaming` if verification passes and the environment allows commits.

Strictly do not add authentication, new serial protocol semantics, hardware-dependent tests, durable event persistence, full Socket.IO server behavior, new route paths as replacements, or unrelated CI/release/Docker changes.
