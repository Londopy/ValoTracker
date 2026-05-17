# ValoTracker

**Real-time VALORANT match tracker — view live player ranks, stats, agents, and party info directly in your terminal.**

ValoTracker reads from VALORANT's own local client API (no login required, no third-party account) and displays a live table of all 10 players the moment you enter agent select.

> **Windows only.** Requires VALORANT to be running.

---

## Features

| Feature | Details |
|---|---|
| **Live match table** | Rank, RR, peak rank, HS%, K/D, WR%, agent — updated every 30 s |
| **Party detection** | Premades highlighted with icons (`★ ▲ ● ■`); enemy parties tinted red |
| **Streamer mode** | Incognito players shown with an `[S]` tag |
| **Match history** | Save matches to a local SQLite database and browse them later |
| **Encounter tracking** | See every previous saved match against a specific player ("Receipts") |
| **Extended analytics** | Agent stats, map stats, smurf flagging, session tracking, nemesis leaderboard |
| **TUI + GUI** | Rich terminal UI by default; optional native egui window |
| **Minimize to tray** | GUI hides to the system tray on close — double-click to restore |
| **Run on startup** | Optionally launch at Windows login, starting hidden in the tray |
| **Python bindings** | `pip install ValoTracker` exposes the engine via PyO3 |

---

## Quick Install

Download the latest installer from the [Releases page](https://github.com/Londopy/ValoTracker/releases/latest):

| Download | Description |
|---|---|
| `ValoTracker-Setup-x.x.x.exe` | **Installer (recommended)** — wizard with component selection and shortcuts |
| `ValoTracker.exe` | Portable terminal (TUI) — drop anywhere and run |
| `ValoTracker-gui.exe` | Portable desktop window (GUI) — drop anywhere and run |

Run the installer, launch VALORANT, queue up, and open ValoTracker once you're in agent select.

For detailed setup instructions see the **[Installation & Setup](Installation-and-Setup)** page.

---

## Navigation

- [Installation & Setup](Installation-and-Setup) — downloading, first run, system requirements
- [Configuration](Configuration) — all config options and the live config editor
- [FAQ / Troubleshooting](FAQ-Troubleshooting) — common issues and fixes

---

## Disclaimer

ValoTracker reads from VALORANT's local client endpoints (`127.0.0.1`) and does not inject into the game, modify files, or bypass any API. Use at your own risk — see the full [Disclaimer](https://github.com/Londopy/ValoTracker/blob/main/DISCLAIMER.md).
