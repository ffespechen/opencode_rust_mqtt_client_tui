# Rust MQTT Client TUI

An interactive terminal user interface (TUI) MQTT 3.1.1 client built with Rust, Ratatui and Crossterm.
Supports TCP and WebSocket connections, topic subscriptions with color-coded messages, and message
publishing in plain text and JSON format with configurable QoS levels.

## Features

- **MQTT 3.1.1 Protocol** — Full compliance with the MQTT 3.1.1 specification via [rumqttc](https://github.com/bytebeamio/rumqtt).
- **TCP & WebSocket Transport** — Toggle between TCP (default port 1883) and WebSocket (default port 8083) connections using `^W`.
- **Authentication** — Username and password fields with optional credentials.
- **Topic Subscriptions** — Subscribe to multiple topics with independent QoS levels (0, 1, 2). Each topic is assigned a unique color for easy visual distinction.
- **Incoming Message Display** — All received messages are displayed in real-time under their respective topic with timestamps, retain flags, and payload.
- **Message Publishing** — Publish messages in plain text or JSON format with configurable QoS (0, 1, 2) and retain flag.
- **JSON Validation** — Payloads marked as JSON are validated before publishing, preventing malformed data from being sent.
- **Contextual Help** — The bottom bar provides context-sensitive help based on the currently focused field.
- **Keyboard-Driven Navigation** — Full keyboard control: Tab/Shift+Tab to navigate fields, Ctrl-combinations for actions.
- **Docker Support** — Multi-stage Dockerfile for building a minimal container image ready for DockerHub.

## Key Bindings

### Global Shortcuts

| Key           | Action                    |
|---------------|---------------------------|
| `Ctrl + C`    | Connect to broker         |
| `Ctrl + D`    | Disconnect from broker    |
| `Ctrl + S`    | Subscribe to topic        |
| `Ctrl + U`    | Unsubscribe from topic    |
| `Ctrl + P`    | Publish message           |
| `Ctrl + W`    | Toggle TCP / WebSocket    |
| `Ctrl + R`    | Toggle Retain flag        |
| `Ctrl + J`    | Toggle JSON / Text format |
| `Ctrl + Q`    | Quit application          |
| `Tab`         | Move to next field        |
| `Shift + Tab` | Move to previous field    |

### Editing Fields

| Key           | Action                    |
|---------------|---------------------------|
| `Arrows L/R`  | Move cursor               |
| `Arrows U/D`  | Navigate topics / lines   |
| `Backspace`   | Delete before cursor      |
| `Delete`      | Delete after cursor       |
| `Home`        | Move cursor to start      |
| `End`         | Move cursor to end        |
| `Enter`       | New line (payload editor) |
| `PgUp / PgDn` | Scroll messages           |

### Field Navigation Order

`Host` → `Port` → `Client ID` → `Username` → `Password` → `Subscribe Topic` → `Subscribe QoS` → `Publish Topic` → `Publish QoS` → `Publish Payload`

When focused on Subscribe QoS or Publish QoS, pressing Tab cycles the QoS value (0→1→2) instead of moving to the next field.

## Prerequisites

- **Rust** 1.88.0+ (stable toolchain)
- **OpenSSL** development libraries (for TLS support on Linux)

### Installing OpenSSL on Linux

```bash
# Debian / Ubuntu
sudo apt-get install pkg-config libssl-dev

# Fedora
sudo dnf install openssl-devel

# Arch Linux
sudo pacman -S openssl
```

## Quick Start

### Build from Source

```bash
git clone https://github.com/ffespechen/rust_mqtt_client_tui.git
cd rust_mqtt_client_tui
cargo build --release
./target/release/rust_mqtt_client_tui
```

### Run with Cargo

```bash
cargo run --release
```

## Usage

1. Launch the application. The initial focus is on the **Host** field.
2. Enter the MQTT broker address (e.g., `localhost` or `broker.emqx.io`).
3. Use Tab to navigate to **Port** and adjust if needed (defaults shown based on transport: 1883 for TCP, 8083 for WebSocket).
4. Optionally enter **Client ID**, **Username**, and **Password**.
5. Press `^W` to toggle between TCP and WebSocket transport if needed.
6. Press `^C` to connect. The status bar will show connection progress.
7. Once connected, navigate to the **Subscribe Topic** field, enter a topic (e.g., `sensors/#`), and press `^S` to subscribe.
8. Incoming messages appear in the **Messages** panel on the left, color-coded by topic.
9. To publish, navigate to the **Publish Topic** and **Payload** fields on the right, enter your message, and press `^P`.
10. Toggle JSON format with `^J` — the payload will be validated as JSON before publishing.

## Architecture

For a comprehensive analysis of the architectural decisions, concurrency model, data flow
patterns, and design trade-offs, see **[ARCHITECTURE.md](ARCHITECTURE.md)**.

```
src/
├── main.rs              # Entry point: terminal init, event loop, orchestration
├── app/
│   ├── mod.rs
│   └── state.rs         # App struct, business logic, state management
├── events/
│   ├── mod.rs
│   └── handler.rs       # Key event → Action translation, key bindings
├── ui/
│   ├── mod.rs           # Layout orchestration (splits: top/content/bottom)
│   ├── top_bar.rs       # Action bar with Ctrl+letter shortcuts
│   ├── subscriptions.rs # Left panel: subscribe form, topic list, messages
│   ├── publish.rs       # Right panel: publish form with payload editor
│   └── help.rs          # Bottom bar: contextual help
└── mqtt/
    ├── mod.rs
    └── client.rs        # Async MQTT task: event loop, command handling
```

- **`src/app`** — Pure business logic. The `App` struct owns all application state. No rendering code.
- **`src/ui`** — Read-only rendering. Receives `&App` and draws using Ratatui widgets. Never mutates state.
- **`src/events`** — Translates Crossterm `KeyEvent` into domain `Action` enums. Context-aware (Tab behavior changes based on focus).
- **`src/mqtt`** — MQTT integration. Spawned as a Tokio task; communicates with the main thread via unbounded MPSC channels (`MqttCmd` → task, `MqttEvent` → UI).

## Docker

### Build the Image

```bash
docker build -t rust-mqtt-client-tui:latest .
```

### Run the Container

```bash
docker run -it --rm \
  -p 1883:1883 \
  -p 9001:9001 \
  rust-mqtt-client-tui
```

The `-it` flags are **mandatory** — the TUI requires an interactive terminal with a TTY.
The container internally uses `script` to guarantee PTY allocation, which bridges the
Docker pseudo-terminal to the Crossterm raw mode backend.

**Port mappings:**
| Host Port | Container Port | Protocol    | Purpose                       |
|-----------|---------------|-------------|-------------------------------|
| 1883      | 1883          | TCP         | MQTT broker (unencrypted)     |
| 9001      | 9001          | TCP         | MQTT WebSocket (unencrypted)  |

If your broker runs on different ports, adjust the `-p` flags accordingly. Common alternatives:
- EMQX: 1883 (TCP), 8083 (WS)
- HiveMQ: 1883 (TCP), 8000 (WS)
- Mosquitto: 1883 (TCP), 9001 (WS)

### Publish to DockerHub

```bash
docker login
docker tag rust-mqtt-client-tui:latest ffespechen/rust-mqtt-client-tui:0.1.0
docker push ffespechen/rust-mqtt-client-tui:latest
docker push ffespechen/rust-mqtt-client-tui:0.1.0
```

## License

MIT

## Dependencies

| Crate      | Purpose                           |
|------------|-----------------------------------|
| `ratatui`  | Terminal UI framework             |
| `crossterm`| Terminal manipulation & events    |
| `rumqttc`  | MQTT 3.1.1 client (async)        |
| `tokio`    | Async runtime                     |
| `serde_json`| JSON parsing and validation      |
| `chrono`   | Message timestamps                |
| `uuid`     | Client ID and message ID generation |
| `anyhow`   | Error handling                    |
