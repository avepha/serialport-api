# Phase 17 Socket.IO Compatibility Handoff

> **For Hermes / next AI implementation session:** Execute this in a fresh session. Load `writing-plans`, `test-driven-development`, and `rust-axum-api-tdd` before editing. This phase should add a deliberately small Socket.IO/Engine.IO compatibility surface for legacy/browser clients that cannot consume the existing SSE endpoint or the Phase 16 native WebSocket endpoint. Keep tests hardware-free. Do **not** replace or alter native SSE/WebSocket behavior.

**Goal:** Add minimal Socket.IO/Engine.IO compatibility for consuming serial events from the existing event snapshot store. This is an event-stream compatibility shim, not a full Socket.IO server. The implementation should support the smallest protocol subset needed for Socket.IO-style clients to receive the same serial event objects already exposed through `GET /api/v1/events` and `GET /api/v1/events/ws`.

**Inferred next phase:** Phase 17 is **Socket.IO compatibility for serial event consumption**. Repository evidence supports implementing a scoped compatibility phase rather than deferring it:

- Phase 16 completed native WebSocket event snapshots at commit `317206f feat: add WebSocket event stream`.
- `README.md` still lists `Socket.IO protocol compatibility, if needed by legacy/browser clients` under planned/not complete work.
- `README.md` explicitly states `/api/v1/events/ws` is a native WebSocket endpoint only and Socket.IO/Engine.IO clients are not compatible with it.
- `docs/open-source-spec.md` documents the old JavaScript migration source as using Socket.IO real-time events, and states Socket.IO/Engine.IO protocol compatibility remains separate future work.
- `Cargo.toml` has Axum WebSocket support and `tokio-tungstenite` test support, but no Socket.IO/Engine.IO dependency or route.
- Current routes expose SSE and native WebSocket event snapshots in `src/api/routes.rs`, making an event-only compatibility shim feasible without changing serial, command, storage, Docker, or systemd behavior.

---

## Strict Orchestration Input Schema

The implementation agent should accept this handoff plus the repository as its complete input. No hidden context is required.

```json
{
  "agent_role": "implementation",
  "phase": "Phase 17",
  "repository": "/home/alfarie/repos/serialport-api",
  "branch": "rewrite/axum-serial-api",
  "base_commit_expected": "317206f feat: add WebSocket event stream",
  "toolchain_env": {
    "PATH_prefix": "$HOME/.cargo/bin"
  },
  "scope": "Add minimal Socket.IO/Engine.IO compatibility for clients to consume existing serial event snapshots while preserving native SSE and WebSocket endpoints",
  "required_artifact_to_read": "docs/phase-17-socketio-compatibility-handoff.md",
  "non_goals": [
    "Full Socket.IO server feature parity",
    "Namespaces beyond the default namespace",
    "Rooms, acknowledgements, binary attachments, middleware, authentication, CORS policy design, or clustering",
    "Client-to-server command submission over Socket.IO",
    "Changing or removing GET /api/v1/events SSE behavior",
    "Changing or removing GET /api/v1/events/ws native WebSocket behavior",
    "Changing command, connection, preset, config, Docker, systemd, release, or serial transport behavior",
    "Hardware-required automated tests",
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
git log --oneline -8
```

Expected:

- Branch is `rewrite/axum-serial-api`.
- Working tree is clean.
- Recent history includes `317206f feat: add WebSocket event stream` or a descendant of it.

If the working tree is not clean before Phase 17 edits, stop and report instead of modifying files.

---

## Strict Orchestration Output Schema

The implementation agent's final response must use this JSON shape:

```json
{
  "agent_role": "implementation",
  "phase": "Phase 17",
  "summary": [
    "Added a minimal Engine.IO/Socket.IO compatibility endpoint for serial event snapshots.",
    "Preserved existing SSE and native WebSocket event endpoints and documented compatibility limitations."
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
      "cargo test --all-features"
    ],
    "status": "passed"
  },
  "commit": "<sha or null>",
  "approval_status": "ready_for_review|blocked|deferred",
  "issues": []
}
```

If blocked, set `commit` to `null`, `approval_status` to `blocked`, and list exact blockers. If maintainers or investigation reject implementation in favor of deferral, set `approval_status` to `deferred`, update only docs that record the explicit deferral, run applicable docs/lightweight checks, and commit with a `docs:` message.

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

Latest known completed phase commit:

```text
317206f feat: add WebSocket event stream
```

Important behavior after Phase 16:

- `cargo run -- serve --host 127.0.0.1 --port 4002` starts the Axum HTTP server in mock/in-memory mode.
- Default startup is hardware-free and does not open physical serial ports.
- `serve --real-serial` opts into opening/writing/reading OS serial ports.
- `serve --preset-db <PATH>` opts into SQLite-backed preset persistence.
- The API exposes health, ports, connections, commands, SSE events, native WebSocket event snapshots, legacy aliases, and preset CRUD routes.
- `GET /api/v1/events` returns Server-Sent Events for recorded serial event snapshots.
- `GET /api/v1/events/ws` is a native WebSocket endpoint that sends JSON text frames shaped as `{ "event": <string>, "data": <json> }` and then closes normally.
- Current serial event names are:
  - `serial.json`
  - `serial.text`
  - `serial.log`
  - `serial.notification`
  - `serial.error`
- The current event store is snapshot-based through `ConnectionManager::events()`, not a live fan-out bus.
- Native WebSocket support is not Socket.IO compatibility; Socket.IO clients expect Engine.IO handshake and Socket.IO packet framing.

Important local toolchain note:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Use that before all `cargo` commands in this WSL environment to avoid Rust toolchain mismatch.

---

## Phase 17 Scope

Do in Phase 17:

- Add a minimal Socket.IO/Engine.IO compatibility endpoint for event consumption.
- Prefer the conventional Socket.IO path:

```text
GET /socket.io/?EIO=4&transport=websocket
```

- Optionally also support a namespaced API path if it is simpler to document and test:

```text
GET /api/v1/events/socket.io?EIO=4&transport=websocket
```

- The conventional `/socket.io/` route is the primary compatibility target because legacy Socket.IO clients commonly use it by default.
- Support Engine.IO v4 WebSocket transport only.
- Use existing recorded serial event snapshots from `connection_manager.events()`.
- Emit each serial event as a Socket.IO event packet whose logical event name is the existing serial event name and whose data is the existing `SerialStreamEvent::data` JSON value.
- Keep the compatibility packet schema deterministic and documented.
- Add hardware-free tests for:
  - successful Engine.IO v4 WebSocket connection,
  - handshake/open packet,
  - Socket.IO connect packet,
  - seeded serial event packets,
  - graceful close or deterministic end behavior,
  - rejection or clear failure for unsupported transports/versions if implemented as HTTP validation.
- Update `README.md` to document the compatibility endpoint, limitations, and examples.
- Optionally update `docs/open-source-spec.md` if needed to keep the long-form spec accurate.

Out of scope / do **not** do in Phase 17:

- Do not implement long-polling transport unless a selected dependency requires it and tests remain small.
- Do not implement the full Socket.IO feature set: rooms, multiple namespaces, acknowledgements, binary payloads, compression tuning, middleware, reconnection state, scaling, or server-side emits beyond serial event snapshots.
- Do not add Socket.IO command submission or map `/commit` semantics onto Socket.IO.
- Do not alter serial manager semantics or event classification.
- Do not redesign event storage into a live broadcast system unless absolutely required by the selected library and kept backward-compatible. Snapshot-based behavior is sufficient.
- Do not alter `GET /api/v1/events` SSE response formatting, content type, or tests.
- Do not alter `GET /api/v1/events/ws` native WebSocket message schema or close behavior.
- Do not change Docker, systemd, release workflows, config precedence, SQLite persistence, or real serial defaults except a tiny docs note if necessary.
- Do not require physical serial hardware in tests or CI.
- Do not push.

---

## Expected Files to Inspect Before Editing

Read these first:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"

# Repository docs and roadmap
README.md
docs/open-source-spec.md
docs/implementation-plan.md
docs/phase-16-websocket-events-handoff.md

# Current routes, event storage, startup, and dependencies
Cargo.toml
src/api/routes.rs
src/serial/manager.rs
src/protocol.rs
src/main.rs
```

Use `read_file`/`search_files` equivalents if operating through tools that prohibit shell readers.

Key strings to search:

```bash
Socket.IO
Engine.IO
events/ws
/api/v1/events
SerialStreamEvent
connection_manager.events
```

---

## Expected Files to Modify or Create

Required if implementing compatibility:

- Modify: `src/api/routes.rs`
  - Add route wiring for the Socket.IO compatibility endpoint.
  - Add handler/helper functions or integration with a minimal dependency.
  - Add hardware-free tests seeded from `InMemoryConnectionManager` events.

- Modify: `README.md`
  - Move Socket.IO compatibility from planned/not complete to implemented once code lands.
  - Document the endpoint, packet/event mapping, example clients, and limitations.
  - Keep native SSE and native WebSocket docs unchanged except cross-links.

Likely required:

- Modify: `Cargo.toml`
  - Add a minimal Socket.IO/Engine.IO dependency only if it materially reduces protocol risk.
  - If implementing the small protocol shim manually, avoid new runtime dependencies unless tests need them.

- Modify: `Cargo.lock`
  - Expected if dependency changes are made.

Optional only if justified:

- Modify: `docs/open-source-spec.md`
  - Refresh Socket.IO compatibility status and limitations after implementation.

Files not expected to change:

- `src/serial/**` unless a tiny trait/helper change is required for testability.
- `src/protocol.rs` unless adding pure packet-format helpers and tests there is cleaner.
- `src/main.rs` unless router construction must change; it should normally not.
- `src/config.rs`.
- `src/storage/**`.
- Dockerfile, `.dockerignore`, `.github/workflows/**`, examples, systemd docs.

If any unexpected file changes become necessary, document the reason explicitly in the final output and keep the change minimal.

---

## Required Compatibility Contract

### Endpoint

Primary endpoint:

```text
GET /socket.io/?EIO=4&transport=websocket
```

Optional secondary endpoint:

```text
GET /api/v1/events/socket.io?EIO=4&transport=websocket
```

### Supported protocol subset

Support only this subset unless a dependency makes slightly broader support trivial and tested:

- Engine.IO protocol version: v4 (`EIO=4`).
- Transport: WebSocket only (`transport=websocket`).
- Socket.IO namespace: default namespace only.
- Direction: server-to-client serial event packets only.
- Event source: current snapshot from `connection_manager.events()`.
- Event names: existing serial event names, unchanged.

### Suggested frame sequence for manual shim

If implementing without a Socket.IO crate, use the Engine.IO/Socket.IO text frame sequence below. This is intentionally minimal and should be tested with a raw WebSocket test client and, if feasible, a real `socket.io-client` manual smoke check.

On WebSocket upgrade:

1. Send Engine.IO open packet:

```text
0{"sid":"<generated>","upgrades":[],"pingInterval":25000,"pingTimeout":20000,"maxPayload":1000000}
```

2. Send Socket.IO connect packet for default namespace:

```text
40
```

3. For each recorded serial event, send Socket.IO event packet. Logical payload shape:

```json
[
  "serial.json",
  {
    "reqId": "1",
    "ok": true
  }
]
```

Wire frame:

```text
42["serial.json",{"reqId":"1","ok":true}]
```

Another example:

```text
42["serial.text","hello robot"]
```

4. Close the socket normally after sending snapshot events, or leave it open only if ping/pong and deterministic tests are implemented. Snapshot-and-close matches current native WebSocket behavior and is preferred for this phase.

### Client input handling

For this minimal snapshot implementation:

- Accept client ping packet `2` and respond with pong packet `3` if the socket remains open long enough to receive it.
- It is acceptable to ignore Socket.IO client event packets and close after snapshot delivery.
- Do not panic on client disconnect or malformed input.

### HTTP/query validation

At minimum, unsupported Engine.IO query parameters should not be silently documented as supported:

- `EIO` missing or not `4`: return HTTP `400` before upgrade if practical, or close the socket with a clear reason if validation happens after upgrade.
- `transport` missing or not `websocket`: return HTTP `400` before upgrade.

If using a library that manages validation differently, document the actual behavior and test the compatibility path.

### Error behavior

- If event snapshot retrieval fails, fail before upgrade with HTTP `500` when possible, consistent with the SSE/native WebSocket handlers.
- If serialization fails unexpectedly, close gracefully or return server error before upgrade where possible.
- If a client disconnects while frames are being sent, stop sending and do not panic.

---

## TDD Tasks

### Task 17.1: Establish baseline and RED target

Run:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
git log --oneline -8
cargo test --all-features
```

Expected baseline:

- Existing tests pass.
- Native SSE and native WebSocket endpoints exist.
- No Socket.IO compatibility route exists yet.

Add or sketch the first failing test before implementation:

- Seed `InMemoryConnectionManager` with `SerialEvent::Json(json!({"reqId":"1","ok":true}))` and `SerialEvent::Text("hello robot".to_string())`.
- Start the router on a local ephemeral TCP listener.
- Connect with `tokio_tungstenite::connect_async("ws://{addr}/socket.io/?EIO=4&transport=websocket")`.
- Expect Engine.IO open frame, Socket.IO connect frame, and two `42[...]` event frames.

### Task 17.2: Decide dependency strategy

Choose one:

1. **Manual minimal shim, preferred if small:** Use Axum WebSocket support already present, manually send the limited frame sequence above, and avoid a new dependency.
2. **Minimal Socket.IO/Engine.IO crate:** Add a Rust crate only if it supports Axum/Tokio cleanly, keeps the route small, and can be tested deterministically without hardware.

Decision criteria:

- Compatibility with Rust 1.75 and existing Axum 0.7/Tokio stack.
- Small dependency footprint.
- Deterministic hardware-free tests.
- Ability to preserve existing routes and snapshots.

Document the decision in code comments only if the implementation would otherwise be surprising; otherwise README limitation docs are enough.

### Task 17.3: Implement route validation and handshake

Add route wiring in `router_with_state`, likely:

```rust
.route("/socket.io/", get(socket_io_events::<L, C>))
```

Handler requirements:

- Extract query params for `EIO` and `transport`.
- Require `EIO=4` and `transport=websocket`.
- Retrieve `connection_manager.events()` before upgrade if possible.
- Upgrade to WebSocket.
- Send Engine.IO open packet and Socket.IO connect packet.

Targeted validation:

```bash
cargo test --all-features socket_io
```

### Task 17.4: Emit serial event packets

For each `SerialStreamEvent`:

- Use the exact event name as packet array element 0.
- Use the existing JSON data as packet array element 1.
- Serialize as Socket.IO event packet text: `42` + JSON array.

Example mapping:

- Internal: `{ event: "serial.json", data: {"reqId":"1","ok":true} }`
- Wire: `42["serial.json",{"reqId":"1","ok":true}]`

Test with JSON parsing after stripping the `42` prefix rather than relying on object key order.

### Task 17.5: Preserve existing SSE and native WebSocket behavior

Run targeted tests:

```bash
cargo test --all-features events_route_streams
cargo test --all-features events_ws
```

Expected:

- Existing SSE tests remain unchanged and pass.
- Existing native WebSocket tests remain unchanged and pass.

### Task 17.6: Add negative-path tests

Add hardware-free tests for unsupported query parameters:

- `/socket.io/?EIO=3&transport=websocket` returns `400 Bad Request` or equivalent documented rejection.
- `/socket.io/?EIO=4&transport=polling` returns `400 Bad Request` or equivalent documented rejection.

If the WebSocket client library makes pre-upgrade status assertions awkward, use `tower::ServiceExt::oneshot` with normal HTTP requests for these validation tests.

### Task 17.7: README/spec update

Update `README.md`:

- Add implemented feature bullet for minimal Socket.IO/Engine.IO event compatibility.
- Remove or revise the planned/not complete Socket.IO bullet after implementation.
- Add a subsection near event streaming docs:

```text
GET /socket.io/?EIO=4&transport=websocket
```

Document:

- Engine.IO v4 WebSocket transport only.
- Event-only snapshot behavior.
- Event packet examples.
- Limitations versus a full Socket.IO server.
- Native SSE and native WebSocket remain the recommended simple protocols for new clients.

Optional `docs/open-source-spec.md` update:

- Change future/deferred statements to say minimal Socket.IO event compatibility exists, while full Socket.IO feature parity is intentionally out of scope.

### Task 17.8: Full verification and commit

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
git commit -m "feat: add Socket.IO event compatibility"
```

Only include `Cargo.toml`, `Cargo.lock`, or `docs/open-source-spec.md` if they changed.

Do not push.

---

## Acceptance Criteria

Phase 17 is complete when all of these are true:

1. A Socket.IO-compatible event endpoint is available at `GET /socket.io/?EIO=4&transport=websocket`.
2. The endpoint performs an Engine.IO v4 WebSocket handshake sufficient for minimal Socket.IO clients or for the documented raw frame sequence.
3. A client receives an Engine.IO open packet and a Socket.IO default namespace connect packet.
4. Seeded existing serial events are emitted as Socket.IO event packets using the existing event names and data values.
5. The event packet schema is documented and tested, e.g. `42["serial.json",{"reqId":"1","ok":true}]`.
6. Unsupported Engine.IO versions and unsupported transports are rejected or explicitly handled and tested.
7. Existing `GET /api/v1/events` SSE tests and behavior remain unchanged.
8. Existing `GET /api/v1/events/ws` native WebSocket tests and behavior remain unchanged.
9. Existing command, connection, ports, health, presets, legacy alias, config, Docker, release, and serial tests still pass.
10. Tests are hardware-free and deterministic.
11. README accurately documents the compatibility endpoint and limitations versus full Socket.IO.
12. `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` pass with `PATH="$HOME/.cargo/bin:$PATH"`.
13. The implementation is committed with a conventional commit message and not pushed.

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
cargo test --all-features socket_io
cargo test --all-features events_route_streams
cargo test --all-features events_ws
grep -R "Socket.IO\|Engine.IO\|/socket.io\|/api/v1/events/ws" -n README.md docs/open-source-spec.md src/api/routes.rs Cargo.toml
```

Docker packaging verification is not required if Docker files are not touched. Hardware verification is not required.

---

## Manual Smoke Checks

After automated tests pass, manual smoke is recommended if a Socket.IO client or raw WebSocket CLI is available.

Start the server:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -- serve --host 127.0.0.1 --port 4002
```

Verify existing endpoints still work:

```bash
curl -s http://127.0.0.1:4002/api/v1/health
curl -i -s http://127.0.0.1:4002/api/v1/events
websocat ws://127.0.0.1:4002/api/v1/events/ws
```

Raw WebSocket check of the Socket.IO compatibility endpoint:

```bash
websocat 'ws://127.0.0.1:4002/socket.io/?EIO=4&transport=websocket'
```

Expected initial frames for a fresh server may be only:

```text
0{"sid":"...","upgrades":[],"pingInterval":25000,"pingTimeout":20000,"maxPayload":1000000}
40
```

A fresh server may have no recorded events, so no `42[...]` event packets may appear before close. Route tests must seed events and prove non-empty behavior.

Optional real Socket.IO client smoke if Node is available:

```bash
node - <<'NODE'
const { io } = require("socket.io-client");
const socket = io("ws://127.0.0.1:4002", { transports: ["websocket"] });
socket.on("connect", () => console.log("connected", socket.id));
socket.on("serial.json", (data) => console.log("serial.json", data));
socket.on("serial.text", (data) => console.log("serial.text", data));
socket.on("disconnect", (reason) => { console.log("disconnect", reason); process.exit(0); });
setTimeout(() => process.exit(0), 3000);
NODE
```

This optional smoke requires installing `socket.io-client`; do not add it to the repository unless maintainers explicitly want JavaScript smoke tooling.

---

## Risks and Mitigations

- **Protocol complexity creep:** Socket.IO has many features. Mitigate by implementing only Engine.IO v4 WebSocket event snapshots and documenting limitations.
- **False compatibility claims:** A raw WebSocket endpoint is not Socket.IO. Mitigate by testing Engine.IO/Socket.IO frame prefixes and README examples.
- **Dependency mismatch:** Socket.IO crates may not align with Axum 0.7 or Rust 1.75. Mitigate by preferring a manual minimal shim or carefully validating dependency compatibility before committing.
- **Snapshot vs live stream expectations:** Current manager exposes snapshots. Mitigate by documenting snapshot-and-close behavior and deferring live broadcast fan-out.
- **Client library expectations:** Real Socket.IO clients may expect ping/pong or namespace semantics. Mitigate by supporting default namespace connect, basic ping/pong if socket remains open, and manual smoke with `socket.io-client` if feasible.
- **Breaking native event endpoints:** Mitigate by leaving existing handlers untouched and running existing SSE/native WebSocket tests.
- **Serialization/order flakiness:** JSON object key order may vary. Mitigate by parsing packet payloads in tests rather than comparing object JSON strings except for stable prefixes.
- **Open socket test hangs:** Mitigate with snapshot-and-close behavior and `tokio::time::timeout` around frame reads.

---

## Commit Guidance

Implementation commit message should be conventional, for example:

```bash
git commit -m "feat: add Socket.IO event compatibility"
```

Commit only the files needed for Phase 17. Do not push.

Before committing, inspect:

```bash
git diff --stat
git diff -- Cargo.toml README.md docs/open-source-spec.md src/api/routes.rs
```

Expected changed files are limited to:

- `src/api/routes.rs`
- `README.md`
- optionally `Cargo.toml`
- optionally `Cargo.lock`
- optionally `docs/open-source-spec.md`

---

## Explicit Deferral Alternative

If implementation investigation finds that minimal Socket.IO compatibility is not feasible or not valuable without a full protocol dependency, do not partially implement. Instead:

1. Leave source code unchanged.
2. Update documentation only to explicitly defer Socket.IO compatibility, including rationale and recommended alternatives (`/api/v1/events` SSE and `/api/v1/events/ws` native WebSocket).
3. Add or update a handoff/decision doc for deferral.
4. Commit with a conventional docs message such as:

```bash
git commit -m "docs: defer Socket.IO compatibility"
```

Use this alternative only with concrete evidence, such as dependency incompatibility, unacceptable protocol scope, or maintainer decision. Based on current repository evidence, the default Phase 17 path is to implement the minimal event-only compatibility shim.

---

## Implementation Agent Short Instruction

Add a minimal event-only Socket.IO/Engine.IO v4 WebSocket compatibility endpoint at `/socket.io/?EIO=4&transport=websocket`, backed by existing serial event snapshots; keep SSE and native WebSocket unchanged; add hardware-free protocol tests and README limitation docs; run full Rust verification; commit; do not push.
