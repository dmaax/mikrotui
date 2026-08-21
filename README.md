# 🌐 MikroTUI

[![Crates.io](https://img.shields.io/crates/v/mikrotui.svg)](https://crates.io/crates/mikrotui)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-2021-blue.svg)](https://www.rust-lang.org/)

**MikroTUI** is a modern, ultra-fast Terminal User Interface (TUI) for **MikroTik RouterOS**, inspired by the classic **WinBox** GUI. It connects over SSH, verifies the router's host key against `known_hosts`, and sends only read commands — any command that would write is refused before it leaves your machine.

---

## ✨ Features

- ⚙ **System Resources Monitor**: CPU gauge, RAM, HDD storage, architecture, board model, uptime and RouterOS version (compatible with ROS v6 and v7). Refreshes on `r`, or on a timer with `--refresh`.
- 🔌 **Network Interfaces**: Full list of interfaces (Ethernet, VLAN, WireGuard, Bridges) with MTU, MAC, Link status (`Running`/`Down`), Rx/Tx packet counters, and comments.
- 🌐 **IP Addresses & Routing**: `/ip address` and `/ip route` tables with CIDR notation, network subnets, gateways, administrative distance, and flags (`Active`, `Dynamic`, `Static`, `Disabled`).
- 💻 **DHCP Server Leases**: Bound leases list displaying IP address, MAC address, device hostname, server name, status, and expiration timer.
- 🛡 **Firewall Filter Rules**: Color-coded firewall rules by action (`accept`, `drop`, `reject`) with packet/byte counters and protocol details.
- 📜 **System Logs Stream**: Live log viewer categorized by topics with color highlights (`system`, `ssh`, `error`, `warning`).
- 📡 **Interactive Ping Diagnostic (`p`)**: Built-in ICMP ping tool running directly from the router to any target IP/hostname, displaying packet loss % and RTT statistics (Min/Avg/Max).
- 🔍 **Item Details Modal (`Enter`)**: Centered popup displaying complete, unclipped properties and long comments for any selected item.
- 🎨 **Clean Theme Engine (`t`)**: Dynamically switch between high-contrast minimalist themes: **WinBox Dark** (default), **Nord Slate**, and **High Contrast**.
- 🔑 **Host Key Verification**: The router's SSH key is checked against `~/.ssh/known_hosts` before any credential is sent; a changed key aborts the connection.
- 🔒 **Credentials**: No password is stored by default — read from stdin, the environment, the OS keyring (`--features keyring`), or an interactive prompt. See [Security](#-security).
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

# Or specify target host directly via CLI flags (password read from a pipe,
# so it never appears in `ps` output or your shell history):
pass show mikrotik | mikrotui --host 192.168.88.1 --user admin --password-stdin

# Or run in Demo Mode (no router required):
mikrotui --demo

# Refresh the visible tab every 5 seconds instead of only on 'r':
mikrotui --refresh 5
```

> MikroTUI fetches only the tab you are looking at, plus the system resource the header
> shows. Switching to a tab for the first time fetches it then. That keeps a refresh to a
> couple of SSH commands rather than the two dozen it would take to update all eight
> tables, which is what makes `--refresh` reasonable to leave on.

### 2. Manage Router Hosts

```bash
# Add a new router interactively (asks where the password should live)
mikrotui host add

# List configured hosts and where each password is stored
mikrotui host list

# Move passwords still obfuscated in config.json into the OS keyring
mikrotui host migrate
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
| **PgUp / PgDn** | Scroll a whole screenful (also `Ctrl+B` / `Ctrl+F`) |
| **Home / End** (or `g` / `G`) | Jump to the first / last row |
| **Enter** | Open Item Details modal (view complete properties & comments) |
| **p** | Open interactive Ping Diagnostic tool (`/ping <target>`) |
| **/** | Activate live filter search (type query, `Enter`/`Esc` to finish) |
| **t** | Cycle color themes (*WinBox Dark*, *Nord Slate*, *High Contrast*) |
| **Ctrl+O** | Switch to another stored router host |
| **r / F5** | Refresh data via SSH in background (*Non-blocking*) |
| **?** | Open / close Keyboard Shortcuts & Help modal |
| **q / Ctrl+C** | Quit MikroTUI |

---

## 🔐 Security

### Host key verification

MikroTUI verifies the router's SSH host key against `~/.ssh/known_hosts` **before** sending
any credential, the same way `ssh` does.

- **First connection** shows the key fingerprint and asks you to confirm it. Check it on the
  router with `/ip ssh print` before accepting.
- **A changed key is always fatal.** MikroTUI never offers to accept it — that is the signal
  of an interception. If the router was genuinely reinstalled, delete the offending line from
  `known_hosts` and reconnect.
- `--accept-new-hostkey` records an unknown key without asking (for scripts). It refuses
  anything that is not a genuine first contact: a changed key, or a host already on file
  offering a key algorithm the file has not recorded.
- `--known-hosts <PATH>` uses a different file.

Inside the TUI there is no way to prompt, so switching host (`Ctrl+O`) to a router whose key is
unknown fails with an explanatory message. Connect to it once from the command line first.

### Read-only enforcement

Every command is checked against an **allowlist** before it is sent: a command runs only if it
names a read-only action (`print`, `get`, `find`, `export`, `monitor`, `ping`, `traceroute`,
`resolve`) and names no mutating one. This covers the forms a denylist misses — the RouterOS v7
slash syntax (`/ip/address/set`), commands chained after a read (`... print; /system reboot`),
`/system reboot` and `/system reset-configuration`, and script wrappers (`:execute`).

Command substitution (`[ ... ]`), script commands (`:execute`), `file=` arguments and
RouterOS prefix abbreviations (`rem` for `remove`) are all refused, since each was a way
around the check.

> **This is a guard rail, not a permission boundary.** It runs on your machine, so it protects
> you from mistakes, not the router from a determined user. Do not rely on it to contain
> someone who is trying to get past it. The real guarantee is a RouterOS account in the
> **`read` group** — give MikroTUI one of those rather than a full admin.

### Authenticating with a key

```bash
mikrotui --host 192.168.88.1 --user admin -i ~/.ssh/id_ed25519
```

`mikrotui host add` can store a key per router, so `Ctrl+O` switching works with it too.
Encrypted keys are supported; the passphrase is asked for before the TUI starts, since
there is nowhere to ask once it has the terminal.

> **RSA keys are refused.** The `rsa` crate carries an unfixed timing sidechannel
> (`RUSTSEC-2023-0071`, no patched release upstream) that applies to private key
> operations. MikroTUI's CI ignores that advisory precisely because it holds no RSA
> private key, and accepting one here would quietly invalidate that reasoning. Use
> `ssh-keygen -t ed25519`. ECDSA works too.

`ssh-agent` is not supported. russh's agent client is the code covered by
`RUSTSEC-2026-0154`, and MikroTUI does not link it.

### Credentials

**MikroTUI does not store passwords by default.** In resolution order:

| Source | How |
| :--- | :--- |
| stdin | `pass show router \| mikrotui --password-stdin` |
| environment | `MIKROTUI_PASSWORD=… mikrotui` |
| OS keyring | requires `--features keyring` (see below) |
| config file | legacy, obfuscated only — warns on every use |
| prompt | asked interactively when nothing else supplies one |

`--password` still works but warns: it is visible in `ps` output and your shell history.

To enable OS keyring storage:

```bash
cargo install mikrotui --features keyring
mikrotui host migrate    # moves existing config.json passwords into the keyring
```

It is off by default because the Linux backend needs a running Secret Service (gnome-keyring,
KeePassXC, …), which headless machines usually lack. The build itself is pure Rust — no
`libdbus-1-dev` or OpenSSL required either way.

> **On the old `enc:v1:` passwords.** Versions up to 0.2.2 XOR-ed the password against a key
> derived from `$USER` and a salt compiled into the published binary, and called it encryption.
> It is not: anyone who can read `config.json` can recover the password. Those entries still
> load, now with a warning. Run `mikrotui host migrate` to move them into the keyring. Local
> storage without a master password can never be more than obfuscation — any key the program
> can recompute unattended, an attacker holding the file can recompute too.

The config file and `known_hosts` are created with mode `0600` **at creation time**, not
chmod-ed afterwards, so credentials are never briefly world-readable.

---

## 📄 License

Distributed under the [MIT License](LICENSE).
