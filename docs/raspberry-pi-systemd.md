# Raspberry Pi / systemd Deployment Guide

## Supported deployment model

Phase 14 documents how to run the current `serialport-api` server on Raspberry Pi OS or another Debian-like Linux distribution that uses systemd. It covers building from source on the device, or copying an already built binary, then running it as a boot-time service.

Release artifacts, packaged `.deb` files, Docker images, and cross-compilation automation are intentionally left for a later phase.

The service remains hardware-free unless `real_serial = true` or `serve --real-serial` is enabled. Use real serial mode only when the configured serial device is attached and the service user has permission to access it.

## Prerequisites

Install OS packages used for building, SQLite-backed presets, serial-device discovery, and smoke checks:

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  pkg-config \
  libudev-dev \
  sqlite3 \
  ca-certificates \
  curl \
  git
```

Install Rust with `rustup` if the target machine will build from source:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"
rustc --version
cargo --version
```

`Cargo.toml` currently requires Rust 1.75 or newer.

## Build and install the binary

### Build from source on the Pi

Use a stable source directory such as `/opt/serialport-api`:

```bash
sudo mkdir -p /opt/serialport-api
sudo chown "$USER:$USER" /opt/serialport-api
git clone https://github.com/avepha/serialport-api.git /opt/serialport-api
cd /opt/serialport-api
cargo build --release
```

Install the compiled binary into a path used by the systemd unit:

```bash
sudo install -m 0755 target/release/serialport-api /usr/local/bin/serialport-api
/usr/local/bin/serialport-api --version
```

### Or copy an existing binary

If you already built a compatible binary for the Pi's CPU/OS, copy it to the same installed path:

```bash
sudo install -m 0755 ./serialport-api /usr/local/bin/serialport-api
/usr/local/bin/serialport-api --version
```

## Create a service user and data/config directories

Run the service as an unprivileged user rather than `root`:

```bash
sudo useradd --system --home /var/lib/serialport-api --shell /usr/sbin/nologin serialport-api
sudo mkdir -p /etc/serialport-api /var/lib/serialport-api
sudo chown root:root /etc/serialport-api
sudo chmod 0755 /etc/serialport-api
sudo chown serialport-api:serialport-api /var/lib/serialport-api
sudo chmod 0750 /var/lib/serialport-api
```

The example configuration lives at `/etc/serialport-api/serialport-api.toml`. The SQLite preset database lives under `/var/lib/serialport-api`, which must be writable by the `serialport-api` user.

If you edit the config as root, keep it readable by the service user:

```bash
sudo chmod 0644 /etc/serialport-api/serialport-api.toml
```

## Serial device permissions

Most USB serial adapters on Raspberry Pi OS are owned by `root:dialout`. Add the service user to `dialout` so it can open devices in real serial mode:

```bash
sudo usermod -aG dialout serialport-api
```

For interactive testing as your login user, also add your user to `dialout`:

```bash
sudo usermod -aG dialout "$USER"
```

Log out and back in, or reboot, after changing group membership. Existing sessions do not automatically pick up new groups.

Prefer stable serial names when available:

```bash
ls -l /dev/serial/by-id/
```

Use a `/dev/serial/by-id/*` path in configuration when possible because `/dev/ttyUSB0` and `/dev/ttyACM0` can change after reboot or when devices are unplugged. Common fallback paths include:

- `/dev/ttyUSB0` for many USB-to-serial adapters.
- `/dev/ttyACM0` for many Arduino/CDC ACM devices.
- `/dev/serial0` for the Pi UART or HAT UART when enabled by the OS.

## Example configuration

Copy the versioned example and edit the serial device path for your hardware:

```bash
sudo cp /opt/serialport-api/examples/serialport-api.toml /etc/serialport-api/serialport-api.toml
sudo nano /etc/serialport-api/serialport-api.toml
```

Production-ish example:

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

Use `host = "127.0.0.1"` instead of `0.0.0.0` when only local processes should reach the API. The same SQLite path can also be supplied with `serve --preset-db /var/lib/serialport-api/presets.db`, but the systemd example keeps it in `[storage] preset_db`.

Do not enable `real_serial = true` together with `mock_device = true` or `mock_script = "..."`; the server rejects real serial plus mock-device/mock-script combinations.

## systemd service

The reusable unit file is versioned at [`../examples/systemd/serialport-api.service`](../examples/systemd/serialport-api.service). It runs:

```text
/usr/local/bin/serialport-api serve --config /etc/serialport-api/serialport-api.toml
```

Unit content:

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

`ProtectSystem=full` keeps most system paths read-only for the service. `ReadWritePaths=/var/lib/serialport-api` permits SQLite writes in the data directory while keeping the config and binary paths read-only.

## Install and manage the service

Install the config and service files:

```bash
sudo cp /opt/serialport-api/examples/serialport-api.toml /etc/serialport-api/serialport-api.toml
sudo cp /opt/serialport-api/examples/systemd/serialport-api.service /etc/systemd/system/serialport-api.service
sudo chown root:root /etc/systemd/system/serialport-api.service /etc/serialport-api/serialport-api.toml
sudo chmod 0644 /etc/systemd/system/serialport-api.service /etc/serialport-api/serialport-api.toml
```

Reload systemd, enable boot startup, and start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable serialport-api
sudo systemctl start serialport-api
sudo systemctl status serialport-api --no-pager
```

View logs:

```bash
sudo journalctl -u serialport-api -f
```

Restart after changing config or installing a new binary:

```bash
sudo systemctl restart serialport-api
```

## Manual smoke checks

These checks can be run on the Pi after the service starts. They do not require serial hardware unless explicitly marked.

Health check:

```bash
curl -s http://127.0.0.1:4002/api/v1/health
```

Expected shape:

```json
{"status":"ok","version":"0.1.0"}
```

Ports check:

```bash
curl -s http://127.0.0.1:4002/api/v1/ports
```

Expected: a JSON object with a `ports` array. The array may be empty when no serial devices are attached or visible.

SQLite preset persistence check:

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/presets \
  -H 'content-type: application/json' \
  -d '{"name":"Read IMU","payload":{"method":"query","topic":"imu.read","data":{}}}'

sudo systemctl restart serialport-api

curl -s http://127.0.0.1:4002/api/v1/presets
```

Expected: the created preset remains after restart when `[storage] preset_db` points at a writable persistent database path.

Config-backed startup check:

```bash
sudo systemctl show serialport-api -p ExecStart --no-pager
sudo journalctl -u serialport-api -n 50 --no-pager
```

Expected: the command includes `serve --config /etc/serialport-api/serialport-api.toml`, and logs do not show config parse errors.

Hardware-required real serial connection check:

```bash
curl -s -X POST http://127.0.0.1:4002/api/v1/connections \
  -H 'content-type: application/json' \
  -d '{"name":"default","port":"/dev/serial/by-id/usb-EXAMPLE","baudRate":115200,"delimiter":"\r\n"}'
```

This succeeds only when hardware is attached, the configured device path exists, `real_serial = true`, and permissions are correct. If it fails, check troubleshooting before assuming the API is broken.

## Troubleshooting

- **Permission denied on serial port:** verify the service user is in `dialout` with `id serialport-api`, reboot or restart sessions after group changes, and check `ls -l /dev/ttyUSB0 /dev/ttyACM0 /dev/serial/by-id/*`.
- **Wrong device path after reboot:** prefer `/dev/serial/by-id/*`. Re-run `ls -l /dev/serial/by-id/` after plugging in the device. Update `default_port` and any manual `POST /api/v1/connections` payloads.
- **Service fails because a path is wrong:** check `systemctl status serialport-api --no-pager` and `journalctl -u serialport-api -n 100 --no-pager`. Confirm `/usr/local/bin/serialport-api`, `/etc/serialport-api/serialport-api.toml`, and `/var/lib/serialport-api` exist.
- **Port already in use:** another process is listening on port 4002. Check with `sudo ss -ltnp 'sport = :4002'`, stop the other process, or change `[server] port`.
- **Cannot reach service from another machine:** `host = "127.0.0.1"` accepts local connections only. Use `host = "0.0.0.0"` only when LAN clients are intentionally allowed and trusted.
- **SQLite database errors:** ensure `/var/lib/serialport-api` is owned and writable by `serialport-api:serialport-api`. The service hardening only grants writes under `/var/lib/serialport-api`.
- **Real serial conflicts with mock settings:** remove `mock_device = true` and `mock_script = "..."` when `real_serial = true`; the server rejects the combination.
- **Config parse errors:** verify that config keys match the supported `[server]`, `[serial]`, and `[storage]` sections and that `preset_db` is under `[storage]`.

## Security and network exposure notes

The API is currently unauthenticated. Binding to `0.0.0.0` exposes serial controls and saved preset routes to the LAN on port 4002. Use `host = "127.0.0.1"` unless LAN clients are intentionally trusted.

Firewall rules, TLS termination, reverse proxies, and authentication middleware are not configured by this phase. Add those controls separately before exposing the API to untrusted networks.

## Uninstall / rollback

Stop and disable the service:

```bash
sudo systemctl stop serialport-api
sudo systemctl disable serialport-api
```

Remove the unit and reload systemd:

```bash
sudo rm -f /etc/systemd/system/serialport-api.service
sudo systemctl daemon-reload
sudo systemctl reset-failed serialport-api
```

Optionally remove installed files and data:

```bash
sudo rm -f /usr/local/bin/serialport-api
sudo rm -rf /etc/serialport-api
sudo rm -rf /var/lib/serialport-api
sudo userdel serialport-api
```

Keep `/var/lib/serialport-api/presets.db` if you may need saved presets later.
