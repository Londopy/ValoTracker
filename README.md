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
- **Python bindings** — `pip install ValoTracker` exposes the engine to Python via PyO3

---

## Disclaimer

`ValoTracker` reads data from VALORANT's local client endpoints
(`https://127.0.0.1:{port}`) using credentials stored on your own machine.
It does not bypass any external API, inject into the game process, or
violate Riot's Terms of Service as interpreted for read-only local tooling.

Use at your own risk. The authors are not affiliated with Riot Games.

---

## Installation

### Pre-built binaries (Windows)

Download `ValoTracker.exe` (TUI) or `ValoTracker-gui.exe` (GUI) from the
[latest release](https://github.com/Londopy/ValoTracker/releases/latest) and
drop it anywhere on your `PATH`.

### Build from source

```powershell
# Clone
git clone https://github.com/Londopy/ValoTracker.git
cd ValoTracker

# TUI (default)
cargo build --release -p ValoTracker-tui
# Binary: target\release\ValoTracker.exe

# GUI (egui)
cargo build --release -p ValoTracker-gui --features gui
# Binary: target\release\ValoTracker-gui.exe
```

**Requirements:** Rust 1.78+, Windows 10/11, VALORANT installed and running.

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
show_level              = true
show_kd                 = true
show_hs                 = true
show_wr                 = true
show_rr_delta           = true

[weapon]
preferred = "Vandal"

[features]
discord_rpc = false
gui         = false
```

You can also edit all display toggles live from within the TUI by pressing
`[c]` to open the config editor.

---

## Project layout

```
ValoTracker/
├── crates/
│   ├── ValoTracker-core/    # Engine (async Rust, no UI code)
│   ├── ValoTracker-tui/     # ratatui terminal frontend
│   ├── ValoTracker-gui/     # egui desktop GUI (--features gui)
│   └── ValoTracker-py/      # PyO3 Python bindings → PyPI
└── python/         # Pure-Python package wrapping ValoTracker-py
    └── ValoTracker/
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