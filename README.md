# 🌐 MikroTUI

[![Crates.io](https://img.shields.io/crates/v/mikrotui.svg)](https://crates.io/crates/mikrotui)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-2021-blue.svg)](https://www.rust-lang.org/)

**MikroTUI** is a modern, ultra-fast Terminal User Interface (TUI) for **MikroTik RouterOS**, inspired by the classic **WinBox** GUI. It connects directly via SSH and operates in a strict **Read-Only** mode with **Safe Mode** enabled by default.

---

## ✨ Features

- ⚙ **System Resources Monitor**: Real-time CPU Gauge, RAM, HDD storage, Architecture, Board model, Uptime, and RouterOS version (fully compatible with ROS v6 and ROS v7).
- 🔌 **Network Interfaces**: Full list of interfaces (Ethernet, VLAN, WireGuard, Bridges) with MTU, MAC, Link status (`Running`/`Down`), Rx/Tx packet counters, and comments.
- 🌐 **IP Addresses & Routing**: `/ip address` and `/ip route` tables with CIDR notation, network subnets, gateways, administrative distance, and flags (`Active`, `Dynamic`, `Static`, `Disabled`).
- 💻 **DHCP Server Leases**: Bound leases list displaying IP address, MAC address, device hostname, server name, status, and expiration timer.
- 🛡 **Firewall Filter Rules**: Color-coded firewall rules by action (`accept`, `drop`, `reject`) with packet/byte counters and protocol details.
- 📜 **System Logs Stream**: Live log viewer categorized by topics with color highlights (`system`, `ssh`, `error`, `warning`).
- 📡 **Interactive Ping Diagnostic (`p`)**: Built-in ICMP ping tool running directly from the router to any target IP/hostname, displaying packet loss % and RTT statistics (Min/Avg/Max).
- 🔍 **Item Details Modal (`Enter`)**: Centered popup displaying complete, unclipped properties and long comments for any selected item.
- 🎨 **Clean Theme Engine (`t`)**: Dynamically switch between high-contrast minimalist themes: **WinBox Dark** (default), **Nord Slate**, and **High Contrast**.
- 🔒 **Secure Host Credentials**: Stored in `~/.config/mikrotui/config.json` with machine-salt obfuscation and restricted `0600` Unix file permissions.
- ⚡ **Non-Blocking Async Core**: Built on Tokio and Ratatui. All SSH data fetching runs in background threads with strict debounce guards to prevent UI lag or freeze.
- 📊 **CLI Automation & JSON Dump**: Non-visual output mode for scripts (`mikrotui dump ip-addresses --format json`).

---

## 🚀 Installation

### Via Crates.io (Cargo)

If you have Rust installed:

```bash
cargo install mikrotui
```

### From Source

```bash
git clone git@github.com:dmaax/mikrotui.git
cd mikrotui
cargo build --release
./target/release/mikrotui
```

---

## 🎮 Usage

### 1. Launch Interactive TUI

```bash
# Launch interactive TUI (prompts for host selection or wizard on first run)
mikrotui

# Or specify target host directly via CLI flags:
mikrotui --host 192.168.88.1 --user admin --password secret

# Or run in Demo Mode (no router required):
mikrotui --demo
```

### 2. Manage Router Hosts

```bash
# Add a new router interactively to encrypted config file
mikrotui host add

# List all configured hosts
mikrotui host list
```

### 3. Non-Visual CLI Dump (Automation & Debug)

```bash
# Dump system resources as JSON
mikrotui dump system --format json

# Dump IP addresses
mikrotui dump ip-addresses --format json

# Execute raw RouterOS CLI command via SSH
mikrotui exec "/ip address print"
```

---

## ⌨ Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| **Tab / Shift+Tab** | Switch active menu tab (or use `←` / `→` Arrow keys / `h` / `l`) |
| **↑ / ↓** (or `k` / `j`) | Navigate up / down through table rows |
| **Enter** | Open Item Details modal (view complete properties & comments) |
| **p** | Open interactive Ping Diagnostic tool (`/ping <target>`) |
| **/** | Activate live filter search (type query, `Enter`/`Esc` to finish) |
| **t** | Cycle color themes (*WinBox Dark*, *Nord Slate*, *High Contrast*) |
| **Ctrl+X** | Toggle Safe Mode indicator (*Enabled by default*) |
| **r / F5** | Refresh data via SSH in background (*Non-blocking*) |
| **?** | Open / close Keyboard Shortcuts & Help modal |
| **q / Ctrl+C** | Quit MikroTUI |

---

## 🔐 Security & Safe Mode

1. **Read-Only Enforcement**: Write operations (`add`, `set`, `remove`, `enable`, `disable`) are strictly blocked at the client layer.
2. **Default Safe Mode**: Safe Mode is enabled by default (`[SAFE MODE: ENABLED]`).
3. **Encrypted Credentials**: Stored passwords in `~/.config/mikrotui/config.json` use machine-salt XOR obfuscation (`enc:v1:...`) combined with Unix `0o600` file permissions to prevent unauthorized plain-text reading.

---

## 📄 License

Distributed under the [MIT License](LICENSE).
