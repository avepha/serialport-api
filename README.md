# serialport-api

`serialport-api` is a Rust service for JSON-based serial-port communication with microcontrollers, robots, and Raspberry Pi deployments. It exposes an HTTP API for listing ports, managing named connections, sending JSON commands, and streaming recorded serial events with Server-Sent Events or native WebSocket snapshots.

## Status

> **Status: rewrite in progress.** The default API server remains mock/in-memory and hardware-free. Optional TOML config defaults, opt-in `--real-serial` mode, and saved command presets with opt-in SQLite persistence are available. Raspberry Pi/systemd deployment docs, Docker runtime packaging, and tag-triggered release automation are available.

Use the current default server to exercise the HTTP API shape, route compatibility, request/response JSON, command framing, and event-stream formatting without hardware. Use `serve --real-serial` only when you intentionally want to open and communicate with attached serial devices.

## Features

Implemented now:

- [x] Rust 2021 project with library + CLI/server binary
- [x] Axum HTTP server via `cargo run -- serve`
- [x] Health endpoint
- [x] Serial port listing endpoint backed by `serialport::available_ports()`
- [x] Mock/in-memory named connection lifecycle
- [x] Command endpoint with generated or preserved `reqId`
- [x] JSON command framing as UTF-8 JSON plus delimiter, usually `\r\n`
- [x] Server-Sent Events endpoint for recorded mock serial events
- [x] Native WebSocket endpoint for recorded serial event snapshots
- [x] Legacy aliases: `/list`, `/connect`, `/disconnect`, `/info`, `/commit`
- [x] Waited command responses matched by string `reqId`
- [x] Opt-in mock-device/scripted responses for hardware-free response tests
- [x] Opt-in real serial mode for opening/writing/reading OS serial ports
- [x] Coordinated real serial read-loop lifecycle with hardware-free tests
- [x] Optional TOML config file defaults for server, serial, and storage settings
- [x] Saved command preset CRUD routes under `/api/v1/presets`
- [x] Opt-in SQLite preset persistence with `--preset-db` or `[storage] preset_db`
- [x] Raspberry Pi install guide and systemd service examples
- [x] Dockerfile and Docker Compose example for local/container runs
- [x] Tag-triggered GitHub release workflow for Linux binary and GHCR image publishing
- [x] GitHub Actions CI for format, clippy, and tests
- [x] Unit and route tests for current behavior

Planned / not complete yet:

- [ ] Socket.IO protocol compatibility, if needed by legacy/browser clients
- [ ] ARM/Raspberry Pi release binary automation

## Install / build

Prerequisites:

- Rust toolchain compatible with Rust 2021
- Rust 1.75 or newer, per `Cargo.toml`

Build from source:

```bash
git clone https://github.com/avepha/serialport-api.git
cd serialport-api
cargo build
```

For local development inside an existing checkout:

```bash
cargo build
```

## Docker quick start

Build and run the container without installing a local Rust toolchain:

```bash
docker build -t serialport-api:local .
docker run --rm -p 4002:4002 serialport-api:local
```

Then check the mock-mode API from another terminal:

```bash
curl -s http://127.0.0.1:4002/api/v1/health
```

The default container command starts `serve --host 0.0.0.0 --port 4002` in mock/in-memory mode. For config mounts, SQLite preset volumes, Linux serial-device pass-through, Docker Compose, and tag-triggered release workflow details, see [`docs/docker-release.md`](docs/docker-release.md).

## Run the server

Start the Axum HTTP server:

```bash
cargo run -- serve --host 127.0.0.1 --port 4002
```

To use SQLite-backed saved command presets instead of the default in-memory preset store, pass a database path:

```bash
cargo run -- serve --host 127.0.0.1 --port 4002 --preset-db ./presets.db
```

Port `4002` is the default compatibility port from the older JavaScript `sg-mcu-com` workflow.

By default, `serve` uses the mock/in-memory transport and does not open physical serial ports. To use attached serial hardware, start the server with the explicit real mode flag:

```bash
cargo run -- serve --host 127.0.0.1 --port 4002 --real-serial
```

In real serial mode, `POST /api/v1/connections` opens the requested OS port using the provided `port`, `baudRate`, and `delimiter`; command routes write framed JSON bytes to that handle; and delimiter-terminated inbound lines are parsed into the existing SSE events and waited-response queues. Hardware smoke testing requires a connected device or loopback adapter. Do not use `--real-serial` together with `--mock-device` or `--mock-script`; the server rejects those combinations.

You can also configure the server with environment variables:

```bash
SERIALPORT_API_HOST=127.0.0.1 SERIALPORT_API_PORT=4002 cargo run -- serve
```

### Configuration file

`serve` can load optional TOML defaults for server startup and future/default serial connection settings. Pass an explicit path with `--config`; if no path is passed, the server auto-loads `./serialport-api.toml` from the current working directory when that file exists. Missing auto-discovered config is non-fatal. Missing, unreadable, or invalid explicit config fails startup clearly.

```bash
cargo run -- serve --config ./serialport-api.toml
```

Example `serialport-api.toml`:

```toml
[server]
host = "127.0.0.1"
port = 4002

[serial]
default_port = "/dev/ttyUSB0"
default_baud_rate = 115200
default_delimiter = "\r\n"
real_serial = false
mock_device = false
mock_script = "./mock-responses.json"

[storage]
preset_db = "./presets.db"
```

Precedence is:

1. Explicit CLI flags such as `--host`, `--port`, `--mock-device`, `--mock-script`, `--real-serial`, and `--preset-db`
2. Environment variables `SERIALPORT_API_HOST` and `SERIALPORT_API_PORT`
3. Config-file values
4. Built-in defaults (`127.0.0.1:4002`, mock/in-memory mode, in-memory presets, baud `115200`, delimiter `\r\n`)

`mock_script` implies mock-device behavior. The server rejects a resolved configuration that combines real serial mode with mock-device or mock-script mode.

## Quick start

In one terminal, run the server:

```bash
cargo run -- serve --host 127.0.0.1 --port 4002
```

In another terminal, try the current mock-backed flow:

```bash
curl -s http://127.0.0.1:4002/api/v1/health
curl -s http://127.0.0.1:4002/api/v1/ports

curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}'

curl -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"method":"query","topic":"sensor.read","data":{}},"waitForResponse":false}'

curl -i -s http://127.0.0.1:4002/api/v1/events

curl -s -X DELETE http://127.0.0.1:4002/api/v1/connections/default
```

Expected notes:

- Health returns `{"status":"ok","version":"0.1.0"}`.
- Ports returns a JSON object with a `ports` array; it may be empty.
- Connect returns `status: connected` and records the named connection in memory.
- Command returns `status: queued` and a `reqId`.
- Events returns SSE headers; a fresh server may have no event body.
- The native WebSocket event endpoint is also available at `/api/v1/events/ws`; a fresh server may close without frames if no events have been recorded.
- Disconnect returns `status: disconnected` for the requested name.

## HTTP API

Base URL for canonical routes:

```text
http://127.0.0.1:4002/api/v1
```

### `GET /api/v1/health`

Check whether the server is running.

```bash
curl -s http://127.0.0.1:4002/api/v1/health
```

Response:

```json
{"status":"ok","version":"0.1.0"}
```

### `GET /api/v1/ports`

List serial ports visible to the operating system through the `serialport` crate.

```bash
curl -s http://127.0.0.1:4002/api/v1/ports
```

Example response shape:

```json
{"ports":[]}
```

When ports are present, entries use this shape:

```json
{
  "ports": [
    {
      "name": "/dev/ttyUSB0",
      "type": "usb",
      "manufacturer": "FTDI",
      "serial_number": "ABC123"
    }
  ]
}
```

Do not rely on a specific device path being present; it depends on the host machine and attached hardware.

### `POST /api/v1/connections`

Create or replace a named connection. In the default mock server this records connection metadata only; in `--real-serial` mode it opens the requested OS serial port.

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}'
```

Response:

```json
{"status":"connected","connection":{"name":"default","status":"connected","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}}
```

### `GET /api/v1/connections`

List current in-memory connections.

```bash
curl -s http://127.0.0.1:4002/api/v1/connections
```

Example response:

```json
{"connections":[{"name":"default","status":"connected","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}]}
```

After all connections are disconnected:

```json
{"connections":[]}
```

### `DELETE /api/v1/connections/:name`

Remove a named in-memory connection.

```bash
curl -s -X DELETE http://127.0.0.1:4002/api/v1/connections/default
```

Response:

```json
{"status":"disconnected","name":"default"}
```

### `POST /api/v1/connections/:name/commands`

Queue a JSON command for a named connection. In the default mock server this records the framed command in memory; in `--real-serial` mode it writes the framed bytes to the opened serial port.

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"method":"query","topic":"sensor.read","data":{}},"waitForResponse":false}'
```

Response when the payload has no `reqId`:

```json
{"status":"queued","reqId":"1"}
```

If `payload.reqId` is present, the server preserves it:

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"reqId":"client-42","method":"query","topic":"sensor.read","data":{}},"waitForResponse":false}'
```

```json
{"status":"queued","reqId":"client-42"}
```

Notes:

- `payload` must be a JSON object.
- If `payload.reqId` is missing, the in-memory manager generates one.
- `waitForResponse: true` waits for an inbound JSON response with the same string `reqId` until `timeoutMs` elapses; timeout returns HTTP `504`.
- `waitForResponse: false` or omitted preserves the fire-and-forget queued response.
- The command is framed as JSON plus the connection delimiter, usually `\r\n`.

### `GET /api/v1/events`

Stream recorded serial events as Server-Sent Events.

```bash
curl -i -s http://127.0.0.1:4002/api/v1/events
```

Expected headers include:

```text
content-type: text/event-stream
cache-control: no-cache
```

Current event names:

- `serial.json`
- `serial.text`
- `serial.log`
- `serial.notification`
- `serial.error`

Important current limitation: the server starts with no pre-seeded events, so a manual `curl` against a fresh server may show SSE headers with no event body. Route tests seed mock events and verify SSE formatting.

### `GET /api/v1/events/ws`

Open a native WebSocket connection and receive recorded serial event snapshots as JSON text frames. On connection, the server sends one frame per event currently returned by the same event snapshot store used by `GET /api/v1/events`, then closes the socket normally.

```bash
websocat ws://127.0.0.1:4002/api/v1/events/ws
# or
npx wscat -c ws://127.0.0.1:4002/api/v1/events/ws
```

Example text frames:

```json
{"event":"serial.json","data":{"reqId":"1","ok":true}}
```

```json
{"event":"serial.text","data":"hello robot"}
```

A fresh server may have no recorded events, so the WebSocket can close without sending frames. This is a native WebSocket endpoint only; Socket.IO/Engine.IO clients are not compatible with `/api/v1/events/ws`.

## Saved command presets

Preset routes store reusable JSON command payloads without opening serial ports or sending commands. By default, presets are in-memory and reset when the server exits. Use `--preset-db <PATH>` or `[storage] preset_db = "..."` to enable SQLite persistence.

Create a preset:

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/presets \
  -H 'content-type: application/json' \
  -d '{"name":"Read IMU","payload":{"method":"query","topic":"imu.read","data":{}}}'
```

List presets:

```bash
curl -s http://127.0.0.1:4002/api/v1/presets
```

Other preset routes:

- `GET /api/v1/presets/:id` returns one preset.
- `PUT /api/v1/presets/:id` updates a preset with a new `name` and JSON-object `payload`.
- `DELETE /api/v1/presets/:id` deletes a preset.

## Docker / release packaging

For Docker-based local runs and release packaging, see [`docs/docker-release.md`](docs/docker-release.md). The repository includes a root `Dockerfile`, an optional [`examples/docker-compose.yml`](examples/docker-compose.yml), and a tag-triggered GitHub Actions release workflow for Linux x86_64 binary archives and GHCR image publishing.

## Raspberry Pi / systemd deployment

For Raspberry Pi OS and Debian-like Linux deployments, see [`docs/raspberry-pi-systemd.md`](docs/raspberry-pi-systemd.md). The guide covers building or copying the binary, serial permissions, `/dev/serial/by-id/*` device paths, example TOML config with `[storage] preset_db`, a systemd unit, smoke checks, troubleshooting, and network exposure notes.

## Legacy compatibility aliases

These routes ease migration from the older JavaScript `sg-mcu-com` workflow while the Rust API settles.

- `GET /list` -> `GET /api/v1/ports`
- `POST /connect` -> `POST /api/v1/connections`
- `GET /info` -> `GET /api/v1/connections`
- `POST /disconnect` -> `DELETE /api/v1/connections/:name` adapter using JSON body `{ "name": "default" }`
- `POST /commit` -> `POST /api/v1/connections/default/commands` adapter where the JSON body is the command payload

### Legacy examples

List ports:

```bash
curl -s http://127.0.0.1:4002/list
```

Connect:

```bash
curl -s -X POST http://127.0.0.1:4002/connect \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}'
```

Inspect connections:

```bash
curl -s http://127.0.0.1:4002/info
```

Send a command to the `default` connection:

```bash
curl -s -X POST http://127.0.0.1:4002/commit \
  -H 'content-type: application/json' \
  -d '{"reqId":"client-42","method":"query","topic":"sensor.read","data":{}}'
```

Current response:

```json
{"status":"queued","reqId":"client-42"}
```

Disconnect:

```bash
curl -s -X POST http://127.0.0.1:4002/disconnect \
  -H 'content-type: application/json' \
  -d '{"name":"default"}'
```

Current response:

```json
{"status":"disconnected","name":"default"}
```

## Serial protocol

Current protocol helpers implement JSON line framing and inbound line classification.

Outbound commands:

- Commands are JSON objects.
- If `reqId` is missing, the server generates one before framing.
- If `reqId` is provided, the server preserves it.
- Commands are encoded as UTF-8 JSON followed by the connection delimiter.
- The default compatibility delimiter is `\r\n`.

Example logical payload:

```json
{"reqId":"1","method":"query","topic":"sensor.read","data":{}}
```

Example framed bytes with the default delimiter:

```text
{"reqId":"1","method":"query","topic":"sensor.read","data":{}}\r\n
```

Inbound lines are currently parsed as:

- JSON response -> `serial.json`
- JSON log when `method == "log"` -> `serial.log`
- JSON notification when `method == "notification"` -> `serial.notification`
- non-JSON or lossy UTF-8 text fallback -> `serial.text`
- future/read errors recorded by the manager -> `serial.error`

The default server remains hardware-free. In `--real-serial` mode, delimiter-terminated inbound hardware lines are parsed into the same event stream and string `reqId` response queues.

## Development

Clone and verify:

```bash
git clone https://github.com/avepha/serialport-api.git
cd serialport-api
cargo fmt --check
cargo check
cargo test
```

Common local commands:

```bash
cargo fmt --check
cargo check
cargo test
cargo run -- serve --host 127.0.0.1 --port 4002
```

Important source files:

- `src/main.rs` — CLI and server startup
- `src/api/routes.rs` — Axum routes and route tests
- `src/serial/manager.rs` — serial abstractions, in-memory manager, command/event behavior
- `src/protocol.rs` — JSON framing and line parser
- `src/error.rs` — shared error type
- `docs/implementation-plan.md` — staged rewrite plan
- `docs/open-source-spec.md` — long-form target specification

## Roadmap

Near-term work:

- Add ARM/Raspberry Pi binary release automation if maintainers want prebuilt Pi artifacts.

Later work:

- Socket.IO protocol compatibility if needed for legacy/browser clients that require Engine.IO framing.

## License

MIT
