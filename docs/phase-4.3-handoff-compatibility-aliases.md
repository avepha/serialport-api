# Phase 4.3 Compatibility Aliases Handoff

> **For Hermes:** Execute this in a fresh session. Load `test-driven-development` before coding. Follow RED-GREEN-REFACTOR: write each route test first, run it and confirm it fails for the expected reason, then implement the minimum code. If you revise this plan, also load `writing-plans`.

**Goal:** Add legacy HTTP compatibility aliases for the already-implemented in-memory connection lifecycle and port listing behavior.

**Architecture:** Keep alias routes in `src/api/routes.rs` only. They should reuse the same `AppState` and in-memory `ConnectionManager` behavior as the canonical `/api/v1/...` routes. Do not duplicate serial domain logic and do not open physical serial ports.

**Tech Stack:** Rust 2021, Axum 0.7, Tokio 1, Serde, existing `serial::manager` module.

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

Relevant completed work:

- Phase 3: Axum `GET /api/v1/health` and `serve` CLI.
- Phase 4.1: `GET /api/v1/ports` with mockable serial port lister.
- Phase 4.2: in-memory connection lifecycle:
  - `POST /api/v1/connections`
  - `GET /api/v1/connections`
  - `DELETE /api/v1/connections/:name`

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
- Existing canonical endpoints still work:
  - `GET /api/v1/health`
  - `GET /api/v1/ports`
  - `POST /api/v1/connections`
  - `GET /api/v1/connections`
  - `DELETE /api/v1/connections/:name`
- Legacy compatibility aliases work against the same in-memory state:

```text
GET  /list
GET  /info
POST /connect
POST /disconnect
```

Suggested manual flow:

```bash
cargo run -- serve --host 127.0.0.1 --port 4002
curl -s http://127.0.0.1:4002/list
curl -s -X POST http://127.0.0.1:4002/connect \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\\r\\n"}'
curl -s http://127.0.0.1:4002/info
curl -s -X POST http://127.0.0.1:4002/disconnect \
  -H 'content-type: application/json' \
  -d '{"name":"default"}'
```

Expected response shapes:

```json
{"ports":[]}
```

```json
{"status":"connected","connection":{"name":"default","status":"connected","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\r\n"}}
```

```json
{"connections":[{"name":"default","status":"connected","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\r\n"}]}
```

```json
{"status":"disconnected","name":"default"}
```

Suggested commit message:

```text
feat: add legacy connection aliases
```

---

## Task 1: Add failing route tests for aliases

**Objective:** Define legacy alias behavior before implementation.

**Files:**

- Modify: `src/api/routes.rs`

**Step 1: Write failing tests**

Add route tests for:

- `GET /list` returns the same shape as `GET /api/v1/ports`.
- `POST /connect` creates a connection using the same request body as `POST /api/v1/connections`.
- `GET /info` lists connections using the same shape as `GET /api/v1/connections`.
- `POST /disconnect` removes a connection by JSON body: `{"name":"default"}`.

Keep the tests in `src/api/routes.rs` and use `router_with_state(AppState { ... })` with `InMemoryConnectionManager::default()`.

**Step 2: Run the alias route test and confirm RED**

Run:

```bash
cargo test api::routes::tests::legacy_alias_routes_share_connection_state -- --nocapture
```

Expected: FAIL because alias routes are not implemented yet, likely returning `404 Not Found`.

---

## Task 2: Implement minimal alias routes

**Objective:** Make alias tests pass by wiring aliases to existing handlers/state.

**Files:**

- Modify: `src/api/routes.rs`

Implementation notes:

- `/list` can reuse the same `ports::<L, C>` handler.
- `/info` can reuse the same `connections::<L, C>` handler.
- `/connect` can reuse the same `connect::<L, C>` handler.
- `/disconnect` needs a small request type, for example:

```rust
#[derive(Debug, Deserialize)]
struct DisconnectRequest {
    name: String,
}
```

and a handler that calls `state.connection_manager.disconnect(&request.name)`.

Because this project currently uses Axum 0.7, keep canonical path captures as `:name`, not `{name}`.

**Step 2: Run the alias route test and confirm GREEN**

Run:

```bash
cargo test api::routes::tests::legacy_alias_routes_share_connection_state -- --nocapture
```

Expected: PASS.

---

## Task 3: Full verification and manual smoke test

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
curl -s http://127.0.0.1:4002/list
curl -s -X POST http://127.0.0.1:4002/connect \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ttyUSB0","baudRate":115200,"delimiter":"\\r\\n"}'
curl -s http://127.0.0.1:4002/info
curl -s -X POST http://127.0.0.1:4002/disconnect \
  -H 'content-type: application/json' \
  -d '{"name":"default"}'
```

Stop the server afterward.

---

## Task 4: Commit

```bash
git add src/api/routes.rs docs/phase-4.3-handoff-compatibility-aliases.md
git commit -m "feat: add legacy connection aliases"
```

If you keep docs separate, use:

```bash
git commit -m "docs: add phase 4.3 handoff plan"
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

Important: avoid context pollution from previous implementation sessions. Start by reading the handoff document and then execute Phase 4.3 exactly from it.

Read this file first:

docs/phase-4.3-handoff-compatibility-aliases.md

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

- Phase 4.2 is done:
  - in-memory connection lifecycle manager added
  - POST /api/v1/connections implemented
  - GET /api/v1/connections implemented
  - DELETE /api/v1/connections/:name implemented
  - manual curl verification passes

Your task is to execute Phase 4.3: Legacy Compatibility Aliases.

Goal:

Add legacy HTTP compatibility aliases backed by the same in-memory state:

- GET /list
- GET /info
- POST /connect
- POST /disconnect

Acceptance criteria:

- Use test-driven development.
- Write failing alias route tests first.
- Run them and confirm they fail for the expected reason.
- Implement only minimal alias route wiring.
- Keep HTTP layer separate from serial domain logic.
- Do not implement serial hardware behavior yet.
- Do not implement command sending, SSE, Socket.IO compatibility, SQLite, or Phase 5.
- cargo fmt --check passes.
- cargo check passes.
- cargo test passes.
- cargo run -- serve --host 127.0.0.1 --port 4002 starts an HTTP server.
- Existing canonical endpoints still work.
- New manual alias flow works:
  - GET /list
  - POST /connect
  - GET /info
  - POST /disconnect

Expected files to modify:

- Modify: src/api/routes.rs

Expected commit message:

feat: add legacy connection aliases

Before starting, verify the baseline:

cd /home/alfarie/repos/serialport-api
git status --short --branch
cargo fmt --check
cargo check
cargo test

Then follow docs/phase-4.3-handoff-compatibility-aliases.md task by task.

When finished, report:

- files changed
- tests run
- manual curl results
- commit hash
- current git status
```
