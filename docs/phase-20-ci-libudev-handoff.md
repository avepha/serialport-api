# Phase 20 CI libudev Build Dependency Handoff

> **For Hermes / next AI implementation session:** Execute this in a fresh session. This handoff is the complete input for Phase 20. The expected fix is limited to GitHub Actions CI native Linux build dependencies. Do **not** change Rust source, Cargo manifests, lockfile dependencies, API behavior, Docker/runtime behavior, release publishing semantics, tags, or generated artifacts.

## Diagnosis

The latest observed GitHub Actions CI failure is caused by the CI workflow running `cargo clippy --all-targets --all-features -- -D warnings` on `ubuntu-latest` without installing the native Linux `libudev` development package that the transitive `serialport` dependency requires through `libudev-sys`.

The implementation fix should add an installation step to `.github/workflows/ci.yml` before Cargo build/check steps that need dependency compilation:

```yaml
- name: Install native Linux build dependencies
  run: |
    sudo apt-get update
    sudo apt-get install -y --no-install-recommends pkg-config libudev-dev
```

Recommended placement: after `dtolnay/rust-toolchain@stable` and before `Swatinem/rust-cache@v2`, `cargo fmt`, `cargo clippy`, and `cargo test`.

## Strict Orchestration Input Schema

The implementation agent must accept this handoff plus the repository as its complete input:

```json
{
  "agent_role": "implementation",
  "phase": "Phase 20",
  "repository": "/home/alfarie/repos/serialport-api",
  "branch": "rewrite/axum-serial-api",
  "github_repository": "avepha/serialport-api",
  "failing_ci_run": {
    "run_id": 25202706501,
    "workflow": "CI",
    "job": "Rust",
    "step": "Run Clippy",
    "head_sha": "33c623b48f3a9fd0b0b3de2b54f004e9ae3ac46f",
    "conclusion": "failure"
  },
  "toolchain_env": {
    "PATH_prefix": "$HOME/.cargo/bin"
  },
  "required_artifact_to_read": "docs/phase-20-ci-libudev-handoff.md",
  "scope": "Fix CI on ubuntu-latest by installing native Linux build dependencies required by libudev-sys before cargo clippy/test.",
  "expected_fix_direction": "Update .github/workflows/ci.yml to install pkg-config and libudev-dev before cargo clippy/test.",
  "non_goals": [
    "Changing Rust source code under src/**",
    "Changing Cargo.toml or Cargo.lock",
    "Changing serialport dependency versions or feature flags",
    "Changing API behavior, protocol behavior, config behavior, storage behavior, SSE/WebSocket/Socket.IO behavior, Docker runtime, systemd docs, or release artifacts",
    "Changing .github/workflows/release.yml unless new evidence shows it is required",
    "Adding cross-compilation targets or ARMv7 artifacts",
    "Pushing commits, tags, releases, packages, or workflow dispatches from the local machine"
  ]
}
```

## Strict Orchestration Output Schema

The implementation agent final response must use this JSON shape:

```json
{
  "agent_role": "implementation",
  "phase": "Phase 20",
  "summary": [
    "Installed native Linux CI build dependencies for libudev-sys before Cargo checks.",
    "Verified formatting, clippy, tests, workflow diff, and working tree state."
  ],
  "files_changed": [
    ".github/workflows/ci.yml"
  ],
  "verification": {
    "commands_run": [
      "git status --short --branch",
      "cargo fmt --check",
      "cargo clippy --all-targets --all-features -- -D warnings",
      "cargo test --all-features",
      "git diff --check",
      "python static workflow validation script from this handoff"
    ],
    "status": "passed"
  },
  "commit": "<sha or null>",
  "approval_status": "ready_for_review|blocked",
  "issues": []
}
```

If blocked, set `commit` to `null`, `approval_status` to `blocked`, and list exact blockers.

## Root-Cause Evidence

### GitHub Actions failure evidence

Verified with:

```bash
gh run view 25202706501 --repo avepha/serialport-api --json databaseId,headSha,conclusion,status,workflowName,displayTitle,url,createdAt,updatedAt,jobs
gh run view 25202706501 --repo avepha/serialport-api --log-failed
```

Observed run metadata:

- Repository: `avepha/serialport-api`
- Workflow: `CI`
- Run id: `25202706501`
- Run URL: `https://github.com/avepha/serialport-api/actions/runs/25202706501`
- Job: `Rust`
- Failed step: `Run Clippy`
- Commit: `33c623b48f3a9fd0b0b3de2b54f004e9ae3ac46f`
- Display title: `docs: defer optional ARMv7 release artifacts`
- Conclusion: `failure`
- `Run tests` step was skipped because `Run Clippy` failed first.

Relevant failed log excerpts:

```text
Run cargo clippy --all-targets --all-features -- -D warnings
...
Compiling libudev-sys v0.1.4
...
error: failed to run custom build command for `libudev-sys v0.1.4`
...
called `Result::unwrap()` on an `Err` value: "`PKG_CONFIG_ALLOW_SYSTEM_CFLAGS=\"1\" PKG_CONFIG_ALLOW_SYSTEM_LIBS=\"1\" \"pkg-config\" \"--libs\" \"--cflags\" \"libudev\"` did not exit successfully: exit status: 1
--- stderr
Package libudev was not found in the pkg-config search path.
Perhaps you should add the directory containing `libudev.pc'
to the PKG_CONFIG_PATH environment variable
Package 'libudev', required by 'virtual:world', not found
"
Process completed with exit code 101.
```

### Repository workflow evidence

Current `.github/workflows/ci.yml` has only these relevant steps in the `Rust` job:

```yaml
- name: Install Rust toolchain
  uses: dtolnay/rust-toolchain@stable
  with:
    components: rustfmt, clippy

- name: Cache Cargo registry and build artifacts
  uses: Swatinem/rust-cache@v2

- name: Check formatting
  run: cargo fmt --check

- name: Run Clippy
  run: cargo clippy --all-targets --all-features -- -D warnings

- name: Run tests
  run: cargo test --all-features
```

There is no CI step installing `pkg-config` or `libudev-dev` before Clippy/tests.

Current `.github/workflows/release.yml` already contains the expected native Linux dependency pattern for x86_64 release builds:

```yaml
- name: Install native Linux build dependencies
  if: matrix.target == 'x86_64-unknown-linux-gnu'
  run: |
    sudo apt-get update
    sudo apt-get install -y --no-install-recommends pkg-config libudev-dev
```

It also passes `package: libudev-dev` to `taiki-e/setup-cross-toolchain-action@v1` for non-x86_64 release targets. This confirms the project already knows Linux builds need libudev development files; CI is the workflow missing that install step.

### Dependency evidence

`Cargo.toml` directly depends on:

```toml
serialport = "4"
```

`Cargo.lock` resolves this dependency chain on Linux:

```text
serialport v4.0.0
  -> libudev v0.2.0
     -> libudev-sys v0.1.4
        -> pkg-config v0.3.19
```

Relevant lockfile entries:

- `Cargo.lock` lines 591-608: `libudev v0.2.0` depends on `libudev-sys`; `libudev-sys v0.1.4` depends on `pkg-config`.
- `Cargo.lock` lines 974-988: `serialport v4.0.0` depends on `libudev`.

Because `libudev-sys` invokes system `pkg-config` for `libudev`, Ubuntu runners must have both:

- `pkg-config`: the executable used by the build script.
- `libudev-dev`: provides headers/library metadata, including `libudev.pc`.

## Files to Inspect Before Editing

Read these before making changes:

```text
.github/workflows/ci.yml
.github/workflows/release.yml
Cargo.toml
Cargo.lock
docs/phase-20-ci-libudev-handoff.md
```

Optional but useful:

```bash
gh run view 25202706501 --repo avepha/serialport-api --json databaseId,headSha,conclusion,status,workflowName,displayTitle,url,createdAt,updatedAt,jobs
gh run view 25202706501 --repo avepha/serialport-api --log-failed
```

## Files to Modify

Required:

- `.github/workflows/ci.yml`
  - Add one native Linux dependency install step in the `jobs.rust.steps` sequence.
  - Install exactly `pkg-config libudev-dev` using apt.
  - Use `sudo apt-get update` before install.
  - Use `--no-install-recommends` for consistency with release workflow.
  - Place the step before `cargo clippy` and `cargo test`; recommended before the Cargo cache step.

Do not modify:

- `src/**`
- `Cargo.toml`
- `Cargo.lock`
- `.github/workflows/release.yml`
- `README.md`
- `docs/**` other than the implementation agent's optional notes if explicitly requested by maintainers
- Dockerfiles, examples, generated build outputs, target directories, release archives, tags, or local databases

## Implementation Guidance

Preferred patch shape:

```diff
       - name: Install Rust toolchain
         uses: dtolnay/rust-toolchain@stable
         with:
           components: rustfmt, clippy

+      - name: Install native Linux build dependencies
+        run: |
+          sudo apt-get update
+          sudo apt-get install -y --no-install-recommends pkg-config libudev-dev
+
       - name: Cache Cargo registry and build artifacts
         uses: Swatinem/rust-cache@v2
```

Rationale for placing before the cache step:

- It mirrors the release workflow dependency install location before building.
- It ensures all subsequent Cargo invocations see `pkg-config` and `libudev.pc`.
- It avoids a cached build artifact path masking missing native dependency behavior.

## Verification Commands

Run from repository root:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"

git status --short --branch
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
git diff --check
python - <<'PY'
from pathlib import Path

ci = Path('.github/workflows/ci.yml').read_text()
release = Path('.github/workflows/release.yml').read_text()

required_ci_snippets = [
    'Install native Linux build dependencies',
    'sudo apt-get update',
    'sudo apt-get install -y --no-install-recommends pkg-config libudev-dev',
    'cargo clippy --all-targets --all-features -- -D warnings',
    'cargo test --all-features',
]
for snippet in required_ci_snippets:
    if snippet not in ci:
        raise SystemExit(f'missing expected CI snippet: {snippet}')

install_pos = ci.index('Install native Linux build dependencies')
clippy_pos = ci.index('Run Clippy')
test_pos = ci.index('Run tests')
if not install_pos < clippy_pos < test_pos:
    raise SystemExit('native dependency install step must appear before clippy and tests')

if 'sudo apt-get install -y --no-install-recommends pkg-config libudev-dev' not in release:
    raise SystemExit('release workflow no longer contains known-good native dependency install reference')

for forbidden in ['Cargo.toml', 'Cargo.lock', 'src/']:
    # This script validates workflow contents only; use git diff --name-only for file scope.
    pass

print('phase 20 CI libudev static workflow check passed')
PY
git diff --name-only
git status --short --branch
```

Expected verification result:

- `cargo fmt --check` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes locally when the local system has libudev development files available.
- `cargo test --all-features` passes locally when the local system has libudev development files available.
- Static workflow validation passes.
- `git diff --name-only` contains only `.github/workflows/ci.yml` before commit.
- Working tree is clean after commit.

If local `cargo clippy` or `cargo test` fails with the same `libudev` pkg-config error, do not change Rust code. Install local system prerequisites if permitted, or report the local environment blocker while still ensuring the GitHub Actions workflow fix is correct.

## Acceptance Criteria

Phase 20 is complete when all of these are true:

1. `.github/workflows/ci.yml` installs `pkg-config libudev-dev` with `apt-get` on `ubuntu-latest` before `cargo clippy` and `cargo test`.
2. The install command uses `sudo apt-get update` followed by `sudo apt-get install -y --no-install-recommends pkg-config libudev-dev`.
3. `.github/workflows/release.yml` remains unchanged unless a new, documented reason requires otherwise.
4. No Rust source files are changed.
5. `Cargo.toml` and `Cargo.lock` are unchanged.
6. The fix aligns with the existing release workflow native dependency pattern.
7. Local verification commands pass, or any local-only blocker is reported clearly with evidence.
8. A conventional CI commit is created, such as `ci: install libudev dependencies for checks`.
9. The implementation agent does not push.
10. The next CI run for the branch reaches past `libudev-sys` compilation in the `Run Clippy` step; ideally the full CI workflow passes.

## Commit Guidance

Implementation agent commit message:

```text
ci: install libudev dependencies for checks
```

Commit only the workflow change:

```bash
git add .github/workflows/ci.yml
git commit -m "ci: install libudev dependencies for checks"
```

Do not include this handoff artifact in the implementation commit if it is already present from the documentation agent commit.

## Risks and Mitigations

- **Risk: Adding the install step after Clippy/tests does not fix CI.** Mitigate by placing it before all Cargo compilation steps.
- **Risk: Installing only `pkg-config` leaves `libudev.pc` absent.** Mitigate by installing both `pkg-config` and `libudev-dev`.
- **Risk: Installing only `libudev-dev` relies on transitive runner packages for `pkg-config`.** Mitigate by explicitly installing both packages.
- **Risk: Changing Cargo dependency versions introduces unrelated behavior changes.** Mitigate by leaving `Cargo.toml` and `Cargo.lock` unchanged.
- **Risk: Release workflow diverges from CI.** Mitigate by matching the already-used release workflow apt command style.
- **Risk: Local WSL environment lacks native prerequisites and cannot run Clippy/tests.** Mitigate by documenting the local blocker and relying on workflow static validation; do not alter source code to work around missing system packages.

## Documentation Agent Notes

This handoff artifact was produced by the Phase 20 documentation agent. The documentation agent intentionally did not implement the CI fix. The only intended documentation-agent commit is this file.
