# ValoTracker

A fast, privacy-first Valorant match tracker written in Rust.

> **Windows only.** Reads from the local Riot client lockfile — no username
> or password required, no external account needed.

---

## Features

- **Live match table** — all 10 players with rank, RR, peak rank, HS%, K/D,
  WR%, and party grouping, updated every 30 seconds
- **Streamer-mode detection** — incognito players shown with an `[S]` tag
- **Party indicators** — premade groups highlighted with icons (`★ ▲ ● ■`),
  enemy premades tinted red
- **Match history** — save matches to a local SQLite database and browse them
  later; `[s]` to save, `[h]` to view
- **Encounter tracking** — see every previous saved match against a given
  player (the "Receipts" feature)
- **Extended analytics** — agent stats, map stats, smurf flagging, party
  win-rate breakdown, session tracking, nemesis/rivalry leaderboard
- **TUI + optional GUI** — rich terminal UI by default; build with
  `--features gui` for a native egui window
- **Minimize to tray** — GUI can hide to the system tray instead of closing;
  double-click the tray icon to restore
- **Run on startup** — optionally launch ValoTracker automatically when Windows
  starts (hidden in tray until needed)
- **Idle waiting screen** — animated "Waiting for VALORANT…" screen when VALORANT isn't running; automatically transitions to the match view the moment it's detected
- **Auto-updater** — silent background update check on startup; installs new versions in-place and shows a one-line notification; respects a 24-hour cooldown and an opt-out flag
- **Windows toast notifications** — desktop notifications for match detection, data loaded, and update complete; opt-out via config
- **Discord Rich Presence** — shows map, mode, party size, and elapsed time in Discord; opt-in via config
- **MSI installer** — no-UAC per-user MSI built with cargo-wix, alongside the existing Inno Setup wizard
- **Scoop bucket** — `scoop install valotracker`
- **Python CLI launchers** — `valotracker` and `valotracker-gui` console scripts bundled with the wheel
- **Python bindings** — `pip install valotracker` exposes the engine to Python via PyO3

---

## Disclaimer

`ValoTracker` reads data from VALORANT's local client endpoints
(`https://127.0.0.1:{port}`) using credentials stored on your own machine.
It does not bypass any external API, inject into the game process, or
violate Riot's Terms of Service as interpreted for read-only local tooling.

Use at your own risk. The authors are not affiliated with Riot Games.

---

## Installation

### Installer (recommended)

Download `ValoTracker-Setup-x.x.x.exe` from the
[latest release](https://github.com/Londopy/ValoTracker/releases/latest) and
run it. The wizard lets you choose which components to install (TUI, GUI, or
both) and optionally adds desktop and Start Menu shortcuts.

### Portable binaries

Prefer a no-install option? Grab `ValoTracker.exe` (TUI) or
`ValoTracker-gui.exe` (GUI) directly from the
[latest release](https://github.com/Londopy/ValoTracker/releases/latest) and
drop it anywhere on your `PATH`.

### Scoop

```powershell
scoop bucket add valotracker https://github.com/Londopy/ValoTracker
scoop install valotracker
```

### pip (Python)

```bash
pip install valotracker
valotracker        # launches TUI
valotracker-gui    # launches GUI
```

### Build from source

```powershell
# Clone
git clone https://github.com/Londopy/ValoTracker.git
cd ValoTracker

# TUI (default)
cargo build --release -p valotracker-tui
# Binary: target\release\ValoTracker.exe

# GUI (egui)
cargo build --release -p valotracker-gui --features gui
# Binary: target\release\ValoTracker-gui.exe

# MSI installer (requires cargo-wix and WiX Toolset v3)
cargo install cargo-wix
cargo wix -p valotracker-installer --no-build --nocapture
```

**Requirements:** Rust 1.78+, Windows 10/11, VALORANT installed and running.

---

## Security

ValoTracker is open source. You can audit every line of code in this repository.

Each release is scanned by VirusTotal before publishing.

[![VirusTotal](https://img.shields.io/badge/VirusTotal-Clean-brightgreen)](VIRUSTOTAL_URL_HERE)

**To update this badge after a new release:**
1. Go to https://www.virustotal.com
2. Upload the `.msi` or `.exe` from the latest GitHub release
3. Copy the results URL from your browser after the scan
4. Replace `VIRUSTOTAL_URL_HERE` in README.md with that URL
5. Update the badge color: `brightgreen` = 0 detections · `yellow` = 1–2 (false positives) · `red` = investigate

SHA-256 checksums for every release artifact are published in each [GitHub release](https://github.com/Londopy/ValoTracker/releases/latest).

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
| `j`/`k` or `↑`/`↓` | Navigate player rows |
| `Enter`/`Tab` | Open encounter history for selected player |
| `q` / `Esc` | Quit |

### GUI

```
ValoTracker-gui.exe
```

Click any player with a 👁 icon to open their encounter history side panel.

---

## Python bindings

```bash
pip install ValoTracker
```

```python
import ValoTracker

client = ValoTracker.ValoTrackerClient()
client.wait_for_match()          # blocks until you enter a match

players = client.get_players()
for p in players:
    print(f"{p.name}#{p.tag}  {p.rank_name} {p.rr}RR  HS:{p.headshot_pct:.0%}")

# Party detection
from collections import defaultdict
parties = defaultdict(list)
for p in players:
    parties[p.party_id].append(p.name)
for pid, members in parties.items():
    if len(members) > 1:
        print(f"Premade: {', '.join(members)}")
```

---

## Configuration

`ValoTracker` stores its config at `%APPDATA%\ValoTracker\config.toml`. It is created
automatically on first run with all defaults.

```toml
[display]
show_streamer_tag       = true
show_party_size         = true
highlight_enemy_parties = true
short_ranks             = false
show_peak_act           = true
show_level              = true
show_kd                 = true
show_hs                 = true
show_wr                 = true
show_rr_delta           = true
auto_clear              = true

[weapon]
preferred = "Vandal"

[features]
discord_rpc                = false        # Enable Discord Rich Presence (no setup required)
discord_app_id             = ""           # Leave blank to use the bundled ValoTracker app; set your own to customise
gui                        = false        # Launch GUI instead of TUI by default
smurf_flag_threshold_tiers = 8
smurf_flag_threshold_days  = 30
minimize_to_tray           = false        # GUI: minimize to tray on close
run_on_startup             = false        # Launch at Windows login
check_updates              = true         # Silent background update check (once per 24 h)
notifications              = true         # Windows desktop toast notifications
```

You can also edit all display toggles live from within the TUI by pressing
`[c]` to open the config editor.

---

## Project layout

```
ValoTracker/
├── crates/
│   ├── valotracker-core/       # Engine (async Rust, no UI code)
│   │   ├── src/updater.rs      # Silent background auto-updater
│   │   ├── src/notifications.rs# Windows toast notifications
│   │   └── src/discord.rs      # Discord Rich Presence
│   ├── valotracker-tui/        # ratatui terminal frontend
│   ├── valotracker-gui/        # egui desktop GUI (--features gui)
│   ├── valotracker-py/         # PyO3 Python bindings → PyPI
│   └── valotracker-installer/  # cargo-wix MSI packaging target
│       └── wix/main.wxs        # WiX descriptor
├── installer/
│   └── ValoTracker.iss         # Inno Setup script (per-user, no UAC)
├── scoop/
│   └── valotracker.json        # Scoop manifest with autoupdate
├── python/                     # Python package (pip install valotracker)
│   ├── valotracker/
│   │   ├── launcher.py         # run_tui() / run_gui() console scripts
│   │   ├── __main__.py         # python -m valotracker
│   │   └── bin/                # Pre-compiled .exe files (staged by CI)
│   ├── pyproject.toml
│   └── MANIFEST.in
└── .github/
    ├── workflows/release.yml   # Full CI: build → MSI → wheel → PyPI → release
    └── release_template.md     # Release body template with SHA-256 table
```

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

PRs welcome. Please run before submitting:

```bash
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

---

## License

MIT — see [LICENSE](LICENSE).

---

## Disclaimer

See [DISCLAIMER.md](DISCLAIMER.md).