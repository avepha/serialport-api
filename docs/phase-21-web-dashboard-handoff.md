# Phase 21 React Dashboard + Release Bundle Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Add a built-in React + TypeScript dashboard for `serialport-api` and make the existing CI/release pipelines build, verify, and package the latest web bundle with every released application archive.

**Architecture:** Add a first-class Vite React TypeScript app under `web/`. Use shadcn/ui as the baseline component system so the dashboard has accessible, copy-owned components without adopting a heavy framework. The frontend consumes existing HTTP/SSE/WebSocket APIs and is compiled into static assets under `web/dist/`. Axum serves the compiled dashboard at `/dashboard`, `/`, and Vite asset paths. GitHub Actions CI must run Node frontend checks alongside existing Rust checks. The tag-triggered release workflow must build the web bundle once per release and include that exact compiled bundle in every binary archive.

**Tech Stack:** Rust 2021, Axum 0.7, Node.js 20, pnpm, Vite, React, TypeScript, Tailwind CSS, shadcn/ui, Radix UI primitives via shadcn components, lucide-react icons, browser `fetch`, browser `EventSource`, existing GitHub Actions workflows.

---

## Current Repository Context

- Repository: `/home/alfarie/repos/serialport-api`
- Branch observed before planning: `rewrite/axum-serial-api`
- Existing CI workflow: `.github/workflows/ci.yml`
  - Rust job already installs `pkg-config libudev-dev`.
  - Rust job currently runs only `cargo fmt --check`, `cargo clippy`, and `cargo test`.
- Existing release workflow: `.github/workflows/release.yml`
  - Tag trigger: `v*`
  - `verify` job currently verifies Rust only.
  - `linux-binary` job packages only `target/${TARGET}/release/serialport-api`, `README.md`, `LICENSE`, and `ARTIFACT.txt`.
  - Release archives are named `serialport-api-${GITHUB_REF_NAME}-${TARGET}.tar.gz`.
  - Docker image job builds/publishes from repo context after `verify`.
- Existing server entry point: `src/main.rs`
- Existing router: `src/api/routes.rs`
- Existing API routes already available:
  - `GET /api/v1/health`
  - `GET /api/v1/ports`
  - `GET /api/v1/connections`
  - `POST /api/v1/connections`
  - `DELETE /api/v1/connections/:name`
  - `POST /api/v1/connections/:name/commands`
  - `GET /api/v1/events` for SSE snapshots/events
  - `GET /api/v1/events/ws` for native WebSocket snapshots
  - `GET /socket.io/?EIO=4&transport=websocket` for Engine.IO/Socket.IO compatibility
  - `GET/POST /api/v1/presets`
  - `GET/PUT/DELETE /api/v1/presets/:id`
  - Legacy aliases: `/list`, `/connect`, `/disconnect`, `/info`, `/commit`

## Hands-Off Implementation Contract for AI Coding Agents

This document is intended to be sufficient for an AI coding agent to implement Phase 21 without additional user clarification. Treat this file as the source of truth and proceed end-to-end unless a hard blocker prevents safe implementation.

### Agent operating rules

1. Work in repository root `/home/alfarie/repos/serialport-api`.
2. Confirm the branch before editing. Expected branch: `rewrite/axum-serial-api`.
3. Do not ask the user for choices that are already resolved in this document.
4. Do not implement out-of-scope features listed in **Non-goals for MVP**.
5. Use test-first development for Rust route/static-serving changes.
6. Keep hardware-dependent behavior out of tests; tests must pass without serial devices attached.
7. Prefer small, focused commits or one final commit if the orchestration environment prefers a single changeset.
8. Never commit credentials, tokens, local `.env` secrets, or machine-specific absolute paths except documentation references to this repository path.
9. If generated code differs from examples because current tool versions changed, keep the same behavior and document the reason in the final summary.
10. If a command fails, diagnose and fix the root cause before moving on; do not skip verification steps.

### Default decisions already made

- Frontend framework: React + TypeScript.
- Frontend bundler/dev server: Vite.
- Node version: Node.js 20.
- Package manager: pnpm with committed `web/pnpm-lock.yaml`.
- UI system: shadcn/ui with copied component source under `web/src/components/ui/`.
- Styling: Tailwind CSS and shadcn CSS variables.
- Dashboard URL: `GET /dashboard`.
- Root URL: serve the same dashboard page or redirect to `/dashboard`; implement whichever is simpler for existing router structure, but add a route test documenting the chosen behavior.
- Static asset URL shape: prefer Vite default `/assets/*` with `base: '/'`.
- State management: local React component state only.
- API behavior: use existing HTTP/SSE/WebSocket endpoints; do not add new serial semantics.
- Release archives: must include compiled dashboard assets under `serialport-api/web/`.
- Docker behavior: include dashboard assets in Docker image if a Dockerfile exists and can be updated cleanly; otherwise document the explicit limitation and ensure missing assets do not crash the server.

### Required implementation sequence

Implement the tasks in this exact order:

1. Create the Vite React TypeScript + shadcn/ui app skeleton.
2. Build the typed dashboard API client and UI using existing API routes.
3. Add Axum dashboard/static asset serving with hardware-free tests.
4. Update CI workflow to verify the web package.
5. Update release workflow to build/package the web bundle and verify archive contents.
6. Resolve Docker packaging according to the default decision above.
7. Update README/docs.
8. Run full verification and prepare final diff/commit.

### Completion definition

Phase 21 is complete only when all acceptance criteria pass, the final verification commands pass, release archive content verification is represented in workflow code, and the final summary reports:

- changed files,
- commands run,
- test/build results,
- Docker decision,
- any intentional deviations from this plan.

## Dashboard MVP Scope

Build a dashboard available at:

- `GET /dashboard`
- `GET /` as the same page or a redirect to `/dashboard`
- Static built assets under `/assets/*` or another documented Vite base path

Dashboard sections:

1. **Server status** — calls `GET /api/v1/health`.
2. **Serial ports** — calls `GET /api/v1/ports`.
3. **Connections** — create/list/disconnect via existing connection routes.
4. **Command console** — sends JSON payloads to `POST /api/v1/connections/:name/commands`, including optional `waitForResponse` and `timeoutMs`.
5. **Events log** — uses `EventSource('/api/v1/events')` first, with graceful handling for finite snapshot streams.
6. **Presets** — list/create/delete/apply saved command presets via existing preset routes.

## CI / Release Bundle Scope

The dashboard is part of the application, not a separate optional artifact. CI and release must support it as follows:

1. **CI workflow** (`.github/workflows/ci.yml`)
   - Install Node.js 20.
   - Cache pnpm dependencies for `web/pnpm-lock.yaml`.
   - Run frontend checks:
     - `pnpm install --frozen-lockfile`
     - `pnpm typecheck`
     - `pnpm build`
   - Keep existing Rust formatting, clippy, and tests.
   - CI should fail if the React dashboard does not typecheck or build.

2. **Release workflow** (`.github/workflows/release.yml`)
   - The `verify` job must run the same frontend checks as CI.
   - The release workflow must build the web bundle before packaging binary archives.
   - Every `serialport-api-${tag}-${target}.tar.gz` archive must include the latest compiled web bundle.
   - Recommended archive layout:
     ```text
     serialport-api/
       serialport-api
       README.md
       LICENSE
       ARTIFACT.txt
       web/
         index.html
         assets/...
     ```
   - `ARTIFACT.txt` should include enough metadata to prove the web bundle was included, for example:
     ```text
     web_bundle=web/dist
     web_built=true
     ```
   - Checksums must continue to be generated after the web bundle is copied into the package directory.

3. **Docker image**
   - If Docker images are expected to serve the dashboard, the Dockerfile/build context must also include the compiled web bundle or build it during the Docker build.
   - If Docker dashboard packaging is deferred, document it explicitly as a follow-up and do not silently ship a Docker image without the dashboard if the binary expects built assets at runtime.

## Non-goals for MVP

Do **not** add these in Phase 21:

- Authentication, accounts, sessions, or HTTPS termination.
- A new API schema or GraphQL layer.
- New serial protocol behavior.
- Long-running event persistence beyond what the existing manager already stores.
- Real hardware-only tests.
- Heavy UI framework dependencies beyond React/Vite/TypeScript/Tailwind/shadcn/ui.
- A separate web-only release artifact unless it is in addition to, not instead of, bundling web assets into the whole-application archives.

---

## Acceptance Criteria

1. `web/package.json`, `web/pnpm-lock.yaml`, `web/tsconfig.json`, `web/vite.config.ts`, `web/index.html`, `web/src/main.tsx`, `web/src/App.tsx`, `web/components.json`, Tailwind config/CSS files, and related frontend files define a React + TypeScript + shadcn/ui dashboard app.
2. The app includes shadcn/ui-owned components under `web/src/components/ui/` rather than importing a prebuilt dashboard template.
3. From `web/`, `pnpm install --frozen-lockfile`, `pnpm typecheck`, and `pnpm build` pass.
4. `pnpm build` produces compiled static assets under `web/dist/`.
5. Axum serves the built dashboard at `GET /dashboard` with `200 OK`, `content-type: text/html`, and a page shell that loads the Vite-built JS bundle.
6. `GET /` serves the same dashboard or redirects to `/dashboard`; route tests document the chosen behavior.
7. Vite asset URLs such as `/assets/*.js` and `/assets/*.css` are served by Axum with successful HTTP responses.
8. Route tests pass without serial hardware.
9. CI workflow includes Node setup and frontend verification steps.
10. Release workflow includes Node setup/frontend build and packages `web/dist` contents into every target archive.
11. Release packaging verification checks archive contents for at least `serialport-api/web/index.html` and one compiled asset under `serialport-api/web/assets/`.
12. Full local Rust verification passes with rustup first in PATH:
    ```bash
    export PATH="$HOME/.cargo/bin:$PATH"
    cargo fmt --check
    cargo check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test --all-features
    ```
13. README documents how to build the dashboard locally, run the server, open `/dashboard`, and what release bundles include.

---

## Recommended Hermes Skills for This Web Application

Use these skills when implementing or reviewing this phase:

### Must-load skills

- **`writing-plans`** — keep the React/dashboard/release work broken into small, reviewable tasks with exact paths and verification commands.
- **`test-driven-development`** — route/static asset behavior and Rust changes should be test-first. For frontend logic, add type-level checks and small tests only if a test runner is introduced later; do not add a browser test framework in MVP unless requested.
- **`rust-axum-api-tdd`** — required for Axum static asset routing, `/dashboard`, `/`, `/assets/*`, and hardware-free route tests.
- **`github-pr-workflow`** — required for editing `.github/workflows/ci.yml` and `.github/workflows/release.yml`, especially Node setup, pnpm cache, artifact packaging, release archive verification, and CI-safe dependency ordering.

### Strongly recommended skills

- **`claude-design`** — use for dashboard UX structure, information hierarchy, empty/error states, and avoiding generic dashboard slop.
- **`popular-web-designs`** — use as visual vocabulary only. Recommended references for this project: Linear, Vercel, Supabase, Sentry, or Warp because this is a developer-tool/hardware-control dashboard.
- **`dogfood`** — after implementation, run browser-based exploratory QA against the live dashboard: console errors, form interactions, command flow, empty states, and visual regressions.
- **`requesting-code-review`** — before committing implementation changes, run independent review over the diff because this phase touches Rust routing, Node dependencies, CI, release packaging, and possibly Docker.

### Optional / situational skills

- **`design-md`** — use only if we decide the dashboard needs a durable design-token spec in the repo. shadcn/Tailwind theme tokens may be enough for MVP.
- **`node-inspect-debugger`** — use if Vite, TypeScript, or Node build/runtime behavior needs breakpoint-level debugging.
- **`subagent-driven-development`** — useful when executing this plan as parallelizable chunks: frontend skeleton, Axum static routing, CI/release packaging, Docker/docs, and QA review.
- **`github-code-review`** — use when reviewing a PR after it is opened, not for local pre-commit review.

---

## Task 1: Create Vite React TypeScript + shadcn/ui app skeleton

**Objective:** Add a minimal typed frontend project under `web/` with shadcn/ui as the baseline component system.

**Files:**
- Create: `web/package.json`
- Create: `web/pnpm-lock.yaml`
- Create: `web/tsconfig.json`
- Create: `web/tsconfig.node.json` if Vite requires it
- Create: `web/vite.config.ts`
- Create: `web/index.html`
- Create: `web/components.json`
- Create: `web/src/main.tsx`
- Create: `web/src/App.tsx`
- Create: `web/src/api.ts`
- Create: `web/src/styles.css` or `web/src/index.css`
- Create: `web/src/lib/utils.ts`
- Create: `web/src/components/ui/*` via shadcn/ui CLI

**Implementation notes:**

- Use Node 20-compatible dependencies.
- Use pnpm for all frontend dependency installation, lockfile generation, local scripts, CI, release, and Docker commands.
- Include an explicit `packageManager` field in `web/package.json` for pnpm, using the pnpm version selected by Corepack during implementation, for example `"packageManager": "pnpm@<version>"`.
- Use Vite React TypeScript as the app foundation.
- Initialize shadcn/ui in `web/` with the shadcn CLI, e.g. `pnpm dlx shadcn@latest init`, then commit the generated `components.json`, utility helper, CSS variables, and copied component source files.
- Add only the shadcn components needed for the MVP. Recommended initial set:
  ```bash
  pnpm dlx shadcn@latest add button card input label textarea select checkbox badge separator scroll-area tabs alert table
  ```
- Expected supporting dependencies include Tailwind CSS, Radix primitives pulled by shadcn components, `class-variance-authority`, `clsx`, `tailwind-merge`, and `lucide-react`.
- Keep scripts explicit:
  ```json
  {
    "scripts": {
      "dev": "vite",
      "build": "tsc -b && vite build",
      "typecheck": "tsc -b --pretty false",
      "preview": "vite preview"
    }
  }
  ```
- Set Vite `base` so built assets work when served by Axum at `/dashboard` and `/assets/*`. Prefer `base: '/'` unless route tests prove a better path is needed.
- Do not add Redux, TanStack Query, React Router, Playwright, charting libraries, or prebuilt dashboard templates in the MVP.

**Verification:**

```bash
cd web
pnpm install --frozen-lockfile
pnpm typecheck
pnpm build
```

Expected: `web/dist/index.html` and compiled assets exist.

---

## Task 2: Implement typed dashboard API client and UI

**Objective:** Build the React dashboard against the existing API contract.

**Files:**
- Modify: `web/src/api.ts`
- Modify: `web/src/App.tsx`
- Modify: `web/src/styles.css` or `web/src/index.css`
- Use: `web/src/components/ui/*`

**Required frontend behavior:**

- Health panel: `GET /api/v1/health`.
- Ports panel: `GET /api/v1/ports`.
- Connections panel:
  - `GET /api/v1/connections`
  - `POST /api/v1/connections`
  - `DELETE /api/v1/connections/:name`
- Command console:
  - JSON payload editor.
  - Optional `waitForResponse` and `timeoutMs`.
  - `POST /api/v1/connections/:name/commands`.
- Events panel:
  - Use `EventSource('/api/v1/events')`.
  - Show stream close/error status without crashing.
- Presets panel:
  - `GET /api/v1/presets`
  - `POST /api/v1/presets`
  - `DELETE /api/v1/presets/:id`
  - Apply preset by copying/sending payload to selected connection.

- Build the layout from shadcn/ui primitives (`Card`, `Button`, `Input`, `Textarea`, `Select`, `Checkbox`, `Badge`, `Tabs`, `ScrollArea`, `Alert`, `Table` if useful).
- Prefer a developer-tool dashboard visual style: dense but readable, clear labels, strong empty/error states, no decorative fake metrics.
- Use lucide-react icons only where they improve scanning.
- Keep component state local for MVP; do not introduce global state libraries.

**Verification:**

```bash
cd web
pnpm typecheck
pnpm build
```

---

## Task 3: Serve built dashboard assets from Axum test-first

**Objective:** Add route tests and server support for Vite build output.

**Files:**
- Modify: `Cargo.toml` if a static-file dependency is required
- Modify: `src/api/routes.rs`
- Test: `src/api/routes.rs` test module

**Preferred implementation direction:**

- Prefer serving files from `web/dist` at runtime using a small static file handler or `tower-http` static services.
- If adding `tower-http`, use only the needed feature, likely `fs`.
- Decide clearly whether missing `web/dist` should:
  - return a helpful dashboard build error page in development, or
  - fail startup/package verification.
- For release archives, the expected runtime path should be relative to the binary working directory:
  ```text
  ./web/index.html
  ./web/assets/...
  ```
  because release packaging will copy `web/dist/*` into `serialport-api/web/`.

**Route tests:**

- `GET /dashboard` returns HTML when test fixture assets are present or when using an injected/static test directory.
- `GET /` behavior is tested.
- `GET /assets/<known-test-asset>.js` or equivalent static asset route is tested.
- Existing API route tests still pass.

**Verification:**

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test dashboard -- --nocapture
cargo test --all-features
```

---

## Task 4: Update CI workflow for frontend checks

**Objective:** Make pull-request/branch CI verify the dashboard package.

**Files:**
- Modify: `.github/workflows/ci.yml`

**Required workflow additions:**

Add Node setup after checkout and before frontend commands. Enable Corepack after Node is installed so the workflow uses the Node 20-provided pnpm shim:

```yaml
- name: Install Node.js
  uses: actions/setup-node@v4
  with:
    node-version: 20
    cache: pnpm
    cache-dependency-path: web/pnpm-lock.yaml

- name: Enable Corepack
  run: corepack enable
```

Add frontend verification steps:

```yaml
- name: Install web dependencies
  working-directory: web
  run: pnpm install --frozen-lockfile

- name: Typecheck web dashboard
  working-directory: web
  run: pnpm typecheck

- name: Build web dashboard
  working-directory: web
  run: pnpm build
```

**Ordering guidance:**

- Keep existing Rust native dependency install before Rust clippy/tests.
- It is acceptable to run frontend steps before or after Rust steps, but the CI job must fail on either Rust or web failures.

**Verification:**

```bash
python3 - <<'PY'
from pathlib import Path
text = Path('.github/workflows/ci.yml').read_text()
required = [
    'corepack enable',
    'actions/setup-node@v4',
    'node-version: 20',
    'cache: pnpm',
    'cache-dependency-path: web/pnpm-lock.yaml',
    'working-directory: web',
    'pnpm install --frozen-lockfile',
    'pnpm typecheck',
    'pnpm build',
]
missing = [item for item in required if item not in text]
assert not missing, missing
PY
```

---

## Task 5: Update release workflow to bundle the web package

**Objective:** Ensure tag releases include the latest compiled dashboard inside every whole-application binary archive.

**Files:**
- Modify: `.github/workflows/release.yml`

**Required workflow behavior:**

1. `verify` job:
   - Install Node.js 20 with pnpm cache.
   - Enable Corepack after Node setup.
   - Run `pnpm install --frozen-lockfile`, `pnpm typecheck`, and `pnpm build` in `web/`.
   - Continue Rust fmt/clippy/tests.

2. `linux-binary` job:
   - Install Node.js 20 with pnpm cache and enable Corepack, or download a web build artifact produced by a separate `web-build` job.
   - Build the web dashboard before packaging, or use the exact artifact from `web-build`.
   - Copy compiled web assets into the package directory:
     ```bash
     mkdir -p "$package_dir/web"
     cp -R web/dist/. "$package_dir/web/"
     ```
   - Add metadata to `ARTIFACT.txt`:
     ```bash
     echo "web_bundle=web/dist"
     echo "web_built=true"
     ```
   - Generate checksums after copying web assets.

**Recommended release workflow shape:**

- Keep `verify` as the gate.
- Add a `web-build` job if avoiding repeated web builds per target is preferred:
  - needs: `verify`
  - runs `pnpm install --frozen-lockfile && pnpm build`
  - uploads `web/dist` as artifact, e.g. `web-dist-${{ github.ref_name }}`
- Make `linux-binary` need `web-build`, download the artifact, and copy it into every target archive.

This avoids rebuilding identical frontend assets once per target and proves every archive uses the same web package.

**Archive-content verification step:**

Add a step after `Package binary` and before `Upload release assets`:

```yaml
- name: Verify package contents
  run: |
    set -euo pipefail
    archive="serialport-api-${GITHUB_REF_NAME}-${TARGET}.tar.gz"
    tar -tzf "$archive" | grep -Fx "serialport-api/web/index.html"
    tar -tzf "$archive" | grep -E '^serialport-api/web/assets/.+\.(js|css)$'
    tar -xOzf "$archive" serialport-api/ARTIFACT.txt | grep -Fx "web_built=true"
```

**Verification:**

```bash
python3 - <<'PY'
from pathlib import Path
text = Path('.github/workflows/release.yml').read_text()
required = [
    'corepack enable',
    'actions/setup-node@v4',
    'node-version: 20',
    'web/pnpm-lock.yaml',
    'pnpm install --frozen-lockfile',
    'pnpm typecheck',
    'pnpm build',
    'web/dist',
    'package_dir/web',
    'web_built=true',
    'tar -tzf',
    'serialport-api/web/index.html',
]
missing = [item for item in required if item not in text]
assert not missing, missing
PY
```

---

## Task 6: Update Docker packaging decision

**Objective:** Avoid ambiguity about whether GHCR Docker images include the dashboard.

**Files:**
- Inspect/modify: `Dockerfile`
- Modify: `README.md` or `docs/docker-release.md` if behavior changes or is deferred

**Hands-off Docker default:**

Implement this decision tree without asking the user:

1. **If `Dockerfile` exists and can be updated cleanly, include the dashboard in Docker now.**
   - Use a Node build stage or an existing build stage to produce `web/dist`.
   - Copy compiled dashboard files into the final image at the runtime path expected by Axum, e.g. `/app/web/index.html` and `/app/web/assets/*`.
   - Ensure Docker build order keeps dependency caching reasonable: copy `web/package.json` and `web/pnpm-lock.yaml`, run `pnpm install --frozen-lockfile`, then copy the rest of `web/` and run `pnpm build`.

2. **If Docker packaging cannot be updated safely in this phase, explicitly defer it.**
   - Release archives must still include the dashboard now.
   - README or `docs/docker-release.md` must state that Docker dashboard packaging is deferred.
   - The binary must not crash in Docker solely because `web/` is absent; `/dashboard` should return a clear missing-dashboard response if assets are not packaged.

Do not silently ship a Docker image that appears to support `/dashboard` but cannot serve the dashboard assets.

---

## Task 7: Documentation updates

**Objective:** Make local development, CI, and release bundle behavior discoverable.

**Files:**
- Modify: `README.md`
- Modify: `docs/docker-release.md` if Docker behavior changes

**README must document:**

- Frontend development:
  ```bash
  cd web
  pnpm install --frozen-lockfile
  pnpm dev
  ```
- Production dashboard build:
  ```bash
  cd web
  pnpm install --frozen-lockfile
  pnpm build
  ```
- Running the Rust server and opening:
  ```text
  http://127.0.0.1:4002/dashboard
  ```
- Release bundle contents include:
  ```text
  serialport-api/web/index.html
  serialport-api/web/assets/...
  ```

---

## Task 8: Full verification and final diff

**Objective:** Prove the feature is safe, lint-clean, tested, and packaged.

**Commands:**

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"

cd web
pnpm install --frozen-lockfile
pnpm typecheck
pnpm build
cd ..

cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features

git diff --check
```

Manual smoke test:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -- serve --host 127.0.0.1 --port 4002
curl -i -s http://127.0.0.1:4002/dashboard | head -20
curl -s http://127.0.0.1:4002/api/v1/health
```

Inspect expected changed files:

```bash
git status --short --branch
git diff -- \
  Cargo.toml Cargo.lock \
  src/api/routes.rs \
  web/package.json web/pnpm-lock.yaml web/tsconfig.json web/vite.config.ts web/index.html web/src \
  .github/workflows/ci.yml .github/workflows/release.yml \
  Dockerfile README.md docs/docker-release.md docs/phase-21-web-dashboard-handoff.md
```

Expected commit message:

```bash
git add .github/workflows/ci.yml .github/workflows/release.yml Cargo.toml Cargo.lock src/api/routes.rs web README.md docs/phase-21-web-dashboard-handoff.md
# Add Dockerfile/docs/docker-release.md too if Docker behavior changed.
git commit -m "feat: add React dashboard release bundle"
```

---

## Suggested Future Phases

### Phase 22: Live streaming polish

If users need truly live streaming rather than event snapshots, add explicit tests and manager support for long-lived broadcast updates. Do this separately from the dashboard packaging baseline.

### Phase 23: Dashboard configuration/status endpoint

Add an endpoint that exposes resolved server mode and safe serial defaults, e.g. mock/real mode, configured preset DB presence, and default baud/delimiter. Avoid leaking filesystem paths or sensitive host details by default.

### Phase 24: Dashboard end-to-end tests

Add Playwright or another browser-level test suite only after the MVP dashboard route and CI/release packaging are stable.

---

## Copy/Paste Prompt for Hands-Off Implementation Agent

You are implementing Phase 21 in `/home/alfarie/repos/serialport-api` on branch `rewrite/axum-serial-api`. Use `docs/phase-21-web-dashboard-handoff.md` as the source of truth and complete the entire phase end-to-end without asking for clarification unless there is a hard safety blocker or missing credential that cannot be worked around.

Required outcome: add a React + TypeScript Vite dashboard under `web/` using shadcn/ui as the baseline component system, with copied/owned components under `web/src/components/ui/`; serve the built `web/dist` assets from Axum at `/dashboard`, `/`, and asset URLs; keep all UI behavior backed by existing API endpoints; update CI and release workflows so the web package is verified and included in every whole-application release archive; resolve Docker packaging per the hands-off Docker default; update docs; run full verification.

Operate in this order:

1. Confirm repository root, branch, remotes, and current git status.
2. Read this full spec and inspect the referenced files before editing.
3. Implement Task 1 through Task 8 in order.
4. Use TDD for Rust route/static asset behavior and keep tests hardware-free.
5. Run frontend verification: `cd web && pnpm install --frozen-lockfile && pnpm typecheck && pnpm build`.
6. Run Rust verification with rustup first in PATH: `export PATH="$HOME/.cargo/bin:$PATH"`, then `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features`.
7. Verify release workflow code checks archive contents for `serialport-api/web/index.html`, at least one compiled asset under `serialport-api/web/assets/`, and `web_built=true` in `ARTIFACT.txt`.
8. Review `git diff --check` and the final changed files.
9. Commit if the execution environment allows commits; otherwise leave a clean, verified working tree summary with exact commands run.

Strictly do not add authentication, new serial behavior, hardware-dependent tests, prebuilt dashboard templates, Redux, TanStack Query, React Router, Playwright, charting libraries, or heavy UI dependencies beyond React/Vite/TypeScript/Tailwind/shadcn/ui. Do not commit secrets. If current package/tool versions require small deviations from examples, preserve the specified behavior and document the deviation in the final summary.

Final response must include changed files, commands run and results, Docker decision, release packaging verification status, and any intentional deviations from the spec.
