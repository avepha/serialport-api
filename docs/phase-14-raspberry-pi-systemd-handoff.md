# Phase 14 Raspberry Pi Install + systemd Handoff

> **For Hermes / next AI implementation session:** Execute this in a fresh session. Load `writing-plans` and any repository documentation/testing skills normally used for handoff execution before editing. This phase is documentation-and-packaging-example focused: produce a Raspberry Pi/Linux install guide and systemd service examples for running the existing Axum serial API reliably on boot. Keep source-code changes out of scope unless a tiny CLI/help bug blocks documented usage. Do not implement release binaries, Docker images, WebSocket/Socket.IO, authentication, UI, or API behavior changes.

**Goal:** Make the current Rust rewrite installable and operable on Raspberry Pi OS / Debian-like embedded Linux by adding clear docs, example config, example systemd unit(s), and smoke-test instructions that match the code after Phase 13 SQLite presets.

**Inferred next phase:** Phase 14 is **Raspberry Pi install guide and systemd service examples**. Phase 13 added SQLite/in-memory preset storage and CRUD routes. The original roadmap now leaves Raspberry Pi install docs/systemd, release binaries/Docker, and WebSocket/Socket.IO. Repository evidence supports Pi/systemd next:

- `README.md` still lists `Raspberry Pi install guide and systemd service` as planned.
- `docs/open-source-spec.md` names Raspberry Pi/embedded Linux operation as a primary goal and lists `Raspberry Pi setup guide` as required documentation.
- The code now has the prerequisites a service guide needs: config-file defaults, `--real-serial`, `--preset-db`, SSE, and hardware-free/mock modes.
- Release binaries/Docker should remain later because this phase can document source-build/systemd deployment without establishing release automation.

**Current important caveat:** `README.md` appears stale after Phase 13 and still lists SQLite presets as planned. The implementation agent should update README status while adding Pi/systemd docs, but should not broaden the README rewrite beyond accuracy and links to the new guide.

---

## Strict Orchestration Input Schema

The implementation agent should accept this handoff plus the repository as its complete input. No hidden context is required.

```json
{
  "agent_role": "implementation",
  "phase": "Phase 14",
  "repository": "/home/alfarie/repos/serialport-api",
  "branch": "rewrite/axum-serial-api",
  "base_commit_expected": "13c80fc feat: add SQLite preset storage",
  "toolchain_env": {
    "PATH_prefix": "$HOME/.cargo/bin"
  },
  "scope": "Raspberry Pi install guide, systemd service examples, example production config, README roadmap/status refresh, docs validation only",
  "non_goals": [
    "Rust API/source behavior changes",
    "Docker image or release automation",
    "WebSocket/Socket.IO",
    "authentication or security middleware",
    "browser UI",
    "hardware-required automated tests"
  ]
}
```

### Required Preconditions

Before editing, run:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
git log --oneline -5
```

Expected:

- Branch is `rewrite/axum-serial-api`.
- Working tree is clean.
- Recent history includes `13c80fc feat: add SQLite preset storage` or a descendant of it.

If the working tree is not clean before Phase 14 edits, stop and report instead of modifying files.

---

## Strict Orchestration Output Schema

The implementation agent’s final response should use this JSON shape:

```json
{
  "agent_role": "implementation",
  "phase": "Phase 14",
  "summary": [
    "Added Raspberry Pi install and systemd deployment documentation.",
    "Added example service/config files and README links/status refresh."
  ],
  "files_changed": [
    "README.md",
    "docs/raspberry-pi-systemd.md",
    "examples/systemd/serialport-api.service",
    "examples/serialport-api.toml"
  ],
  "verification": {
    "commands_run": [
      "cargo fmt --check",
      "cargo clippy --all-targets --all-features -- -D warnings",
      "cargo test --all-features",
      "grep/link/doc smoke checks listed in this handoff"
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

```bash
rewrite/axum-serial-api
```

Known latest pushed/committed Phase 13 commit:

```text
13c80fc feat: add SQLite preset storage
```

Important current behavior as of Phase 13:

- `cargo run -- serve --host 127.0.0.1 --port 4002` starts the Axum HTTP server in mock/in-memory mode.
- Default server remains hardware-free and does not open physical serial ports.
- `serve --real-serial` opts into real OS serial port open/read/write behavior.
- `serve --preset-db <PATH>` opts into SQLite-backed preset persistence.
- Optional config file loading exists via `--config <PATH>` and auto-discovered `./serialport-api.toml`.
- Config currently includes `[server]`, `[serial]`, and `[storage] preset_db = "..."` support.
- CLI/env/config/default precedence for host/port and config-file defaults is already implemented.
- The API exposes health, ports, connections, commands, SSE events, legacy aliases, and Phase 13 preset CRUD routes.

Important local toolchain note:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Use that before all `cargo` commands in this WSL environment to avoid Rust toolchain mismatch.

---

## Phase 14 Scope

Do in Phase 14:

- Add a Raspberry Pi / Debian Linux deployment guide that is accurate for the current source-build workflow.
- Include commands for installing OS prerequisites on Raspberry Pi OS/Debian.
- Include Rust install/build commands or clearly document copying an already built binary.
- Include recommended Linux serial-device permissions, especially `dialout` group membership and the need to log out/reboot after group changes.
- Include stable serial-device naming guidance using `/dev/serial/by-id/*` when available, with fallback notes for `/dev/ttyUSB0`, `/dev/ttyACM0`, and HAT/UART devices such as `/dev/serial0`.
- Include a production-ish config example covering:
  - binding to `0.0.0.0` or `127.0.0.1` depending on exposure intent,
  - default serial port/baud/delimiter,
  - `real_serial = true`,
  - `[storage] preset_db = "..."`.
- Add a systemd unit example for running the service on boot.
- Include `systemctl` install/enable/start/status/logs commands.
- Include manual smoke checks with `curl` for health, port listing, config-backed startup, preset persistence, and real-serial connection only when hardware is attached.
- Include troubleshooting for common Pi/Linux issues:
  - permission denied on serial port,
  - wrong device path after reboot,
  - service fails because binary/config/database directory path is wrong,
  - port already in use,
  - binding to LAN vs localhost,
  - SQLite database directory ownership/permissions,
  - real serial conflicting with mock-device/mock-script settings.
- Refresh `README.md` only enough to:
  - mark SQLite saved presets as implemented or remove it from planned status,
  - link to the new Raspberry Pi/systemd guide,
  - mention `--preset-db` and `[storage] preset_db` if README still lacks it,
  - keep roadmap ordering accurate.
- Add small example files if useful and keep them versionable text.
- Keep all automated verification hardware-free.

Out of scope / do **not** do in Phase 14:

- Do not change Rust route behavior, serial manager behavior, storage behavior, config precedence, or public API schemas.
- Do not add Dockerfiles, release workflows, cross-compilation scripts, package managers, or binary publishing automation.
- Do not add WebSocket/Socket.IO support.
- Do not add auth, TLS termination, firewall automation, reverse-proxy config, or UI.
- Do not make SQLite mandatory for default startup.
- Do not require connected serial hardware in CI or automated tests.
- Do not rename existing endpoints or legacy aliases.
- Do not add a systemd unit that runs as `root` unless explicitly documenting it as not recommended. Prefer a dedicated unprivileged user.

---

## Expected Files to Inspect Before Editing

Read these first:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"

# Repository docs and roadmap
sed -n '1,520p' README.md
sed -n '1,220p' docs/phase-13-sqlite-presets-handoff.md
sed -n '450,520p' docs/open-source-spec.md
sed -n '470,485p' docs/implementation-plan.md

# Current CLI/config behavior that docs must match
sed -n '1,180p' src/main.rs
sed -n '1,230p' src/config.rs
sed -n '1,120p' Cargo.toml

# Current preset routes for README examples if needed
sed -n '1,260p' src/api/routes.rs
grep -R "api/v1/presets\|preset_db\|--preset-db" -n README.md src docs || true
```

Use `read_file`/`search_files` equivalents if operating through tools that prohibit shell readers.

---

## Expected Files to Modify or Create

Required:

- Create: `docs/raspberry-pi-systemd.md`
  - Main installation and operations guide.
  - Must be complete enough for a new Pi user/developer to follow without this handoff.

- Modify: `README.md`
  - Fix stale Phase 13 status about SQLite presets.
  - Add a concise Raspberry Pi/systemd section or link to `docs/raspberry-pi-systemd.md`.
  - Keep README concise; put detailed operations content in the new guide.

Strongly recommended if it keeps docs more executable:

- Create: `examples/systemd/serialport-api.service`
  - A reusable systemd service unit matching the docs.

- Create: `examples/serialport-api.toml`
  - A sample config matching current `FileConfig` fields.

Optional only if the implementation agent finds a clear need:

- Create: `docs/hardware-troubleshooting.md`
  - Only if the troubleshooting section would make `docs/raspberry-pi-systemd.md` unwieldy. Otherwise keep troubleshooting in one guide.

Files not expected to change:

- `src/**`
- `Cargo.toml`
- `Cargo.lock`
- `.github/**`
- Docker/release files

If any source or cargo file changes become necessary, document the reason explicitly in the final output and keep the change tiny.

---

## Required Content Contract

### `docs/raspberry-pi-systemd.md`

The guide should include these sections, in this order unless there is a good reason to reorder:

1. `# Raspberry Pi / systemd Deployment Guide`
2. `## Supported deployment model`
   - State that Phase 14 documents building from source or copying a binary; release artifacts are a later phase.
   - State target OS family: Raspberry Pi OS / Debian-like Linux with systemd.
3. `## Prerequisites`
   - OS packages such as `build-essential`, `pkg-config`, `libudev-dev`, `sqlite3`, `ca-certificates`, `curl`, and `git` as appropriate.
   - Rust toolchain install note.
4. `## Build and install the binary`
   - Include a source-build path, for example under `/opt/serialport-api`.
   - Include a recommended installed binary path such as `/usr/local/bin/serialport-api`.
5. `## Create a service user and data/config directories`
   - Recommend user `serialport-api` or similar.
   - Recommend config dir `/etc/serialport-api` and data dir `/var/lib/serialport-api`.
   - Include ownership and permissions notes.
6. `## Serial device permissions`
   - Document `dialout` group and reboot/log-out requirement.
   - Document `/dev/serial/by-id` and common Pi serial paths.
7. `## Example configuration`
   - Include TOML matching current code:

```toml
[server]
host = "0.0.0.0"
port = 4002

[serial]
default_port = "/dev/serial/by-id/usb-EXAMPLE"
default_baud_rate = 115200
default_delimiter = "\r\n"
real_serial = true
mock_device = false

[storage]
preset_db = "/var/lib/serialport-api/presets.db"
```

8. `## systemd service`
   - Include unit content or link to `examples/systemd/serialport-api.service`.
   - Unit should run the binary with `serve --config /etc/serialport-api/serialport-api.toml`.
   - Include restart policy and working directory.
   - Prefer unprivileged user and `dialout` supplementary group.
9. `## Install and manage the service`
   - Include `cp`, `systemctl daemon-reload`, `enable`, `start`, `status`, `journalctl` commands.
10. `## Manual smoke checks`
   - Health check:

```bash
curl -s http://127.0.0.1:4002/api/v1/health
```

   - Ports check:

```bash
curl -s http://127.0.0.1:4002/api/v1/ports
```

   - Preset SQLite persistence check using `POST /api/v1/presets`, service restart, then `GET /api/v1/presets`.
   - Real serial connect/command checks must be marked hardware-required and must use the configured device path.
11. `## Troubleshooting`
12. `## Security and network exposure notes`
   - Warn that binding to `0.0.0.0` exposes the unauthenticated API on the LAN.
   - Recommend localhost binding unless LAN clients are intentionally trusted, or use firewall/reverse proxy later.
13. `## Uninstall / rollback`
   - Include service stop/disable and removal commands.

### `examples/systemd/serialport-api.service`

Recommended unit shape:

```ini
[Unit]
Description=serialport-api JSON serial HTTP service
Documentation=https://github.com/avepha/serialport-api
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=serialport-api
Group=serialport-api
SupplementaryGroups=dialout
WorkingDirectory=/var/lib/serialport-api
ExecStart=/usr/local/bin/serialport-api serve --config /etc/serialport-api/serialport-api.toml
Restart=on-failure
RestartSec=2s
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=full
ProtectHome=true
ReadWritePaths=/var/lib/serialport-api

[Install]
WantedBy=multi-user.target
```

The implementer may adjust hardening if it breaks expected SQLite/config behavior, but should explain the choice in the docs.

### `examples/serialport-api.toml`

Must match actual `src/config.rs` field names exactly. Do not invent unsupported keys.

---

## Bite-Sized TDD / Docs-First Tasks

Because Phase 14 is documentation/example focused, use docs-validation checks rather than Rust behavior tests as the RED/GREEN loop. Still run the Rust test suite at the end to prove no regressions.

### Task 14.1: Establish baseline and stale-doc findings

RED/check first:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
grep -R "SQLite saved presets\|Raspberry Pi install guide\|systemd" -n README.md docs || true
```

Expected finding:

- README still contains stale planned SQLite/Raspberry Pi roadmap text.
- New Pi/systemd guide does not yet exist.

GREEN:

- Decide exact docs/examples to add using this handoff.
- Do not edit source code.

### Task 14.2: Add Raspberry Pi/systemd guide

RED/check first:

```bash
test -f docs/raspberry-pi-systemd.md
```

Expected RED reason:

- File is absent before this phase.

GREEN:

- Create `docs/raspberry-pi-systemd.md` with all required content-contract sections.
- Ensure all commands use current CLI/config semantics.
- Ensure hardware-dependent checks are explicitly labeled.

Validation:

```bash
grep -n "Raspberry Pi / systemd Deployment Guide" docs/raspberry-pi-systemd.md
grep -n "preset_db" docs/raspberry-pi-systemd.md
grep -n "SupplementaryGroups=dialout\|dialout" docs/raspberry-pi-systemd.md
grep -n "curl -s http://127.0.0.1:4002/api/v1/health" docs/raspberry-pi-systemd.md
grep -n "0.0.0.0" docs/raspberry-pi-systemd.md
grep -n "unauthenticated" docs/raspberry-pi-systemd.md
```

### Task 14.3: Add example service/config files

RED/check first:

```bash
test -f examples/systemd/serialport-api.service
test -f examples/serialport-api.toml
```

Expected RED reason:

- Example files are absent unless a prior agent already created them.

GREEN:

- Create `examples/systemd/serialport-api.service`.
- Create `examples/serialport-api.toml`.
- Keep examples consistent with the guide and actual config schema.

Validation:

```bash
grep -n "ExecStart=/usr/local/bin/serialport-api serve --config /etc/serialport-api/serialport-api.toml" examples/systemd/serialport-api.service
grep -n "SupplementaryGroups=dialout" examples/systemd/serialport-api.service
grep -n "ReadWritePaths=/var/lib/serialport-api" examples/systemd/serialport-api.service
grep -n "\[storage\]" examples/serialport-api.toml
grep -n "preset_db" examples/serialport-api.toml
```

### Task 14.4: Refresh README status and links

RED/check first:

```bash
grep -n "SQLite saved presets" README.md || true
grep -n "raspberry-pi-systemd.md" README.md || true
grep -n -- "--preset-db" README.md || true
```

Expected finding:

- SQLite may still be listed as planned.
- Raspberry Pi/systemd guide link may be absent.
- `--preset-db` may be absent.

GREEN:

- Update status/features/roadmap accurately for Phase 13 and Phase 14 docs.
- Add concise saved-presets/SQLite opt-in usage if missing.
- Add a link to `docs/raspberry-pi-systemd.md`.
- Do not rewrite unrelated README sections.

Validation:

```bash
grep -n "SQLite" README.md
grep -n "Raspberry Pi" README.md
grep -n "docs/raspberry-pi-systemd.md" README.md
grep -n -- "--preset-db" README.md
```

### Task 14.5: Full verification and review

Run:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
git diff -- README.md docs/raspberry-pi-systemd.md examples/systemd/serialport-api.service examples/serialport-api.toml
git status --short --branch
```

Expected:

- Cargo checks pass.
- Diff contains docs/example changes only.
- No source-code files are modified.

---

## Manual Smoke Checks for the Implementation Agent to Document

The new guide should contain commands equivalent to the following. The implementation agent does not need actual Raspberry Pi hardware in CI, but the instructions must be accurate enough for manual execution on a Pi.

### Service starts and health responds

```bash
sudo systemctl status serialport-api --no-pager
curl -s http://127.0.0.1:4002/api/v1/health
```

Expected health shape:

```json
{"status":"ok","version":"0.1.0"}
```

### Ports endpoint works without assuming attached hardware

```bash
curl -s http://127.0.0.1:4002/api/v1/ports
```

Expected:

- JSON object with a `ports` array.
- Array may be empty.

### SQLite presets persist through service restart

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/presets \
  -H 'content-type: application/json' \
  -d '{"name":"Read IMU","payload":{"method":"query","topic":"imu.read","data":{}}}'

sudo systemctl restart serialport-api

curl -s http://127.0.0.1:4002/api/v1/presets
```

Expected:

- Created preset remains after restart when `[storage] preset_db` points at a writable persistent DB path.

### Hardware-required real serial connection check

Only when a device is attached and the config path is correct:

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/serial/by-id/usb-EXAMPLE","baudRate":115200,"delimiter":"\r\n"}'
```

Expected:

- Success only if `real_serial = true`, the port exists, and permissions are correct.
- Failure should direct the user to troubleshooting, not imply the API is broken.

---

## Acceptance Criteria

Phase 14 is complete when all of the following are true:

- `docs/raspberry-pi-systemd.md` exists and includes every section in the required content contract.
- `README.md` no longer incorrectly says SQLite presets are planned/incomplete after Phase 13.
- `README.md` links to the Raspberry Pi/systemd guide.
- If example files are created, `examples/systemd/serialport-api.service` and `examples/serialport-api.toml` exactly match documented paths/field names or the guide explains any differences.
- Systemd docs run the service with `serve --config /etc/serialport-api/serialport-api.toml`.
- Serial permissions docs mention `dialout`, `SupplementaryGroups=dialout`, and `/dev/serial/by-id`.
- SQLite docs mention `--preset-db` and/or `[storage] preset_db` and writable data directory ownership.
- Security notes warn that the API is unauthenticated and binding to `0.0.0.0` exposes it on the network.
- Automated verification passes:
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all-features`
- Final `git diff --name-only` contains only expected docs/example files.
- Commit is created with a conventional commit message.

---

## Commit Guidance

Use a docs conventional commit. Recommended:

```bash
git add README.md docs/raspberry-pi-systemd.md examples/systemd/serialport-api.service examples/serialport-api.toml
git commit -m "docs: add Raspberry Pi systemd deployment guide"
```

If only docs are changed and examples are not created, adjust `git add` accordingly. Do not push unless explicitly instructed by the orchestrator.

---

## Risks and Mitigations

- **Stale README status after Phase 13:** README currently appears to still list SQLite presets as planned. Mitigate by inspecting current routes/config and updating status narrowly.
- **Unsupported config keys in examples:** `src/config.rs` only supports `[server]`, `[serial]`, and `[storage] preset_db`. Mitigate by copying field names exactly.
- **Running as root by default:** Avoid root service examples. Use a dedicated user with `dialout` supplementary group and writable data directory.
- **Serial paths change after reboot:** Prefer `/dev/serial/by-id/*`; document fallbacks and `ls -l` discovery commands.
- **Network exposure:** Binding `0.0.0.0` is convenient on a LAN but exposes an unauthenticated API. Document localhost as safer default and state that auth/TLS/firewall hardening is outside Phase 14.
- **systemd hardening breaks SQLite writes:** Include `ReadWritePaths=/var/lib/serialport-api` and ensure the guide sets ownership correctly.
- **Hardware availability:** Keep automated checks hardware-free; label real-serial smoke checks as manual/hardware-required.
- **Overreaching into releases/Docker:** Do not add Dockerfiles, CI release jobs, or cross-compilation automation in this phase.

---

## Recommended Final Implementation Summary

When done, report:

- What docs/examples were added.
- README stale-status corrections.
- Verification commands and pass/fail status.
- Commit SHA.
- Any deviations from this handoff, especially if any source-code file changed.
