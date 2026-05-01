# Phase 15 Release Binaries + Docker Handoff

> **For Hermes / next AI implementation session:** Execute this in a fresh session. Load `writing-plans`, `test-driven-development`, and any repository documentation/testing skills normally used for handoff execution before editing. This phase is packaging/release-readiness focused: add Docker runtime support, optional compose examples, and tag-triggered release automation for the existing Rust Axum `serialport-api` service. Do **not** implement WebSocket/Socket.IO, authentication, UI, or new API behavior in this phase.

**Goal:** Make the current rewrite easy to run without a local Rust toolchain and establish a safe, repeatable path for release binaries and Docker image builds. The output should let users build/run a container locally, optionally mount config/SQLite/device paths, and let maintainers publish GitHub release artifacts from tags.

**Inferred next phase:** Phase 15 is **release binaries / Docker image readiness**. Repository evidence supports this ordering:

- `README.md` currently lists `Release binaries / Docker image` as the next planned item and `WebSocket or Socket.IO support` as later work.
- Phase 14 added Raspberry Pi/systemd deployment docs and explicitly left release artifacts, Docker images, and cross-compilation automation for a later phase.
- Existing `.github/workflows/ci.yml` only runs format, clippy, and tests; no release workflow exists yet.
- `Cargo.toml` has package metadata, a binary named `serialport-api`, Rust 1.75 minimum, and current dependencies suitable for a containerized Linux build.
- No `Dockerfile`, `.dockerignore`, compose example, or release workflow is present.

---

## Strict Orchestration Input Schema

The implementation agent should accept this handoff plus the repository as its complete input. No hidden context is required.

```json
{
  "agent_role": "implementation",
  "phase": "Phase 15",
  "repository": "/home/alfarie/repos/serialport-api",
  "branch": "rewrite/axum-serial-api",
  "base_commit_expected": "e7357fb docs: add Raspberry Pi systemd deployment guide",
  "toolchain_env": {
    "PATH_prefix": "$HOME/.cargo/bin"
  },
  "scope": "Dockerfile, .dockerignore, Docker/compose usage docs, tag-triggered release workflow, README roadmap/status refresh, packaging validation only",
  "non_goals": [
    "Rust API/source behavior changes",
    "WebSocket or Socket.IO support",
    "authentication, TLS, reverse proxy, or firewall automation",
    "browser UI",
    "hardware-required automated tests",
    "publishing secrets or pushing tags/releases from the implementation session",
    "changing default mock/in-memory runtime behavior"
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
- Recent history includes `e7357fb docs: add Raspberry Pi systemd deployment guide` or a descendant of it.

If the working tree is not clean before Phase 15 edits, stop and report instead of modifying files.

---

## Strict Orchestration Output Schema

The implementation agent’s final response should use this JSON shape:

```json
{
  "agent_role": "implementation",
  "phase": "Phase 15",
  "summary": [
    "Added Docker build/runtime support for serialport-api.",
    "Added release workflow/docs for tag-triggered binary and image builds."
  ],
  "files_changed": [
    "Dockerfile",
    ".dockerignore",
    ".github/workflows/release.yml",
    "README.md",
    "docs/docker-release.md",
    "examples/docker-compose.yml"
  ],
  "verification": {
    "commands_run": [
      "cargo fmt --check",
      "cargo clippy --all-targets --all-features -- -D warnings",
      "cargo test --all-features",
      "docker build -t serialport-api:local .",
      "docker run --rm serialport-api:local --version",
      "docker run --rm -p 4002:4002 serialport-api:local serve --host 0.0.0.0 --port 4002",
      "curl -s http://127.0.0.1:4002/api/v1/health",
      "workflow/static docs validation checks listed in this handoff"
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

Known latest completed Phase 14 commit:

```text
e7357fb docs: add Raspberry Pi systemd deployment guide
```

Important current behavior after Phase 14:

- `cargo run -- serve --host 127.0.0.1 --port 4002` starts the Axum HTTP server in mock/in-memory mode.
- Default startup is hardware-free and does not open physical serial ports.
- `serve --real-serial` opts into opening/writing/reading OS serial ports.
- `serve --preset-db <PATH>` opts into SQLite-backed preset persistence.
- Optional config file loading exists via `serve --config <PATH>` and auto-discovered `./serialport-api.toml`.
- Config supports `[server]`, `[serial]`, and `[storage] preset_db = "..."`.
- The API exposes health, ports, connections, commands, SSE events, legacy aliases, and preset CRUD routes.
- Raspberry Pi/systemd docs and examples exist at `docs/raspberry-pi-systemd.md`, `examples/serialport-api.toml`, and `examples/systemd/serialport-api.service`.

Important local toolchain note:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Use that before all `cargo` commands in this WSL environment to avoid Rust toolchain mismatch.

---

## Phase 15 Scope

Do in Phase 15:

- Add a root `Dockerfile` for building and running `serialport-api`.
- Add a root `.dockerignore` that keeps builds reproducible and excludes heavy/local files such as `target/`, `.git/`, local DB files, editor metadata, and logs.
- Prefer a multi-stage Dockerfile:
  - Builder image: official Rust image with a toolchain compatible with Rust 1.75+.
  - Runtime image: Debian slim or distroless-like image that includes only required runtime packages/certificates for the compiled binary.
  - Binary path: `/usr/local/bin/serialport-api`.
  - Default exposed port: `4002`.
  - Default command should start the server in container-friendly mock mode, e.g. `serve --host 0.0.0.0 --port 4002`.
- Decide whether the runtime image needs OS packages for serial device enumeration/access. If using Debian slim, include at least `ca-certificates` and any serialport/libudev runtime dependency found necessary by actual `docker run` smoke checks.
- Add a concise Docker/release guide, recommended path `docs/docker-release.md`, covering:
  - local image build,
  - mock-mode run,
  - config-file mount,
  - SQLite preset DB volume mount,
  - real serial device pass-through on Linux with `--device` and group/permission notes,
  - compose usage if added,
  - limitations on macOS/Windows/WSL hardware pass-through,
  - release-tag workflow behavior.
- Add `examples/docker-compose.yml` if it makes the documented config/volume flow clearer. Keep it optional and safe by default; do not require real hardware.
- Add a GitHub Actions release workflow if feasible and safe:
  - Trigger only on version tags such as `v*`.
  - Build/test first.
  - Produce Linux release binaries for at least `x86_64-unknown-linux-gnu` if cross-target setup is reliable.
  - Prefer also adding `aarch64-unknown-linux-gnu` and/or `armv7-unknown-linux-gnueabihf` only if the workflow can install/link dependencies reliably without making CI brittle.
  - Build and optionally publish a Docker image to GHCR using `docker/build-push-action`.
  - Use least-privilege permissions needed for releases/packages.
  - Do not require secrets beyond the standard `GITHUB_TOKEN`.
  - Do not make normal CI depend on Docker publishing.
- Update `README.md` only enough to:
  - document Docker quick start,
  - link to the new Docker/release guide,
  - mark release binaries / Docker image support appropriately in the roadmap/status,
  - keep WebSocket/Socket.IO listed as future work.
- Keep all automated checks hardware-free.

Out of scope / do **not** do in Phase 15:

- Do not change Rust route behavior, serial manager behavior, storage behavior, config precedence, or public API schemas.
- Do not add WebSocket or Socket.IO endpoints.
- Do not add authentication, TLS termination, reverse-proxy config, firewall automation, or a browser UI.
- Do not make SQLite mandatory for default startup.
- Do not require connected serial hardware in CI or automated tests.
- Do not push tags, create GitHub releases manually, publish packages from the local machine, or require repository secrets.
- Do not alter the existing Phase 14 systemd deployment model except for README links/cross-references if needed.
- Do not rename existing endpoints, command flags, binary name, package name, or legacy aliases.

---

## Expected Files to Inspect Before Editing

Read these first:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"

# Repository docs and roadmap
sed -n '1,520p' README.md
sed -n '1,220p' docs/phase-14-raspberry-pi-systemd-handoff.md
sed -n '1,360p' docs/raspberry-pi-systemd.md
sed -n '450,510p' docs/open-source-spec.md

# Current CI/workflow baseline
sed -n '1,200p' .github/workflows/ci.yml

# Current package/binary/config behavior
sed -n '1,120p' Cargo.toml
sed -n '1,220p' src/main.rs
sed -n '1,230p' src/config.rs

# Current examples that Docker docs may reference
sed -n '1,120p' examples/serialport-api.toml
sed -n '1,160p' examples/systemd/serialport-api.service
```

Use `read_file`/`search_files` equivalents if operating through tools that prohibit shell readers.

---

## Expected Files to Modify or Create

Required:

- Create: `Dockerfile`
  - Multi-stage build and runtime image for `serialport-api`.
  - Must be usable from repository root with `docker build -t serialport-api:local .`.
  - Must run the server by default on `0.0.0.0:4002`.

- Create: `.dockerignore`
  - Must exclude local build artifacts and non-source data that should not enter the Docker build context.

- Create: `docs/docker-release.md`
  - Main Docker and release usage guide.
  - Must be complete enough for a new user/developer to build/run locally without this handoff.

- Modify: `README.md`
  - Add concise Docker quick start and link to `docs/docker-release.md`.
  - Refresh roadmap/status so Docker/release support is no longer listed as fully absent after implementation.

Strongly recommended:

- Create: `examples/docker-compose.yml`
  - Demonstrate mock-mode container startup with port mapping and volume mounts for optional config/data.
  - Keep real serial device pass-through commented or documented, not enabled by default.

- Create: `.github/workflows/release.yml`
  - Tag-triggered release workflow for binary artifacts and Docker image build/publish.

Optional only if justified by implementation findings:

- Create: `docs/release-process.md`
  - Only if release workflow usage would make `docs/docker-release.md` too long. Prefer one guide unless it becomes unwieldy.

Files not expected to change:

- `src/**`
- `Cargo.toml`
- `Cargo.lock`
- `examples/systemd/serialport-api.service`
- `docs/raspberry-pi-systemd.md`

If any source or Cargo file changes become necessary, document the reason explicitly in the final output and keep the change tiny. A Docker-only phase should normally not need Rust code changes.

---

## Required Content Contract

### `Dockerfile`

Recommended shape:

```dockerfile
# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/serialport-api /usr/local/bin/serialport-api
EXPOSE 4002
ENTRYPOINT ["/usr/local/bin/serialport-api"]
CMD ["serve", "--host", "0.0.0.0", "--port", "4002"]
```

The implementer may improve this for caching or runtime dependencies, but must verify it with actual Docker commands. If the runtime binary fails due to missing dynamic libraries, either install the minimal needed packages in the runtime image or choose a different safe runtime strategy and document why.

Do not use Alpine/musl unless the implementation agent verifies the `serialport` and SQLite dependencies work reliably in that image.

### `.dockerignore`

Must include at least:

```gitignore
.git
.gitignore
target
Dockerfile
.dockerignore
*.db
*.sqlite
*.sqlite3
*.log
.env
.idea
.vscode
```

It is acceptable to include docs/examples in the build context if the Dockerfile only copies needed files. Do not ignore `Cargo.lock`.

### `docs/docker-release.md`

The guide should include these sections, in this order unless there is a good reason to reorder:

1. `# Docker and Release Guide`
2. `## Supported deployment model`
   - State Docker is useful for mock/API testing and Linux hosts with device pass-through.
   - State Raspberry Pi/systemd remains documented separately.
3. `## Build a local image`

```bash
docker build -t serialport-api:local .
```

4. `## Run in default mock mode`

```bash
docker run --rm -p 4002:4002 serialport-api:local
curl -s http://127.0.0.1:4002/api/v1/health
```

5. `## Use a config file and SQLite preset volume`
   - Show mounting `examples/serialport-api.toml` or a user config into `/config/serialport-api.toml`.
   - Show mounting a data directory into `/data` and setting `[storage] preset_db = "/data/presets.db"`.
   - Show command with `serve --config /config/serialport-api.toml`.
6. `## Real serial devices in Docker`
   - Mark Linux-only/hardware-required.
   - Explain `--device=/dev/ttyUSB0:/dev/ttyUSB0` or stable `/dev/serial/by-id` paths.
   - Explain permissions may require `--group-add`, host `dialout` group, or a udev/ownership adjustment.
   - Warn that Docker Desktop on macOS/Windows and WSL may not expose physical serial devices like a native Linux host.
7. `## Docker Compose example`
   - Link to `examples/docker-compose.yml` if added.
8. `## Release workflow`
   - Document trigger tag pattern, e.g. pushing `v0.1.0`.
   - Document expected artifacts/images.
   - State the implementation session should not push tags or publish releases manually.
9. `## Manual smoke checks`
10. `## Troubleshooting`
    - Port already in use.
    - Container starts but health endpoint unreachable due to host binding/port mapping.
    - Config path mounted incorrectly.
    - SQLite DB directory not writable.
    - Serial device permission denied or missing device path.
    - Runtime image missing shared library, if encountered and fixed.
11. `## Security and network exposure notes`
    - Warn that binding container to host ports exposes the unauthenticated API to reachable networks depending on Docker/host firewall settings.

### `examples/docker-compose.yml`

Recommended default should be hardware-free:

```yaml
services:
  serialport-api:
    build:
      context: ..
      dockerfile: Dockerfile
    image: serialport-api:local
    ports:
      - "4002:4002"
    command: ["serve", "--host", "0.0.0.0", "--port", "4002"]
    # For config + SQLite persistence, create a config that uses /data/presets.db:
    # volumes:
    #   - ./serialport-api.toml:/config/serialport-api.toml:ro
    #   - serialport-api-data:/data
    # command: ["serve", "--config", "/config/serialport-api.toml"]
    # For real serial on native Linux, also add a device mapping, for example:
    # devices:
    #   - "/dev/ttyUSB0:/dev/ttyUSB0"

# volumes:
#   serialport-api-data:
```

If the compose file is placed under `examples/`, ensure its build context path is correct when running from that directory. Document the exact command, for example:

```bash
docker compose -f examples/docker-compose.yml up --build
```

### `.github/workflows/release.yml`

Recommended workflow requirements:

- Trigger:

```yaml
on:
  push:
    tags:
      - "v*"
```

- Permissions should be least privilege, typically:

```yaml
permissions:
  contents: write
  packages: write
```

- Must run Rust verification before packaging:
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all-features`
- Must use `cargo build --release --locked` for binaries.
- Artifact names should include version/tag and target where relevant, for example:
  - `serialport-api-${{ github.ref_name }}-x86_64-unknown-linux-gnu.tar.gz`
- If Docker image publishing is included:
  - Use `docker/login-action` with `registry: ghcr.io` and `GITHUB_TOKEN`.
  - Use `docker/metadata-action` and `docker/build-push-action`.
  - Tags should include the git tag and possibly `latest` only for semver release tags; avoid `latest` on arbitrary pre-release tags unless intentionally documented.
- If cross-compilation for ARM is added, document dependencies and keep the workflow maintainable. If it proves brittle, limit Phase 15 release binaries to native Linux x86_64 and document ARM/Raspberry Pi binaries as future work.

---

## Bite-Sized TDD / Docs-Validation Tasks

Because Phase 15 is packaging/docs/workflow focused, use validation checks as the RED/GREEN loop. Still run the Rust suite at the end to prove no regressions.

### Task 15.1: Establish baseline and missing packaging findings

RED/check first:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
test -f Dockerfile
test -f .dockerignore
test -f docs/docker-release.md
test -f .github/workflows/release.yml
grep -R "Release binaries / Docker image\|Docker" -n README.md docs || true
```

Expected finding before implementation:

- `Dockerfile`, `.dockerignore`, `docs/docker-release.md`, and release workflow are absent.
- README lists release binaries / Docker as planned.

GREEN:

- Decide exact files to add using this handoff.
- Do not edit Rust source code.

### Task 15.2: Add Dockerfile and .dockerignore

RED/check first:

```bash
test -f Dockerfile
test -f .dockerignore
```

Expected RED reason:

- Files are absent before this phase.

GREEN:

- Add a multi-stage Dockerfile and .dockerignore.
- Keep Dockerfile comments minimal and maintainable.
- Build with locked dependencies.

Validate:

```bash
docker build -t serialport-api:local .
docker run --rm serialport-api:local --version
```

### Task 15.3: Container smoke test default mock mode

RED/check first:

```bash
docker run --rm -p 4002:4002 serialport-api:local
```

Expected initial failure before GREEN may include missing image or missing runtime dependency.

GREEN:

- Start the container on `0.0.0.0:4002`.
- In another shell, run:

```bash
curl -s http://127.0.0.1:4002/api/v1/health
curl -s http://127.0.0.1:4002/api/v1/ports
```

Expected:

- Health returns `{"status":"ok","version":"0.1.0"}` or the current package version.
- Ports returns a JSON object with a `ports` array; it may be empty.

### Task 15.4: Add Docker/compose/release docs

RED/check first:

```bash
test -f docs/docker-release.md
grep -n "Docker and Release Guide" docs/docker-release.md
grep -n "docker build -t serialport-api:local" docs/docker-release.md
grep -n "--device" docs/docker-release.md
grep -n "v\*" docs/docker-release.md
```

Expected RED reason:

- Guide is absent before this phase.

GREEN:

- Add `docs/docker-release.md` with the required content contract.
- Add `examples/docker-compose.yml` if useful, and document exact run command.
- Keep hardware-required sections clearly marked manual/optional.

### Task 15.5: Add release workflow

RED/check first:

```bash
test -f .github/workflows/release.yml
grep -n "tags:" .github/workflows/release.yml
grep -n "cargo build --release --locked" .github/workflows/release.yml
```

Expected RED reason:

- Workflow is absent before this phase.

GREEN:

- Add `.github/workflows/release.yml`.
- Keep it tag-triggered only.
- Ensure it does not require external secrets beyond `GITHUB_TOKEN`.
- Ensure normal branch pushes continue to use `.github/workflows/ci.yml` as before.

Validate statically:

```bash
python - <<'PY'
from pathlib import Path
p = Path('.github/workflows/release.yml')
text = p.read_text()
required = ['tags:', 'cargo fmt --check', 'cargo clippy --all-targets --all-features -- -D warnings', 'cargo test --all-features', 'cargo build --release --locked']
missing = [s for s in required if s not in text]
if missing:
    raise SystemExit(f'missing release workflow snippets: {missing}')
print('release workflow static check passed')
PY
```

If `actionlint` is available, also run:

```bash
actionlint .github/workflows/release.yml
```

Do not install extra global tooling solely for `actionlint` unless already standard in the environment.

### Task 15.6: Refresh README

RED/check first:

```bash
grep -n "Release binaries / Docker image" README.md || true
grep -n "docker build" README.md || true
grep -n "docs/docker-release.md" README.md || true
```

Expected finding before README edits:

- Docker/release is only listed as planned and no Docker quick start exists.

GREEN:

- Add a concise Docker quick-start section or subsection.
- Link to `docs/docker-release.md`.
- Update roadmap/status accurately after the new packaging files exist.
- Keep WebSocket/Socket.IO as later/future work.

### Task 15.7: Final regression and packaging verification

Run:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
docker build -t serialport-api:local .
docker run --rm serialport-api:local --version
```

Then run a server smoke check:

```bash
docker run --rm -p 4002:4002 serialport-api:local
```

In another shell:

```bash
curl -s http://127.0.0.1:4002/api/v1/health
curl -s http://127.0.0.1:4002/api/v1/ports
```

Stop the container after smoke checks.

If Docker is unavailable in the execution environment, do **not** fake the result. Mark Docker runtime verification as blocked/unavailable in the final output, but still provide static Dockerfile/docs/workflow validation and Rust checks.

---

## Acceptance Criteria

Phase 15 is complete when all of the following are true:

- A root `Dockerfile` exists and builds a runnable `serialport-api` image from repository root.
- A root `.dockerignore` exists and excludes local build artifacts/data without excluding `Cargo.lock`.
- The container starts in default mock mode on `0.0.0.0:4002` and the health endpoint is reachable through host port mapping.
- Docker docs explain mock mode, config mounts, SQLite data volume, and Linux real-serial device pass-through.
- README has a concise Docker quick start and links to the detailed guide.
- A tag-triggered release workflow exists or, if deliberately deferred due to a concrete blocker, the blocker is documented clearly in the final output and docs avoid claiming release automation is complete.
- Existing Rust checks pass: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features`.
- No source/API behavior changes were made unless explicitly justified.
- Hardware-required real serial behavior remains manual and optional.
- Working tree is clean after committing.

---

## Manual Smoke Checks

Run these after implementation when Docker is available:

```bash
# Build the image.
docker build -t serialport-api:local .

# Confirm the binary can start and report version.
docker run --rm serialport-api:local --version

# Start API in default mock mode.
docker run --rm -p 4002:4002 serialport-api:local
```

In a second shell:

```bash
curl -s http://127.0.0.1:4002/api/v1/health
curl -s http://127.0.0.1:4002/api/v1/ports
curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/ROBOT","baudRate":115200,"delimiter":"\r\n"}'
curl -s -X DELETE http://127.0.0.1:4002/api/v1/connections/default
```

Optional SQLite/config smoke check:

```bash
mkdir -p /tmp/serialport-api-data
cat > /tmp/serialport-api-container.toml <<'TOML'
[server]
host = "0.0.0.0"
port = 4002

[storage]
preset_db = "/data/presets.db"
TOML

docker run --rm \
  -p 4002:4002 \
  -v /tmp/serialport-api-container.toml:/config/serialport-api.toml:ro \
  -v /tmp/serialport-api-data:/data \
  serialport-api:local serve --config /config/serialport-api.toml
```

In a second shell:

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/presets \
  -H 'content-type: application/json' \
  -d '{"name":"Read IMU","payload":{"method":"query","topic":"imu.read","data":{}}}'
```

Stop/restart the container with the same volume and verify:

```bash
curl -s http://127.0.0.1:4002/api/v1/presets
```

Optional real serial check on a native Linux host with hardware attached:

```bash
docker run --rm \
  -p 4002:4002 \
  --device=/dev/ttyUSB0:/dev/ttyUSB0 \
  serialport-api:local serve --host 0.0.0.0 --port 4002 --real-serial
```

Then connect using the existing API with `port` set to `/dev/ttyUSB0`. This is manual only and must not be required in CI.

---

## Risks and Guidance

- **Native dependency risk:** `serialport` may need `libudev` at build time and/or runtime on Linux. If a slim runtime image fails to list ports or start due to missing shared libraries, inspect the error and add the minimal Debian runtime package. Document any added package.
- **SQLite linking risk:** `rusqlite` currently uses the `bundled` feature, which helps avoid runtime SQLite library dependencies. Do not remove it.
- **Cross-compilation risk:** ARM release binaries may require cross-linkers and additional packages. Prefer a reliable x86_64 release workflow over brittle multi-arch binaries. Document ARM binaries as future work if not completed.
- **Docker hardware risk:** Serial device pass-through is host/platform-specific. Keep real serial examples Linux/manual and preserve mock mode as the default.
- **Security risk:** The API currently has no authentication. Docker examples binding to host ports can expose it to the LAN depending on Docker/host firewall settings. Document this clearly.
- **Workflow permissions risk:** Release workflows with `contents: write` and `packages: write` should trigger only on tags, not all branch pushes.
- **Context size risk:** Do not copy the entire repository into Docker layers unnecessarily. Use `.dockerignore` and targeted `COPY` statements.

---

## Commit Guidance

Expected conventional commit message:

```text
chore: add Docker and release packaging
```

If the implementation is docs-heavy and release workflow is deferred due to a real blocker, use a narrower message such as:

```text
docs: add Docker release guide
```

Before committing:

```bash
git status --short
git diff -- Dockerfile .dockerignore README.md docs/docker-release.md examples/docker-compose.yml .github/workflows/release.yml
```

Commit only intentional Phase 15 files. Do not include `target/`, local SQLite databases, temporary config files, generated archives, or release artifacts.

After committing:

```bash
git status --short --branch
git log --oneline -1
```

Do not push.

---

## Next-Agent One-Paragraph Instruction

Implement Phase 15 release/Docker readiness in `/home/alfarie/repos/serialport-api` on branch `rewrite/axum-serial-api`: add a Dockerfile, .dockerignore, Docker/release docs, optional compose example, tag-triggered release workflow, and README updates. Keep Rust source/API behavior unchanged, keep all tests hardware-free, verify with Rust checks plus Docker build/run smoke checks when Docker is available, commit with a conventional commit, and do not push.
