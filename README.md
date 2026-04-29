# serialport-api

`serialport-api` is a Rust service for JSON-based serial-port communication with microcontrollers, robots, and Raspberry Pi deployments. It exposes an HTTP API for listing ports, managing named connections, sending JSON commands, and streaming serial events with Server-Sent Events.

## Status

> **Status: rewrite in progress.** The current API server, connection manager, command queue, and event stream are mock/in-memory foundations. Physical serial read/write loops, waited command responses, preset storage, CI, and Raspberry Pi packaging are planned but not complete yet.

Use the current server to exercise the HTTP API shape, route compatibility, request/response JSON, command framing, and event-stream formatting. Do not expect it to communicate with real serial hardware yet, except for listing ports via `serialport::available_ports()`.

## Features

Implemented now:

- [x] Rust 2021 project with library + CLI/server binary
- [x] Axum HTTP server via `cargo run -- serve`
- [x] Health endpoint
- [x] Serial port listing endpoint backed by `serialport::available_ports()`
- [x] Mock/in-memory named connection lifecycle
- [x] Mock command endpoint with generated or preserved `reqId`
- [x] JSON command framing as UTF-8 JSON plus delimiter, usually `\r\n`
- [x] Server-Sent Events endpoint for recorded mock serial events
- [x] Legacy aliases: `/list`, `/connect`, `/disconnect`, `/info`, `/commit`
- [x] Unit and route tests for current behavior

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
- [ ] WebSocket or Socket.IO support

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

## Run the server

Start the Axum HTTP server:

```bash
cargo run -- serve --host 127.0.0.1 --port 4002
```

Port `4002` is the default compatibility port from the older JavaScript `sg-mcu-com` workflow.

You can also configure the server with environment variables:

```bash
SERIALPORT_API_HOST=127.0.0.1 SERIALPORT_API_PORT=4002 cargo run -- serve
```

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

Create or replace a named mock/in-memory connection. This records connection metadata but does not open the physical serial port yet.

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

Queue a JSON command for a named connection. Current behavior records the framed command in memory and returns immediately; it does not write to a real serial port yet.

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
- `waitForResponse` and `timeoutMs` are accepted in the request shape, but waited responses and timeout behavior are not implemented yet.
- The queued command is framed as JSON plus the connection delimiter, usually `\r\n`.

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

The parser and mock event stream are implemented, but background serial read loops that feed real inbound hardware data into the event stream are not implemented yet.

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

- Wire real serial transport while preserving mockable tests.
- Add background read loops that parse serial lines into events.
- Implement request/response matching by `reqId`.
- Honor `waitForResponse` and `timeoutMs` for command calls.
- Add mock serial device mode and scripted responses.
- Add config file support for server and serial defaults.

Later work:

- SQLite-backed saved presets.
- Raspberry Pi install docs and systemd unit examples.
- GitHub Actions CI.
- Release binaries and/or Docker image.
- WebSocket or Socket.IO compatibility if needed for browser clients.

## License

MIT
