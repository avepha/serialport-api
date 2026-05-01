# Docker and Release Guide

## Supported deployment model

Docker is useful for hardware-free mock/API testing, local integration checks without a Rust toolchain, and native Linux hosts that can pass serial devices into containers. The default container command starts `serialport-api` in mock/in-memory mode and does not open physical serial ports.

Raspberry Pi and systemd deployments remain documented separately in [`raspberry-pi-systemd.md`](raspberry-pi-systemd.md). Use that guide when you want a native service managed by systemd instead of a container.

## Build a local image

From the repository root:

```bash
docker build -t serialport-api:local .
```

The image builds the Rust binary with locked Cargo dependencies and installs it at `/usr/local/bin/serialport-api`. It also builds the React dashboard with Node.js 20/pnpm and copies the compiled `web/dist` bundle into the runtime image at `/app/web/`, so `/dashboard` is available from the default container. The runtime image includes `ca-certificates` and `libudev1`; `libudev1` is needed by Linux serial-device enumeration through the `serialport` crate.

## Run in default mock mode

```bash
docker run --rm -p 4002:4002 serialport-api:local
curl -s http://127.0.0.1:4002/api/v1/health
```

The default command is equivalent to:

```bash
serialport-api serve --host 0.0.0.0 --port 4002
```

Expected health response shape:

```json
{"status":"ok","version":"0.1.0"}
```

Port listing remains hardware-dependent, but the endpoint is safe in mock mode:

```bash
curl -s http://127.0.0.1:4002/api/v1/ports
```

## Use a config file and SQLite preset volume

Create or edit a config file that is suitable for the container. For persistent presets, point `[storage] preset_db` at a writable directory mounted into the container:

```toml
[server]
host = "0.0.0.0"
port = 4002

[serial]
real_serial = false

[storage]
preset_db = "/data/presets.db"
```

You can start from the repository example, but adjust its paths and `real_serial` setting before using it in a container because `examples/serialport-api.toml` is production/Pi-oriented and enables real serial mode.

Run with a read-only config mount and a writable data mount:

```bash
mkdir -p /tmp/serialport-api-data
cat > /tmp/serialport-api-container.toml <<'TOML'
[server]
host = "0.0.0.0"
port = 4002

[serial]
real_serial = false

[storage]
preset_db = "/data/presets.db"
TOML

docker run --rm \
  -p 4002:4002 \
  -v /tmp/serialport-api-container.toml:/config/serialport-api.toml:ro \
  -v /tmp/serialport-api-data:/data \
  serialport-api:local serve --config /config/serialport-api.toml
```

If your mounted config contains `preset_db = "/data/presets.db"`, saved presets will survive container restarts as long as the same host data directory or Docker volume is reused.

## Real serial devices in Docker

Real serial access is Linux-only and hardware-required. Keep mock mode as the default unless you intentionally want the container to open and communicate with host serial devices.

On a native Linux host, pass the device path into the container and enable real serial mode:

```bash
docker run --rm \
  -p 4002:4002 \
  --device=/dev/ttyUSB0:/dev/ttyUSB0 \
  serialport-api:local serve --host 0.0.0.0 --port 4002 --real-serial
```

Stable `/dev/serial/by-id/*` paths are preferred when available because `/dev/ttyUSB0` and `/dev/ttyACM0` can change after reconnects or reboots:

```bash
docker run --rm \
  -p 4002:4002 \
  --device=/dev/serial/by-id/usb-EXAMPLE:/dev/serial/by-id/usb-EXAMPLE \
  serialport-api:local serve --host 0.0.0.0 --port 4002 --real-serial
```

Permissions depend on the host device owner/group. If opening the device fails with permission denied, check `ls -l` on the device path and consider one of these host/container permission fixes:

- Add the host user or service account to the host `dialout` group and restart the session.
- Add the matching group inside the container with `--group-add <gid>`.
- Adjust host udev rules or device ownership for the serial adapter.

Docker Desktop on macOS/Windows and WSL may not expose physical serial devices like a native Linux host. Use native Linux for real serial container testing when possible.

## Docker Compose example

An optional compose file is available at [`../examples/docker-compose.yml`](../examples/docker-compose.yml). It starts the service in safe mock mode by default:

```bash
docker compose -f examples/docker-compose.yml up --build
```

The file includes commented examples for config and SQLite volume mounts and a commented Linux serial device mapping.

## Release workflow

The release workflow at [`../.github/workflows/release.yml`](../.github/workflows/release.yml) runs only when a version tag matching `v*` is pushed, for example:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The workflow:

- Runs `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` before packaging or publishing.
- Builds locked Linux release binaries with explicit targets using `cargo build --release --locked --target "$TARGET"`.
- Builds the React dashboard once with Node.js 20/pnpm and packages that compiled bundle into every Linux archive.
- Publishes deterministic tarballs named `serialport-api-${TAG}-${TARGET}.tar.gz` with matching `serialport-api-${TAG}-${TARGET}.tar.gz.sha256` checksum files.
- Packages each archive with one top-level `serialport-api/` directory containing the `serialport-api` executable, `README.md`, `LICENSE`, `ARTIFACT.txt` metadata, and `web/index.html` plus `web/assets/...` dashboard files.
- Verifies archive contents before upload by checking `serialport-api/web/index.html`, at least one compiled asset under `serialport-api/web/assets/`, and `web_built=true` in `ARTIFACT.txt`.
- Builds and publishes a GHCR image tagged with the pushed version tag using the repository `GITHUB_TOKEN`.

Automated Linux binary targets:

- `x86_64-unknown-linux-gnu` for Linux desktops and servers.
- `aarch64-unknown-linux-gnu` for Raspberry Pi OS 64-bit / ARM64 Linux.

Example release assets for `v0.1.0`:

```text
serialport-api-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
serialport-api-v0.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256
serialport-api-v0.1.0-aarch64-unknown-linux-gnu.tar.gz
serialport-api-v0.1.0-aarch64-unknown-linux-gnu.tar.gz.sha256
```

Verify a downloaded archive before extracting it:

```bash
sha256sum -c serialport-api-v0.1.0-aarch64-unknown-linux-gnu.tar.gz.sha256
```

The ARM64 artifact is cross-built in GitHub Actions without requiring Raspberry Pi hardware in CI. CI proves build/package success only; real serial-device behavior remains a manual hardware check on the target host.

ARMv7 / 32-bit Raspberry Pi release artifacts are not currently published because that target needs separate linker/sysroot validation. Build from source on 32-bit Pi OS, or use a 64-bit OS and the `aarch64-unknown-linux-gnu` artifact.

Implementation sessions should not push tags, create releases manually, or publish packages from a local machine.

## Manual smoke checks

When Docker is available, run these from the repository root:

```bash
docker build -t serialport-api:local .
docker run --rm serialport-api:local --version
```

Start the API in one terminal:

```bash
docker run --rm -p 4002:4002 serialport-api:local
```

In another terminal:

```bash
curl -i -s http://127.0.0.1:4002/dashboard | head -20
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

## Troubleshooting

- **Port already in use:** another process is listening on host port 4002. Stop it or change the host mapping, for example `-p 5000:4002`.
- **Container starts but health endpoint is unreachable:** confirm the command binds to `0.0.0.0`, not `127.0.0.1`, inside the container and confirm `-p 4002:4002` is present.
- **Config path mounted incorrectly:** check that the left side of `-v host-path:/config/serialport-api.toml:ro` exists on the host and that the container command uses `serve --config /config/serialport-api.toml`.
- **SQLite DB directory not writable:** mount a writable host directory or Docker volume at `/data` and ensure `[storage] preset_db = "/data/presets.db"`.
- **Serial device permission denied or missing device path:** verify the device exists on the host, pass it with `--device`, and match host group/device permissions with `--group-add`, `dialout`, or udev ownership changes.
- **Runtime image missing shared library:** rebuild after confirming the Dockerfile installs the required runtime package. The current runtime includes `libudev1` for serial enumeration and `ca-certificates` for TLS trust roots.

## Security and network exposure notes

The API is currently unauthenticated. Publishing `-p 4002:4002` can expose serial controls and preset routes to any network that can reach the Docker host, depending on Docker and host firewall settings. Bind or firewall the service appropriately and expose it only to trusted clients.

Docker examples in this guide do not add TLS termination, authentication, reverse-proxy rules, or firewall automation.
