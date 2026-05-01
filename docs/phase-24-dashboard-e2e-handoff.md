# Phase 24 Dashboard Browser E2E Test Handoff

> **For Hermes / next AI implementation session:** Execute this in a fresh implementation session. Load `writing-plans`, `test-driven-development`, and `github-pr-workflow` before editing. Use browser-level tests to lock down stable dashboard behavior only. Keep the suite hardware-free, deterministic, and conservative; do not expand product features while adding tests.

**Goal:** Add a Playwright-based browser end-to-end test suite for the built-in React dashboard that verifies stable same-origin UI behavior against mocked API responses and, optionally, one local server smoke path. The suite must run without physical serial devices and without requiring `--real-serial`.

**Source context:** `docs/phase-21-web-dashboard-handoff.md` suggested Phase 24: “Dashboard end-to-end tests — Add Playwright or another browser-level test suite only after the MVP dashboard route and CI/release packaging are stable.” Phases 21-23 are now complete: the dashboard is served by Axum/release/Docker packaging, live serial event streaming is implemented, and `GET /api/v1/status` supplies safe dashboard runtime metadata.

---

## Strict Orchestration Input Schema

The implementation agent should accept this document plus the repository as its complete input. No hidden user choices are required.

```json
{
  "agent_role": "implementation",
  "phase": "Phase 24",
  "repository": "/home/alfarie/repos/serialport-api",
  "branch": "rewrite/axum-serial-api",
  "base_commit_expected_any_descendant_of": [
    "699007f feat: add React dashboard release bundle",
    "0a06465 feat: add live serial event streaming",
    "15a81cb feat: add safe dashboard status endpoint"
  ],
  "required_artifact_to_read": "docs/phase-24-dashboard-e2e-handoff.md",
  "toolchain_env": {
    "PATH_prefix": "$HOME/.cargo/bin",
    "node_version": "20",
    "package_manager": "pnpm",
    "browser_test_runner": "@playwright/test"
  },
  "scope": "Add conservative browser-level dashboard E2E tests for stable UI behavior without serial hardware",
  "non_goals": [
    "Authentication, accounts, authorization, sessions, TLS, CORS policy design, or secret handling",
    "New dashboard features, redesign, routing changes, new API endpoints, or serial protocol changes",
    "Tests requiring physical serial devices, OS-specific serial ports, real serial mode, Docker, GHCR, GitHub releases, or external network access",
    "Visual snapshot/regression infrastructure unless explicitly limited to Playwright traces/screenshots on failure",
    "Broad browser matrix expansion beyond Chromium in CI unless maintainers request it later",
    "Replacing existing Rust route tests, TypeScript typechecks, dashboard build checks, or release packaging checks",
    "Making browser tests flaky by depending on timing of real EventSource streams, host hardware, current time, or generated asset filenames",
    "Pushing commits or tags to a remote"
  ]
}
```

---

## Strict Orchestration Output Schema

The implementation agent's final response must use this JSON shape:

```json
{
  "agent_role": "implementation",
  "phase": "Phase 24",
  "summary": [
    "Added Playwright browser E2E coverage for dashboard initial render, mocked API data, connection/preset flows, and graceful error states.",
    "Integrated the browser test command into local verification and CI with hardware-free mocked network fixtures."
  ],
  "files_changed": [
    "web/package.json",
    "web/pnpm-lock.yaml",
    "web/playwright.config.ts",
    "web/e2e/dashboard.spec.ts",
    "web/e2e/fixtures.ts",
    ".github/workflows/ci.yml",
    "README.md",
    "docs/phase-24-dashboard-e2e-handoff.md"
  ],
  "verification": {
    "commands_run": [
      "cd web && pnpm install --frozen-lockfile",
      "cd web && pnpm typecheck",
      "cd web && pnpm build",
      "cd web && pnpm exec playwright install --with-deps chromium",
      "cd web && pnpm e2e",
      "cargo fmt --check",
      "cargo check",
      "cargo clippy --all-targets --all-features -- -D warnings",
      "cargo test --all-features",
      "git diff --check"
    ],
    "status": "passed|blocked"
  },
  "commit": "<sha or null>",
  "approval_status": "ready_for_review|blocked|deferred",
  "issues": []
}
```

If blocked, set `commit` to `null`, `approval_status` to `blocked`, and list exact blockers. If implementation is intentionally deferred after investigation, update only docs that record the deferral, run applicable lightweight checks, and commit with a `docs:` message.

---

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
- Recent history includes Phase 21, Phase 22, and Phase 23 commits or descendants:
  - `699007f feat: add React dashboard release bundle`
  - `0a06465 feat: add live serial event streaming`
  - `15a81cb feat: add safe dashboard status endpoint`

If the working tree is not clean before Phase 24 edits, stop and report instead of modifying files unless the only dirty file is this handoff artifact created by the planning agent.

---

## Current Repository Context Observed During Planning

Repository path:

```text
/home/alfarie/repos/serialport-api
```

Expected branch:

```text
rewrite/axum-serial-api
```

Recent observed history at planning time:

```text
15a81cb feat: add safe dashboard status endpoint
acdaf1c docs: add phase 23 dashboard status handoff
9774afc docs: describe live event stream behavior
0a06465 feat: add live serial event streaming
822eb34 docs: add phase 22 live streaming handoff
699007f feat: add React dashboard release bundle
```

Important current code shape:

- `web/package.json`
  - Package name: `serialport-api-dashboard`.
  - Package manager: `pnpm@10.33.0`.
  - Existing scripts: `dev`, `build`, `typecheck`, `preview`.
  - No browser E2E script or Playwright dependency exists yet.
- `web/src/App.tsx`
  - Single React dashboard component with local state.
  - On mount, calls `refresh()` and opens `new EventSource("/api/v1/events")`.
  - `refresh()` fetches `health`, `status`, `ports`, `connections`, and `presets` concurrently.
  - Renders status cards: Server, Mode, Presets, Ports, EventSource.
  - Renders tabs: Control, Events, Presets.
  - Control tab supports connection create/drop, selected command target, JSON command textarea, `waitForResponse`, timeout, send command, and save-as-preset.
  - Events tab displays live SSE event history and empty state.
  - Presets tab supports preset create, load, send, and delete.
  - UI currently relies on accessible text labels more than `data-testid` attributes. E2E selectors should prefer roles/labels/text first; add a small number of stable `data-testid` values only when needed.
- `web/src/api.ts`
  - Defines typed dashboard API response/request shapes.
  - API paths used by the UI:
    - `GET /api/v1/health`
    - `GET /api/v1/status`
    - `GET /api/v1/ports`
    - `GET /api/v1/connections`
    - `POST /api/v1/connections`
    - `DELETE /api/v1/connections/:name`
    - `POST /api/v1/connections/:name/commands`
    - `GET /api/v1/presets`
    - `POST /api/v1/presets`
    - `DELETE /api/v1/presets/:id`
  - `parseJsonObject()` throws if the command payload is not a JSON object.
- `src/api/routes.rs`
  - Serves dashboard at `/` and `/dashboard` and static Vite assets at `/assets/:file`.
  - Provides `GET /api/v1/status` with safe response shape:
    - `server.status`, `server.version`
    - `runtime.mode`, `runtime.realSerial`, `runtime.mockDevice`, `runtime.mockScriptConfigured`
    - `serialDefaults.defaultPortConfigured`, `serialDefaults.baudRate`, `serialDefaults.delimiter`
    - `storage.presets`, `storage.persistentPresets`
  - Provides long-lived SSE at `GET /api/v1/events`; fresh servers may have no event data until commands or mock/real serial input produce events.
  - Existing Rust tests cover hardware-free route/static/dashboard/status/stream behavior.
- `.github/workflows/ci.yml`
  - Single `rust` job on Ubuntu.
  - Installs Node.js 20 with pnpm cache, enables Corepack, installs web dependencies, typechecks and builds dashboard.
  - Installs `pkg-config libudev-dev`, then runs `cargo fmt --check`, clippy, and `cargo test --all-features`.
- `.github/workflows/release.yml`
  - Already verifies frontend build and release packages dashboard assets.
  - Phase 24 should not need release workflow changes unless the browser test command is intentionally added to release verification. Prefer CI-only E2E at first to keep tag releases conservative.
- Docs:
  - `README.md` documents dashboard build/dev/server usage and API routes.
  - `docs/phase-21-web-dashboard-handoff.md`, `docs/phase-22-live-streaming-handoff.md`, and `docs/phase-23-dashboard-status-handoff.md` are the phase sources of truth.

---

## Phase 24 Scope

Do in Phase 24:

1. Add Playwright as the browser-level test runner under `web/`.
2. Add a deterministic E2E test suite that runs against Vite's web server and mocks same-origin API requests in the browser context.
3. Cover stable dashboard behaviors that should not require serial hardware:
   - initial dashboard render using mocked health/status/ports/connections/presets responses,
   - displayed status cards and empty states,
   - connection creation request body and refreshed connection list,
   - command submission request body for selected connection,
   - invalid JSON payload error shown in UI without sending an API request,
   - preset create/load/delete behaviors with mocked API responses,
   - graceful API failure notice for at least one failed endpoint,
   - EventSource behavior does not make tests hang; the page should show either a mocked stream status or a stable fallback state.
4. Integrate a local script such as `pnpm e2e` into `web/package.json`.
5. Integrate the E2E command into `.github/workflows/ci.yml` after the dashboard build, using Chromium only.
6. Document local E2E setup and execution in `README.md` or a small `docs/dashboard-e2e.md` linked from README.
7. Keep the suite hardware-free and independent of physical serial ports.
8. Run full verification before committing.

Preferred tool choice: Playwright (`@playwright/test`) with Chromium in CI. If Playwright is impossible in the environment, use an equivalent browser-level framework only if it supports the same mocked API coverage and CI behavior; document the deviation clearly.

---

## Non-goals / Explicitly Out of Scope

Do **not** do these in Phase 24:

- Do not add authentication, authorization, user management, TLS, CORS policy design, or secrets.
- Do not redesign the dashboard or add new product features.
- Do not change serial connection semantics, command framing, event schemas, event retention, preset storage semantics, or config/status endpoint schemas except for minimal testability attributes.
- Do not require attached serial devices or stable host serial port names in any automated test.
- Do not require the Rust server for the primary browser suite if Vite plus route mocks can cover the dashboard faster and more deterministically.
- Do not add a broad browser matrix in CI. Use Chromium only initially; leave Firefox/WebKit as a follow-up if needed.
- Do not add screenshot/visual snapshot assertions that are likely to be brittle. Playwright traces/screenshots/videos on failure are acceptable.
- Do not replace existing Rust route tests, frontend typecheck/build checks, release packaging checks, or manual smoke docs.
- Do not make E2E tests depend on wall-clock timing, generated asset filenames, event ordering from real streams, or external network services.
- Do not commit `web/test-results/`, `web/playwright-report/`, videos, screenshots, traces, downloaded browser binaries, `node_modules/`, or local cache directories.

---

## Stable Test Fixture Schemas

Use fixtures that mirror the current API contracts exactly. Keep fixture builders in a dedicated test helper, e.g. `web/e2e/fixtures.ts`, so schema drift is visible during reviews.

### `GET /api/v1/health`

Response fixture:

```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

### `GET /api/v1/status`

Response fixture:

```json
{
  "server": { "status": "ok", "version": "0.1.0" },
  "runtime": {
    "mode": "memory",
    "realSerial": false,
    "mockDevice": false,
    "mockScriptConfigured": false
  },
  "serialDefaults": {
    "defaultPortConfigured": false,
    "baudRate": 115200,
    "delimiter": "\r\n"
  },
  "storage": {
    "presets": "memory",
    "persistentPresets": false
  }
}
```

### `GET /api/v1/ports`

Default response fixture:

```json
{
  "ports": []
}
```

Optional populated response fixture:

```json
{
  "ports": [
    {
      "name": "/dev/ttyUSB0",
      "type": "usb",
      "manufacturer": "Test Adapter",
      "serial_number": "TEST123"
    }
  ]
}
```

Do not assert that this device exists on the host. It is only a mocked response.

### `GET /api/v1/connections`

Empty response fixture:

```json
{
  "connections": []
}
```

Populated response fixture:

```json
{
  "connections": [
    {
      "name": "default",
      "status": "connected",
      "port": "/dev/ROBOT",
      "baudRate": 115200,
      "delimiter": "\r\n"
    }
  ]
}
```

### `POST /api/v1/connections`

Expected request body:

```json
{
  "name": "default",
  "port": "/dev/ROBOT",
  "baudRate": 115200,
  "delimiter": "\r\n"
}
```

Response fixture:

```json
{
  "status": "connected",
  "connection": {
    "name": "default",
    "status": "connected",
    "port": "/dev/ROBOT",
    "baudRate": 115200,
    "delimiter": "\r\n"
  }
}
```

### `POST /api/v1/connections/:name/commands`

Expected fire-and-forget request body:

```json
{
  "payload": {
    "method": "query",
    "topic": "sensor.read",
    "data": {}
  },
  "waitForResponse": false
}
```

Response fixture:

```json
{
  "status": "queued",
  "reqId": "1"
}
```

Expected waited request body when the UI checkbox is enabled:

```json
{
  "payload": {
    "method": "query",
    "topic": "sensor.read",
    "data": {}
  },
  "waitForResponse": true,
  "timeoutMs": 2000
}
```

Waited response fixture:

```json
{
  "status": "ok",
  "reqId": "1",
  "response": { "ok": true }
}
```

### `GET /api/v1/presets`

Empty response fixture:

```json
{
  "presets": []
}
```

Populated response fixture:

```json
{
  "presets": [
    {
      "id": 1,
      "name": "Read sensor",
      "payload": {
        "method": "query",
        "topic": "sensor.read",
        "data": {}
      }
    }
  ]
}
```

### `POST /api/v1/presets`

Expected request body:

```json
{
  "name": "Read sensor",
  "payload": {
    "method": "query",
    "topic": "sensor.read",
    "data": {}
  }
}
```

Response fixture:

```json
{
  "preset": {
    "id": 1,
    "name": "Read sensor",
    "payload": {
      "method": "query",
      "topic": "sensor.read",
      "data": {}
    }
  }
}
```

### `DELETE /api/v1/presets/:id`

Response fixture:

```json
{
  "status": "deleted",
  "id": 1
}
```

### Error response fixture

Use for negative-path tests:

```json
{
  "error": "simulated failure"
}
```

Return it with an appropriate HTTP status such as `500` and status text inherited from Playwright/Fetch.

---

## Recommended File Plan

Create:

- `web/playwright.config.ts`
- `web/e2e/dashboard.spec.ts`
- `web/e2e/fixtures.ts`
- Optional: `web/e2e/README.md` or `docs/dashboard-e2e.md` if README would become too long.

Modify:

- `web/package.json`
- `web/pnpm-lock.yaml`
- `.github/workflows/ci.yml`
- `README.md`
- Optional, only if selectors are otherwise brittle: `web/src/App.tsx`
- Optional, only if needed for repo hygiene: `.gitignore`

Do not modify unless a verified bug blocks tests:

- `src/api/routes.rs`
- `web/src/api.ts`
- `.github/workflows/release.yml`
- Docker files

---

## Test Design Requirements

### Test runner configuration

Add a Playwright config under `web/` with these conservative defaults:

- `testDir: './e2e'`
- one Chromium project only for CI speed and stability,
- `webServer` starts Vite with a deterministic port, e.g. `pnpm dev --host 127.0.0.1 --port 4174`,
- `baseURL: 'http://127.0.0.1:4174'`,
- `reuseExistingServer: !process.env.CI`,
- traces on first retry or on failure,
- low retries locally and one retry in CI if desired,
- reporter suitable for CI, e.g. `list` or `github` plus HTML report generated but not committed.

Example direction, adjust to current Playwright syntax:

```ts
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? 'github' : 'list',
  use: {
    baseURL: 'http://127.0.0.1:4174',
    trace: 'on-first-retry',
  },
  webServer: {
    command: 'pnpm dev --host 127.0.0.1 --port 4174',
    url: 'http://127.0.0.1:4174',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
  ],
});
```

### API mocking approach

Prefer Playwright `page.route()` for all `/api/v1/**` requests. Mock before `page.goto('/')` so the dashboard's mount-time `refresh()` cannot race the route setup.

Implement helper functions in `web/e2e/fixtures.ts` similar to:

- `installDashboardApiMocks(page, overrides?)`
- `expectJsonRequest(route, expectedBody)` or a captured request log helper
- fixture builders for health/status/ports/connections/presets

The mocks should support state transitions. Example: after `POST /api/v1/connections`, the next `GET /api/v1/connections` returns the created connection.

### EventSource handling

The dashboard opens `/api/v1/events` through the browser's `EventSource`, which may not be fully controlled by `page.route()` in all browser contexts. Use one of these deterministic options:

1. Preferred: inject a minimal mock `EventSource` before page scripts run with `page.addInitScript()` in the E2E helper. It should:
   - expose `CONNECTING`, `OPEN`, and `CLOSED` constants,
   - call `onopen` asynchronously,
   - support `addEventListener(eventName, handler)`,
   - optionally emit one synthetic `serial.log` or `serial.json` event for an Events-tab test,
   - implement `close()` and `readyState`.
2. Acceptable: route `/api/v1/events` to a small finite `text/event-stream` response and assert only that the UI does not crash. Do not depend on real server live-stream timing.

Do not start the Rust server just to satisfy EventSource in the primary E2E suite.

### Selector policy

Use resilient user-facing selectors:

- `getByRole('heading', { name: /Serial control dashboard/i })`
- `getByRole('button', { name: /Refresh/i })`
- `getByRole('tab', { name: /Presets/i })`
- `getByLabel('Name')`, `getByLabel('Port')`, `getByLabel('Baud')`, `getByLabel('Delimiter')`
- `getByText('No active connections')`, `getByText('No presets saved')`, etc.

If duplicate labels or custom components make selectors ambiguous, add minimal `data-testid` attributes in `web/src/App.tsx` for stable controls only. Keep the production UI unchanged.

---

## Required E2E Scenarios

Implement these as focused tests. Prefer a small number of scenario tests over many brittle micro-tests.

### 1. Initial dashboard render with empty hardware-free state

Arrange:

- Mock health/status/ports/connections/presets with the default empty fixtures.
- Mock EventSource to open successfully.

Act:

- Visit `/` or `/dashboard` on the Vite dev server.

Assert:

- Main heading `Serial control dashboard` is visible.
- Status cards show server version/status, `memory` mode, memory preset storage, `0 visible` ports, and connected EventSource state or a stable mocked equivalent.
- Control tab shows no active connections.
- Serial ports card shows no ports reported.
- No uncaught console errors occur.

### 2. Initial dashboard render with populated fixtures

Arrange:

- Mock one port, one connection, and one preset.

Assert:

- Connection table shows `default`, `/dev/ROBOT`, and connected status.
- Command target can select or displays `default`.
- Presets tab shows `Read sensor` and its JSON payload.
- Ports table shows mocked port data.

### 3. Connect flow posts decoded delimiter and refreshes list

Arrange:

- Start with empty connections.
- Capture `POST /api/v1/connections` request body.
- After POST, mocked state returns the new connection in subsequent `GET /api/v1/connections`.

Act:

- Fill Name, Port, Baud, and Delimiter fields if needed.
- Click `Connect`.

Assert:

- Request body contains `baudRate` as a number and delimiter decoded from `\\r\\n` to `\r\n`.
- Success notice says `Connected default` or subsequent ready message is visible.
- Connection row appears.

### 4. Send command posts parsed JSON to selected connection

Arrange:

- Mock one existing connection named `default`.
- Capture `POST /api/v1/connections/default/commands` request body.

Act:

- Ensure JSON textarea contains the default command payload.
- Click `Send command`.

Assert:

- Request body includes parsed object payload, not a raw string.
- `waitForResponse` is `false` unless the checkbox was toggled.
- UI displays `Command 1 queued` or equivalent success notice.

### 5. Invalid JSON payload fails locally without API request

Arrange:

- Mock one existing connection.
- Track command POST call count.

Act:

- Replace JSON textarea with invalid JSON or a JSON array.
- Click `Send command`.

Assert:

- UI displays an error notice (`Action failed`) and the parse error message.
- No command POST request is sent.

### 6. Preset create/load/delete flow

Arrange:

- Start with no presets.
- Capture preset POST body.
- After save, mocked state returns the created preset.

Act/Assert:

- Click `Save as preset` or use the Presets tab create form.
- Request body contains preset name and parsed payload.
- Preset appears in Presets tab.
- Clicking `Load` copies the preset payload into the command textarea and shows loaded notice.
- Clicking delete sends `DELETE /api/v1/presets/1` and removes the preset from the UI after refresh.

### 7. API failure notice is visible

Arrange:

- Make one refresh endpoint, preferably `/api/v1/status` or `/api/v1/ports`, return HTTP 500 with `{ "error": "simulated failure" }`.

Assert:

- Dashboard does not crash.
- Error notice includes `Action failed` or `Status unavailable` and mentions the simulated failure or HTTP status.

### 8. Events tab shows mocked live event if feasible

Arrange:

- Mock EventSource to emit `serial.log` or `serial.json` after open.

Act:

- Open Events tab.

Assert:

- Event name and payload appear in the log.

If this proves brittle due to browser/runtime constraints, keep the mocked EventSource open-state test and document a follow-up for richer stream event assertions.

---

## CI Integration Requirements

Modify `.github/workflows/ci.yml` conservatively:

1. Keep existing Node setup, Corepack, `pnpm install --frozen-lockfile`, `pnpm typecheck`, and `pnpm build` steps.
2. Add a Chromium browser install step after dependency installation and before E2E execution:

```yaml
- name: Install Playwright Chromium
  working-directory: web
  run: pnpm exec playwright install --with-deps chromium
```

3. Add E2E execution after `pnpm build`:

```yaml
- name: Run dashboard browser E2E tests
  working-directory: web
  run: pnpm e2e
```

4. Keep Rust native dependency install and Rust checks intact.
5. Do not add E2E to the release workflow unless the implementation agent explicitly verifies it will not make tag builds slow/flaky. CI coverage is sufficient for Phase 24.
6. If Playwright report artifacts are useful on CI failure, add `actions/upload-artifact` only for `web/playwright-report` and/or `web/test-results` with `if: failure()`. Do not require this if the workflow should stay minimal.

---

## TDD / Task Steps

Execute in this order:

### Task 1: Add Playwright dependency and scripts

Files:

- Modify: `web/package.json`
- Modify: `web/pnpm-lock.yaml`

Implementation:

- Add dev dependency `@playwright/test`.
- Add scripts:

```json
{
  "e2e": "playwright test",
  "e2e:ui": "playwright test --ui"
}
```

Optional scripts:

```json
{
  "e2e:headed": "playwright test --headed",
  "e2e:debug": "playwright test --debug"
}
```

Verification:

```bash
cd web
pnpm install
pnpm exec playwright --version
```

Use `pnpm install --frozen-lockfile` after the lockfile is updated.

### Task 2: Create Playwright config

Files:

- Create: `web/playwright.config.ts`

Implementation:

- Configure Chromium-only tests, base URL, Vite web server, trace behavior, and CI-safe settings as described above.

Verification:

```bash
cd web
pnpm exec playwright test --list
```

Expected: command succeeds after at least one spec exists; before specs exist it may report no tests.

### Task 3: Build mocked fixture/helper layer test-first

Files:

- Create: `web/e2e/fixtures.ts`

Implementation:

- Define fixture objects using the schemas in this document.
- Define route/mock helpers and optional `MockEventSource` install script.
- Define request capture utilities for asserting exact POST/DELETE bodies.

Verification:

- Covered by the spec tests in Task 4.

### Task 4: Add dashboard E2E specs

Files:

- Create: `web/e2e/dashboard.spec.ts`
- Optional: Modify `web/src/App.tsx` only for minimal stable selectors.

Implementation:

- Add scenarios listed in **Required E2E Scenarios**.
- Fail tests on unexpected page errors and severe console errors. Allow known Vite dev logs if needed.
- Keep waits based on visible UI expectations, not fixed sleeps.

Verification:

```bash
cd web
pnpm exec playwright install chromium
pnpm e2e
```

If Linux dependencies are missing locally, use:

```bash
cd web
pnpm exec playwright install --with-deps chromium
pnpm e2e
```

### Task 5: Integrate CI

Files:

- Modify: `.github/workflows/ci.yml`

Implementation:

- Add Playwright Chromium install step.
- Add `pnpm e2e` step after frontend build.
- Keep existing Rust verification unchanged.

Verification:

```bash
python3 - <<'PY'
from pathlib import Path
text = Path('.github/workflows/ci.yml').read_text()
required = [
    'pnpm install --frozen-lockfile',
    'pnpm typecheck',
    'pnpm build',
    'pnpm exec playwright install --with-deps chromium',
    'pnpm e2e',
]
missing = [item for item in required if item not in text]
assert not missing, missing
PY
```

### Task 6: Update docs and gitignore

Files:

- Modify: `README.md`
- Optional: Create `docs/dashboard-e2e.md`
- Optional: Modify `.gitignore`

Documentation must include:

- local browser E2E install and run commands,
- note that tests use mocked API fixtures and do not require serial hardware,
- CI behavior summary,
- locations of specs and config.

If not already ignored, add:

```gitignore
web/playwright-report/
web/test-results/
```

Do not ignore committed spec/config files.

### Task 7: Full verification and final commit

Run:

```bash
cd /home/alfarie/repos/serialport-api
export PATH="$HOME/.cargo/bin:$PATH"

cd web
pnpm install --frozen-lockfile
pnpm typecheck
pnpm build
pnpm exec playwright install --with-deps chromium
pnpm e2e
cd ..

cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features

git diff --check
git status --short
```

Expected commit message for implementation:

```bash
git add web/package.json web/pnpm-lock.yaml web/playwright.config.ts web/e2e .github/workflows/ci.yml README.md docs/dashboard-e2e.md .gitignore
# Only add web/src/App.tsx if minimal selectors were required.
git commit -m "test: add dashboard browser e2e coverage"
```

---

## Acceptance Criteria

Phase 24 is complete only when all of these are true:

1. `web/package.json` includes a browser E2E script (`pnpm e2e`) and Playwright dev dependency.
2. `web/pnpm-lock.yaml` is updated consistently.
3. `web/playwright.config.ts` exists and starts Vite via Playwright `webServer` on a deterministic local port.
4. Browser tests live under `web/e2e/` and cover the required stable dashboard scenarios without serial hardware.
5. All API calls used by primary E2E tests are mocked or otherwise deterministic.
6. The E2E suite does not require `cargo run`, `--real-serial`, attached devices, Docker, release artifacts, or external services.
7. Tests assert exact request bodies for connect, command, and preset create flows.
8. Tests prove invalid JSON is handled in the browser without sending a command request.
9. Tests prove at least one API failure renders a visible dashboard error state without crashing.
10. EventSource is mocked or handled deterministically so the suite does not hang or flap.
11. CI installs the Playwright Chromium browser/dependencies and runs `pnpm e2e` after frontend build.
12. README or linked docs explain how to run the E2E suite locally and that tests are hardware-free.
13. Playwright reports/results are ignored or only uploaded as CI failure artifacts; generated artifacts are not committed.
14. Existing checks still pass:
    - `cd web && pnpm typecheck`
    - `cd web && pnpm build`
    - `cargo fmt --check`
    - `cargo check`
    - `cargo clippy --all-targets --all-features -- -D warnings`
    - `cargo test --all-features`
15. `git diff --check` passes.
16. Final commit contains only intentional Phase 24 changes.

---

## Optional Follow-ups Outside Phase 24

These are intentionally not required now:

- Add Firefox/WebKit to Playwright projects after Chromium is stable in CI.
- Add an Axum-backed browser smoke test that builds `web/dist`, starts `cargo run -- serve`, and visits `/dashboard` against the real server.
- Add screenshot visual regression tests after visual baseline policy is agreed.
- Add accessibility scanning with a dedicated tool such as axe after selectors and semantics stabilize.
- Add release-workflow browser E2E if maintainers decide tag builds should pay the additional runtime cost.

---

## Copy/Paste Prompt for Hands-Off Implementation Agent

You are implementing Phase 24 in `/home/alfarie/repos/serialport-api` on branch `rewrite/axum-serial-api`. Use `docs/phase-24-dashboard-e2e-handoff.md` as the source of truth and complete the phase end-to-end without asking for clarification unless there is a hard safety blocker or missing credential that cannot be worked around.

Required outcome: add conservative Playwright browser E2E coverage for the React dashboard under `web/`, using mocked same-origin API fixtures so tests are deterministic and hardware-free. Cover stable initial render, status/empty states, connection creation, command sending, invalid JSON handling, preset create/load/delete, API failure notice, and deterministic EventSource behavior. Add `pnpm e2e`, wire it into CI, document local usage, run full verification, and commit the implementation.

Operate in this order:

1. Confirm repository root, branch, recent history, and clean git status.
2. Read this full spec and inspect `web/package.json`, `web/src/App.tsx`, `web/src/api.ts`, `src/api/routes.rs`, `.github/workflows/ci.yml`, and relevant docs before editing.
3. Add Playwright dependency/scripts and lockfile updates.
4. Add Playwright config using Vite `webServer` and Chromium-only project.
5. Build fixture/mock helpers for exact API schemas and deterministic EventSource.
6. Add dashboard E2E specs for the required scenarios.
7. Add minimal selectors to `web/src/App.tsx` only if role/label/text selectors are inadequate.
8. Update CI to install Chromium and run `pnpm e2e` after frontend build.
9. Update README/docs and ignore generated Playwright artifacts if necessary.
10. Run frontend, Playwright, Rust, and diff verification commands.
11. Commit with `test: add dashboard browser e2e coverage` if verification passes.

Strictly do not add authentication, new dashboard features, new serial behavior, hardware-dependent tests, broad browser matrix, visual snapshot requirements, release workflow churn, or external-service dependencies. Do not commit generated reports, traces, videos, screenshots, browser binaries, `node_modules`, or secrets. If current package/tool versions require small deviations from examples, preserve the specified behavior and document the deviation in the final summary.
