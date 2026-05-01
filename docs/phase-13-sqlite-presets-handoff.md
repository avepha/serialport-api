# Phase 13 SQLite Saved Presets Handoff

> **For Hermes / next AI coding session:** Execute this in a fresh session. Load `writing-plans`, `test-driven-development`, and `rust-axum-api-tdd` before editing. This phase should add a narrow saved-command preset API with optional SQLite persistence. Keep the work TDD-first, hardware-free, and self-contained. Do not add Raspberry Pi packaging, Docker/release automation, WebSocket/Socket.IO, authentication, broad API error-envelope refactors, auto-connect behavior, or route-shape changes outside the new presets endpoints.

**Goal:** Add saved JSON command presets so users can store, list, retrieve, update, and delete reusable command payloads locally. The live server should remain hardware-free by default, but presets should be persistable through a SQLite database when explicitly configured. The feature should be independent of serial transports: saving a preset must not open ports, send commands, or require connected hardware.

**Inferred next phase:** Phase 13 is **SQLite saved presets**. This follows Phase 12 because the current README lists SQLite saved presets / persistent saved profiles as the next incomplete item, and `docs/open-source-spec.md` defines a `Presets / Saved Commands` section with `GET /api/v1/presets`, `POST /api/v1/presets`, and `DELETE /api/v1/presets/{id}`. Phase 12 completed config-file defaults, leaving persistence as the next major v1 gap before Raspberry Pi packaging/release polish.

**Architecture:** Add a small storage boundary that is separate from serial connection management and route handlers. Routes should call a `PresetStore` trait through Axum app state. Provide an in-memory store for tests/default operation and a SQLite-backed store for opt-in persistence. Keep serial managers unaware of presets.

**Tech Stack:** Rust 2021, Axum 0.7, Tokio 1, Serde/Serde JSON, Thiserror, Clap, TOML config already present, and a small SQLite dependency. Prefer `rusqlite` for a synchronous, low-scope local database unless the implementer has a strong reason to choose `sqlx`; if using `rusqlite`, keep blocking DB work inside the store and guard shared connections with a mutex. Automated tests must remain hardware-free and should use temporary DB files under `std::env::temp_dir()`.

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

Latest known commits:

```text
2ed803c feat: add config file defaults
8204c5e docs: add phase 12 config handoff
9ea7d2b docs: refresh README roadmap
07bf24c docs: update README progress
7822584 fix: coordinate real serial read loops
a07a7ac feat: add opt-in real serial lifecycle
f627a21 docs: add phase 11 real serial handoff
```

Phase 12 review status:

- Independent review verdict: **APPROVED**.
- No blocking issues.
- Current pushed remote `master` is expected at `2ed803c feat: add config file defaults`.

Completed functionality as of this handoff:

- Axum server starts with `cargo run -- serve --host 127.0.0.1 --port 4002`.
- Defaults remain `127.0.0.1:4002`, mock/in-memory, and hardware-free.
- Optional TOML config support exists via `serve --config <PATH>`.
- Auto-discovery loads `./serialport-api.toml` when present; missing discovered config is non-fatal.
- Explicit missing/unreadable/invalid config fails clearly.
- Config precedence is CLI explicit values > environment variables > config file > built-in defaults.
- Port listing uses `serialport::available_ports()`.
- Named connection lifecycle exists for canonical routes and legacy aliases.
- Commands generate or preserve string `reqId`, frame JSON with the connection delimiter, and write through the active serial transport.
- Waited command responses match inbound JSON by connection name and string `reqId`.
- SSE events include `serial.json`, `serial.text`, `serial.log`, `serial.notification`, and `serial.error`.
- Opt-in `--mock-device` and `--mock-script <PATH>` can synthesize hardware-free responses.
- Opt-in `--real-serial` can open/write/read OS serial ports.
- `--real-serial` is rejected with mock-device/mock-script after config/env/CLI resolution.

Important local toolchain note:

```bash
# In this WSL environment, prefer rustup's toolchain first in PATH.
# Otherwise /usr/bin rustc/cargo can mix with rustup cargo-clippy and cause metadata errors.
export PATH="$HOME/.cargo/bin:$PATH"
```

Baseline verification before starting implementation:

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

## Phase 13 Scope

Do in Phase 13:

- Add saved command preset models and storage boundary.
- Add an in-memory preset store for default/test operation.
- Add a SQLite preset store for opt-in persistent storage.
- Add REST endpoints under `/api/v1/presets`:
  - `GET /api/v1/presets` lists presets.
  - `POST /api/v1/presets` creates a preset.
  - `GET /api/v1/presets/:id` returns one preset.
  - `PUT /api/v1/presets/:id` updates an existing preset.
  - `DELETE /api/v1/presets/:id` deletes an existing preset.
- Use a minimal preset shape compatible with the open-source spec:

```json
{
  "id": 1,
  "name": "Read IMU",
  "payload": {
    "method": "query",
    "topic": "imu.read",
    "data": {}
  }
}
```

- Validate that `name` is non-empty after trimming.
- Validate that `payload` is a JSON object. It may contain any command fields, including an optional `reqId`.
- Return deterministic HTTP status codes:
  - `200 OK` for list/get/update/delete success unless choosing `204 No Content` for delete; pick one and document it.
  - `201 Created` for create success.
  - `400 Bad Request` for invalid name/payload.
  - `404 Not Found` for missing preset id.
  - `500 Internal Server Error` for unexpected storage failures.
- Add CLI/config wiring for SQLite opt-in. Recommended narrow behavior:
  - Add `serve --preset-db <PATH>` to use SQLite persistence.
  - Add optional config `[storage] preset_db = "./serialport-api.db"` or `[storage] path = "./serialport-api.db"`; choose one and document it.
  - Precedence should mirror Phase 12: CLI explicit `--preset-db` > config file > built-in default of in-memory presets.
- Keep default `cargo run -- serve` using in-memory presets only, so no database file is created unless the user opts in.
- Update README with a concise saved-presets section, endpoint examples, and SQLite opt-in usage.
- Add tests for storage, route behavior, validation, and SQLite persistence across store reopen.

Do **not** do in Phase 13:

- Do not make preset creation send a command to serial.
- Do not require serial hardware or real serial mode for any test.
- Do not change existing health, ports, connections, command, SSE, or legacy alias response shapes.
- Do not add `/commit` behavior that implicitly reads presets.
- Do not implement browser UI, Socket.IO, WebSocket, authentication, multi-user ownership, cloud sync, tags/search/pagination, or import/export.
- Do not add Raspberry Pi systemd docs, Docker images, release binaries, or CI release automation.
- Do not broaden the global API error envelope across existing routes. It is acceptable to add small local error mapping for the new preset routes.
- Do not make SQLite mandatory for default server startup.

If a tempting change involves durable connection profiles, auto-connect, migrations beyond one simple table, user accounts, or route compatibility rewrites, leave it for a later phase.

---

## Expected Files to Modify or Create

Expected implementation changes:

- Create: `src/storage/mod.rs`
  - Define `Preset`, request/response-friendly model structs if not placed in `api/routes.rs`.
  - Define a `PresetStore` trait or equivalent storage interface.
  - Define `InMemoryPresetStore`.
  - Define shared validation helpers if useful.
- Create: `src/storage/sqlite.rs`
  - Define `SqlitePresetStore`.
  - Create the presets table if it does not exist.
  - Store `payload` as JSON text after validating it is a JSON object.
  - Add unit tests with temporary DB paths.
- Modify: `src/lib.rs`
  - Export `pub mod storage;`.
- Modify: `Cargo.toml`
  - Add SQLite dependency, recommended:

```toml
rusqlite = { version = "0.31", features = ["bundled"] }
```

  - If using `sqlx` instead, keep the feature set minimal and document why.
- Modify: `src/api/routes.rs`
  - Extend `AppState` to include a preset store while preserving existing route constructors.
  - Add preset route handlers and route tests.
  - Keep existing route tests passing.
- Modify: `src/config.rs`
  - Add optional storage config and resolved serve setting for preset DB path.
  - Preserve existing Phase 12 precedence and tests.
- Modify: `src/main.rs`
  - Add `serve --preset-db <PATH>`.
  - Instantiate `SqlitePresetStore` only when a DB path is resolved.
  - Otherwise instantiate `InMemoryPresetStore`.
- Modify: `src/error.rs`
  - Add narrow storage/preset validation variants if helpful.
- Modify: `README.md`
  - Document saved-presets endpoints and SQLite opt-in usage.

Optional if useful:

- Create: `docs/presets-api.md` only if README would become too long. Keep README concise either way.
- Create: `examples/presets.http` only if it helps manual smoke and stays small.

---

## Current Code to Understand First

Read these files before editing:

```bash
cd /home/alfarie/repos/serialport-api
sed -n '1,260p' src/api/routes.rs
sed -n '260,760p' src/api/routes.rs
sed -n '760,1260p' src/api/routes.rs
sed -n '1,260p' src/config.rs
sed -n '260,520p' src/config.rs
sed -n '1,280p' src/main.rs
sed -n '1,120p' src/lib.rs
sed -n '1,160p' src/error.rs
sed -n '1,120p' Cargo.toml
sed -n '1,520p' README.md
```

Key current facts:

- `routes::AppState<L, C>` currently holds `port_lister` and `connection_manager`.
- `routes::router()` uses `SystemPortLister` and `InMemoryConnectionManager::default()`.
- `routes::router_with_state(...)` is generic over `SerialPortLister` and `ConnectionManager`.
- Existing route tests live in `src/api/routes.rs`; follow their style for Axum request/response tests.
- `config.rs` already owns `FileConfig`, `ResolvedServeConfig`, discovery, env parsing, and CLI/env/config/default precedence.
- `main.rs` already resolves config before binding, then chooses default/mock-device/real-serial router setup.

Recommended design adjustment:

- Extend app state to `AppState<L, C, P>` where `P: PresetStore`.
- Provide type-friendly constructors so existing callers remain simple:
  - `router()` can use an in-memory preset store.
  - `router_with_state(AppState::new(port_lister, connection_manager, preset_store))` can support tests and main wiring.
- If generic arity creates too much churn, use `Arc<dyn PresetStore>` for the preset store. Keep the trait object `Send + Sync + Clone`-friendly through `Arc`.

---

## Suggested API Contract

### `GET /api/v1/presets`

Response:

```json
{
  "presets": [
    {
      "id": 1,
      "name": "Read IMU",
      "payload": {"method":"query","topic":"imu.read","data":{}}
    }
  ]
}
```

### `POST /api/v1/presets`

Request:

```json
{
  "name": "Read IMU",
  "payload": {"method":"query","topic":"imu.read","data":{}}
}
```

Response status: `201 Created`

Response body:

```json
{
  "preset": {
    "id": 1,
    "name": "Read IMU",
    "payload": {"method":"query","topic":"imu.read","data":{}}
  }
}
```

### `GET /api/v1/presets/:id`

Response status: `200 OK` when found, `404 Not Found` when missing.

### `PUT /api/v1/presets/:id`

Request body should match create request. Response status: `200 OK` when updated, `404 Not Found` when missing.

### `DELETE /api/v1/presets/:id`

Recommended response status: `200 OK` with body:

```json
{"status":"deleted","id":1}
```

This mirrors the existing disconnect route style better than `204 No Content`. If the implementer chooses `204`, update tests and README consistently.

---

## Bite-Sized TDD Tasks

### Task 13.1: Add preset domain model and in-memory store

RED test first:

- Add tests for an `InMemoryPresetStore` proving:
  - creating a preset assigns id `1`.
  - listing returns created presets in id order.
  - invalid empty/whitespace-only name is rejected.
  - non-object JSON payload is rejected.
  - get/update/delete missing id returns a not-found error.

Command:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
cargo test storage:: --all-features
```

Expected RED reason:

- `src/storage` and `PresetStore` do not exist.

GREEN implementation:

- Add `src/storage/mod.rs`.
- Export `pub mod storage;` from `src/lib.rs`.
- Implement `Preset`, `CreatePreset`, `PresetStore`, `InMemoryPresetStore`, and narrow `PresetError`/`StorageError` types.
- Keep payload as `serde_json::Value` and enforce `Value::Object`.

Expected GREEN:

- Storage unit tests pass without SQLite or hardware.

### Task 13.2: Add preset routes using in-memory store

RED test first:

- Add Axum route tests for:
  - `GET /api/v1/presets` returns `{"presets":[]}` on a fresh app.
  - `POST /api/v1/presets` with valid object payload returns `201` and a preset with id `1`.
  - `GET /api/v1/presets/1` returns the created preset.
  - `PUT /api/v1/presets/1` updates name/payload.
  - `DELETE /api/v1/presets/1` deletes it.
  - invalid payload array/string returns `400`.
  - missing id returns `404`.

Command:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test api::routes::tests::preset --all-features -- --nocapture
```

Expected RED reason:

- Preset routes are not registered and app state has no preset store.

GREEN implementation:

- Extend `AppState` with preset store support.
- Register `/api/v1/presets` and `/api/v1/presets/:id` routes.
- Add handlers and response structs.
- Map validation and not-found errors to `400`/`404`.
- Preserve all existing route constructors and tests.

Expected GREEN:

- New route tests pass.
- Existing health/ports/connection/command/SSE/legacy tests still pass.

### Task 13.3: Add SQLite preset store

RED test first:

- Add tests using a temp file path proving:
  - `SqlitePresetStore::open(path)` creates schema.
  - created presets survive dropping/reopening the store.
  - update/delete persist across reopen.
  - invalid payload/name validation is still enforced.

Command:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test storage::sqlite:: --all-features -- --nocapture
```

Expected RED reason:

- No SQLite dependency or store exists.

GREEN implementation:

- Add the SQLite dependency to `Cargo.toml`.
- Create `src/storage/sqlite.rs`.
- Use a simple schema, for example:

```sql
CREATE TABLE IF NOT EXISTS presets (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

- Store canonical JSON text from `serde_json::to_string(&payload)`.
- Parse JSON text back to `serde_json::Value` on reads.
- Keep DB tests deterministic and clean up temp files if practical.

Expected GREEN:

- SQLite store tests pass.

### Task 13.4: Wire SQLite opt-in through config and CLI

RED test first:

- Add config resolver tests proving:
  - no storage config and no CLI flag resolves to in-memory presets.
  - config storage path resolves to SQLite path.
  - CLI `--preset-db` overrides config storage path.
- Add CLI parse test for `serve --preset-db ./serialport-api.db`.

Command:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test config:: --all-features
cargo test --all-features serve_cli_accepts_preset_db -- --nocapture
```

Expected RED reason:

- Config and CLI do not know about preset database paths.

GREEN implementation:

- Add a narrow storage config section to `FileConfig`.
- Add `preset_db: Option<PathBuf>` or equivalent to resolved serve config.
- Add `--preset-db <PATH>` to `ServeArgs`.
- Preserve all existing Phase 12 config behavior and tests.
- In `main.rs`, instantiate SQLite store only when a path is present; otherwise use in-memory.

Expected GREEN:

- Config and CLI tests pass.
- Server default still does not create a DB file.

### Task 13.5: README update

RED check first:

```bash
cd /home/alfarie/repos/serialport-api
rg -n "preset|SQLite|preset-db|/api/v1/presets" README.md
```

Expected RED reason:

- README currently lists SQLite saved presets as planned/not complete and lacks endpoint examples.

GREEN implementation:

- Move saved presets from planned to implemented if complete.
- Add a concise `Saved presets` section with endpoint examples.
- Document default in-memory behavior and SQLite opt-in with `--preset-db` and/or config.
- Keep Raspberry Pi packaging and release binaries listed as later work.

Expected GREEN:

- README accurately documents the implemented scope.

---

## Verification Commands

Run these before committing Phase 13 implementation:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
git diff --check
```

Expected:

- Branch remains `rewrite/axum-serial-api`.
- Formatting, clippy, and tests pass.
- `git diff --check` has no whitespace errors.
- No test requires real serial hardware.
- Default server still starts without creating a SQLite file.

---

## Manual Smoke Test Flow

Use a non-conflicting port. These checks should not require hardware.

### 1. Default in-memory presets

Start server:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -- serve --host 127.0.0.1 --port 4013
```

In another terminal:

```bash
curl -s http://127.0.0.1:4013/api/v1/health
curl -s http://127.0.0.1:4013/api/v1/presets
curl -i -s -X POST http://127.0.0.1:4013/api/v1/presets \
  -H 'content-type: application/json' \
  -d '{"name":"Read IMU","payload":{"method":"query","topic":"imu.read","data":{}}}'
curl -s http://127.0.0.1:4013/api/v1/presets/1
curl -s -X PUT http://127.0.0.1:4013/api/v1/presets/1 \
  -H 'content-type: application/json' \
  -d '{"name":"Read temperature","payload":{"method":"query","topic":"temperature.read","data":{}}}'
curl -s -X DELETE http://127.0.0.1:4013/api/v1/presets/1
curl -i -s http://127.0.0.1:4013/api/v1/presets/1
```

Expected:

- Health returns `{"status":"ok","version":"0.1.0"}`.
- Fresh list returns `{"presets":[]}`.
- Create returns `201` and id `1`.
- Get returns the created preset.
- Update returns the updated preset.
- Delete returns success.
- Get after delete returns `404`.

### 2. SQLite persistence

Start with a temporary DB:

```bash
rm -f /tmp/serialport-api-phase13.db
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -- serve --host 127.0.0.1 --port 4014 --preset-db /tmp/serialport-api-phase13.db
```

Create a preset:

```bash
curl -s -X POST http://127.0.0.1:4014/api/v1/presets \
  -H 'content-type: application/json' \
  -d '{"name":"Persistent ping","payload":{"method":"query","topic":"ping","data":{}}}'
```

Stop and restart the server with the same `--preset-db`, then run:

```bash
curl -s http://127.0.0.1:4014/api/v1/presets
```

Expected:

- The created preset is still present after restart.
- No serial hardware is opened.

### 3. Invalid preset validation

```bash
curl -i -s -X POST http://127.0.0.1:4014/api/v1/presets \
  -H 'content-type: application/json' \
  -d '{"name":"","payload":{"method":"query"}}'

curl -i -s -X POST http://127.0.0.1:4014/api/v1/presets \
  -H 'content-type: application/json' \
  -d '{"name":"Bad payload","payload":["not","object"]}'
```

Expected:

- Both return `400 Bad Request` with a clear error body.

---

## Expected Commit Message

Use one implementation commit unless the README update is intentionally separated:

```text
feat: add saved command presets
```

Optional second commit if documentation is separate:

```text
docs: document saved command presets
```

---

## Copy/Paste Implementation Prompt for Next Subagent

```text
You are implementing Phase 13 for /home/alfarie/repos/serialport-api on branch rewrite/axum-serial-api.

Read docs/phase-13-sqlite-presets-handoff.md first. Implement saved command presets for the Rust Axum serialport-api rewrite. Keep the scope narrow, TDD-first, and hardware-free.

Current baseline: Phase 12 config-file defaults are implemented and approved. Latest commits are 2ed803c feat: add config file defaults and 8204c5e docs: add phase 12 config handoff. Current pushed remote master is expected at 2ed803c.

Goal: Add /api/v1/presets CRUD for reusable JSON command payloads, backed by an in-memory store by default and optional SQLite persistence when explicitly configured with a CLI/config database path. Saving presets must not open serial ports or send commands. Preserve all existing routes, response shapes, mock/default behavior, real-serial opt-in behavior, and Phase 12 config precedence.

Expected files: create src/storage/mod.rs and src/storage/sqlite.rs; modify src/lib.rs, Cargo.toml, src/api/routes.rs, src/config.rs, src/main.rs, possibly src/error.rs, and README.md. Do not add Raspberry Pi packaging, Docker/release automation, WebSocket/Socket.IO, authentication, broad API error-envelope refactors, auto-connect behavior, UI work, or hardware-dependent tests.

Use PATH="$HOME/.cargo/bin:$PATH" for Rust commands. Start with baseline verification:
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features

Then follow the bite-sized TDD tasks in the handoff. Before committing, run:
export PATH="$HOME/.cargo/bin:$PATH"
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
git diff --check

Commit as: feat: add saved command presets

After independent approval, the orchestrator will push the approved completed cycle to origin/master.
```

---

## Orchestrator Note

After Phase 13 implementation is complete and independently approved, the orchestrator is expected to push the approved completed cycle to `origin/master`.
