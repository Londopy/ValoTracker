# FAQ / Troubleshooting

---

## "No match detected" / table never loads

**ValoTracker is waiting for you to enter agent select.** It polls the local VALORANT WebSocket and only starts fetching data once a match is found.

- Make sure VALORANT is **open and running** — not just the launcher.
- You must be **in a match or agent select**, not in the main menu.
- Try pressing `r` to force a manual refresh.

---

## "Could not read lockfile" / auth error

ValoTracker reads credentials from the Riot lockfile at:

```
%LOCALAPPDATA%\Riot Games\Riot Client\Config\lockfile
```

This file is created by the Riot client when VALORANT starts and deleted when it closes.

**Fixes:**
- Make sure the Riot client (not just the game) is running.
- Restart VALORANT fully (close and relaunch via the Riot client).
- Make sure you haven't moved or renamed the Riot Games install directory.

---

## Player ranks show as "Unrated" or "Unknown"

This usually means ValoTracker couldn't reach Riot's PD/GLZ API endpoints in time.

- Wait for the next auto-refresh (every 30 s) or press `r`.
- Check your internet connection — ValoTracker makes outbound requests to Riot's own servers to resolve ranks.
- If the match just started, it can take a few seconds for rank data to become available.

---

## The terminal output is garbled / overlapping text

- Set `auto_clear = true` in your config (it's the default).
- Make sure your terminal supports ANSI escape codes. **Windows Terminal** and **PowerShell 7+** both work well. The old `cmd.exe` may have issues.
- Try resizing your terminal to be wider — the table needs at least ~120 columns to display comfortably.

---

## ValoTracker crashes immediately on launch

- Check that you're on **Windows 10 or 11**. ValoTracker does not support earlier versions.
- Make sure you downloaded the correct binary (`ValoTracker.exe` for TUI, `ValoTracker-gui.exe` for GUI).
- Run from a PowerShell or Windows Terminal window to see any error output instead of the window closing instantly.

---

## The GUI window (`ValoTracker-gui.exe`) doesn't open

- Some older GPU drivers can cause egui to fail silently. Try updating your graphics drivers.
- If you just want the tracker without a native window, use `ValoTracker.exe` (the TUI) instead — it's fully featured.

---

## "APPDATA environment variable not set"

This is a very uncommon Windows configuration issue. ValoTracker needs `%APPDATA%` to find and save `config.toml`.

- Open PowerShell and run `echo $env:APPDATA`. If it returns nothing, your environment variables may be corrupted.
- Try running ValoTracker as administrator once to let it create the config directory.

---

## I changed config.toml but nothing changed

- The config is read at startup. **Restart ValoTracker** after editing `config.toml` manually.
- Alternatively, use the live config editor inside the TUI (`c` key) — changes there apply immediately without restarting.
- Make sure you saved the file and that the TOML syntax is valid. Invalid TOML will cause ValoTracker to fall back to defaults silently.

---

## Will I get banned for using this?

ValoTracker reads from VALORANT's local client API (`127.0.0.1`) — the same endpoints used by tools like Blitz and Tracker.gg overlays. It does not inject into the game, read memory, or modify any files.

That said, **Riot Games can update their policies at any time**. You use this tool at your own risk. See the full [Disclaimer](https://github.com/Londopy/ValoTracker/blob/main/DISCLAIMER.md) before using.

---

## Something else is broken

[Open an issue](https://github.com/Londopy/ValoTracker/issues) and include:
- What you were doing when it broke
- Any error message shown in the terminal
- Your Windows version and VALORANT region
