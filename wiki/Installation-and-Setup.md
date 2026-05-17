# Installation & Setup

## System Requirements

- **OS:** Windows 10 or Windows 11 (64-bit)
- **VALORANT** installed and running (you must be in a match or agent select)
- No Rust, Python, or other runtime needed for the pre-built binaries

---

## Option 1 — MSI Installer (Recommended)

The MSI installer requires **no administrator rights** and installs per-user to `%LOCALAPPDATA%\ValoTracker`.

1. Go to the [Releases page](https://github.com/Londopy/ValoTracker/releases/latest).
2. Download `ValoTracker-x.x.x-x86_64.msi` and double-click it.
3. The wizard lets you choose:
   - Which components to install (TUI, GUI, or both)
   - Whether to add desktop shortcuts (on by default)
4. Launch VALORANT, queue for a match, and open ValoTracker once you're in agent select.

The config file is created automatically at `%APPDATA%\ValoTracker\config.toml` on first run.

---

## Option 2 — Inno Setup Wizard

A classic setup wizard is also available as `ValoTracker-Setup-x.x.x.exe`. It installs per-user to `%LOCALAPPDATA%\ValoTracker` with no UAC prompt.

---

## Option 3 — Scoop

```powershell
scoop bucket add valotracker https://github.com/Londopy/ValoTracker
scoop install valotracker
```

Scoop keeps your install up-to-date with `scoop update valotracker`.

---

## Option 4 — Portable Binaries

Download the standalone executables directly from the [Releases page](https://github.com/Londopy/ValoTracker/releases/latest):

- `ValoTracker.exe` — terminal interface (TUI)
- `ValoTracker-gui.exe` — native desktop window (GUI)

Place either file anywhere and run — no setup needed.

---

## Option 5 — pip (Python)

```bash
pip install valotracker
valotracker        # launch the terminal UI
valotracker-gui    # launch the desktop GUI
```

The wheel bundles the pre-compiled Rust binaries so no separate download is needed. Use `python -m valotracker` to launch the TUI from a Python environment.

You can also use the Python API directly:

```python
import valotracker

client = valotracker.ValoTrackerClient()
client.wait_for_match()   # blocks until you enter a match

for p in client.get_players():
    print(f"{p.name}#{p.tag}  {p.rank_name} {p.rr}RR  HS:{p.headshot_pct:.0%}")
```

---

## Option 6 — Build from Source

### Prerequisites

- [Rust](https://rustup.rs/) 1.78 or newer
- Windows 10/11
- For the MSI: [WiX Toolset v3](https://wixtoolset.org/) and `cargo install cargo-wix`

### Steps

```powershell
# Clone the repository
git clone https://github.com/Londopy/ValoTracker.git
cd ValoTracker

# Build the TUI
cargo build --release -p valotracker-tui
# Output: target\release\ValoTracker.exe

# Build the GUI
cargo build --release -p valotracker-gui --features gui
# Output: target\release\valotracker-gui.exe

# Build the MSI (after installing cargo-wix and WiX Toolset v3)
cargo install cargo-wix
cargo wix -p valotracker-installer --no-build --nocapture
```

---

## First Run

When you launch ValoTracker for the first time:

1. It displays an animated **"Waiting for VALORANT…"** screen if VALORANT isn't running — no error, just a friendly idle state. It polls every 2 seconds and transitions automatically when VALORANT is detected.
2. Once VALORANT is open, it connects to VALORANT's local API (no account or credentials needed).
3. When a match is detected it fetches player data, renders the table, and fires a **Windows toast notification** (if enabled in config).
4. A default `config.toml` is written to `%APPDATA%\ValoTracker\config.toml` if one doesn't exist yet.

> **Tip:** You can launch ValoTracker before or after queuing — it will wait patiently until it detects agent select.

---

## Auto-Updater

ValoTracker silently checks for updates in the background on startup, **at most once every 24 hours**. If a newer version is available it is downloaded and the binary is replaced in-place. You'll see a one-line notification in the status bar (TUI) or a toast in the bottom-right corner (GUI):

```
ValoTracker updated to v1.2.0 — restart to apply
```

To opt out, set `check_updates = false` in your `config.toml`.

---

## TUI Keybindings

| Key | Action |
|---|---|
| `r` | Force refresh |
| `s` | Save current match to history |
| `h` | Open match history |
| `c` | Open config editor |
| `j` / `k` or `↑` / `↓` | Navigate player rows |
| `Enter` / `Tab` | Open encounter history for selected player |
| `q` / `Esc` | Quit |

---

## GUI

Launch `ValoTracker-gui.exe`. Click any player row with a 👁 icon to open their encounter history in a side panel. The GUI shows the animated idle screen when VALORANT isn't running — no blank screen.

---

## Next Steps

- Customize display options and enable Discord RPC or notifications: see [Configuration](Configuration)
- Something not working? See [FAQ / Troubleshooting](FAQ-Troubleshooting)
