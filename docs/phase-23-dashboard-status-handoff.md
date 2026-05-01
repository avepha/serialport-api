# Phase 23 Dashboard Status/Configuration Endpoint Handoff

> **For Hermes / next AI implementation session:** Execute this in a fresh implementation session. Load `writing-plans`, `test-driven-development`, and `rust-axum-api-tdd` before editing. This phase adds a conservative dashboard-readable status/config endpoint that exposes only safe, resolved runtime facts needed by the UI. Do not leak filesystem paths, secrets, environment variables, OS/user/hostname details, or serial hardware metadata beyond existing explicit API routes.

**Goal:** Add a read-only endpoint for the React dashboard to discover safe server mode and serial/storage defaults, then use it to initialize and display dashboard runtime status without hard-coded connection defaults.

**Source context:** `docs/phase-21-web-dashboard-handoff.md` suggested Phase 23: “Dashboard configuration/status endpoint — Add an endpoint that exposes resolved server mode and safe serial defaults, e.g. mock/real mode, configured preset DB presence, and default baud/delimiter. Avoid leaking filesystem paths or sensitive host details by default.” Phase 22 has already added live event streaming.

---

## Strict Orchestration Input Schema

The implementation agent should accept this document plus the repository as its complete input. No hidden user choices are required.

```json
{
  "agent_role": "implementation",
  "phase": "Phase 23",
  "repository": "/home/alfarie/repos/serialport-api",
  "branch": "rewrite/axum-serial-api",
  "base_commit_expected_any_descendant_of": [
    "699007f feat: add React dashboard release bundle",
    "822eb34 docs: add phase 22 live streaming handoff",
    "0a06465 feat: add live serial event streaming",
    "9774afc docs: describe live event stream behavior"
  ],
  "required_artifact_to_read": "docs/phase-23-dashboard-status-handoff.md",
  "toolchain_env": {
    "PATH_prefix": "$HOME/.cargo/bin",
    "node_version": "20",
    "package_manager": "pnpm"
  },
  "scope": "Add a safe read-only dashboard status/config endpoint and consume it in the existing React dashboard",
  "non_goals": [
    "Authentication, authorization, sessions, TLS, CORS policy design, accounts, tokens, or secrets management",
    "Changing serial connection behavior, command framing, response matching, event streaming semantics, or real-vs-mock defaults",
    "Changing preset CRUD behavior or database schema",
    "Exposing absolute or relative filesystem paths for config files, mock scripts, dashboard assets, preset DBs, current directory, or devices",
    "Exposing host OS, username, hostname, network interfaces, process environment variables, CPU/memory/disk details, or CLI/env/config source provenance",
    "Adding writable config endpoints or runtime mutation of server settings",
    "Adding browser E2E tooling or major frontend dependencies",
    "Changing release archive layout, Docker runtime behavior, systemd service behavior, or GitHub Actions except if a failing existing check needs a minimal fix",
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
  "phase": "Phase 23",
  "summary": [
    "Added GET /api/v1/status returning safe resolved server mode, serial defaults, and storage mode.",
    "Updated the dashboard to fetch status, display mode/storage facts, and initialize connection defaults from the endpoint."
  ],
  "files_changed": [
    "README.md",
    "src/api/routes.rs",
    "src/config.rs",
    "src/main.rs",
    "web/src/App.tsx",
    "web/src/api.ts"
  ],
  "verification": {
    "commands_run": [
      "cargo fmt --check",
      "cargo check",
      "cargo clippy --all-targets --all-features -- -D warnings",
      "cargo test --all-features",
      "cd web && pnpm typecheck",
      "cd web && pnpm build",
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
- Recent history includes Phase 21 and Phase 22 commits or descendants:
  - `699007f feat: add React dashboard release bundle`
  - `822eb34 docs: add phase 22 live streaming handoff`
  - `0a06465 feat: add live serial event streaming`
  - `9774afc docs: describe live event stream behavior`

If the working tree is not clean before Phase 23 edits, stop and report instead of modifying files unless the only dirty file is this handoff artifact created by the planning agent.

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
9774afc docs: describe live event stream behavior
0a06465 feat: add live serial event streaming
822eb34 docs: add phase 22 live streaming handoff
699007f feat: add React dashboard release bundle
```

Important current code shape:

- `src/config.rs`
  - Defines built-in defaults:
    - `DEFAULT_HOST = "127.0.0.1"`
    - `DEFAULT_PORT = 4002`
    - `DEFAULT_SERIAL_BAUD_RATE = 115_200`
    - `DEFAULT_SERIAL_DELIMITER = "\r\n"`
  - `ResolvedServeConfig` currently contains:
    - `host: String`
    - `port: u16`
    - `mock_device: bool`
    - `mock_script: Option<PathBuf>`
    - `real_serial: bool`
    - `serial_defaults: SerialDefaults`
    - `preset_db: Option<PathBuf>`
  - `SerialDefaults` currently contains:
    - `default_port: Option<String>`
    - `default_baud_rate: u32`
    - `default_delimiter: String`
  - Resolution precedence is CLI > env host/port > TOML config > built-in defaults.
  - `mock_script` implies mock-device behavior.
  - Real serial cannot be combined with mock-device or mock-script.
- `src/main.rs`
  - Resolves `ResolvedServeConfig` inside `serve`.
  - Opens SQLite preset store when `resolved.preset_db` is present; otherwise uses `InMemoryPresetStore`.
  - Chooses real serial manager when `resolved.real_serial` is true.
  - Chooses mock responder manager when `resolved.mock_device` is true.
  - Otherwise uses `InMemoryConnectionManager`.
  - Currently logs `default_port`, `default_baud_rate`, `default_delimiter`, and `preset_db`; this phase need not change logging unless implementation requires it.
  - Currently constructs `routes::AppState::with_preset_store_arc(...)` without passing resolved status/config metadata.
- `src/api/routes.rs`
  - `AppState<L, C>` currently stores:
    - `port_lister`
    - `connection_manager`
    - `preset_store`
    - `dashboard_assets`
  - `router_with_state` exposes `GET /api/v1/health`, `GET /api/v1/events`, `GET /api/v1/events/ws`, `GET /api/v1/ports`, preset routes, connection routes, and legacy aliases.
  - `GET /api/v1/health` returns only `{ "status": "ok", "version": env!("CARGO_PKG_VERSION") }`.
  - Hardware-free route tests exist in the same file using `router()`, `router_with_port_lister`, and explicit `AppState` construction.
- `web/src/api.ts`
  - Contains typed API helpers for health, ports, connections, commands, and presets.
  - No status/config type or API helper exists yet.
- `web/src/App.tsx`
  - Fetches health, ports, connections, and presets in `refresh()`.
  - Opens `EventSource('/api/v1/events')`.
  - Hard-codes initial connect form values:
    - `name: "default"`
    - `port: "/dev/ROBOT"`
    - `baudRate: "115200"`
    - `delimiter: "\\r\\n"`
  - Displays top cards for Server, Ports, and EventSource.
- `README.md`
  - Documents config defaults, dashboard, real serial mode, mock mode, SQLite presets, and live event streams.

---

## Phase 23 Scope

Do in Phase 23:

1. Add a new read-only endpoint:
   - `GET /api/v1/status`
2. Return a deliberately small, stable, safe JSON response containing:
   - service health/version, equivalent to the current health endpoint but nested under `server`.
   - resolved serial runtime mode as a non-secret enum.
   - whether a mock script was configured, as a boolean only.
   - whether a configured default port exists, as a boolean only.
   - safe serial defaults: baud rate and delimiter string.
   - safe storage mode: in-memory vs SQLite-backed presets, as a non-secret enum/boolean only.
3. Thread safe resolved status metadata from `ResolvedServeConfig` into `AppState` when serving from `src/main.rs`.
4. Keep `router()` and test-only/default state creation hardware-free and deterministic by using safe built-in defaults.
5. Update the React dashboard to:
   - add a typed `DashboardStatusResponse` and `api.status()` helper.
   - include the status request in `refresh()`.
   - display runtime mode/storage/default baud in status cards or another compact existing dashboard area.
   - initialize connection baud/delimiter from the endpoint when returned.
   - initialize connection port only if the endpoint explicitly exposes a non-sensitive `defaultPort` value. The preferred schema below does **not** expose the port value, so the UI should keep a user-editable placeholder/default and may display “default port configured” as a boolean.
6. Update README API docs to mention the new endpoint and its privacy guarantees.
7. Add Rust tests proving both the exact safe response shape and the absence of leaked path-like fields.
8. Run the full verification suite.

---

## Non-goals / Explicitly Out of Scope

Do **not** do these in Phase 23:

- Do not add authentication, sessions, TLS, CORS, accounts, tokens, or role-based access.
- Do not add writable endpoints such as `PUT /api/v1/status` or runtime config mutation.
- Do not change `GET /api/v1/health`; keep it backwards-compatible.
- Do not change existing serial behavior, default mode, read loop, event streaming, command framing, response matching, preset CRUD, or legacy aliases.
- Do not expose filesystem paths for:
  - `--config`
  - auto-discovered `serialport-api.toml`
  - `--preset-db` / `[storage] preset_db`
  - `--mock-script` / `[serial] mock_script`
  - dashboard assets directory
  - current directory
- Do not expose environment variable names/values, CLI/config source provenance, hostnames, usernames, OS names, network interfaces, process IDs, CPU/memory/disk state, or serial device inventory through this new endpoint.
- Do not expose `ResolvedServeConfig.host` by default. It can reveal binding/network posture and is not needed by the same-origin dashboard.
- Do not expose `ResolvedServeConfig.port` unless there is a strong implementation reason. The browser already knows the current origin; returning the server bind port is unnecessary and can be misleading behind proxies.
- Do not expose `SerialDefaults.default_port` string in the MVP response. Serial device names can reveal host/device details and are already available through the explicit `GET /api/v1/ports` route when needed.
- Do not introduce a browser E2E framework, Redux/TanStack Query/React Router, or additional UI libraries.
- Do not alter CI/release packaging unless current tests reveal a small unrelated break that must be fixed to verify this phase.

---

## Stable API and Wire Schemas

### New endpoint

Path:

```text
GET /api/v1/status
```

Request:

```http
GET /api/v1/status HTTP/1.1
Accept: application/json
```

Response:

```http
HTTP/1.1 200 OK
content-type: application/json
```

Strict JSON response schema:

```json
{
  "server": {
    "status": "ok",
    "version": "<cargo package version>"
  },
  "runtime": {
    "mode": "mock|mock-script|real|memory",
    "realSerial": true,
    "mockDevice": false,
    "mockScriptConfigured": false
  },
  "serialDefaults": {
    "defaultPortConfigured": false,
    "baudRate": 115200,
    "delimiter": "\r\n"
  },
  "storage": {
    "presets": "memory|sqlite",
    "persistentPresets": false
  }
}
```

Field requirements:

- `server.status`
  - Type: string.
  - Required literal value: `"ok"`.
- `server.version`
  - Type: string.
  - Required value: `env!("CARGO_PKG_VERSION")`.
- `runtime.mode`
  - Type: string enum.
  - Allowed values:
    - `"real"` when `ResolvedServeConfig.real_serial == true`.
    - `"mock-script"` when not real and `mock_script.is_some()`.
    - `"mock"` when not real and `mock_device == true`.
    - `"memory"` when not real, no mock script, and `mock_device == false`.
  - This preserves the current code’s distinction between the default in-memory manager and explicit mock-device/script responders.
- `runtime.realSerial`
  - Type: boolean.
  - Mirrors resolved `real_serial`.
- `runtime.mockDevice`
  - Type: boolean.
  - Mirrors resolved `mock_device` after mock-script implication.
- `runtime.mockScriptConfigured`
  - Type: boolean.
  - `true` only when a mock script path was configured; never return the path.
- `serialDefaults.defaultPortConfigured`
  - Type: boolean.
  - `true` only when `SerialDefaults.default_port.is_some()`; never return the port value in this endpoint.
- `serialDefaults.baudRate`
  - Type: positive integer/u32 serialized as JSON number.
  - Mirrors resolved `SerialDefaults.default_baud_rate`.
- `serialDefaults.delimiter`
  - Type: string.
  - Mirrors resolved `SerialDefaults.default_delimiter`.
  - This can contain escaped control characters such as `"\r\n"` in JSON.
- `storage.presets`
  - Type: string enum.
  - `"sqlite"` when `ResolvedServeConfig.preset_db.is_some()`.
  - `"memory"` otherwise.
- `storage.persistentPresets`
  - Type: boolean.
  - `true` when `storage.presets == "sqlite"`; `false` otherwise.

### Example: default local server

```json
{
  "server": {
    "status": "ok",
    "version": "0.1.0"
  },
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

### Example: mock-device with SQLite presets and configured serial defaults

```json
{
  "server": {
    "status": "ok",
    "version": "0.1.0"
  },
  "runtime": {
    "mode": "mock",
    "realSerial": false,
    "mockDevice": true,
    "mockScriptConfigured": false
  },
  "serialDefaults": {
    "defaultPortConfigured": true,
    "baudRate": 57600,
    "delimiter": "\n"
  },
  "storage": {
    "presets": "sqlite",
    "persistentPresets": true
  }
}
```

### Example: mock-script mode

```json
{
  "server": {
    "status": "ok",
    "version": "0.1.0"
  },
  "runtime": {
    "mode": "mock-script",
    "realSerial": false,
    "mockDevice": true,
    "mockScriptConfigured": true
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

### Example: real serial mode

```json
{
  "server": {
    "status": "ok",
    "version": "0.1.0"
  },
  "runtime": {
    "mode": "real",
    "realSerial": true,
    "mockDevice": false,
    "mockScriptConfigured": false
  },
  "serialDefaults": {
    "defaultPortConfigured": true,
    "baudRate": 115200,
    "delimiter": "\r\n"
  },
  "storage": {
    "presets": "sqlite",
    "persistentPresets": true
  }
}
```

### Strict privacy constraints for response body

The response body must not contain keys or values named/shaped as:

- `host`, `bind`, `addr`, `address`, `port` for server binding details.
- `path`, `file`, `dir`, `cwd`, `home`, `config`, `mockScript`, `presetDb`, `dashboardAssets` as filesystem details. The boolean `mockScriptConfigured` is allowed; the enum `storage.presets` is allowed.
- `env`, `environment`, `SERIALPORT_API_*`, CLI argument source names, usernames, hostnames, OS names, process IDs, CPU/memory/disk metrics.
- Any absolute path starting with `/`, Windows-drive-like path such as `C:\`, or explicit relative paths such as `./presets.db`, `../...`, `web/dist`, or `mock-responses.json`.

Note: `serialDefaults.delimiter` may contain `/` if a user intentionally configures it, but tests should focus on preventing known path/source fields and configured path values from appearing. Do not reject or sanitize user delimiters.

---

## Recommended Internal Rust Types

Keep implementation simple and explicit. Recommended additions:

- In `src/api/routes.rs`, add serializable response structs near `HealthResponse`:
  - `DashboardStatusResponse`
  - `DashboardServerStatus`
  - `DashboardRuntimeStatus`
  - `DashboardSerialDefaults`
  - `DashboardStorageStatus`
- Add a cloneable safe state field to `AppState`:
  - `dashboard_status: DashboardStatusResponse`
- Add constructors/helpers so existing tests do not need to hand-fill the field everywhere:
  - Preferred: update all `AppState` constructors to initialize `DashboardStatusResponse::default_memory()` or similar.
  - Add `AppState::with_dashboard_status(mut self, status: DashboardStatusResponse) -> Self` for `src/main.rs`.
  - Alternative: store `Option<DashboardStatusResponse>` and default in handler, but a concrete value is simpler and more testable.
- To avoid leaking `PathBuf`, build the status response from `ResolvedServeConfig` in a conversion function that reduces paths to booleans before state creation:
  - Option A: `impl DashboardStatusResponse { pub fn from_resolved_config(config: &ResolvedServeConfig) -> Self }` in `routes.rs`.
  - Option B: create a safe `ResolvedDashboardStatus` type in `src/config.rs` and convert from `ResolvedServeConfig`, then pass it into routes.
  - Prefer Option A if it avoids exposing API response types from the config module. If `routes.rs` cannot depend on config cleanly, use Option B.

Do not make `AppState` store `ResolvedServeConfig` directly. It contains path-bearing fields and makes accidental leaks more likely.

---

## Exact File Plan

### Must modify

- `src/api/routes.rs`
  - Add response structs and route handler.
  - Add `GET /api/v1/status` to `router_with_state`.
  - Extend `AppState` with safe dashboard status metadata.
  - Add/adjust constructors to keep tests concise.
  - Add hardware-free route tests for default and configured status responses.
- `src/main.rs`
  - After resolving config, build safe status metadata before moving `resolved` into route state.
  - Pass safe metadata into every route-state construction path: real serial, mock-device/script, and default in-memory.
- `web/src/api.ts`
  - Add `DashboardStatusResponse` type and `api.status()` helper.
- `web/src/App.tsx`
  - Fetch status alongside health/ports/connections/presets.
  - Display runtime/storage/default facts.
  - Initialize baud/delimiter from `serialDefaults` when status is available.
  - Do not require the status endpoint for core dashboard rendering; if it fails, show an error notice and leave existing manual form behavior usable.
- `README.md`
  - Document `GET /api/v1/status`, its response shape, and privacy guardrails.

### Maybe modify, only if required by type ownership or docs consistency

- `src/lib.rs`
  - Only if new config/status conversion type needs module visibility changes.
- `docs/docker-release.md`
  - Only if it already has an API route list that would become misleading; otherwise skip.
- `docs/phase-23-dashboard-status-handoff.md`
  - Do not modify during implementation except to correct a discovered factual error in the handoff.

### Must not modify in this phase unless fixing verification fallout

- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- Dockerfile / compose files
- `Cargo.toml` / `Cargo.lock` except if an existing dependency is missing for tests; adding new dependencies should not be necessary.
- Serial transport/manager/read-loop internals.

---

## TDD Task List

Implement in this order.

### Task 1: Add safe status response tests first

Objective: Pin the public wire schema and privacy constraints before implementation.

Files:

- Modify: `src/api/routes.rs`

Add tests similar to:

1. `status_route_returns_safe_default_dashboard_status`
   - Build app with `router()` or `AppState::new(...)` default state.
   - Request `GET /api/v1/status`.
   - Assert `200 OK`.
   - Assert full JSON equals the default example with `version: "0.1.0"` or `env!("CARGO_PKG_VERSION")` as currently used by health tests.
   - Assert no extra fields by exact equality.
2. `status_route_reports_configured_modes_without_paths`
   - Construct an `AppState` with safe status equivalent to resolved config:
     - mode `mock-script`
     - `mockDevice: true`
     - `mockScriptConfigured: true`
     - `defaultPortConfigured: true`
     - `baudRate: 57600`
     - `delimiter: "\n"`
     - storage `sqlite`, `persistentPresets: true`
   - Request `GET /api/v1/status`.
   - Assert exact JSON response.
   - Convert body to string and assert it does not contain path-like configured values such as:
     - `mock-responses.json`
     - `presets.db`
     - `/dev/ttyUSB0`
     - `web/dist`
     - `serialport-api.toml`
     - `preset_db`
     - `mock_script`
     - `dashboard_assets`
3. If implementing `DashboardStatusResponse::from_resolved_config`, add a unit test proving conversion from a `ResolvedServeConfig` with `PathBuf` fields reduces those fields to booleans/enums and never stores the path strings.

Run targeted failure command after writing tests:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test status_route --all-features
```

Expected initially: tests fail to compile or fail because endpoint is absent.

### Task 2: Implement backend endpoint and safe state threading

Objective: Make tests pass without leaking unsafe details.

Files:

- Modify: `src/api/routes.rs`
- Modify: `src/main.rs`

Implementation requirements:

- Add `GET /api/v1/status` before dynamic/catch-all-like routes. There are no catch-alls now, but keep API routes grouped near health.
- Handler should be simple:

```rust
async fn status<L, C>(State(state): State<AppState<L, C>>) -> Json<DashboardStatusResponse>
where
    L: SerialPortLister,
    C: ConnectionManager,
{
    Json(state.dashboard_status.clone())
}
```

- Response structs must derive `Debug`, `Clone`, `Serialize`, and enough traits for tests (`PartialEq`, `Eq` helpful but optional).
- Use serde renames for camelCase fields:
  - `realSerial`
  - `mockDevice`
  - `mockScriptConfigured`
  - `serialDefaults`
  - `defaultPortConfigured`
  - `baudRate`
  - `persistentPresets`
- Use string enums or literal strings. If using Rust enums, set `#[serde(rename_all = "kebab-case")]` where needed and test exact output.
- Do not expose `ResolvedServeConfig` or `PathBuf` through serde.
- In `src/main.rs`, avoid moving `resolved` before building status. Recommended pattern:

```rust
let dashboard_status = routes::DashboardStatusResponse::from_resolved_config(&resolved);
...
routes::AppState::with_preset_store_arc(...).with_dashboard_status(dashboard_status.clone())
```

- All three route state construction branches must use the same `dashboard_status` clone.
- Default constructors (`AppState::new`, `with_preset_store`, `with_preset_store_arc`, `router`, `router_with_port_lister`) should continue to compile and use memory/default status.

Verification:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test status_route --all-features
cargo test health_route_returns_status_and_version --all-features
cargo test --all-features
```

### Task 3: Add config conversion tests if not already covered

Objective: Prove resolved CLI/config modes map correctly to the endpoint-safe representation.

Files:

- Modify: `src/api/routes.rs` or `src/config.rs` depending on where conversion lives.

Recommended cases:

- Default `ResolvedServeConfig` -> `runtime.mode == "memory"`, storage memory, baud `115200`, delimiter `"\r\n"`, no default port configured.
- `mock_device == true` without script -> `runtime.mode == "mock"`.
- `mock_script.is_some()` -> `runtime.mode == "mock-script"`, `mockDevice == true`, `mockScriptConfigured == true`.
- `real_serial == true` -> `runtime.mode == "real"`.
- `preset_db.is_some()` -> storage sqlite/persistent true.
- `serial_defaults.default_port.is_some()` -> `defaultPortConfigured == true` but value not exposed.

Verification:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test dashboard_status --all-features
```

### Task 4: Update frontend API types and dashboard use

Objective: Let the UI consume the endpoint while remaining robust if the endpoint fails.

Files:

- Modify: `web/src/api.ts`
- Modify: `web/src/App.tsx`

API type must match the strict schema exactly:

```ts
export type DashboardStatusResponse = {
  server: { status: "ok" | string; version: string };
  runtime: {
    mode: "memory" | "mock" | "mock-script" | "real";
    realSerial: boolean;
    mockDevice: boolean;
    mockScriptConfigured: boolean;
  };
  serialDefaults: {
    defaultPortConfigured: boolean;
    baudRate: number;
    delimiter: string;
  };
  storage: {
    presets: "memory" | "sqlite";
    persistentPresets: boolean;
  };
};
```

Add:

```ts
status: () => requestJson<DashboardStatusResponse>("/api/v1/status")
```

Dashboard behavior:

- Add `const [status, setStatus] = useState<DashboardStatusResponse | null>(null);`.
- In `refresh()`, request `api.status()` in the existing `Promise.all`.
- Set status state on success.
- Display safe facts, for example:
  - Server card value may prefer `status ? `${status.server.status} · v${status.server.version}` : ...`.
  - Add or repurpose a card for `Mode` with `status.runtime.mode`.
  - Add or repurpose a card for `Presets` with `sqlite`/`memory`.
  - Show baud/delimiter in the connection card description or a small muted line.
- Initialize connect form with status defaults carefully:
  - Replace hard-coded `baudRate: "115200"` with the first successfully fetched `status.serialDefaults.baudRate` if the user has not manually changed it yet.
  - Replace hard-coded delimiter with an encoded display string derived from `status.serialDefaults.delimiter` if the user has not manually changed it yet.
  - Do not set port from status because the preferred schema does not expose `defaultPort` value. Keep existing `"/dev/ROBOT"` placeholder/default or make it an empty editable field if UX remains clear.
- Add a helper to display control characters in the delimiter input:

```ts
function encodeDelimiter(value: string) {
  return value.replace(/\r/g, "\\r").replace(/\n/g, "\\n").replace(/\t/g, "\\t");
}
```

- Avoid infinite refresh/update loops. Prefer applying fetched defaults inside `refresh()` only when current form values still match initial built-in defaults, or track a `defaultsApplied` boolean.
- The dashboard must still compile and render if status is `null`.

Frontend verification:

```bash
cd web
pnpm typecheck
pnpm build
```

### Task 5: Update README docs

Objective: Document the endpoint and its privacy model.

Files:

- Modify: `README.md`

Add a concise section near existing health/API/config docs:

- Route: `GET /api/v1/status`.
- Purpose: dashboard-safe resolved status/config.
- Include a compact JSON example.
- Explicitly state that the endpoint intentionally returns booleans/enums instead of filesystem paths, host details, environment values, or serial default port value.
- Mention `GET /api/v1/ports` remains the explicit route for serial port inventory.

Do not over-document internals or add promises about auth/CORS.

### Task 6: Full verification and final commit

Run from repository root:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cd web
pnpm typecheck
pnpm build
cd ..
git diff --check
git status --short
git diff --stat
```

If all pass, commit implementation changes with a concise message, recommended:

```bash
git add README.md src/api/routes.rs src/main.rs web/src/api.ts web/src/App.tsx
git commit -m "feat: add safe dashboard status endpoint"
```

If an implementation also modifies `src/config.rs` or other necessary files, include them in `git add` and explain why in final output.

---

## Acceptance Criteria

Phase 23 is complete only when all of these are true:

1. `GET /api/v1/status` exists and returns `200 OK` JSON.
2. Response body exactly follows the strict schema in this handoff.
3. Default route response is:
   - `server.status == "ok"`
   - `server.version == env!("CARGO_PKG_VERSION")`
   - `runtime.mode == "memory"`
   - `runtime.realSerial == false`
   - `runtime.mockDevice == false`
   - `runtime.mockScriptConfigured == false`
   - `serialDefaults.defaultPortConfigured == false`
   - `serialDefaults.baudRate == 115200`
   - `serialDefaults.delimiter == "\r\n"`
   - `storage.presets == "memory"`
   - `storage.persistentPresets == false`
4. Real serial, mock-device, mock-script, SQLite preset, configured default port, configured baud, and configured delimiter states are represented only by safe booleans/enums/scalars.
5. The endpoint does not expose any filesystem paths, host bind address, server port, environment variables, CLI/config source provenance, OS/user/hostname details, dashboard asset paths, mock script paths, preset DB paths, or serial default port value.
6. `GET /api/v1/health` remains backwards-compatible.
7. Existing routes and legacy aliases continue to pass tests.
8. `AppState` does not store `ResolvedServeConfig` or any `PathBuf` solely for the status endpoint.
9. `src/main.rs` passes the same resolved safe status into all runtime branches: real serial, mock device/script, and default in-memory.
10. React API types include the status response schema.
11. Dashboard fetches the status endpoint during refresh and displays runtime mode/storage/default facts without breaking existing health/ports/connections/presets behavior.
12. Dashboard connection form uses fetched safe baud/delimiter defaults when available, without requiring status to expose a serial port string.
13. README documents the route and privacy guardrails.
14. Full Rust and frontend verification commands pass.
15. Final implementation commit is created and reported.

---

## Security and Privacy Review Checklist

Before committing, inspect the diff and answer yes to all:

- Does the endpoint return only `server`, `runtime`, `serialDefaults`, and `storage` top-level keys?
- Are `mock_script`, `preset_db`, config path, and dashboard asset path reduced to booleans/enums before serialization?
- Is `SerialDefaults.default_port` reduced to `defaultPortConfigured` without returning the string?
- Is server host/port omitted?
- Are environment variable values omitted?
- Are CLI/config source names omitted?
- Are OS/user/hostname/process/system metrics omitted?
- Do tests assert exact JSON and absence of representative path strings?
- Does `GET /api/v1/ports` remain the only route that reports serial port names/inventory?
- Does the dashboard display avoid suggesting that hidden paths can be recovered from this endpoint?

If any answer is no, fix before running final verification.

---

## Suggested Manual Smoke Test

After automated checks pass, optionally run a local server and inspect responses:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo run -- serve --mock-device --preset-db ./presets.db
```

In another terminal:

```bash
curl -s http://127.0.0.1:4002/api/v1/status | python3 -m json.tool
curl -s http://127.0.0.1:4002/api/v1/health | python3 -m json.tool
```

Expected `status` response should show `runtime.mode` as `mock`, `storage.presets` as `sqlite`, and must not include `./presets.db`.

If running this smoke test, stop the server before finalizing.

---

## Commit Guidance

Recommended implementation commit message:

```text
feat: add safe dashboard status endpoint
```

Commit body optional. If used, mention:

- new `GET /api/v1/status` endpoint,
- safe response schema and path-omission guardrails,
- dashboard consumption of runtime/default metadata.

Do not squash this planning artifact into the implementation commit if it already exists in history. The planning artifact should be committed separately by the Phase 23 Planning Agent with:

```text
docs: add phase 23 dashboard status handoff
```
