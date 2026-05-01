# Phase 18 ARM / Raspberry Pi Release Binary Automation Handoff

> **For Hermes / next AI implementation session:** Execute this in a fresh session. Load `writing-plans`, `test-driven-development`, and any repository workflow/release-engineering skills normally used for GitHub Actions work before editing. This phase is release automation only: add automated Linux release binary builds for Raspberry Pi/ARM targets while preserving existing runtime behavior. Keep all tests hardware-free. Do **not** change API, serial, storage, WebSocket, Socket.IO, Docker runtime behavior, or systemd service behavior in this phase.

**Goal:** Extend the existing tag-triggered release automation so maintainers can publish deterministic prebuilt Linux binaries for Raspberry Pi/ARM users, alongside the existing Linux x86_64 artifact and GHCR image. The implementation should make release artifacts predictable, checksummed, documented, and verifiable without requiring real Raspberry Pi hardware in CI.

**Inferred next phase:** Phase 18 is **ARM/Raspberry Pi release binary automation**. Repository evidence supports this as the next appropriate phase after Phase 17:

- Phase 17 completed Socket.IO event compatibility at commit `d19333a feat: add Socket.IO event compatibility`.
- `README.md` now lists only one planned/not complete item: `ARM/Raspberry Pi release binary automation`.
- `docs/docker-release.md` explicitly says ARM/Raspberry Pi release binaries are future work because cross-linking native dependencies should be added deliberately.
- `.github/workflows/release.yml` currently builds one Linux binary artifact named `serialport-api-${GITHUB_REF_NAME}-x86_64-unknown-linux-gnu.tar.gz` and publishes a GHCR image.
- `docs/open-source-spec.md` lists optional release automation for Linux x86_64 plus Raspberry Pi ARMv7/aarch64 binaries.
- Phase 14 added Raspberry Pi/systemd deployment docs with a copy-existing-binary path, and Phase 15 added initial x86_64 release/Docker packaging. Phase 18 should connect those by adding Pi-compatible release binaries.

---

## Strict Orchestration Input Schema

The implementation agent should accept this handoff plus the repository as its complete input. No hidden context is required.

```json
{
  "agent_role": "implementation",
  "phase": "Phase 18",
  "repository": "/home/alfarie/repos/serialport-api",
  "branch": "rewrite/axum-serial-api",
  "base_commit_expected": "d19333a feat: add Socket.IO event compatibility",
  "toolchain_env": {
    "PATH_prefix": "$HOME/.cargo/bin"
  },
  "scope": "Add GitHub Actions release binary automation for Linux x86_64 plus Raspberry Pi/ARM targets, with deterministic archives, checksums, documentation, and hardware-free verification",
  "required_artifact_to_read": "docs/phase-18-arm-release-automation-handoff.md",
  "non_goals": [
    "Changing Rust runtime/API behavior",
    "Changing serial manager, protocol, config, preset storage, WebSocket, Socket.IO, legacy alias, Docker runtime, or systemd service behavior",
    "Requiring physical Raspberry Pi hardware or serial devices in CI",
    "Publishing releases, pushing tags, pushing commits, or manually uploading artifacts from the local machine",
    "Adding package-manager installers such as .deb, apt repositories, Homebrew, or systemd installation scripts",
    "Adding authentication, TLS, reverse proxy, firewall automation, UI, or cloud deployment",
    "Making ARM artifacts block normal branch CI",
    "Replacing the existing GHCR image publishing path unless needed for release workflow maintainability"
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
- Recent history includes `d19333a feat: add Socket.IO event compatibility` or a descendant of it.

If the working tree is not clean before Phase 18 edits, stop and report instead of modifying files.

---

## Strict Orchestration Output Schema

The implementation agent's final response must use this JSON shape:

```json
{
  "agent_role": "implementation",
  "phase": "Phase 18",
  "summary": [
    "Added ARM/Raspberry Pi Linux release binary automation to the tag-triggered release workflow.",
    "Documented deterministic artifact names, checksums, target support, and Pi install usage."
  ],
  "files_changed": [
    ".github/workflows/release.yml",
    "README.md",
    "docs/docker-release.md",
    "docs/raspberry-pi-systemd.md"
  ],
  "verification": {
    "commands_run": [
      "cargo fmt --check",
      "cargo clippy --all-targets --all-features -- -D warnings",
      "cargo test --all-features",
      "cargo check --release --target x86_64-unknown-linux-gnu",
      "cargo check --release --target aarch64-unknown-linux-gnu",
      "workflow/static validation checks listed in this handoff"
    ],
    "status": "passed"
  },
  "commit": "<sha or null>",
  "approval_status": "ready_for_review|blocked|deferred",
  "issues": []
}
```

If blocked, set `commit` to `null`, `approval_status` to `blocked`, and list exact blockers. If ARMv7 is deliberately deferred while aarch64 succeeds, set `approval_status` to `ready_for_review` only if docs clearly mark ARMv7 as optional/future and all required acceptance criteria pass. If all ARM targets must be deferred due to concrete toolchain/linking blockers, update docs only to record the deferral, commit with a `docs:` message, and set `approval_status` to `deferred`.

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
d19333a feat: add Socket.IO event compatibility
```

Current release/package behavior:

- `.github/workflows/ci.yml` runs on branches/PRs and performs:
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all-features`
- `.github/workflows/release.yml` runs only on tags matching `v*`.
- Current release workflow jobs:
  - `verify`: Rust format/clippy/test.
  - `linux-binary`: native Ubuntu x86_64 release binary using `cargo build --release --locked`, packaging `README.md` and `LICENSE` into a `.tar.gz`, plus `.sha256`.
  - `docker-image`: GHCR image build/publish.
- Current release artifact pattern:
  - `serialport-api-${GITHUB_REF_NAME}-x86_64-unknown-linux-gnu.tar.gz`
  - `serialport-api-${GITHUB_REF_NAME}-x86_64-unknown-linux-gnu.tar.gz.sha256`
- Current Docker runtime already installs `libudev1`; builder installs `pkg-config libudev-dev`.
- Current `Cargo.toml` package metadata:
  - package/binary name: `serialport-api`
  - version: `0.1.0`
  - Rust edition: `2021`
  - `rust-version = "1.75"`
  - native dependency risk: `serialport = "4"` commonly needs Linux udev development libraries when dynamically linked.
  - `rusqlite = { version = "0.31", features = ["bundled"] }`, which reduces runtime SQLite system-library risk.
- Current docs:
  - `README.md` marks ARM/Raspberry Pi release binary automation as planned.
  - `docs/docker-release.md` documents x86_64 release artifacts and says ARM/Pi artifacts are future work.
  - `docs/raspberry-pi-systemd.md` documents building from source or copying an existing compatible binary.
  - `docs/open-source-spec.md` lists optional Linux x86_64, Raspberry Pi ARMv7/aarch64, and Docker release automation.

Important local toolchain note:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Use that before all `cargo` commands in this WSL environment.

---

## Phase 18 Scope

Do in Phase 18:

- Extend release automation to build Linux binaries for Raspberry Pi/ARM targets without hardware in CI.
- Keep the existing x86_64 Linux release artifact working.
- Prefer adding at least this required target:
  - `aarch64-unknown-linux-gnu` for Raspberry Pi OS 64-bit / ARM64 Linux.
- Keep this target strongly recommended if feasible and reliable:
  - `x86_64-unknown-linux-gnu` as the existing Linux desktop/server baseline.
- Treat this target as optional, allowed only if the implementation remains maintainable and verified:
  - `armv7-unknown-linux-gnueabihf` for Raspberry Pi OS 32-bit / ARMv7 hard-float Linux.
- Install/use the correct Rust targets and cross-linker/sysroot dependencies in GitHub Actions.
- Package artifacts with deterministic names that include tag and target triple.
- Generate a separate SHA-256 checksum file for every binary archive.
- Ensure archives contain at least:
  - `serialport-api` executable for the target.
  - `README.md`.
  - `LICENSE`.
  - Optionally a short `ARTIFACT.txt` or `VERSION.txt` file containing tag, target triple, and commit SHA if helpful.
- Update docs so users can choose the correct Raspberry Pi artifact and install it with the Phase 14 systemd guide.
- Keep local and CI verification hardware-free.
- Keep release workflow trigger constrained to version tags and/or a safe manual dispatch.
- Preserve least-privilege GitHub Actions permissions.

Out of scope / do **not** do in Phase 18:

- Do not change `src/**` runtime/API behavior.
- Do not alter health, ports, connections, command, presets, SSE, native WebSocket, Socket.IO, legacy aliases, config, mock mode, real serial semantics, or event schemas.
- Do not require a real Raspberry Pi, serial adapter, `/dev/tty*`, or `/dev/serial/by-id/*` in automated tests.
- Do not publish a release, push a tag, push commits, or require maintainers to add third-party secrets.
- Do not add package manager installers (`.deb`, apt repository, Homebrew, Nix, etc.).
- Do not redesign Docker images or change the runtime image unless a tiny workflow-only adjustment is needed and documented.
- Do not require Docker multi-arch image publishing for Phase 18. Multi-arch Docker manifests are optional future work unless trivially supported without destabilizing existing GHCR publishing.
- Do not remove the existing x86_64 artifact or GHCR publishing job.

---

## Expected Files to Inspect Before Editing

Read these first:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"

# Roadmap and docs
README.md
docs/docker-release.md
docs/raspberry-pi-systemd.md
docs/open-source-spec.md
docs/phase-15-release-docker-handoff.md
docs/phase-14-raspberry-pi-systemd-handoff.md
docs/phase-17-socketio-compatibility-handoff.md

# Workflows and packaging
.github/workflows/release.yml
.github/workflows/ci.yml
Dockerfile
.dockerignore
examples/docker-compose.yml

# Cargo metadata and dependency/linking context
Cargo.toml
Cargo.lock
```

Use `read_file`/`search_files` equivalents if operating through tools that prohibit shell readers.

Key strings to search:

```bash
ARM/Raspberry
Raspberry Pi release
x86_64-unknown-linux-gnu
aarch64-unknown-linux-gnu
armv7-unknown-linux-gnueabihf
cargo build --release --locked
sha256sum
libudev
pkg-config
```

---

## Expected Files to Modify or Create

Required:

- Modify: `.github/workflows/release.yml`
  - Add ARM/Raspberry Pi binary build automation.
  - Keep existing tag release and GHCR behavior.
  - Keep verification job before publishing.
  - Upload all archives and checksum files to the GitHub release.

- Modify: `README.md`
  - Move ARM/Raspberry Pi release binary automation from planned/not complete to implemented once workflow support exists.
  - Add a concise release artifact note naming supported target triples.
  - Link to `docs/docker-release.md` and/or `docs/raspberry-pi-systemd.md` for installation details.

- Modify: `docs/docker-release.md`
  - Update the release workflow section to list ARM/Pi artifacts.
  - Explain artifact naming and checksum files.
  - Document that ARM releases are built in CI without Pi hardware.
  - Remove or replace the old statement that ARM/Pi binaries are future work, unless only ARMv7 remains future/optional.

- Modify: `docs/raspberry-pi-systemd.md`
  - Update the install/copy-binary path to include downloading the correct GitHub release tarball for 64-bit Raspberry Pi OS (`aarch64-unknown-linux-gnu`) and, if implemented, 32-bit Raspberry Pi OS (`armv7-unknown-linux-gnueabihf`).
  - Include checksum verification before installing the binary.
  - Keep source-build instructions as a fallback.

Likely not needed:

- `Cargo.toml` / `Cargo.lock`: Do not change unless a dedicated cross-compilation helper crate/tool dependency is absolutely necessary. Prefer workflow-level tooling over project dependency changes.
- `Dockerfile`: Do not change unless a documented release build strategy reuses Docker build stages and requires a minimal, verified adjustment.

Optional only if justified:

- Create: `docs/release-artifacts.md`
  - Only if release artifact instructions would make existing docs too long. Prefer updating existing docs rather than adding a new guide.

Files not expected to change:

- `src/**`
- `examples/systemd/serialport-api.service`
- `examples/serialport-api.toml`
- `examples/docker-compose.yml`
- `.github/workflows/ci.yml` unless there is a compelling reason to add static workflow validation only. ARM release checks should normally stay in `release.yml`.

If any unexpected file changes become necessary, document the exact reason in the final output and keep the change minimal.

---

## Release Artifact Contract

### Required target triples

At minimum, Phase 18 should produce these Linux binary archives on release tags:

```text
x86_64-unknown-linux-gnu
aarch64-unknown-linux-gnu
```

The existing x86_64 target already exists and must remain supported. `aarch64-unknown-linux-gnu` is the primary Raspberry Pi 64-bit target and is required unless a concrete blocker is discovered and documented.

Optional if feasible:

```text
armv7-unknown-linux-gnueabihf
```

`armv7-unknown-linux-gnueabihf` is useful for Raspberry Pi OS 32-bit, but should not be added if linker/sysroot setup is brittle or unverified.

### Artifact names

Use deterministic archive names:

```text
serialport-api-${TAG}-${TARGET}.tar.gz
serialport-api-${TAG}-${TARGET}.tar.gz.sha256
```

Examples for tag `v0.1.0`:

```text
serialport-api-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
serialport-api-v0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256
serialport-api-v0.1.0-aarch64-unknown-linux-gnu.tar.gz
serialport-api-v0.1.0-aarch64-unknown-linux-gnu.tar.gz.sha256
serialport-api-v0.1.0-armv7-unknown-linux-gnueabihf.tar.gz
serialport-api-v0.1.0-armv7-unknown-linux-gnueabihf.tar.gz.sha256
```

### Archive contents

Each archive should expand to one top-level directory, preferably:

```text
serialport-api/
  serialport-api
  README.md
  LICENSE
```

Optional metadata file:

```text
serialport-api/
  ARTIFACT.txt
```

If added, `ARTIFACT.txt` should contain deterministic metadata such as:

```text
name=serialport-api
tag=${GITHUB_REF_NAME}
target=${TARGET}
commit=${GITHUB_SHA}
```

Do not include `target/`, source trees, local databases, workflow logs, or generated release artifacts inside the archive.

### Checksum contract

Generate SHA-256 checksums with the archive filename in the checksum file:

```bash
sha256sum "$archive" > "$archive.sha256"
```

Users should be able to verify with:

```bash
sha256sum -c serialport-api-v0.1.0-aarch64-unknown-linux-gnu.tar.gz.sha256
```

---

## GitHub Actions Guidance

Recommended implementation options, in preference order:

### Option A: Matrix job with maintained cross toolchain action

Use a matrix for target triples and a maintained cross-compilation setup action such as `taiki-e/setup-cross-toolchain-action` if it cleanly supports the selected targets and Rust 1.75+ compatibility. This often manages cross-linker/sysroot details better than ad-hoc apt packages.

Matrix sketch:

```yaml
strategy:
  fail-fast: false
  matrix:
    include:
      - target: x86_64-unknown-linux-gnu
        os: ubuntu-latest
      - target: aarch64-unknown-linux-gnu
        os: ubuntu-latest
      # optional only if verified:
      # - target: armv7-unknown-linux-gnueabihf
      #   os: ubuntu-latest
```

Then build:

```bash
cargo build --release --locked --target "$TARGET"
```

Package from:

```text
target/${TARGET}/release/serialport-api
```

For x86_64 native builds, either continue using `target/release/serialport-api` or normalize all targets to `--target x86_64-unknown-linux-gnu` so packaging paths are identical.

### Option B: Matrix job with explicit apt cross-linkers

If avoiding a third-party action, install target-specific packages directly. Likely packages:

```bash
sudo apt-get update
sudo apt-get install -y --no-install-recommends pkg-config libudev-dev gcc-aarch64-linux-gnu g++-aarch64-linux-gnu
rustup target add aarch64-unknown-linux-gnu
```

Add Cargo linker config through environment variables or `.cargo/config.toml` generated in the workflow, not committed unless necessary:

```bash
mkdir -p .cargo
cat > .cargo/config.toml <<'TOML'
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
TOML
```

Potential complication: `serialport`/`libudev-sys` may need target `pkg-config` metadata/sysroot setup, not just a compiler. If direct apt linking becomes brittle, use Option A or defer the target with clear rationale rather than committing a flaky workflow.

### Option C: Use `cross` only if maintainable

The `cross` tool can simplify native-dependency cross builds, but introducing it in GitHub Actions is acceptable only if:

- It is pinned or installed deterministically.
- It builds `serialport-api` for required targets with `--locked`.
- Artifact paths and checksums remain deterministic.
- Docs mention no local dependency on `cross` unless users opt into local release builds.

Do not add `cross` as a runtime/project dependency.

### Workflow permissions and triggers

Keep release workflow permissions least privilege:

```yaml
permissions:
  contents: write
  packages: write
```

Keep tag trigger:

```yaml
on:
  push:
    tags:
      - "v*"
```

Optional but recommended: add manual dispatch for workflow testing without pushing a tag only if it cannot publish confusing release artifacts by accident. If adding `workflow_dispatch`, guard release upload/publish steps so manual runs upload artifacts to the workflow run but do not create/update GitHub releases unless explicitly intended and documented.

Do not make regular branch CI run cross-release builds unless maintainers explicitly want the cost.

---

## TDD / Verification Tasks

Because Phase 18 is workflow/docs focused, use static workflow validation and local cross-checks as the RED/GREEN loop. Still run the full Rust suite at the end to prove no runtime regressions.

### Task 18.1: Establish baseline and release gap

Run:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
git status --short --branch
git log --oneline -8
grep -R "ARM/Raspberry\|aarch64\|armv7\|x86_64-unknown-linux-gnu" -n README.md docs .github/workflows Cargo.toml Dockerfile examples || true
```

Expected findings before implementation:

- `README.md` lists ARM/Raspberry Pi release binary automation as planned.
- `docs/docker-release.md` says ARM/Pi artifacts are future work.
- `.github/workflows/release.yml` only builds x86_64 Linux binary artifacts.

### Task 18.2: Refactor release binary job safely

RED/check first:

```bash
grep -n "Linux x86_64 binary" .github/workflows/release.yml
grep -n "cargo build --release --locked" .github/workflows/release.yml
grep -n "x86_64-unknown-linux-gnu" .github/workflows/release.yml
grep -n "aarch64-unknown-linux-gnu" .github/workflows/release.yml || true
```

Expected RED reason:

- Existing workflow has x86_64 only.

GREEN:

- Convert the release binary job to a target matrix or add a separate ARM job.
- Preserve existing x86_64 artifact upload behavior.
- Add required aarch64 target build and package.
- Keep the `verify` job as a dependency.
- Use `cargo build --release --locked --target <target>` for target-specific builds.
- Ensure every target archive and checksum is uploaded by `softprops/action-gh-release@v2` or equivalent.

### Task 18.3: Validate cross-target build setup

At minimum locally validate target installation and metadata as far as the environment permits:

```bash
rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
cargo check --release --target x86_64-unknown-linux-gnu
cargo check --release --target aarch64-unknown-linux-gnu
```

If local cross-linking fails because the WSL environment lacks target linker/sysroot packages, do not fake success. Instead:

- Record the exact local blocker in final `issues`.
- Ensure the GitHub Actions workflow installs/configures the missing tools.
- Add static workflow validation checks that prove the workflow contains the intended setup.

If adding optional ARMv7:

```bash
rustup target add armv7-unknown-linux-gnueabihf
cargo check --release --target armv7-unknown-linux-gnueabihf
```

Only include ARMv7 in release artifacts if the workflow has target-specific linker/sysroot support and the docs accurately state 32-bit support.

### Task 18.4: Static workflow validation

Run a static validation script similar to:

```bash
python - <<'PY'
from pathlib import Path
p = Path('.github/workflows/release.yml')
text = p.read_text()
required = [
    'tags:',
    'cargo fmt --check',
    'cargo clippy --all-targets --all-features -- -D warnings',
    'cargo test --all-features',
    'cargo build --release --locked',
    'x86_64-unknown-linux-gnu',
    'aarch64-unknown-linux-gnu',
    'sha256sum',
    'softprops/action-gh-release@v2',
]
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

Do not install extra global tooling solely for `actionlint` unless it is standard in the environment.

### Task 18.5: Package script validation without publishing

Where possible, validate packaging commands locally for at least the native target:

```bash
cargo build --release --locked --target x86_64-unknown-linux-gnu
rm -rf /tmp/serialport-api-release-check
mkdir -p /tmp/serialport-api-release-check/serialport-api
cp target/x86_64-unknown-linux-gnu/release/serialport-api /tmp/serialport-api-release-check/serialport-api/
cp README.md LICENSE /tmp/serialport-api-release-check/serialport-api/
tar -C /tmp/serialport-api-release-check -czf /tmp/serialport-api-vLOCAL-x86_64-unknown-linux-gnu.tar.gz serialport-api
sha256sum /tmp/serialport-api-vLOCAL-x86_64-unknown-linux-gnu.tar.gz > /tmp/serialport-api-vLOCAL-x86_64-unknown-linux-gnu.tar.gz.sha256
sha256sum -c /tmp/serialport-api-vLOCAL-x86_64-unknown-linux-gnu.tar.gz.sha256
```

Clean temporary files after validation if they are created inside the repository. Do not commit generated archives.

### Task 18.6: Documentation updates

Update `README.md`, `docs/docker-release.md`, and `docs/raspberry-pi-systemd.md` after the workflow is in place.

Required docs content:

- Supported release target triples.
- Which Raspberry Pi OS users should choose:
  - 64-bit Pi OS: `aarch64-unknown-linux-gnu`.
  - 32-bit Pi OS: `armv7-unknown-linux-gnueabihf` only if implemented; otherwise source-build fallback.
- Artifact naming pattern.
- SHA-256 verification command.
- Install command copying `serialport-api` to `/usr/local/bin/serialport-api`.
- Systemd restart command after replacing the binary.
- Note that CI does not require Pi hardware.
- Note that source builds remain valid if an artifact does not match the target OS/architecture.

Suggested Pi download/install snippet for `docs/raspberry-pi-systemd.md`:

```bash
version="v0.1.0"
target="aarch64-unknown-linux-gnu"
curl -LO "https://github.com/avepha/serialport-api/releases/download/${version}/serialport-api-${version}-${target}.tar.gz"
curl -LO "https://github.com/avepha/serialport-api/releases/download/${version}/serialport-api-${version}-${target}.tar.gz.sha256"
sha256sum -c "serialport-api-${version}-${target}.tar.gz.sha256"
tar -xzf "serialport-api-${version}-${target}.tar.gz"
sudo install -m 0755 serialport-api/serialport-api /usr/local/bin/serialport-api
/usr/local/bin/serialport-api --version
sudo systemctl restart serialport-api
```

### Task 18.7: Full regression and review

Run:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
python - <<'PY'
from pathlib import Path
checks = {
    '.github/workflows/release.yml': ['aarch64-unknown-linux-gnu', 'sha256sum', 'cargo build --release --locked'],
    'README.md': ['ARM/Raspberry Pi', 'aarch64-unknown-linux-gnu'],
    'docs/docker-release.md': ['aarch64-unknown-linux-gnu', 'sha256sum'],
    'docs/raspberry-pi-systemd.md': ['aarch64-unknown-linux-gnu', 'sha256sum -c'],
}
for file, needles in checks.items():
    text = Path(file).read_text()
    missing = [needle for needle in needles if needle not in text]
    if missing:
        raise SystemExit(f'{file} missing {missing}')
print('phase 18 docs/workflow static check passed')
PY
git diff --stat
git status --short --branch
```

Expected:

- Rust checks pass.
- Static docs/workflow checks pass.
- Diff contains only expected workflow/docs changes.
- No source files are changed.

---

## Acceptance Criteria

Phase 18 is complete when all of these are true:

1. `.github/workflows/release.yml` still runs only on `v*` tags unless a safe, documented `workflow_dispatch` mode is added.
2. Release verification still runs format, clippy, and tests before packaging/publishing.
3. Existing Linux x86_64 release artifact remains available with the same deterministic naming pattern.
4. A Linux aarch64 artifact is built and uploaded for tag releases, named like `serialport-api-v0.1.0-aarch64-unknown-linux-gnu.tar.gz`.
5. Every release binary archive has a matching `.sha256` checksum file uploaded.
6. Packaging commands use locked dependencies: `cargo build --release --locked`, with explicit target triples for target-specific builds.
7. Archive contents are deterministic and contain the `serialport-api` binary plus `README.md` and `LICENSE` at minimum under a single top-level directory.
8. ARMv7 support is either implemented and documented with verified workflow setup, or explicitly documented as optional/future with source-build fallback.
9. README no longer lists ARM/Raspberry Pi release binary automation as incomplete if aarch64 automation is implemented.
10. `docs/docker-release.md` documents release artifact names, target triples, checksum verification, and CI hardware-free nature.
11. `docs/raspberry-pi-systemd.md` documents how to download, verify, install, and restart the service using the correct Pi artifact.
12. Existing Docker image publishing remains intact unless a documented blocker requires deferral.
13. No runtime source/API behavior changes are made.
14. Tests remain hardware-free.
15. Required verification passes or blockers are reported honestly:
    - `cargo fmt --check`
    - `cargo clippy --all-targets --all-features -- -D warnings`
    - `cargo test --all-features`
    - static workflow/docs checks
    - cross-target local checks where the environment supports them
16. Implementation is committed with a conventional commit message and not pushed.

---

## Manual Checks for Maintainers

After Phase 18 lands and is pushed, maintainers can verify the workflow by pushing a test release tag or using a guarded manual dispatch if implemented.

### Release tag check

```bash
git tag v0.1.0
git push origin v0.1.0
```

Expected GitHub Actions result:

- `Verify` job passes.
- Binary artifact job builds at least:
  - `serialport-api-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`
  - `serialport-api-v0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256`
  - `serialport-api-v0.1.0-aarch64-unknown-linux-gnu.tar.gz`
  - `serialport-api-v0.1.0-aarch64-unknown-linux-gnu.tar.gz.sha256`
- Optional ARMv7 artifacts appear only if implemented.
- GHCR image publishing still completes.

### Raspberry Pi install smoke

On a Raspberry Pi running 64-bit Raspberry Pi OS:

```bash
uname -m
version="v0.1.0"
target="aarch64-unknown-linux-gnu"
curl -LO "https://github.com/avepha/serialport-api/releases/download/${version}/serialport-api-${version}-${target}.tar.gz"
curl -LO "https://github.com/avepha/serialport-api/releases/download/${version}/serialport-api-${version}-${target}.tar.gz.sha256"
sha256sum -c "serialport-api-${version}-${target}.tar.gz.sha256"
tar -xzf "serialport-api-${version}-${target}.tar.gz"
./serialport-api/serialport-api --version
```

If using the Phase 14 systemd service:

```bash
sudo install -m 0755 serialport-api/serialport-api /usr/local/bin/serialport-api
sudo systemctl restart serialport-api
sudo systemctl status serialport-api --no-pager
curl -s http://127.0.0.1:4002/api/v1/health
```

Real serial checks remain hardware-required and should follow `docs/raspberry-pi-systemd.md`.

---

## Risks and Mitigations

- **Cross-linking native dependencies:** `serialport`/`libudev-sys` can require target-specific udev headers/pkg-config setup. Mitigate by using a maintained cross toolchain action or carefully installing target cross packages; do not commit an unverified brittle setup.
- **ARMv7 complexity:** 32-bit hard-float ARM often needs more sysroot/linker care than aarch64. Mitigate by making aarch64 required and ARMv7 optional unless reliably verified.
- **False hardware confidence:** CI cannot prove serial hardware behavior on a Pi. Mitigate by limiting CI claims to build/package success and keeping Pi hardware smoke checks manual.
- **Release workflow publishing mistakes:** Tag-triggered workflows can create public assets. Mitigate by preserving `v*` tag trigger and guarding any `workflow_dispatch` publishing path.
- **Artifact naming drift:** Inconsistent names make docs and automation hard. Mitigate with a single packaging helper block using `${GITHUB_REF_NAME}` and `${TARGET}`.
- **Checksum mismatch from path names:** `sha256sum -c` depends on filename text. Mitigate by generating checksums in the same directory as archives and testing verification commands.
- **Breaking existing x86_64 release:** Matrix refactors can accidentally change paths. Mitigate by including x86_64 in the same static checks and preserving artifact names.
- **Over-documenting unsupported targets:** Do not claim ARMv7 support unless artifacts are actually built and uploaded.
- **Action supply-chain risk:** If adding third-party actions, pin major versions at minimum and choose maintained, common release-engineering actions. Do not add secrets beyond `GITHUB_TOKEN`.

---

## Commit Guidance

Expected conventional commit message:

```text
ci: add ARM release binary automation
```

Alternative if the change is workflow plus docs and maintainers prefer `chore`:

```text
chore: add ARM release binary automation
```

If only a deferral doc is committed due to concrete blockers:

```text
docs: defer ARM release binary automation
```

Before committing:

```bash
git status --short
git diff -- .github/workflows/release.yml README.md docs/docker-release.md docs/raspberry-pi-systemd.md
```

Commit only intentional Phase 18 files. Do not include generated archives, `target/`, local SQLite databases, temporary configs, or workflow logs. Do not push.

After committing:

```bash
git status --short --branch
git log --oneline -1
```

---

## Implementation Agent Short Instruction

Implement Phase 18 in `/home/alfarie/repos/serialport-api` on branch `rewrite/axum-serial-api`: extend the tag-triggered GitHub release workflow to build deterministic, checksummed Linux release archives for x86_64 and Raspberry Pi 64-bit ARM (`aarch64-unknown-linux-gnu`), add ARMv7 only if reliable, update README/Docker/Pi docs with artifact selection and checksum install instructions, keep runtime behavior unchanged and tests hardware-free, run full Rust plus static workflow/docs verification, commit conventionally, and do not push.
