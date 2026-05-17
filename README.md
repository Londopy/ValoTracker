<div align="center">

# ValoTracker

**Real-time VALORANT match tracker — ranks, stats, and encounter history for all 10 players, live.**

[![CI](https://github.com/Londopy/ValoTracker/actions/workflows/release.yml/badge.svg)](https://github.com/Londopy/ValoTracker/actions/workflows/release.yml)
[![Latest Release](https://img.shields.io/github/v/release/Londopy/ValoTracker?color=ff4655&label=release)](https://github.com/Londopy/ValoTracker/releases/latest)
[![PyPI](https://img.shields.io/pypi/v/valotracker)](https://pypi.org/project/valotracker/3.0.1/) 
[![Downloads](https://img.shields.io/github/downloads/Londopy/ValoTracker/total?color=4a9eff)](https://github.com/Londopy/ValoTracker/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-0078d4?logo=windows&logoColor=white)](https://github.com/Londopy/ValoTracker/releases/latest)
[![Rust](https://img.shields.io/badge/rust-1.78%2B-orange?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![VirusTotal](https://img.shields.io/badge/VirusTotal-Clean-brightgreen?logo=virustotal&logoColor=white)](https://www.virustotal.com/gui/url/9075d2f1dff71f71a3552631983715f1caf47c2bf951463a4083d2d5355075df/detection)

Written in Rust · No account login · No API keys · Reads only from `127.0.0.1`

</div>

---

## Features

| | |
|---|---|
| 🎮 **Live match table** | All 10 players — rank, RR, peak rank, HS%, K/D, WR%, party grouping — refreshed every 30 s |
| 👁 **Encounter tracking** | Click any player to see every previous match you've shared with them |
| 💾 **Match history** | Save matches to a local SQLite database and browse them any time |
| 🎉 **Party detection** | Premade groups highlighted with icons (`★ ▲ ● ■`); enemy premades tinted red |
| 🕵️ **Streamer mode** | Incognito players shown with an `[S]` tag |
| 🖥 **Desktop GUI** | Native egui window with side panels, settings modal, and minimize-to-tray |
| 💻 **Terminal UI** | Full-featured ratatui TUI — works over SSH, no GPU required |
| 🔔 **Toast notifications** | Desktop popups for match detection and update completion |
| 🎮 **Discord Rich Presence** | Map, mode, party size, and elapsed time shown in Discord |
| 🔄 **Auto-updater** | Silent background check on startup; installs new versions in-place |
| 🚀 **Run on startup** | Optionally launch hidden in the system tray at Windows login |
| 🐍 **Python bindings** | `pip install valotracker` — full engine exposed via PyO3 |

---

## Installation

### Option 1 — Inno Setup Wizard (recommended for most users)

Download **`ValoTracker-Setup-x.x.x.exe`** from the
[latest release](https://github.com/Londopy/ValoTracker/releases/latest) and run it.
The wizard installs the TUI, GUI, or both — with optional desktop and Start Menu shortcuts.

### Option 2 — MSI Installer (enterprise / silent install)

Download **`ValoTracker-x.x.x-x86_64.msi`** from the
[latest release](https://github.com/Londopy/ValoTracker/releases/latest).

- Per-user install, **no UAC prompt required**
- Supports silent deployment: `msiexec /i ValoTracker.msi /qn`

### Option 3 — Portable Binaries

Grab a standalone `.exe` directly from the
[latest release](https://github.com/Londopy/ValoTracker/releases/latest) — no install needed, just drop it anywhere on your `PATH`.

| File | Description |
|------|-------------|
| `ValoTracker.exe` | Terminal UI |
| `ValoTracker-gui.exe` | Desktop GUI |

### Option 4 — Scoop

```powershell
scoop bucket add valotracker https://github.com/Londopy/ValoTracker
scoop install valotracker
```

Scoop handles updates automatically (`scoop update valotracker`).

### Option 5 — pip (Python)

```bash
pip install valotracker
valotracker        # TUI
valotracker-gui    # GUI
```

The wheel bundles pre-compiled Windows binaries — no Rust toolchain required.

### Option 6 — Build from Source

```powershell
git clone https://github.com/Londopy/ValoTracker.git
cd ValoTracker

# TUI
cargo build --release -p valotracker-tui
# → target\release\ValoTracker.exe

# GUI
cargo build --release -p valotracker-gui --features gui
# → target\release\ValoTracker-gui.exe

# MSI (requires cargo-wix and WiX Toolset v3)
cargo install cargo-wix
cargo wix -p valotracker-installer --no-build --nocapture
```

**Requirements:** Rust 1.78+, Windows 10/11, VALORANT installed.

---

## Usage

### TUI

```
ValoTracker.exe
```

| Key | Action |
|-----|--------|
| `r` | Force refresh |
| `s` | Save current match to history |
| `h` | Open match history |
| `c` | Open config editor |
| `j` / `k` or `↑` / `↓` | Navigate player rows |
| `Enter` / `Tab` | Open encounter history for selected player |
| `q` / `Esc` | Quit |

### GUI

```
ValoTracker-gui.exe
```

- Click any player with a 👁 icon to open their **encounter history** side panel
- Use the **⚙ Settings** button to configure tray behaviour and startup options
- The window minimizes to the **system tray** when closed (if enabled in Settings)

---

## Python Bindings

```bash
pip install valotracker
```

```python
import valotracker

client = valotracker.ValoTrackerClient()
client.wait_for_match()          # blocks until a match is detected

for p in client.get_players():
    print(f"{p.name}#{p.tag}  {p.rank_name} {p.rr}RR  HS:{p.headshot_pct:.0%}")
```

---

## Configuration

Config is stored at `%APPDATA%\ValoTracker\config.toml` and created automatically on first run.

```toml
[display]
show_streamer_tag       = true
show_party_size         = true
highlight_enemy_parties = true
short_ranks             = false
show_level              = true
show_kd                 = true
show_hs                 = true
show_wr                 = true
show_rr_delta           = true

[features]
minimize_to_tray   = false   # GUI: minimize to tray on close instead of quitting
run_on_startup     = false   # Add ValoTracker to Windows startup (hidden in tray)
check_updates      = true    # Silent background update check (once per 24 h)
notifications      = true    # Windows desktop toast notifications
discord_rpc        = false   # Discord Rich Presence
discord_app_id     = ""      # Leave blank to use the bundled app ID
```

Press `[c]` in the TUI to edit display settings live without touching the file.

---

## Security

ValoTracker is fully open source — every line of code is auditable in this repository.

- Connects **only to `127.0.0.1`** (the local Riot client); no data leaves your machine
- Release binaries are scanned by VirusTotal before publishing
- SHA-256 checksums for every artifact are included in each [GitHub release](https://github.com/Londopy/ValoTracker/releases/latest)

---

## Project Layout

```
ValoTracker/
├── crates/
│   ├── valotracker-core/       # Engine: API, history, updater, Discord, notifications
│   ├── valotracker-tui/        # ratatui terminal frontend
│   ├── valotracker-gui/        # egui desktop GUI (--features gui)
│   │   └── src/views/          # Modular view files (match, history, encounter…)
│   ├── valotracker-py/         # PyO3 Python bindings → PyPI wheel
│   └── valotracker-installer/  # cargo-wix MSI target
├── installer/
│   └── ValoTracker.iss         # Inno Setup script (per-user, no UAC)
├── scoop/
│   └── valotracker.json        # Scoop manifest with autoupdate
├── python/                     # Python package source (pip install valotracker)
└── .github/
    └── workflows/release.yml   # CI: build → MSI → wheel → PyPI → GitHub release
```

---

## Disclaimer

ValoTracker reads data from VALORANT's local client endpoints (`https://127.0.0.1:{port}`)
using credentials stored on your own machine. It does not bypass any external API, inject
into the game process, or violate Riot's Terms of Service as interpreted for read-only local
tooling.

Use at your own risk. The authors are not affiliated with Riot Games.

See [DISCLAIMER.md](DISCLAIMER.md).

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). PRs welcome!

Please run before submitting:

```bash
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

---

## License

MIT — see [LICENSE](LICENSE).
