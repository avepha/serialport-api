# Phase 11 Real Serial Lifecycle Handoff

> **For Hermes / next AI coding session:** Execute this in a fresh session. Load `writing-plans`, `test-driven-development`, and `rust-axum-api-tdd` before editing. This phase should add an opt-in real serial-port lifecycle for the Axum server while preserving all mock/default behavior. Keep the scope narrow and test-first. Do not add SQLite presets, broad config-file discovery, Docker/systemd packaging, WebSocket/Socket.IO, authentication, a large API error-envelope refactor, or hardware-dependent CI tests.

**Goal:** Add a production-facing real serial backend that can open named OS serial ports, write framed commands to those ports, and feed read lines back into the existing `serial.json`/`serial.text`/`serial.log`/`serial.notification`/`serial.error` event and waited-response paths. The real backend must be explicitly opt-in so the current hardware-free mock server remains the default.

**Architecture:** Build on Phases 7-10 instead of bypassing them:

- Phase 7 introduced `SerialTransport` and `MockSerialTransport`.
- Phase 8 introduced parsed read-loop event recording through `read_loop` seams.
- Phase 9 introduced response queues keyed by connection name and string `reqId` plus waited command handling.
- Phase 10 introduced opt-in mock-device scripted responses that record synthesized inbound JSON through the same manager/event/response path.

Phase 11 should add a real `SerialTransport` implementation and a real serial read-loop source/runner. Real inbound serial data should still be parsed by `protocol::parse_line` and recorded through `record_event_for_connection`; waited command success should come from Phase 9 response queues without special route logic.

**Tech Stack:** Rust 2021, Axum 0.7, Tokio 1, Serde/Serde JSON, Thiserror, Tracing, Clap, `serialport`, test-first Rust unit tests and Axum route tests. Avoid new dependencies unless a small dependency is clearly justified.

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

Latest known relevant commits after Phase 10:

```text
8a039ad feat: add mock device scripted responses
fcbae2a docs: add phase 10 mock device handoff
bf2d64a feat: add waited command responses
2231156 docs: add phase 9 reqid response matching handoff
1ae4dbb feat: record serial read loop events
6e2d2ac feat: add mock serial read source
2e14727 feat: add serial transport boundary
8fc5e77 ci: add Rust verification workflow
0961b3d docs: rewrite project README
```

Phase 10 review status:

- Verdict: **APPROVED**.
- No critical or important issues.
- Verification passed: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features`.
- Manual smoke passed for:
  - default server;
  - opt-in `serve --mock-device`;
  - opt-in `serve --mock-script <PATH>`.

Completed functionality as of this handoff:

- Axum server starts with `cargo run -- serve --host 127.0.0.1 --port 4002`.
- Default server remains hardware-free and mock-backed.
- Port listing uses `serialport::available_ports()`.
- Named connection lifecycle exists for canonical routes and legacy aliases.
- Commands generate or preserve string `reqId`, frame JSON with the connection delimiter, and write through `SerialTransport`.
- `ConnectionManagerWithTransport<T>` owns connection registry, request id counter, SSE event storage, response queues, transport, and an optional Phase 10 mock responder.
- `src/serial/transport.rs` currently has `SerialTransport` and `MockSerialTransport` only.
- `src/serial/read_loop.rs` currently has `MockSerialReadSource`, `SerialReadSource`, `SerialEventRecorder`, `drain_serial_read_items`, and a small `spawn_mock_read_loop` helper.
- Phase 9 waited responses work when inbound JSON with a matching string `reqId` is recorded.
- Phase 10 mock-device responses use `record_event_for_connection`, reusing Phase 9 response queues and SSE storage.
- CLI flags exist: `serve --mock-device` and `serve --mock-script <PATH>`; `--mock-script` implies mock-device.
- `/commit` remains fire-and-forget and legacy aliases are preserved.

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
- Current baseline from Phase 10 approval is 42 library tests plus 1 main test.

---

## Why Phase 11 Is Real Serial Lifecycle

The original open-source spec requires actual serial communication for Raspberry Pi and microcontroller deployments. The project now has the testable seams needed to add it safely:

- mockable transport boundary for open/close/write;
- read-loop parsing and event recording;
- waited response matching by `reqId`;
- hardware-free mock-device smoke tests proving API compatibility.

The main remaining v1 runtime gap is that the live server still uses `MockSerialTransport` by default and does not open physical serial ports. Phase 11 should close that gap behind an explicit opt-in flag while leaving the default development server deterministic and hardware-free.

---

## Phase 11 Scope

Do in Phase 11:

- Add a real serial transport implementation, likely `src/serial/real_transport.rs`.
- Open real serial ports on `connect` using each connection's `port`, `baudRate`, and delimiter metadata.
- Store and manage real serial port handles by connection name.
- Write framed command bytes to the correct real port when the canonical command route or `/commit` calls `send_command`.
- Close/drop real serial handles on disconnect.
- Add a real serial read loop or read source that reads lines from opened real ports, parses them with `protocol::parse_line`, and records events through the existing manager path.
- Add an explicit CLI flag such as `serve --real-serial` to opt into the real backend.
- Preserve `serve` with no flags as the current mock/default server.
- Decide and document precedence for incompatible flags. Recommended: reject `--real-serial` combined with `--mock-device` or `--mock-script` with a clear startup error.
- Add unit tests using fake/in-memory serial-port handles or adapter traits; tests must not require `/dev/ttyUSB0` or any physical device.
- Add route/CLI tests proving default mock mode is unchanged and real-mode wiring can be constructed with test doubles.
- Add a short README note for `--real-serial`, including the warning that hardware-dependent manual smoke requires a connected device or loopback adapter.

Do **not** do in Phase 11:

- Do not make real serial mode the default.
- Do not add hardware-dependent tests to normal `cargo test`.
- Do not require any specific local serial path such as `/dev/ttyUSB0`, `/dev/ttyACM0`, or `/dev/ROBOT` in automated tests.
- Do not remove or rename `MockSerialTransport`, mock-device mode, or mock-script behavior.
- Do not change SSE event names or wrap event data in a new envelope.
- Do not change the response shapes of fire-and-forget commands, waited commands, connection lifecycle routes, or legacy aliases.
- Do not change generated `reqId` sequencing.
- Do not add SQLite presets, config-file discovery, Docker, systemd, release binaries, WebSocket/Socket.IO, authentication, or broad API error refactors.
- Do not implement reconnect/backoff, hotplug monitoring, persistent connection profiles, or multi-process serial-port locking. Those can be later phases.

If a tempting change requires long-running daemon supervision, persistent config, saved presets, real hardware in CI, or a comprehensive production deployment story, leave it for a later phase.

---

## Expected Files to Modify or Create

Expected implementation changes:

- Create: `src/serial/real_transport.rs`
  - Real serial transport implementation.
  - A testable handle/factory seam around `serialport::new(...).open()`.
  - Unit tests using fake handles/factories.
- Modify: `src/serial/mod.rs`
  - Export `real_transport`.
- Modify: `src/serial/read_loop.rs`
  - Add a real read-loop abstraction or helper that can read complete delimiter-terminated frames/lines from a real serial handle and record parsed events.
  - Keep existing mock read-loop helpers and tests passing.
- Modify: `src/serial/manager.rs`
  - Only if needed to expose connection metadata, transport hooks, or real-read-loop integration points.
  - Avoid route-specific real serial logic in the manager if a transport/read-loop seam is cleaner.
- Modify: `src/main.rs`
  - Add `serve --real-serial`.
  - Construct the real backend only when requested.
  - Reject incompatible `--real-serial` + mock-device/script combinations.
- Modify: `src/api/routes.rs`
  - Only if needed for an injectable app-state/router constructor or tests. Do not add hardware-specific route branches.
- Modify: `src/error.rs`
  - Add narrow error variants only if necessary for startup flag validation or serial I/O error context.
- Modify: `README.md`
  - Add a concise real-serial usage note and manual smoke guidance.

Optional but useful:

- Create `examples/` or `docs/` snippets later, not in Phase 11 unless very small.

No dependency additions should be necessary. The project already depends on `serialport`.

---

## Current Code to Understand First

Read these files before editing:

```bash
cd /home/alfarie/repos/serialport-api
sed -n '1,220p' src/serial/transport.rs
sed -n '1,360p' src/serial/manager.rs
sed -n '360,820p' src/serial/manager.rs
sed -n '1,320p' src/serial/read_loop.rs
sed -n '1,240p' src/serial/mock_device.rs
sed -n '1,220p' src/protocol.rs
sed -n '1,360p' src/api/routes.rs
sed -n '360,980p' src/api/routes.rs
sed -n '1,180p' src/main.rs
sed -n '1,140p' src/error.rs
sed -n '1,120p' Cargo.toml
```

Key current facts:

- `SerialTransport` is synchronous and cloneable:
  - `open(&ConnectionInfo) -> Result<()>`
  - `close(name) -> Result<()>`
  - `write_frame(name, frame) -> Result<()>`
- `MockSerialTransport` records opened names, closed names, and written frames.
- `ConnectionManagerWithTransport<T>` is generic over `SerialTransport` and currently works with `MockSerialTransport` in the live server.
- `send_command` frames JSON, calls `transport.write_frame`, and optionally records a Phase 10 mock response.
- `record_event_for_connection(connection, SerialEvent::Json(value))` stores both SSE events and waitable responses by string `reqId`.
- `read_loop::drain_serial_read_items` already converts `SerialReadItem::Line` into parsed protocol events and `SerialReadItem::Error` into `serial.error`.
- `main.rs` chooses between `routes::router()` and a mock-device-enabled manager. Phase 11 should add a third explicit real-serial path.

---

## Recommended Design

### 1. Add a real serial handle/factory seam

A direct implementation can use `serialport::new(connection.port.clone(), connection.baud_rate).timeout(...).open()`, but tests should not open hardware. Introduce a small internal seam so unit tests can use fake handles.

Suggested shape:

```rust
pub trait SerialPortHandle: Send {
    fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()>;
    fn flush(&mut self) -> std::io::Result<()>;
    fn read_byte(&mut self) -> std::io::Result<Option<u8>>;
}

pub trait SerialPortFactory: Clone + Send + Sync + 'static {
    type Handle: SerialPortHandle;

    fn open(&self, connection: &ConnectionInfo) -> Result<Self::Handle>;
}
```

The exact names can differ. Keep the seam private to `real_transport.rs` unless needed elsewhere.

For the production factory, wrap `Box<dyn serialport::SerialPort>` and map:

- `write_all` to `std::io::Write::write_all`;
- `flush` to `std::io::Write::flush`;
- reads to `std::io::Read::read` one byte or in chunks.

Use a small timeout such as 50-100 ms for real port reads so background loops can stop promptly. Avoid blocking forever.

### 2. Implement `RealSerialTransport`

Suggested responsibilities:

- Store handles by connection name in `Arc<Mutex<BTreeMap<String, Arc<Mutex<Handle>>>>>` or equivalent.
- `open(connection)` opens a handle through the factory and stores it under `connection.name`.
- If opening fails, return a `SerialportApiError` mapped from `serialport::Error` or `std::io::Error`.
- `write_frame(name, frame)` finds the handle, writes all bytes, flushes, and returns `ConnectionNotFound` or a serial I/O error if missing/failing.
- `close(name)` removes the handle. Dropping the handle closes the OS port.

Recommended public aliases:

```rust
pub type SystemRealSerialTransport = RealSerialTransport<SystemSerialPortFactory>;
```

Keep tests focused on fake factories/handles.

### 3. Add real read-loop wiring without route-specific logic

The cleanest read side depends on the chosen handle storage design. Two acceptable approaches:

1. **Transport-owned readable handles:** `RealSerialTransport` exposes a testable method such as `read_available_line(connection_name, delimiter)` or `try_read_frame` that reads from the same stored handle.
2. **Real read source:** Add `RealSerialReadSource` that shares the handle registry with `RealSerialTransport` and implements a drain/read method.

Either way, real inbound lines should end up at:

```rust
manager.record_event_for_connection(connection_name, protocol::parse_line(&line));
```

Recommended runner shape:

```rust
pub fn spawn_real_read_loop<M, R>(
    manager: M,
    read_source: R,
    connection_name: String,
    delimiter: String,
) -> tokio::task::JoinHandle<()>
where
    M: SerialEventRecorder,
    R: RealSerialReadSourceLike,
```

The loop should:

- continue until the connection/handle is gone or a stop signal is received;
- read delimiter-terminated frames/lines;
- parse via `protocol::parse_line`;
- record errors as `serial.error` but avoid a tight error loop;
- use `tokio::task::spawn_blocking` or a dedicated blocking thread for blocking serial reads if needed.

Keep this first version simple. If fully automatic spawn-on-connect becomes too invasive, document and implement the narrowest safe runner that `main.rs` can call in real mode.

### 4. Add explicit CLI mode

Extend `ServeArgs` in `src/main.rs`:

```text
--real-serial
```

Recommended behavior:

- `cargo run -- serve --host 127.0.0.1 --port 4002` remains the current default mock transport with no auto-response.
- `cargo run -- serve --host 127.0.0.1 --port 4002 --mock-device` remains Phase 10 mock-device mode.
- `cargo run -- serve --host 127.0.0.1 --port 4002 --mock-script ./mock-responses.json` remains Phase 10 scripted mock-device mode.
- `cargo run -- serve --host 127.0.0.1 --port 4002 --real-serial` uses the real serial transport and read loop.
- `--real-serial --mock-device` or `--real-serial --mock-script` should fail before binding/listening or return a clear startup error. Prefer testing a pure validation helper.

### 5. Preserve all API routes

Routes should remain generic over `ConnectionManager`. Avoid adding `if real_serial` in handlers. Connect, disconnect, command, waited response, events, and legacy aliases should continue to call the same manager methods.

---

## Bite-Sized TDD Tasks

### Task 1: Add a real serial transport with fake-handle tests

**Objective:** Prove a real transport can open, write, flush, and close named handles without touching hardware.

**Files:**

- Create: `src/serial/real_transport.rs`
- Modify: `src/serial/mod.rs`

**Step 1: Write failing tests**

Add tests similar to:

```rust
#[test]
fn real_transport_opens_writes_flushes_and_closes_named_connection() {
    let factory = FakeSerialPortFactory::default();
    let transport = RealSerialTransport::new(factory.clone());
    let connection = ConnectionInfo {
        name: "default".to_string(),
        status: "connected",
        port: "/dev/ttyTEST0".to_string(),
        baud_rate: 115200,
        delimiter: "\r\n".to_string(),
    };

    transport.open(&connection).unwrap();
    transport
        .write_frame("default", b"{\"reqId\":\"1\"}\r\n")
        .unwrap();
    transport.close("default").unwrap();

    assert_eq!(factory.opened_ports(), vec![("/dev/ttyTEST0".to_string(), 115200)]);
    assert_eq!(factory.written_for("default"), b"{\"reqId\":\"1\"}\r\n".to_vec());
    assert_eq!(factory.flush_count_for("default"), 1);
    assert!(!transport.is_open("default"));
}

#[test]
fn real_transport_write_missing_connection_returns_connection_not_found() {
    let transport = RealSerialTransport::new(FakeSerialPortFactory::default());

    let error = transport.write_frame("missing", b"{}").unwrap_err();

    assert!(matches!(error, SerialportApiError::ConnectionNotFound(name) if name == "missing"));
}
```

Adapt helper names to your implementation. Keep fake factory/handle in `#[cfg(test)]`.

**Step 2: Run tests to verify RED**

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test serial::real_transport::tests::real_transport_opens_writes_flushes_and_closes_named_connection -- --nocapture
cargo test serial::real_transport::tests::real_transport_write_missing_connection_returns_connection_not_found -- --nocapture
```

Expected RED reason: FAIL because `serial::real_transport`, `RealSerialTransport`, and fake seam do not exist.

**Step 3: Implement minimal GREEN**

- Add the module and export it from `src/serial/mod.rs`.
- Add a testable `RealSerialTransport<F>` with fake factory support.
- Implement `SerialTransport` for it.
- Map missing connection writes to `SerialportApiError::ConnectionNotFound`.
- Map factory/write/flush errors into existing or narrow new error variants.

**Step 4: Run tests to verify GREEN**

```bash
cargo test serial::real_transport::tests::real_transport_opens_writes_flushes_and_closes_named_connection -- --nocapture
cargo test serial::real_transport::tests::real_transport_write_missing_connection_returns_connection_not_found -- --nocapture
```

Expected: PASS.

---

### Task 2: Add production serialport factory wiring

**Objective:** Prove the production factory preserves path/baud/timeout configuration in unit-testable code and compiles against the `serialport` crate.

**Files:**

- Modify: `src/serial/real_transport.rs`
- Modify if needed: `src/error.rs`

**Step 1: Write failing tests for pure config building**

Add a pure helper if needed:

```rust
#[test]
fn serial_open_settings_are_derived_from_connection_info() {
    let connection = ConnectionInfo {
        name: "robot".to_string(),
        status: "connected",
        port: "/dev/ttyUSB0".to_string(),
        baud_rate: 345600,
        delimiter: "\n".to_string(),
    };

    let settings = SerialOpenSettings::from_connection(&connection);

    assert_eq!(settings.port, "/dev/ttyUSB0");
    assert_eq!(settings.baud_rate, 345600);
    assert!(settings.timeout_ms > 0);
}
```

**Step 2: Run test to verify RED/GREEN**

```bash
cargo test serial::real_transport::tests::serial_open_settings_are_derived_from_connection_info -- --nocapture
```

Expected RED reason: FAIL until the pure settings helper exists. If Task 1 already made an equivalent helper, keep this as regression coverage.

**Step 3: Implement production factory**

- Add `SystemSerialPortFactory` or equivalent.
- Use `serialport::new(settings.port, settings.baud_rate).timeout(Duration::from_millis(settings.timeout_ms)).open()`.
- Wrap `Box<dyn serialport::SerialPort>` in the handle adapter.
- Ensure `cargo clippy` does not complain about unused production code.

**Step 4: Run focused tests**

```bash
cargo test serial::real_transport::tests -- --nocapture
```

Expected: PASS without real hardware.

---

### Task 3: Add real-line read helper with fake input

**Objective:** Prove delimiter-terminated serial bytes can be read into complete lines/frames and parsed through existing event recording, without real hardware.

**Files:**

- Modify: `src/serial/read_loop.rs`
- Modify if needed: `src/serial/real_transport.rs`

**Step 1: Write failing tests**

A small byte-oriented fake is enough:

```rust
#[test]
fn real_read_source_drains_complete_delimited_lines() {
    let source = FakeRealSerialReadSource::default();
    source.push_bytes("default", b"{\"reqId\":\"1\",\"ok\":true}\r\nhello");

    let lines = source.drain_lines("default", "\r\n").unwrap();

    assert_eq!(lines, vec![b"{\"reqId\":\"1\",\"ok\":true}\r\n".to_vec()]);
    assert_eq!(source.buffered_bytes("default"), b"hello".to_vec());
}

#[test]
fn real_read_lines_record_json_response_for_waited_commands() {
    let manager = InMemoryConnectionManager::default();
    let source = FakeRealSerialReadSource::default();
    source.push_bytes("default", b"{\"reqId\":\"abc\",\"ok\":true}\r\n");

    let processed = drain_real_serial_lines(&manager, &source, "default", "\r\n").unwrap();

    assert_eq!(processed, 1);
    assert_eq!(
        manager.take_response("default", "abc").unwrap(),
        Some(serde_json::json!({"reqId":"abc","ok":true}))
    );
    assert_eq!(manager.events().unwrap()[0].event, "serial.json");
}
```

Adapt names if the real read source is exposed from `real_transport.rs` instead.

**Step 2: Run tests to verify RED**

```bash
cargo test serial::read_loop::tests::real_read_source_drains_complete_delimited_lines -- --nocapture
cargo test serial::read_loop::tests::real_read_lines_record_json_response_for_waited_commands -- --nocapture
```

Expected RED reason: FAIL until real read-source/line-drain helpers exist.

**Step 3: Implement minimal GREEN**

- Add a trait/helper for draining complete lines by delimiter.
- Preserve incomplete trailing bytes for the next drain/read iteration.
- Feed complete lines into `protocol::parse_line`.
- Record events through `SerialEventRecorder` so Phase 9 response queues and SSE storage are reused.
- Do not change existing mock read-loop semantics.

**Step 4: Run tests to verify GREEN**

```bash
cargo test serial::read_loop::tests::real_read_source_drains_complete_delimited_lines -- --nocapture
cargo test serial::read_loop::tests::real_read_lines_record_json_response_for_waited_commands -- --nocapture
```

Expected: PASS.

---

### Task 4: Add a stoppable spawned real read loop

**Objective:** Prove a background real-read runner can record inbound events and stop cleanly in tests.

**Files:**

- Modify: `src/serial/read_loop.rs`
- Modify if needed: `src/serial/real_transport.rs`

**Step 1: Write failing async test**

Example shape:

```rust
#[tokio::test]
async fn spawned_real_read_loop_records_lines_and_stops() {
    let manager = InMemoryConnectionManager::default();
    let source = FakeRealSerialReadSource::default();
    let stop = RealReadLoopStop::new();

    let handle = spawn_real_read_loop(
        manager.clone(),
        source.clone(),
        "default".to_string(),
        "\r\n".to_string(),
        stop.clone(),
    );

    source.push_bytes("default", b"{\"reqId\":\"loop-1\",\"ok\":true}\r\n");
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if manager.take_response("default", "loop-1").unwrap().is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();

    stop.stop();
    handle.await.unwrap();
}
```

Use your actual stop-token type. If a stop token is too much for Phase 11, make the loop exit when the source reports disconnected and test that.

**Step 2: Run test to verify RED**

```bash
cargo test serial::read_loop::tests::spawned_real_read_loop_records_lines_and_stops -- --nocapture
```

Expected RED reason: FAIL until a real loop runner and stop path exist.

**Step 3: Implement minimal GREEN**

- Use `tokio::spawn` plus `spawn_blocking` if the read operation can block.
- Sleep/yield between empty reads to avoid busy loops.
- Record read errors as `serial.error` but avoid infinite immediate error spam.
- Ensure the test can stop the loop deterministically.

**Step 4: Run test to verify GREEN**

```bash
cargo test serial::read_loop::tests::spawned_real_read_loop_records_lines_and_stops -- --nocapture
```

Expected: PASS.

---

### Task 5: Wire `serve --real-serial` without changing defaults

**Objective:** Prove CLI/server construction selects the real backend only when requested and rejects incompatible mock/real combinations.

**Files:**

- Modify: `src/main.rs`
- Modify if needed: `src/api/routes.rs`
- Modify if needed: `src/error.rs`

**Step 1: Write failing CLI tests**

Add or adapt tests in `src/main.rs`:

```rust
#[test]
fn serve_cli_accepts_real_serial_flag() {
    let cli = Cli::parse_from(["serialport-api", "serve", "--real-serial"]);

    let Some(Command::Serve(args)) = cli.command else {
        panic!("expected serve command");
    };
    assert!(args.real_serial);
    assert!(!args.mock_device);
    assert!(args.mock_script.is_none());
}

#[test]
fn serve_args_reject_real_serial_with_mock_device() {
    let cli = Cli::parse_from(["serialport-api", "serve", "--real-serial", "--mock-device"]);
    let Some(Command::Serve(args)) = cli.command else {
        panic!("expected serve command");
    };

    let error = validate_serve_args(&args).unwrap_err();

    assert!(error.to_string().contains("--real-serial"));
    assert!(error.to_string().contains("--mock-device"));
}
```

**Step 2: Run tests to verify RED**

```bash
cargo test serve_cli_accepts_real_serial_flag -- --nocapture
cargo test serve_args_reject_real_serial_with_mock_device -- --nocapture
```

Expected RED reason: FAIL until `real_serial` and validation exist.

**Step 3: Implement minimal GREEN**

- Add `real_serial: bool` to `ServeArgs`.
- Add a pure `validate_serve_args` helper called before binding/listening.
- In `serve(args)`, choose:
  - real serial router/manager for `--real-serial`;
  - Phase 10 mock-device router for `--mock-device`/`--mock-script`;
  - existing `routes::router()` for default.
- If read-loop spawning on connection is not yet fully integrated, keep the real transport wiring complete and add a narrow documented limitation. Prefer full integration if feasible within this phase.

**Step 4: Run focused tests to verify GREEN**

```bash
cargo test serve_cli_accepts_real_serial_flag -- --nocapture
cargo test serve_args_reject_real_serial_with_mock_device -- --nocapture
```

Expected: PASS.

---

### Task 6: Preserve default, mock-device, waited-response, and legacy behavior

**Objective:** Ensure real serial support is opt-in and does not regress the existing API.

**Files:**

- Modify tests only if coverage is missing.

**Step 1: Run existing focused tests**

```bash
cargo test api::routes::tests::command_route_queues_payload_for_named_connection -- --nocapture
cargo test api::routes::tests::command_route_times_out_waiting_for_response -- --nocapture
cargo test api::routes::tests::command_route_waits_for_mock_device_response -- --nocapture
cargo test api::routes::tests::commit_alias_queues_payload_for_default_connection -- --nocapture
cargo test serial::manager::tests::default_manager_does_not_auto_record_mock_response -- --nocapture
```

Expected: PASS.

**Step 2: Add regression tests only if needed**

If implementation changes route construction, add tests proving:

- `routes::router()` still uses mock/default behavior.
- `--mock-script` still implies mock-device.
- `/commit` still returns `queued` and never waits.
- Event names remain unchanged.

**Step 3: Run module test suites**

```bash
cargo test serial::real_transport::tests -- --nocapture
cargo test serial::read_loop::tests -- --nocapture
cargo test api::routes::tests -- --nocapture
cargo test --all-features
```

Expected: PASS.

---

## Manual Smoke Test Flow

Manual hardware smoke is optional unless a serial loopback adapter or known microcontroller is attached. Do not fake a hardware smoke result. If no hardware is attached, run only the default/mock smoke flows and state that real hardware smoke was skipped.

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
- Waited command returns `504 Gateway Timeout` on the default server.
- `/commit` returns `queued`.
- Disconnect works.

Stop the server.

### 2. Mock-device regression smoke

Start mock-device server:

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
```

Expected response:

```json
{"status":"ok","reqId":"manual-mock-1","response":{"reqId":"manual-mock-1","ok":true,"data":{"mock":true,"topic":"sensor.read"}}}
```

Stop the server.

### 3. Real serial hardware smoke, only if hardware is available

Prerequisite: an actual serial device or loopback adapter. Substitute the actual port path and baud rate. Examples may be `/dev/ttyUSB0`, `/dev/ttyACM0`, or another OS-visible serial port from `/api/v1/ports`.

Start real server:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -- serve --host 127.0.0.1 --port 4002 --real-serial
```

In another terminal, list ports and connect:

```bash
curl -s http://127.0.0.1:4002/api/v1/ports
curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\r\n"}'
```

For a loopback adapter, send a JSON command with an explicit `reqId` and check events:

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"reqId":"loopback-1","method":"query","topic":"ping","data":{}},"waitForResponse":false}'
curl -i -s --max-time 2 http://127.0.0.1:4002/api/v1/events
```

Expected with a loopback or device echoing JSON lines:

- Connect opens the real serial port or returns a clear serial open error.
- Command write succeeds or returns a clear serial write error.
- Echoed/returned serial lines appear as SSE events.
- If the device emits JSON with a matching `reqId`, waited commands can return `status: ok` through the existing Phase 9 path.

If no hardware is available, explicitly report: "Real serial hardware smoke skipped; no device/loopback adapter available." Automated tests must still pass.

---

## Acceptance Criteria

By the end of Phase 11:

- `src/serial/real_transport.rs` exists and is exported from `src/serial/mod.rs`.
- A real serial transport can open, write, flush, and close named serial ports through the existing `SerialTransport` trait.
- Real transport unit tests use fake handles/factories and do not require hardware.
- Real read-loop/read-source logic can read delimiter-terminated lines, parse them through `protocol::parse_line`, and record them through `record_event_for_connection` or the `SerialEventRecorder` trait.
- Inbound real JSON with string `reqId` reuses Phase 9 response queues, enabling waited command success without route-specific matching code.
- `serve --real-serial` opts into real serial mode.
- Default `serve` remains mock-backed and hardware-free.
- `serve --mock-device` and `serve --mock-script <PATH>` keep Phase 10 behavior.
- Incompatible `--real-serial` + mock flags are rejected clearly.
- Fire-and-forget command responses remain exactly `{"status":"queued","reqId":"..."}`.
- `/commit` remains fire-and-forget.
- Legacy aliases still pass tests.
- SSE event names and data shapes remain unchanged.
- No SQLite/config-discovery/WebSocket/Socket.IO/packaging/authentication changes are added.
- `cargo fmt --check` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `cargo test --all-features` passes.
- Manual smoke proves default and mock-device regressions. Real hardware smoke is run only if hardware is available and is honestly reported.

Likely implementation commits:

```text
feat: add real serial transport
feat: wire opt-in real serial server mode
```

If the implementation is small enough, a single commit is acceptable:

```text
feat: add opt-in real serial lifecycle
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

Then manually smoke the server as described above. Stop any server process before committing.

After final implementation commit(s):

```bash
git status --short --branch
git log --oneline -10
```

Expected final status:

- Branch is `rewrite/axum-serial-api`.
- Working tree is clean.
- Latest commit(s) are Phase 11 real serial lifecycle commits.

---

## Copy/Paste Prompt for the Next Coding Session

```text
We are in /home/alfarie/repos/serialport-api on branch rewrite/axum-serial-api. Please execute docs/phase-11-real-serial-lifecycle-handoff.md.

Load the writing-plans, test-driven-development, and rust-axum-api-tdd skills before editing. This phase adds an opt-in real serial lifecycle after approved Phase 10 mock-device scripted responses. First verify baseline with:

export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features

Use TDD. Add a hardware-free-tested real serial transport, likely src/serial/real_transport.rs, that implements the existing SerialTransport trait by opening named OS serial ports with the serialport crate, writing framed bytes, flushing, and closing/dropping handles on disconnect. Add a fake handle/factory seam so automated tests never require /dev/ttyUSB0 or physical hardware. Add real read-loop/read-source wiring that reads delimiter-terminated lines from real serial handles, parses with protocol::parse_line, and records through record_event_for_connection/SerialEventRecorder so existing SSE storage and Phase 9 reqId response queues are reused. Add an explicit serve --real-serial CLI flag and keep default serve mock-backed. Preserve Phase 10 serve --mock-device and --mock-script behavior; reject --real-serial combined with mock flags clearly. Preserve all route response shapes, /commit fire-and-forget behavior, generated reqId sequencing, legacy aliases, and SSE event names/shapes. Do not add SQLite, broad config files, WebSocket/Socket.IO, Docker/systemd, authentication, hardware-dependent tests, or broad API error refactors.

After implementation, run cargo fmt --check, cargo clippy --all-targets --all-features -- -D warnings, and cargo test --all-features. Manually smoke the documented default server and mock-device regression flows. Run real serial hardware smoke only if a real device or loopback adapter is available; otherwise explicitly report it was skipped. Stop the server and commit focused Phase 11 changes with either:

feat: add real serial transport
feat: wire opt-in real serial server mode

or one small combined commit:

feat: add opt-in real serial lifecycle
```
