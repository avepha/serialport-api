# Phase 6.1 README Rewrite Handoff

> **For Hermes / next AI coding session:** Execute this in a fresh session. Load `writing-plans` and `rust-axum-api-tdd` before editing. This phase is documentation-only: rewrite `README.md` to accurately describe the current Axum/Rust rewrite and the implemented mock API surface. Do not implement new runtime features while doing this handoff.

**Goal:** Replace the placeholder README with a useful open-source README that explains what `serialport-api` is, how to run it, which endpoints currently work, how the legacy compatibility aliases map to the new API, and what remains on the roadmap.

**Architecture:** Document the project as it exists now: a Rust 2021 library plus Axum CLI/server binary, with protocol parsing, in-memory/mock connection state, mock command framing, and a mock-backed SSE event endpoint. Be explicit that physical serial I/O, background serial read loops, waited command responses, SQLite presets, CI, and Raspberry Pi packaging are not implemented yet.

**Tech Stack:** Rust 2021, Axum 0.7, Tokio 1, Clap 4, Serde/Serde JSON, Thiserror, Tracing, `serialport`, `tokio-stream`, Server-Sent Events.

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
6109494 feat: add serial event stream endpoint
abb9a8c feat: add mock command endpoint
a61cab7 docs: add phase 5.1 command endpoint handoff
34d7aec feat: add legacy connection aliases
8f66424 feat: add mock connection lifecycle endpoints
6edb02d feat: add serial port listing endpoint
cd16800 feat: add axum health endpoint
6b3a2f9 feat: add serial protocol parsing foundation
```

Completed functionality as of this handoff:

- Rust 2021 crate metadata is present in `Cargo.toml`.
- Library/binary split exists:
  - `src/lib.rs`
  - `src/main.rs`
  - `src/protocol.rs`
  - `src/error.rs`
  - `src/api/routes.rs`
  - `src/serial/manager.rs`
- `cargo run -- serve --host 127.0.0.1 --port 4002` starts the Axum server.
- Protocol helpers exist for JSON + `\r\n` framing and inbound line parsing.
- Mock/in-memory API support exists for ports, connections, commands, and events.
- Latest full test suite before this handoff passed with 19 tests.

Baseline verification before starting:

```bash
cd /home/alfarie/repos/serialport-api
git status --short --branch
cargo fmt --check
cargo check
cargo test
```

Expected:

- Branch is `rewrite/axum-serial-api`.
- Working tree is clean except possibly this handoff doc commit if already applied.
- All cargo commands pass.

---

## Do Not Do in Phase 6.1

This phase is a README rewrite only. Do **not** add or modify runtime behavior.

Do **not** add:

- real physical serial port opening/writing beyond what currently exists
- `tokio-serial` or async serial transport
- background serial read loops
- command `waitForResponse` behavior
- request/response matching by `reqId`
- SQLite / preset storage
- config file loading
- GitHub Actions CI
- Docker/systemd packaging
- WebSocket or Socket.IO support
- hardware-dependent tests
- API behavior changes

If the README needs to mention those items, label them clearly as **planned** or **not implemented yet**.

---

## README Requirements

Rewrite `README.md` from the current placeholder into a polished project README.

The README should include these sections, in this order unless there is a strong reason to reorder:

1. `# serialport-api`
2. One-paragraph description
3. `## Status`
4. `## Features`
5. `## Install / build`
6. `## Run the server`
7. `## Quick start`
8. `## HTTP API`
9. `## Legacy compatibility aliases`
10. `## Serial protocol`
11. `## Development`
12. `## Roadmap`
13. `## License`

### Tone

Use clear open-source documentation style:

- concise, practical, and honest
- avoid overclaiming production readiness
- call the current server mock-backed/in-memory where relevant
- include copy-pasteable commands
- prefer bullets and code blocks over prose walls

### Current README to replace

The current `README.md` is only:

```markdown
## Serialport communication module in rust

### TODO
- [ ] Provide universal json-based serialport API.
- [ ] MQTT over websocket.
- [ ] Suport multiple serialport.
- [ ] GraphQL API.
```

Replace it entirely.

---

## Accurate Project Description

Suggested opening paragraph:

```markdown
`serialport-api` is a Rust service for JSON-based serial-port communication with microcontrollers, robots, and Raspberry Pi deployments. It exposes an HTTP API for listing ports, managing named connections, sending JSON commands, and streaming serial events with Server-Sent Events.
```

Immediately follow with a status warning similar to:

```markdown
> Status: rewrite in progress. The current API server, connection manager, command queue, and event stream are mock/in-memory foundations. Physical serial read/write loops, waited command responses, preset storage, and packaging are planned but not complete yet.
```

Keep this status warning prominent so future users do not expect production serial hardware support before it exists.

---

## Feature List to Document

Use checkboxes so readers can see current vs planned scope.

Currently implemented:

- [x] Rust 2021 project with library + CLI/server binary
- [x] Axum HTTP server via `cargo run -- serve`
- [x] Health endpoint
- [x] Serial port listing endpoint backed by `serialport::available_ports()`
- [x] Mock/in-memory named connection lifecycle
- [x] Mock command endpoint with generated/preserved `reqId`
- [x] JSON command framing as JSON + `\r\n`
- [x] Server-Sent Events endpoint for recorded mock serial events
- [x] Legacy aliases: `/list`, `/connect`, `/disconnect`, `/info`, `/commit`
- [x] Unit/route tests for current behavior

Planned / not complete yet:

- [ ] Real serial transport read/write loops
- [ ] Matching command responses by `reqId`
- [ ] Command timeout handling
- [ ] Config file support
- [ ] Mock serial device mode / scripted responses
- [ ] SQLite saved presets
- [ ] Raspberry Pi install guide and systemd service
- [ ] GitHub Actions CI
- [ ] Release binaries / Docker image

---

## Commands to Include

### Build

```bash
cargo build
```

### Run tests

```bash
cargo fmt --check
cargo check
cargo test
```

### Start server

```bash
cargo run -- serve --host 127.0.0.1 --port 4002
```

Mention that port `4002` is chosen for compatibility with the older JavaScript service.

---

## API Reference to Include

Document both canonical `/api/v1` endpoints and legacy aliases. Keep examples current with actual implemented behavior.

### Health

```bash
curl -s http://127.0.0.1:4002/api/v1/health
```

Expected response:

```json
{"status":"ok","version":"0.1.0"}
```

### List ports

Canonical:

```bash
curl -s http://127.0.0.1:4002/api/v1/ports
```

Legacy:

```bash
curl -s http://127.0.0.1:4002/list
```

Example response shape:

```json
{"ports":[]}
```

Do not promise that a specific port exists.

### Connect

Canonical:

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}'
```

Legacy:

```bash
curl -s -X POST http://127.0.0.1:4002/connect \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}'
```

Example response:

```json
{"status":"connected","connection":{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n","status":"connected"}}
```

Before committing README, verify the exact serialized field set from the current code or use language like "similar to" if the exact shape differs.

### List connections

Canonical:

```bash
curl -s http://127.0.0.1:4002/api/v1/connections
```

Legacy:

```bash
curl -s http://127.0.0.1:4002/info
```

Example response shape:

```json
{"connections":[{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n","status":"connected"}]}
```

Again, verify exact shape while editing.

### Send command

Canonical:

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"method":"query","topic":"sensor.read","data":{}},"waitForResponse":false}'
```

Expected current behavior:

```json
{"status":"queued","reqId":"1"}
```

Legacy `/commit` sends to the `default` connection:

```bash
curl -s -X POST http://127.0.0.1:4002/commit \
  -H 'content-type: application/json' \
  -d '{"reqId":"client-42","method":"query","topic":"sensor.read","data":{}}'
```

Expected current behavior:

```json
{"status":"queued","reqId":"client-42"}
```

Important note to include:

- Current command handling records/framing in memory and returns `queued`.
- `waitForResponse` and `timeoutMs` are accepted in the request shape but waited responses are not implemented yet.

### Disconnect

Canonical:

```bash
curl -s -X DELETE http://127.0.0.1:4002/api/v1/connections/default
```

Legacy:

```bash
curl -s -X POST http://127.0.0.1:4002/disconnect \
  -H 'content-type: application/json' \
  -d '{"name":"default"}'
```

Expected response:

```json
{"status":"disconnected","name":"default"}
```

### Event stream

```bash
curl -i http://127.0.0.1:4002/api/v1/events
```

Document current event types:

- `serial.json`
- `serial.text`
- `serial.log`
- `serial.notification`
- `serial.error`

Important note:

- The current server starts with no pre-seeded events, so manual `curl` may show SSE headers with no event body.
- Route tests seed mock events and verify SSE formatting.

Expected headers include:

```text
content-type: text/event-stream
cache-control: no-cache
```

---

## Legacy Compatibility Mapping

Include this mapping in plain bullets instead of a Markdown table if desired:

- `GET /list` -> `GET /api/v1/ports`
- `POST /connect` -> `POST /api/v1/connections`
- `GET /info` -> `GET /api/v1/connections`
- `POST /disconnect` -> `DELETE /api/v1/connections/:name` adapter using JSON body `{ "name": "default" }`
- `POST /commit` -> `POST /api/v1/connections/default/commands` adapter where the JSON body is the command payload

Mention that these aliases exist to ease migration from the older `sg-mcu-com` workflow.

---

## Serial Protocol Section

Include a short explanation:

- Outbound commands are JSON objects.
- If `reqId` is missing, the server generates one.
- If `reqId` is provided, the server preserves it.
- Commands are framed as UTF-8 JSON followed by `\r\n` by default.
- Inbound lines are parsed as:
  - JSON response (`serial.json`)
  - JSON log when `method == "log"` (`serial.log`)
  - JSON notification when `method == "notification"` (`serial.notification`)
  - text fallback (`serial.text`)
  - future/read error (`serial.error`)

Use examples from `docs/open-source-spec.md` if useful, but keep them accurate to the current implementation.

---

## Development Section

Include:

```bash
git clone https://github.com/avepha/serialport-api.git
cd serialport-api
cargo fmt --check
cargo check
cargo test
```

Mention important source files:

- `src/main.rs` — CLI and server startup
- `src/api/routes.rs` — Axum routes and route tests
- `src/serial/manager.rs` — serial abstractions, in-memory manager, command/event behavior
- `src/protocol.rs` — framing and line parser
- `docs/implementation-plan.md` — roadmap and staged rewrite plan
- `docs/open-source-spec.md` — long-form target specification

---

## Suggested README Skeleton

Use this as a starting point, but verify exact response examples before committing:

```markdown
# serialport-api

`serialport-api` is a Rust service for JSON-based serial-port communication with microcontrollers, robots, and Raspberry Pi deployments. It exposes an HTTP API for listing ports, managing named connections, sending JSON commands, and streaming serial events with Server-Sent Events.

> Status: rewrite in progress. The current API server, connection manager, command queue, and event stream are mock/in-memory foundations. Physical serial read/write loops, waited command responses, preset storage, and packaging are planned but not complete yet.

## Features

- [x] Axum HTTP API and CLI server
- [x] Protocol parser and JSON + CRLF framing
- [x] Port listing
- [x] Mock/in-memory connection lifecycle
- [x] Mock command queue with generated/preserved `reqId`
- [x] SSE event endpoint
- [x] Legacy compatibility aliases
- [ ] Real serial read/write loops
- [ ] Waited command responses and timeouts
- [ ] SQLite presets
- [ ] Raspberry Pi/systemd docs
- [ ] CI and release packaging

## Install / build

```bash
cargo build
```

## Run the server

```bash
cargo run -- serve --host 127.0.0.1 --port 4002
```

## Quick start

```bash
curl -s http://127.0.0.1:4002/api/v1/health
curl -s http://127.0.0.1:4002/api/v1/ports
curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}'
curl -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"method":"query","topic":"sensor.read","data":{}},"waitForResponse":false}'
curl -i http://127.0.0.1:4002/api/v1/events
```

## HTTP API

[Add endpoint details here.]

## Legacy compatibility aliases

[Add alias mapping here.]

## Serial protocol

[Add framing and event parsing details here.]

## Development

```bash
cargo fmt --check
cargo check
cargo test
```

## Roadmap

[Add planned features here.]

## License

MIT
```

---

## Acceptance Criteria

By the end of Phase 6.1:

- `README.md` is fully rewritten and no longer contains the old placeholder TODO list.
- README has a clear status note that the project is still a rewrite and hardware serial I/O is not complete.
- README includes copy-pasteable build/test/run commands.
- README documents all currently implemented canonical endpoints:
  - `GET /api/v1/health`
  - `GET /api/v1/ports`
  - `POST /api/v1/connections`
  - `GET /api/v1/connections`
  - `DELETE /api/v1/connections/:name`
  - `POST /api/v1/connections/:name/commands`
  - `GET /api/v1/events`
- README documents all currently implemented legacy aliases:
  - `GET /list`
  - `POST /connect`
  - `POST /disconnect`
  - `GET /info`
  - `POST /commit`
- README explains current command behavior: generated/preserved `reqId`, framed JSON + delimiter, queued-only response.
- README explains current event behavior and SSE event names.
- README clearly separates implemented features from planned roadmap items.
- No Rust runtime code is changed unless required to correct documentation examples after discovering a real mismatch; if that happens, stop and reassess the phase scope before proceeding.
- `cargo fmt --check`, `cargo check`, and `cargo test` still pass after the README edit.
- Manual smoke commands in the README have been checked against a local server where practical.
- Commit is created with a message like:

```bash
git add README.md
git commit -m "docs: rewrite project README"
```

---

## Manual Verification Flow

After editing README, run:

```bash
cd /home/alfarie/repos/serialport-api
cargo fmt --check
cargo check
cargo test
```

Then start the server:

```bash
cargo run -- serve --host 127.0.0.1 --port 4002
```

In another terminal, run a shortened smoke test matching the README examples:

```bash
curl -s http://127.0.0.1:4002/api/v1/health
curl -s http://127.0.0.1:4002/api/v1/ports
curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}'
curl -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"method":"query","topic":"sensor.read","data":{}},"waitForResponse":false}'
curl -s -X POST http://127.0.0.1:4002/commit \
  -H 'content-type: application/json' \
  -d '{"reqId":"client-42","method":"query","topic":"sensor.read","data":{}}'
curl -i -s http://127.0.0.1:4002/api/v1/events
curl -s -X POST http://127.0.0.1:4002/disconnect \
  -H 'content-type: application/json' \
  -d '{"name":"default"}'
```

Expected smoke notes:

- Health returns `{"status":"ok","version":"0.1.0"}`.
- Ports returns a JSON object with `ports` array; the array may be empty.
- Connect returns `status: connected`.
- Command returns `status: queued` and a generated `reqId`.
- Legacy `/commit` preserves `client-42`.
- Events returns `content-type: text/event-stream`; body may be empty on a fresh server.
- Disconnect returns `status: disconnected` for `default`.

Stop the server before committing or final status.

---

## Copy/Paste Prompt for the Next Coding Session

```text
We are in /home/alfarie/repos/serialport-api on branch rewrite/axum-serial-api. Please execute docs/phase-6.1-handoff-readme-rewrite.md.

This phase is documentation-only: rewrite README.md for the current Rust/Axum serialport-api rewrite. Load the writing-plans and rust-axum-api-tdd skills before editing. First verify baseline with git status --short --branch, cargo fmt --check, cargo check, and cargo test. Then replace the placeholder README with an accurate open-source README that documents current implemented mock/in-memory behavior, canonical /api/v1 endpoints, legacy aliases, serial protocol framing, SSE event names, development commands, and roadmap.

Do not implement new runtime features, physical serial I/O, waited responses, SQLite, CI, config files, Docker/systemd, or WebSocket/Socket.IO in this phase. After editing README, run cargo fmt --check, cargo check, cargo test, manually smoke the documented curl examples against cargo run -- serve --host 127.0.0.1 --port 4002 where practical, stop the server, and commit with: docs: rewrite project README.
```
