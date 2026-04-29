# Phase 3 HTTP API Skeleton Handoff

> **For Hermes:** Execute this in a fresh session. Load `test-driven-development` before coding. If you turn this into a more detailed plan, also load `writing-plans`. Follow RED-GREEN-REFACTOR: write each test first, run it and confirm it fails, then implement the minimum code.

**Goal:** Add the first Axum-based HTTP API skeleton for `serialport-api`, with a working `GET /api/v1/health` endpoint and a `serve` CLI command.

**Architecture:** Keep the HTTP layer separate from the serial protocol library. `src/api/routes.rs` owns the Axum router and route handlers. `src/main.rs` only parses CLI args, configures tracing, binds the TCP listener, and serves the router.

**Tech Stack:** Rust 2021, Axum 0.7, Tokio 1, Clap 4, Serde, existing protocol library.

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

Relevant commits already completed:

```text
6b3a2f9 feat: add serial protocol parsing foundation
7085601 chore: modernize Rust project foundation
fc270c0 docs: define open source serialport api rewrite
```

Current important files:

- `Cargo.toml`
  - package name: `serialport-api`
  - edition: `2021`
  - rust-version: `1.75`
  - dependencies already present: `serialport`, `serde`, `serde_json`, `thiserror`, `tracing`, `tracing-subscriber`
- `src/lib.rs`
  - exports `error`
  - exports `protocol`
- `src/error.rs`
  - shared `SerialportApiError`
  - shared `Result<T>`
- `src/protocol.rs`
  - `frame_json`
  - `parse_line`
  - `SerialEvent`
  - protocol tests passing
- `src/main.rs`
  - currently only prints `serialport-api: rewrite in progress`

Baseline verification before starting:

```bash
cd /home/alfarie/repos/serialport-api
git status --short --branch
cargo fmt --check
cargo check
cargo test
```

Expected: clean branch and all commands pass.

---

## Do Not Do Yet

This phase is intentionally small. Do **not** implement serial hardware behavior yet.

Do not add:

- `/api/v1/ports`
- `/list`
- serial connection manager
- command sending
- SSE events
- Socket.IO compatibility
- hardware tests

Those belong to later phases.

---

## Acceptance Criteria

By the end of this phase:

- `cargo fmt --check` passes.
- `cargo check` passes.
- `cargo test` passes.
- `cargo run -- serve --host 127.0.0.1 --port 4002` starts an HTTP server.
- `curl http://127.0.0.1:4002/api/v1/health` returns exactly:

```json
{"status":"ok","version":"0.1.0"}
```

- Work is committed with a conventional commit message, suggested:

```text
feat: add axum health endpoint
```

---

## Task 1: Add Axum, Tokio, Clap, and Test Dependencies

**Objective:** Add the minimal dependencies needed for the HTTP server, CLI, and router unit tests.

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock` through Cargo

**Step 1: Edit `Cargo.toml`**

Add these under `[dependencies]`:

```toml
axum = "0.7"
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive", "env"] }
```

Add this under `[dev-dependencies]`:

```toml
tower = { version = "0.5", features = ["util"] }
```

Resulting dependency sections should look like this:

```toml
[dependencies]
axum = "0.7"
clap = { version = "4", features = ["derive", "env"] }
serialport = "4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
pretty_assertions = "1"
tower = { version = "0.5", features = ["util"] }
```

Alphabetical order is preferred but not mandatory if `cargo fmt` and tests pass.

**Step 2: Verify dependency resolution**

Run:

```bash
cargo check
```

Expected: success.

**Step 3: Commit or continue?**

Either commit this tiny dependency-only step or continue to Task 2 and commit both together. If committing separately:

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add axum server dependencies"
```

---

## Task 2: Add a Failing Router Test for `GET /api/v1/health`

**Objective:** Define the desired behavior before implementation.

**Files:**

- Create: `src/api/mod.rs`
- Create: `src/api/routes.rs`
- Modify: `src/lib.rs`

**Step 1: Export the API module in `src/lib.rs`**

Add `pub mod api;` above or below the existing modules:

```rust
pub mod api;
pub mod error;
pub mod protocol;
```

**Step 2: Create `src/api/mod.rs`**

```rust
pub mod routes;
```

**Step 3: Create `src/api/routes.rs` with only the test first**

Start with this intentionally incomplete file:

```rust
#[cfg(test)]
mod tests {
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn health_route_returns_status_and_version() {
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload, json!({"status":"ok","version":"0.1.0"}));
    }
}
```

**Step 4: Run the specific test and confirm RED**

Run:

```bash
cargo test api::routes::tests::health_route_returns_status_and_version -- --nocapture
```

Expected: FAIL because `router` is not defined. This is the required RED step.

If it fails because of a typo or missing dependency instead, fix that until the only meaningful failure is missing production implementation.

---

## Task 3: Implement the Minimal Health Router

**Objective:** Make the test pass with the smallest useful Axum router.

**Files:**

- Modify: `src/api/routes.rs`

**Step 1: Add implementation above the test module**

`src/api/routes.rs` should become:

```rust
use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

pub fn router() -> Router {
    Router::new().route("/api/v1/health", get(health))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[cfg(test)]
mod tests {
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn health_route_returns_status_and_version() {
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload, json!({"status":"ok","version":"0.1.0"}));
    }
}
```

**Step 2: Run the specific test and confirm GREEN**

Run:

```bash
cargo test api::routes::tests::health_route_returns_status_and_version -- --nocapture
```

Expected: PASS.

---

## Task 4: Add the `serve` CLI Command

**Objective:** Make the binary start the Axum server from the command line.

**Files:**

- Modify: `src/main.rs`

**Step 1: Replace `src/main.rs`**

Use this implementation:

```rust
use std::net::SocketAddr;

use clap::{Args, Parser, Subcommand};
use serialport_api::api::routes;

#[derive(Debug, Parser)]
#[command(name = "serialport-api", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve(ServeArgs),
}

#[derive(Debug, Args)]
struct ServeArgs {
    #[arg(long, default_value = "127.0.0.1", env = "SERIALPORT_API_HOST")]
    host: String,

    #[arg(long, default_value_t = 4002, env = "SERIALPORT_API_PORT")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Serve(args)) => serve(args).await?,
        None => println!("serialport-api: rewrite in progress"),
    }

    Ok(())
}

async fn serve(args: ServeArgs) -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!(%addr, "listening");

    axum::serve(listener, routes::router()).await?;

    Ok(())
}
```

**Notes:**

- `cargo run` with no args should still print `serialport-api: rewrite in progress`.
- `cargo run -- serve --host 127.0.0.1 --port 4002` should start the server.
- The env vars `SERIALPORT_API_HOST` and `SERIALPORT_API_PORT` are supported by Clap because the `env` feature was added.

**Step 2: Compile**

Run:

```bash
cargo check
```

Expected: PASS.

---

## Task 5: End-to-End Manual Verification

**Objective:** Prove the binary serves the health endpoint over HTTP.

**Step 1: Start the server**

Use a background process in Hermes if available. In a normal shell:

```bash
cargo run -- serve --host 127.0.0.1 --port 4002
```

Expected log includes something like:

```text
listening addr=127.0.0.1:4002
```

**Step 2: Curl the endpoint from another shell**

```bash
curl -s http://127.0.0.1:4002/api/v1/health
```

Expected exact response:

```json
{"status":"ok","version":"0.1.0"}
```

**Step 3: Stop the server**

Stop the background process or press `Ctrl-C` in the server shell.

---

## Task 6: Full Verification and Commit

**Objective:** Ensure the codebase is clean and record the work.

Run:

```bash
cargo fmt
cargo fmt --check
cargo check
cargo test
git status --short --branch
```

Expected:

- format check passes
- check passes
- all tests pass, including the existing 6 protocol tests and the new health route test
- only intended files are modified

Expected modified/created files:

```text
Cargo.lock
Cargo.toml
src/api/mod.rs
src/api/routes.rs
src/lib.rs
src/main.rs
```

Commit:

```bash
git add Cargo.toml Cargo.lock src/api/mod.rs src/api/routes.rs src/lib.rs src/main.rs
git commit -m "feat: add axum health endpoint"
```

Final verification:

```bash
git status --short --branch
git log --oneline -4
```

Expected latest commit:

```text
feat: add axum health endpoint
```

---

## Common Pitfalls

- If `tower::ServiceExt` is missing, confirm this dev dependency exists:

```toml
tower = { version = "0.5", features = ["util"] }
```

- If `to_bytes` complains about arguments, Axum 0.7 expects a body and a limit:

```rust
let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
```

- If `env = ...` on Clap args does not compile, confirm Clap has the `env` feature:

```toml
clap = { version = "4", features = ["derive", "env"] }
```

- If `localhost` fails to parse as a `SocketAddr`, use `127.0.0.1`. The planned CLI accepts IP-style host values for now.

- If port `4002` is already in use, choose another port for manual verification, for example:

```bash
cargo run -- serve --host 127.0.0.1 --port 4003
curl -s http://127.0.0.1:4003/api/v1/health
```

---

## Suggested Next Phase After This

After Phase 3 is complete and committed, proceed to Phase 4: serial manager foundation.

Start with a mockable/testable port-listing layer before opening hardware ports. The first target endpoint should be:

```text
GET /api/v1/ports
```

Compatibility alias later:

```text
GET /list
```

Do not begin Phase 4 until Phase 3 has a committed, passing health endpoint.
