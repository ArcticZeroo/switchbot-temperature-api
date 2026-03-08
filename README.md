# SwitchBot Climate API

A lightweight Rust server that polls temperature and humidity data from a SwitchBot Hub and stores it in SQLite. Designed to be consumed by an Android app for headache prediction.

## How It Works

- Polls your SwitchBot device **every hour** via the SwitchBot Cloud API
- Stores temperature + humidity readings in a local SQLite database
- Exposes a simple REST API to query current and historical readings
- Automatically cleans up readings older than 180 days

## Setup

### 1. Get SwitchBot API Credentials

In the SwitchBot mobile app: **Profile → Preferences → tap "App Version" 10 times → Developer Options**. Copy your **token** and **secret**.

### 2. Configure Environment

```sh
cp .env.example .env
```

Add your token and secret to `.env`:

```
SWITCHBOT_TOKEN=your-token
SWITCHBOT_SECRET=your-secret
```

### 3. Discover Your Device ID

```sh
cargo run -- discover
```

This prints a table of all SwitchBot devices that have temperature/humidity sensors (Hub 2, Hub 3, Meter, etc.). Copy the device ID you want to track and add it to `.env`:

```
SWITCHBOT_DEVICE_ID=ABCDEF123456
```

### 4. Start the Server

```sh
cargo run --release -- serve
```

The server starts on `0.0.0.0:3000` by default and immediately begins polling.

## API Endpoints

### `GET /health`

Returns server status.

```json
{ "status": "ok" }
```

### `GET /api/climate/current`

Fetches a live reading from the SwitchBot API.

```json
{
  "device_id": "ABCDEF123456",
  "temperature": 22.4,
  "humidity": 58,
  "timestamp": "2026-03-08T03:00:00Z"
}
```

### `GET /api/climate/history?start={ISO8601}&end={ISO8601}`

Returns stored readings within the given time range. Maximum span of **7 days** per request.

```
GET /api/climate/history?start=2026-03-01T00:00:00Z&end=2026-03-07T00:00:00Z
```

```json
{
  "device_id": "ABCDEF123456",
  "start": "2026-03-01T00:00:00Z",
  "end": "2026-03-07T00:00:00Z",
  "readings": [
    { "temperature": 22.4, "humidity": 58, "recorded_at": "2026-03-01T00:00:00Z" },
    { "temperature": 21.8, "humidity": 62, "recorded_at": "2026-03-01T01:00:00Z" }
  ],
  "count": 2
}
```

## Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `SWITCHBOT_TOKEN` | Yes | — | SwitchBot API token |
| `SWITCHBOT_SECRET` | Yes | — | SwitchBot API secret |
| `SWITCHBOT_DEVICE_ID` | For `serve` | — | Device to poll (use `discover` to find it) |
| `DATABASE_PATH` | No | `climate.db` | Path to SQLite database file |
| `LISTEN_ADDR` | No | `0.0.0.0:3000` | Address and port to listen on |

## Deploying to a VPS

Build a release binary and copy it to your server:

```sh
cargo build --release
# binary is at target/release/switchbot-api
```

On the server, create a `.env` file with your config and run:

```sh
./switchbot-api serve
```

To run as a systemd service:

```ini
# /etc/systemd/system/switchbot-api.service
[Unit]
Description=SwitchBot Climate API
After=network.target

[Service]
ExecStart=/path/to/switchbot-api serve
WorkingDirectory=/path/to/working-dir
EnvironmentFile=/path/to/working-dir/.env
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

```sh
sudo systemctl enable --now switchbot-api
```

## Running Tests

```sh
cargo test
```
