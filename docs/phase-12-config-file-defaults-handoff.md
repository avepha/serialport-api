# Phase 12 Config File Defaults Handoff

> **For Hermes / next AI coding session:** Execute this in a fresh session. Load `writing-plans`, `test-driven-development`, and `rust-axum-api-tdd` before editing. This phase should add optional TOML config-file support for server defaults and serial connection defaults. Keep the scope narrow and test-first. Do not add SQLite/persistent preset CRUD, Docker/systemd packaging, WebSocket/Socket.IO, authentication, release automation, or a large API error-envelope refactor.

**Goal:** Add a small, documented configuration layer so operators can start the Axum server with defaults from a TOML file while preserving the current CLI/env behavior and the hardware-free default server. The config file should be optional. Explicit CLI arguments and existing environment variables must continue to work and should override config-file values.

**Architecture:** Build a pure `config` module that parses and merges configuration before server startup. `main.rs` should remain the only place that translates CLI/env/config into a bound address and router mode. Do not push file-loading or config precedence into Axum route handlers or serial managers.

**Tech Stack:** Rust 2021, Axum 0.7, Tokio 1, Clap 4, Serde, TOML parsing, Thiserror, Tracing, test-first Rust unit tests and CLI/startup validation tests.

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

Latest known relevant commits after Phase 11:

```text
9ea7d2b docs: refresh README roadmap
07bf24c docs: update README progress
7822584 fix: coordinate real serial read loops
a07a7ac feat: add opt-in real serial lifecycle
f627a21 docs: add phase 11 real serial handoff
8a039ad feat: add mock device scripted responses
fcbae2a docs: add phase 10 mock device handoff
bf2d64a feat: add waited command responses
```

Phase 11 review status:

- Independent re-review verdict through `07bf24c`: **APPROVED**.
- Blocking issues were fixed in `7822584 fix: coordinate real serial read loops`.
- README progress/roadmap was refreshed in `07bf24c` and `9ea7d2b`.
- Current pushed remote `master` is expected to be `9ea7d2b docs: refresh README roadmap`.

Completed functionality as of this handoff:

- Axum server starts with `cargo run -- serve --host 127.0.0.1 --port 4002`.
- Default server remains hardware-free and mock-backed.
- Port listing uses `serialport::available_ports()`.
- Named connection lifecycle exists for canonical routes and legacy aliases.
- Commands generate or preserve string `reqId`, frame JSON with the connection delimiter, and write through the active serial transport.
- Waited command responses match inbound JSON by connection name and string `reqId`.
- SSE events include `serial.json`, `serial.text`, `serial.log`, `serial.notification`, and `serial.error`.
- Opt-in `--mock-device` and `--mock-script <PATH>` can synthesize hardware-free responses.
- Opt-in `--real-serial` can open/write/read OS serial ports and coordinates read-loop stop tokens/join handles on connect/reconnect/disconnect.
- `--real-serial` is rejected with `--mock-device` or `--mock-script`.
- Automated tests remain hardware-free.

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

## Why Phase 12 Is Config File Defaults

The open-source spec and refreshed README identify config file support as the next near-term gap after real serial lifecycle. The service now has the runtime capabilities that config should orchestrate:

- server host/port;
- mock-device and mock-script modes;
- real-serial mode;
- serial connection defaults such as `port`, `baud_rate`, and `delimiter`.

Adding config now is useful, but it must stay smaller than persistent presets. Phase 12 should only make startup defaults configurable; persistent saved profiles and SQLite CRUD should remain a later phase.

---

## Phase 12 Scope

Do in Phase 12:

- Add an optional TOML config file parser, likely `src/config.rs`.
- Support an explicit config path flag, recommended: `serve --config <PATH>`.
- Support at least the local project config path `serialport-api.toml` when no explicit config path is passed.
- Optionally support one user-level path only if simple and fully tested, for example `$XDG_CONFIG_HOME/serialport-api/config.toml` or `~/.config/serialport-api/config.toml`.
- Keep missing config files non-fatal when auto-discovered.
- Make an explicitly provided `--config <PATH>` fail clearly if the file cannot be read or parsed.
- Parse a minimal config shape:

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
```

- Preserve existing environment variables:
  - `SERIALPORT_API_HOST`
  - `SERIALPORT_API_PORT`
- Preserve existing CLI flags:
  - `--host`
  - `--port`
  - `--mock-device`
  - `--mock-script <PATH>`
  - `--real-serial`
- Define and test precedence: CLI explicit values > environment variables > config file > built-in defaults.
- Keep `--mock-script` implying mock-device behavior as it does now.
- Keep rejecting real serial combined with mock-device/script after merging all sources.
- Expose parsed default serial values in a narrow way that can be reused by later phases. It is acceptable in Phase 12 for these defaults to only document/fill future auto-connect/default-connect behavior if current routes still require request bodies.
- Update `README.md` with concise config-file usage and precedence notes.

Do **not** do in Phase 12:

- Do not add SQLite, `sqlx`, `rusqlite`, migrations, or persistent preset CRUD.
- Do not add `GET/POST/DELETE /api/v1/presets` yet.
- Do not auto-connect physical serial ports at startup unless every interaction is opt-in, tested with fakes, and clearly non-blocking. Recommended: leave auto-connect out of Phase 12.
- Do not make `--real-serial` the default.
- Do not require hardware for tests or smoke verification.
- Do not change canonical route response shapes or legacy aliases.
- Do not change SSE event names or generated `reqId` sequencing.
- Do not add broad CORS/authentication/API-error-envelope refactors.
- Do not add Docker, systemd, release binaries, or Raspberry Pi packaging.

If a tempting change requires durable storage, schema migration, daemon supervision, hardware-specific CI, or route semantics changes, leave it for a later phase.

---

## Expected Files to Modify or Create

Expected implementation changes:

- Create: `src/config.rs`
  - Config structs with `serde::Deserialize`.
  - TOML parsing from string and file path.
  - Merge/preference logic that is unit-testable without running the server.
- Modify: `src/lib.rs`
  - Export `pub mod config;`.
- Modify: `Cargo.toml`
  - Add a small TOML dependency, recommended `toml = "0.8"`, unless the implementer chooses an already-present parser (none is present at this handoff).
- Modify: `src/main.rs`
  - Add `serve --config <PATH>`.
  - Separate CLI parsing from resolved runtime settings so precedence is testable.
  - Use resolved settings to bind host/port and choose default/mock/real router mode.
  - Preserve validation for incompatible modes after all config/env/CLI values are merged.
- Modify: `src/error.rs`
  - Add narrow config read/parse error variants only if helpful. Avoid a broad error-model rewrite.
- Modify: `README.md`
  - Add a short config-file section with example TOML, discovery rules, and precedence.

Optional if helpful:

- Create: `examples/serialport-api.toml`
  - Only if README would otherwise become too long. Keep it simple and add it to the same implementation commit if created.

---

## Current Code to Understand First

Read these files before editing:

```bash
cd /home/alfarie/repos/serialport-api
sed -n '1,220p' src/main.rs
sed -n '1,220p' src/error.rs
sed -n '1,120p' src/lib.rs
sed -n '1,220p' src/api/routes.rs
sed -n '1,260p' src/serial/manager.rs
sed -n '1,320p' src/serial/real_transport.rs
sed -n '1,260p' src/serial/mock_device.rs
sed -n '1,120p' Cargo.toml
sed -n '1,460p' README.md
```

Key current facts:

- `ServeArgs` currently uses Clap defaults for host and port:
  - `#[arg(long, default_value = "127.0.0.1", env = "SERIALPORT_API_HOST")]`
  - `#[arg(long, default_value_t = 4002, env = "SERIALPORT_API_PORT")]`
- Because Clap fills defaults immediately, implementing config precedence may require distinguishing explicit CLI values from defaults. A common approach is to remove Clap defaults from raw CLI fields and merge them into a separate `ResolvedServeConfig`.
- `main.rs` currently selects one of three router paths:
  - default mock router;
  - mock-device/script router;
  - real-serial router.
- `validate_serve_args` currently rejects `--real-serial` with `--mock-device` or `--mock-script` before serving. Phase 12 should preserve this validation on the resolved config, not just raw CLI args.
- The live server currently does not need config for routes. Keep route handlers unaware of config unless a very small injection is necessary.

---

## Recommended Design

### 1. Keep raw CLI separate from resolved config

Use optional raw values in `ServeArgs` so config can fill gaps:

```rust
#[derive(Debug, Args)]
struct ServeArgs {
    #[arg(long, env = "SERIALPORT_API_HOST")]
    host: Option<String>,

    #[arg(long, env = "SERIALPORT_API_PORT")]
    port: Option<u16>,

    #[arg(long)]
    config: Option<PathBuf>,

    #[arg(long)]
    mock_device: bool,

    #[arg(long)]
    mock_script: Option<PathBuf>,

    #[arg(long)]
    real_serial: bool,
}
```

Then merge into a separate resolved type:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedServeConfig {
    host: String,
    port: u16,
    mock_device: bool,
    mock_script: Option<PathBuf>,
    real_serial: bool,
    serial_defaults: SerialDefaults,
}
```

The exact type names can differ. The important part is that tests can prove precedence without binding sockets.

### 2. Add pure config structures

Suggested config structs:

```rust
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct FileConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub serial: SerialConfig,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct ServerConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct SerialConfig {
    pub default_port: Option<String>,
    pub default_baud_rate: Option<u32>,
    pub default_delimiter: Option<String>,
    pub real_serial: Option<bool>,
    pub mock_device: Option<bool>,
    pub mock_script: Option<PathBuf>,
}
```

Use `#[serde(deny_unknown_fields)]` only if you want strict config. If strictness creates poor forward compatibility, omit it and document that unknown fields are ignored. Pick one behavior and test it.

### 3. Discovery rules

Recommended narrow discovery:

1. If `--config <PATH>` is provided, load exactly that path; missing/unreadable/invalid is an error.
2. Else, if `./serialport-api.toml` exists in the current working directory, load it.
3. Else, use built-in defaults.

Do not traverse arbitrary parent directories. Do not require `/etc` or home-directory behavior in Phase 12 unless added with deterministic unit tests.

### 4. Precedence rules

Expected final precedence:

1. Explicit CLI flag values.
2. Values provided via Clap environment variables (`SERIALPORT_API_HOST`, `SERIALPORT_API_PORT`).
3. Config-file values.
4. Built-in defaults:
   - host: `127.0.0.1`
   - port: `4002`
   - `mock_device`: `false`
   - `mock_script`: `None`
   - `real_serial`: `false`
   - default baud rate: `115200`
   - default delimiter: `\r\n`

Because Clap's `env` source may be difficult to distinguish from CLI in unit tests, it is acceptable to parse env separately in a pure resolver instead of relying on Clap `env`, as long as existing user-facing env behavior is preserved.

---

## Bite-Sized TDD Tasks

### Task 12.1: Add config parser module

RED test first:

- Add `src/config.rs` tests for parsing a TOML string with `[server]` and `[serial]`.
- Test that omitted sections produce `Default`/`None` values.

Command:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
cargo test config:: --all-features
```

Expected RED reason:

- `src/config.rs` and/or `serialport_api::config` do not exist, and no TOML parser dependency exists.

GREEN implementation:

- Add `toml = "0.8"` to `Cargo.toml`.
- Create `src/config.rs` with config structs and `FileConfig::from_toml_str` or equivalent.
- Export `pub mod config;` from `src/lib.rs`.

Expected GREEN:

- Config parser tests pass.

### Task 12.2: Add explicit config-file loading behavior

RED test first:

- Add tests using a temporary file path or standard-library temp directory to prove:
  - explicit existing `--config`/path loads successfully;
  - explicit missing path errors;
  - auto-discovery missing path returns defaults/non-error.

Command:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test config:: --all-features
```

Expected RED reason:

- No file-loading/discovery API exists.

GREEN implementation:

- Add a small API such as `load_explicit_config(path)` and `load_discovered_config(cwd)`.
- Keep file system behavior deterministic and unit-testable.

Expected GREEN:

- Config file loading tests pass without hardware or network.

### Task 12.3: Add resolved serve config and precedence tests

RED test first:

- Add `main.rs` unit tests or move resolver logic into `src/config.rs` and test there.
- Prove config host/port fill defaults.
- Prove env overrides config for host/port.
- Prove explicit CLI host/port override env/config.
- Prove `mock_script` implies mock-device behavior.
- Prove resolved `real_serial + mock_device/mock_script` is rejected.

Command:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --all-features
```

Expected RED reason:

- Existing `ServeArgs` uses concrete Clap defaults and there is no resolved merge layer.

GREEN implementation:

- Introduce raw optional CLI fields where needed.
- Add a pure `resolve_serve_config(...)` helper.
- Preserve existing env var names and built-in defaults.
- Move incompatible-mode validation to resolved config.

Expected GREEN:

- Existing CLI tests are updated and pass.
- New precedence tests pass.

### Task 12.4: Wire resolved config into `serve`

RED test first:

- Add tests that parse representative CLI invocations:
  - `serve --config ./serialport-api.toml`
  - `serve --host 0.0.0.0 --port 5000`
  - `serve --real-serial --mock-device` rejects after resolution.

Command:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --all-features
```

Expected RED reason:

- CLI lacks `--config`; `serve` still consumes raw `ServeArgs` directly.

GREEN implementation:

- Add `--config <PATH>`.
- Resolve config before binding `TcpListener`.
- Use resolved values for `SocketAddr` and router selection.
- Do not alter route response shapes.

Expected GREEN:

- All tests pass; no server socket is bound by unit tests.

### Task 12.5: README documentation update

RED check first:

- Search README for config-file guidance; it currently lists config support as planned but not implemented.

Command:

```bash
cd /home/alfarie/repos/serialport-api
rg -n "config|serialport-api.toml|SERIALPORT_API" README.md
```

Expected RED reason:

- README lacks implemented config-file usage and precedence documentation.

GREEN implementation:

- Add a concise `Configuration file` section after environment-variable usage or in `Run the server`.
- Include example TOML and precedence.
- Move config file support from planned to implemented in the feature/status lists if the implementation is complete.

Expected GREEN:

- README accurately reflects the implemented config scope.

---

## Verification Commands

Run these before committing Phase 12 implementation:

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

---

## Manual Smoke Test Flow

Use a temporary copy of the server on a non-conflicting port. These checks should not require hardware.

### 1. Config file supplies host/port

Create a local config file:

```bash
cd /home/alfarie/repos/serialport-api
cat > /tmp/serialport-api-phase12.toml <<'EOF'
[server]
host = "127.0.0.1"
port = 4012

[serial]
mock_device = true
default_baud_rate = 115200
default_delimiter = "\r\n"
EOF
```

Start server:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -- serve --config /tmp/serialport-api-phase12.toml
```

In another terminal:

```bash
curl -s http://127.0.0.1:4012/api/v1/health
curl -s -X POST http://127.0.0.1:4012/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}'
curl -s -X POST http://127.0.0.1:4012/api/v1/connections/default/commands \
  -H 'content-type: application/json' \
  -d '{"payload":{"reqId":"phase12-smoke","method":"query","topic":"sensor.read","data":{}},"waitForResponse":true,"timeoutMs":1000}'
```

Expected:

- Health returns `{"status":"ok","version":"0.1.0"}`.
- Connect returns `status: connected`.
- Waited command succeeds if config enabled mock-device mode.

### 2. CLI overrides config host/port

With the same config file, start:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -- serve --config /tmp/serialport-api-phase12.toml --host 127.0.0.1 --port 4013
```

Check:

```bash
curl -s http://127.0.0.1:4013/api/v1/health
```

Expected:

- Server listens on `4013`, proving CLI override.

### 3. Invalid incompatible modes reject clearly

Create:

```bash
cat > /tmp/serialport-api-invalid.toml <<'EOF'
[serial]
real_serial = true
mock_device = true
EOF
```

Run:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -- serve --config /tmp/serialport-api-invalid.toml
```

Expected:

- Startup fails with a clear message that real serial cannot be combined with mock-device/mock-script.

---

## Expected Commit Message

Use one implementation commit unless the README update is intentionally separated:

```text
feat: add config file defaults
```

Optional second commit if documentation is separate:

```text
docs: document config file defaults
```

---

## Copy/Paste Implementation Prompt for Next Subagent

```text
You are implementing Phase 12 for /home/alfarie/repos/serialport-api on branch rewrite/axum-serial-api.

Read docs/phase-12-config-file-defaults-handoff.md first. Implement optional TOML config-file defaults for the Rust Axum serialport-api rewrite. Keep the scope narrow and TDD-first.

Current baseline: Phase 11 real serial lifecycle is implemented and approved. Latest commits include 9ea7d2b docs: refresh README roadmap, 07bf24c docs: update README progress, 7822584 fix: coordinate real serial read loops, a07a7ac feat: add opt-in real serial lifecycle, and f627a21 docs: add phase 11 real serial handoff. Current pushed remote master is expected at 9ea7d2b.

Goal: Add optional config-file support for server host/port and serial startup/default settings without changing route response shapes or requiring hardware. Add a `serve --config <PATH>` flag, load `./serialport-api.toml` automatically if present, keep missing auto-discovered config non-fatal, and make explicit missing/invalid config fail clearly. Preserve existing env vars SERIALPORT_API_HOST and SERIALPORT_API_PORT. Precedence must be CLI explicit values > env vars > config file > built-in defaults. Preserve `--mock-script` implying mock-device and preserve rejection of real-serial combined with mock-device/script after all sources are merged.

Expected files: create src/config.rs; modify src/lib.rs, Cargo.toml, src/main.rs, possibly src/error.rs, and README.md. Do not add SQLite/presets, auto-connect, Docker/systemd, WebSocket/Socket.IO, authentication, release automation, broad error-envelope refactors, or hardware-dependent tests.

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

Commit as: feat: add config file defaults

After independent approval, the orchestrator will push the approved completed cycle to origin/master.
```

---

## Orchestrator Note

After Phase 12 implementation is complete and independently approved, the orchestrator is expected to push the approved completed cycle to `origin/master`.
