# Installation & Setup

## System Requirements

- **OS:** Windows 10 or Windows 11 (64-bit)
- **VALORANT** installed and running (you must be in a match or agent select)
- No Rust, Python, or other runtime needed for the pre-built binaries

---

## Option 1 — Installer (Recommended)

1. Go to the [Releases page](https://github.com/Londopy/ValoTracker/releases/latest).
2. Download `ValoTracker-Setup-x.x.x.exe` and run it.
3. The setup wizard will guide you through:
   - Choosing an install location
   - Selecting components (TUI, GUI, or both)
   - Adding desktop and Start Menu shortcuts
4. Launch VALORANT, queue for a match, and open ValoTracker once you're in agent select.

The config file is created automatically at `%APPDATA%\ValoTracker\config.toml` on first run.

---

## Option 2 — Portable Binaries

Prefer a no-install option? Download the standalone executables directly from the [Releases page](https://github.com/Londopy/ValoTracker/releases/latest):

- `ValoTracker.exe` — terminal interface (TUI)
- `ValoTracker-gui.exe` — native desktop window (GUI)

Place either file anywhere on your machine and run it — no setup needed.

---

## Option 3 — Build from Source

### Prerequisites

- [Rust](https://rustup.rs/) 1.78 or newer
- Windows 10/11

### Steps

```powershell
# Clone the repository
git clone https://github.com/Londopy/ValoTracker.git
cd ValoTracker

# Build the TUI (default)
cargo build --release -p valotracker-tui
# Output: target\release\ValoTracker.exe

# Build the GUI
cargo build --release -p valotracker-gui
# Output: target\release\ValoTracker-gui.exe
```

---

## Option 4 — Python Bindings

If you want to script against ValoTracker from Python:

```bash
pip install ValoTracker
```

```python
import ValoTracker

client = ValoTracker.ValoTrackerClient()
client.wait_for_match()   # blocks until you enter a match

for p in client.get_players():
    print(f"{p.name}#{p.tag}  {p.rank_name} {p.rr}RR  HS:{p.headshot_pct:.0%}")
```

---

## First Run

When you launch ValoTracker for the first time:

1. It reads the Riot lockfile from `%LOCALAPPDATA%\Riot Games\Riot Client\Config\lockfile` — this is written by the Riot client automatically and contains the local API credentials.
2. It connects to VALORANT's local WebSocket to detect when a match starts.
3. Once a match is detected, it fetches player data and renders the table.
4. A default `config.toml` is written to `%APPDATA%\ValoTracker\config.toml` if one doesn't exist yet.

> **Tip:** You can launch ValoTracker before or after queuing — it will wait until it detects agent select.

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

Launch `ValoTracker-gui.exe` instead of `ValoTracker.exe`. Click any player row with a 👁 icon to open their encounter history in a side panel.

---

## Next Steps

- Customize display options: see [Configuration](Configuration)
- Something not working? See [FAQ / Troubleshooting](FAQ-Troubleshooting)
