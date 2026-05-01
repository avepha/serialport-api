# Phase 19 ARMv7 / 32-bit Raspberry Pi Release Decision Handoff

> **For Hermes / next AI implementation session:** Execute this in a fresh session. This handoff is the complete input for Phase 19. Phase 19 is a roadmap/documentation cleanup phase, not a release-workflow expansion phase. Do **not** add ARMv7 release artifacts unless maintainers explicitly override this decision with a verified 32-bit target requirement and a reliable linker/sysroot plan.

## Decision

**Phase 19 should explicitly defer ARMv7 / 32-bit Raspberry Pi release artifacts as optional/out-of-scope unless maintainers request them.**

The implementation agent should remove ARMv7 from the active planned roadmap while preserving clear source-build fallback documentation for 32-bit Raspberry Pi OS users.

## Decision Rationale

Repository evidence after Phase 18:

- Phase 18 completed and is present at commit `b50b355 ci: add ARM release binary automation`.
- `.github/workflows/release.yml` now publishes Linux release archives for:
  - `x86_64-unknown-linux-gnu`
  - `aarch64-unknown-linux-gnu`
- `README.md` lists ARM64 Raspberry Pi automation as implemented, but still has exactly one active planned item:
  - `ARMv7 / 32-bit Raspberry Pi release artifacts, if maintainers want a verified 32-bit target`
- `docs/docker-release.md` already states ARMv7 / 32-bit Raspberry Pi artifacts are not currently published because the target needs separate linker/sysroot validation.
- `docs/raspberry-pi-systemd.md` already gives 32-bit users a source-build fallback and tells `armv7l` users not to expect a release archive.
- `docs/open-source-spec.md` describes Raspberry Pi ARMv7/aarch64 binaries as optional release automation, not mandatory runtime functionality.
- Phase 18 handoff explicitly treated `armv7-unknown-linux-gnueabihf` as optional and warned not to add it if linker/sysroot setup is brittle or unverified.

Because 64-bit Raspberry Pi users now have a supported `aarch64-unknown-linux-gnu` artifact and 32-bit users have a documented source-build path, ARMv7 release artifacts are not a required completion blocker for the current rewrite. Keeping ARMv7 as the only active roadmap item creates an indefinite open-ended task without a maintainer requirement or hardware/toolchain validation commitment.

## Strict Orchestration Input Schema

The implementation agent must accept this handoff plus the repository as its complete input:

```json
{
  "agent_role": "implementation",
  "phase": "Phase 19",
  "repository": "/home/alfarie/repos/serialport-api",
  "branch": "rewrite/axum-serial-api",
  "base_commit_expected": "b50b355 ci: add ARM release binary automation or a descendant",
  "toolchain_env": {
    "PATH_prefix": "$HOME/.cargo/bin"
  },
  "decision": "defer ARMv7 / 32-bit Raspberry Pi release artifacts as optional/out-of-scope unless maintainers request verified support",
  "required_artifact_to_read": "docs/phase-19-armv7-release-decision-handoff.md",
  "scope": "Remove ARMv7 from active planned roadmap while preserving clear docs fallback for source builds on 32-bit Raspberry Pi OS",
  "non_goals": [
    "Implementing ARMv7 release artifacts now",
    "Adding armv7-unknown-linux-gnueabihf to the GitHub Actions release matrix",
    "Changing Rust runtime/API behavior",
    "Changing serial manager, protocol, config, storage, SSE, WebSocket, Socket.IO, Docker runtime, or systemd service behavior",
    "Removing the documented source-build path for 32-bit Raspberry Pi OS",
    "Claiming ARMv7 binary support without release workflow artifacts and verification",
    "Publishing releases, pushing tags, pushing commits, or manually uploading artifacts from the local machine"
  ]
}
```

## Strict Orchestration Output Schema

The implementation agent final response must use this JSON shape:

```json
{
  "agent_role": "implementation",
  "phase": "Phase 19",
  "summary": [
    "Removed ARMv7 / 32-bit Raspberry Pi release artifacts from the active planned roadmap.",
    "Preserved documented 32-bit Raspberry Pi source-build fallback and ARM64 artifact guidance."
  ],
  "files_changed": [
    "README.md",
    "docs/docker-release.md",
    "docs/raspberry-pi-systemd.md"
  ],
  "verification": {
    "commands_run": [
      "git status --short --branch",
      "cargo fmt --check",
      "cargo clippy --all-targets --all-features -- -D warnings",
      "cargo test --all-features",
      "python static docs/roadmap validation script from this handoff",
      "git diff --check"
    ],
    "status": "passed"
  },
  "commit": "<sha or null>",
  "approval_status": "ready_for_review|blocked",
  "issues": []
}
```

If blocked, set `commit` to `null`, `approval_status` to `blocked`, and list exact blockers.

## Required Preconditions

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
- Recent history includes `b50b355 ci: add ARM release binary automation` or a descendant.

If the working tree is not clean before Phase 19 edits, stop and report instead of modifying files.

## Files to Inspect

Read these files before editing:

```text
README.md
docs/docker-release.md
docs/raspberry-pi-systemd.md
docs/open-source-spec.md
docs/phase-18-arm-release-automation-handoff.md
.github/workflows/release.yml
```

Search for these strings:

```text
ARMv7
armv7
32-bit Raspberry Pi
32-bit Pi OS
armv7-unknown-linux-gnueabihf
aarch64-unknown-linux-gnu
Planned / not complete yet
Roadmap
source-build fallback
```

## Files to Modify

Required:

- `README.md`
  - Remove the `Planned / not complete yet` section if ARMv7 is the only remaining item.
  - Remove the near-term roadmap bullet that says to add ARMv7 artifacts only if maintainers need them.
  - Keep the release packaging section clear that currently published binary targets are x86_64 and aarch64 only.
  - Preserve a concise note that ARMv7 / 32-bit Raspberry Pi release artifacts are optional/not currently published and 32-bit users should build from source.

Likely required:

- `docs/docker-release.md`
  - Keep current target list as x86_64 and aarch64.
  - Preserve or tighten the existing sentence explaining ARMv7 is not published due separate linker/sysroot validation.
  - Avoid making ARMv7 look like active planned work.

- `docs/raspberry-pi-systemd.md`
  - Keep architecture selection guidance.
  - Preserve `armv7l` / 32-bit source-build fallback.
  - Avoid making ARMv7 release archives look imminent or planned.

Do not modify unless a direct roadmap cleanup requires it:

- `docs/open-source-spec.md`
  - It is acceptable for the long-form spec to keep ARMv7/aarch64 as optional release automation because it is explicitly optional. Only change it if needed to remove contradiction.

Do not modify:

- `.github/workflows/release.yml`
- `.github/workflows/ci.yml`
- `src/**`
- `Cargo.toml`
- `Cargo.lock`
- `Dockerfile`
- `examples/**`

## Implementation Guidance

Preferred result:

- README no longer has an active incomplete/planned checklist item solely for ARMv7.
- README still tells 32-bit Raspberry Pi users that no ARMv7 archive is currently published and to build from source.
- Docker and Raspberry Pi docs continue to name the supported release targets exactly:
  - `x86_64-unknown-linux-gnu`
  - `aarch64-unknown-linux-gnu`
- Docs do not claim `armv7-unknown-linux-gnueabihf` artifacts exist.
- Release workflow remains unchanged.

Recommended README wording pattern:

```markdown
ARMv7 / 32-bit Raspberry Pi release artifacts are optional and not currently published; 32-bit Pi OS users should build from source unless maintainers request and verify a dedicated `armv7-unknown-linux-gnueabihf` release target.
```

## Verification Commands

Run from repository root:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"

git status --short --branch
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
python - <<'PY'
from pathlib import Path

readme = Path('README.md').read_text()
docker = Path('docs/docker-release.md').read_text()
pi = Path('docs/raspberry-pi-systemd.md').read_text()
workflow = Path('.github/workflows/release.yml').read_text()

if '- [ ] ARMv7 / 32-bit Raspberry Pi release artifacts' in readme:
    raise SystemExit('README still contains ARMv7 active planned checklist item')
if 'Add ARMv7 / 32-bit Raspberry Pi release artifacts only if maintainers need them' in readme:
    raise SystemExit('README still contains ARMv7 active near-term roadmap item')
for target in ['x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu']:
    if target not in readme or target not in docker or target not in workflow:
        raise SystemExit(f'missing supported target documentation/workflow reference: {target}')
if 'armv7-unknown-linux-gnueabihf' in workflow:
    raise SystemExit('release workflow unexpectedly includes ARMv7 target')
for text, name in [(readme, 'README.md'), (docker, 'docs/docker-release.md'), (pi, 'docs/raspberry-pi-systemd.md')]:
    if 'ARMv7' not in text and '32-bit' not in text:
        raise SystemExit(f'{name} no longer documents 32-bit/ARMv7 fallback')
    if 'build from source' not in text.lower() and 'source-build' not in text.lower():
        raise SystemExit(f'{name} no longer documents source-build fallback')
print('phase 19 ARMv7 deferral static check passed')
PY
git diff --check
git diff --stat
git status --short --branch
```

Expected:

- Rust checks pass.
- Static docs check passes.
- `git diff --stat` contains only expected documentation files.
- No workflow or source files are changed.

## Acceptance Criteria

Phase 19 is complete when all of these are true:

1. ARMv7 / 32-bit Raspberry Pi release artifacts are explicitly deferred as optional/out-of-scope unless maintainers request verified support.
2. `README.md` no longer lists ARMv7 as an active planned/not-complete checklist item.
3. `README.md` no longer has a near-term roadmap bullet that implies ARMv7 artifacts are the next active task.
4. Docs still clearly state that published binary release targets are `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`.
5. Docs still clearly state that ARMv7 / 32-bit Raspberry Pi artifacts are not currently published.
6. Docs still provide or point to a source-build fallback for 32-bit Raspberry Pi OS users.
7. `.github/workflows/release.yml` remains unchanged and does not include `armv7-unknown-linux-gnueabihf`.
8. No runtime source/API behavior changes are made.
9. No generated artifacts, release archives, tags, or local databases are committed.
10. Verification commands pass or blockers are reported honestly.
11. Changes are committed with a conventional commit message and not pushed.

## Risks and Mitigations

- **Risk: Users think 32-bit Raspberry Pi is unsupported entirely.** Mitigate by keeping source-build fallback language visible in README and Pi docs.
- **Risk: Users think ARMv7 release archives exist.** Mitigate by never listing `armv7-unknown-linux-gnueabihf` as a current release target and by stating that no ARMv7 archives are currently published.
- **Risk: Roadmap churn.** Mitigate by removing ARMv7 from active planned work but preserving a maintainers-may-request note.
- **Risk: Workflow drift.** Mitigate by not editing `.github/workflows/release.yml` in this phase and by statically checking that it does not include ARMv7.
- **Risk: Overclaiming CI validation.** Mitigate by continuing to claim only x86_64 and aarch64 release artifacts are automated.

## Commit Guidance

Expected conventional commit message:

```text
docs: defer optional ARMv7 release artifacts
```

Before committing:

```bash
git status --short
git diff -- README.md docs/docker-release.md docs/raspberry-pi-systemd.md
```

Commit only intentional Phase 19 documentation changes. Do not push.

After committing:

```bash
git status --short --branch
git log --oneline -1
```

## Implementation Agent Short Instruction

Implement Phase 19 in `/home/alfarie/repos/serialport-api` on branch `rewrite/axum-serial-api`: remove ARMv7 / 32-bit Raspberry Pi release artifacts from the active planned roadmap, preserve explicit source-build fallback docs for 32-bit Pi OS, do not change release workflow or source behavior, run full Rust plus static documentation verification, commit with `docs: defer optional ARMv7 release artifacts`, and do not push.
