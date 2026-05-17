# Configuration

ValoTracker stores its config at:

```
%APPDATA%\ValoTracker\config.toml
```

The file is created automatically on first run with all defaults. You can edit it in any text editor, or use the **live config editor** in the TUI by pressing `c`.

---

## Full Default Config

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
discord_rpc                  = false
discord_app_id               = ""
gui                          = false
smurf_flag_threshold_tiers   = 8
smurf_flag_threshold_days    = 30
minimize_to_tray             = false
run_on_startup               = false
check_updates                = true
notifications                = true
```

---

## `[display]` Options

| Key | Type | Default | Description |
|---|---|---|---|
| `show_streamer_tag` | bool | `true` | Show `[S]` next to players with Streamer Mode enabled |
| `show_party_size` | bool | `true` | Show party size number next to the party icon, e.g. `★(3)` |
| `highlight_enemy_parties` | bool | `true` | Tint enemy premade groups red |
| `short_ranks` | bool | `false` | Use short rank names — `D2` instead of `Diamond 2` |
| `show_peak_act` | bool | `true` | Show the act alongside peak rank (e.g. `Diamond 2 — Act 3`) |
| `show_level` | bool | `true` | Show player account level |
| `show_kd` | bool | `true` | Show K/D ratio column |
| `show_hs` | bool | `true` | Show headshot percentage column |
| `show_wr` | bool | `true` | Show win rate percentage column |
| `show_rr_delta` | bool | `true` | Show RR gained/lost last match |
| `auto_clear` | bool | `true` | Clear the terminal between refreshes for a clean redraw |

---

## `[weapon]` Options

| Key | Type | Default | Description |
|---|---|---|---|
| `preferred` | string | `"Vandal"` | Preferred weapon to highlight in the stats table. Use the exact in-game name, e.g. `"Phantom"`, `"Operator"`, `"Sheriff"`. |

---

## `[features]` Options

| Key | Type | Default | Description |
|---|---|---|---|
| `discord_rpc` | bool | `false` | Enable Discord Rich Presence. Shows your current map, mode, party size, and elapsed time in Discord. |
| `discord_app_id` | string | *(official ID)* | Override the Discord application ID. Leave blank (the default) to use the bundled ValoTracker app. Set your own if you want a custom presence name or assets. |
| `gui` | bool | `false` | Launch the egui GUI window instead of the TUI on startup. |
| `smurf_flag_threshold_tiers` | int | `8` | Flag a player as a potential smurf if they climbed this many rank tiers… |
| `smurf_flag_threshold_days` | int | `30` | …within this many days. |
| `minimize_to_tray` | bool | `false` | **GUI only.** Hide to the system tray on close. Double-click the tray icon to restore. |
| `run_on_startup` | bool | `false` | **GUI only.** Launch at Windows login, starting hidden in the tray. |
| `check_updates` | bool | `true` | Check for updates silently in the background on startup, at most once per 24 hours. Set to `false` to opt out entirely. |
| `notifications` | bool | `true` | Send Windows desktop toast notifications for match detection, data loaded, and update completion. Set to `false` to silence all notifications. |

---

## Discord Rich Presence Setup

Discord presence is opt-in but requires **no setup from you**. ValoTracker ships with an official Discord application pre-registered — just flip the toggle:

```toml
[features]
discord_rpc = true
```

Restart ValoTracker. Your Discord status will show your current map, mode, party size, and elapsed time automatically.

ValoTracker connects to Discord IPC silently — if Discord isn't running it skips the connection. It reconnects automatically if Discord restarts.

**Advanced — use your own Discord application:**
If you want to use a custom presence (different name, assets, etc.) you can override the app ID:

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications) and create a new application.
2. Upload your rich presence assets under **Rich Presence → Art Assets** (use `valotracker_logo` as the key, or your own name).
3. Copy the **Application ID** from the General Information page.
4. Add it to `config.toml`:
   ```toml
   [features]
   discord_rpc    = true
   discord_app_id = "YOUR_APP_ID_HERE"
   ```
5. Restart ValoTracker.

---

## Auto-Updater

ValoTracker checks for new releases on startup, at most once per 24 hours. If a new version is available:

- The binary is downloaded and replaced in-place (no installer required).
- **TUI:** A one-line message appears in the status bar: `ValoTracker updated to v1.x.x — restart to apply`
- **GUI:** A green toast notification appears in the bottom-right corner for 6 seconds.

The check has a hard 3-second network timeout — if the check fails for any reason, ValoTracker continues normally with no error shown.

To disable: set `check_updates = false` in `config.toml`.

---

## Live Config Editor (TUI)

Press `c` while ValoTracker is running to open the built-in config editor. Toggle any display option on or off and the table updates immediately — no restart required. Changes are saved to `config.toml` automatically.

## Settings Panel (GUI)

Click the **⚙** button in the top-right corner to open the Settings panel. Toggle **Minimize to tray** and **Run on startup** with checkboxes — changes are written to `config.toml` and the Windows registry immediately.

---

## Tips

- **Hide columns you don't care about** — set `show_kd`, `show_hs`, etc. to `false` for a more compact table.
- **Smurf detection** — the defaults (`8` tiers in `30` days) flag someone who went from Silver to Diamond in a month. Tune these to be more or less aggressive.
- **Short ranks** — useful if you have a narrow terminal; `D2 100RR` takes less space than `Diamond 2 100RR`.
- **Tray + startup combo** — enable both `minimize_to_tray` and `run_on_startup` so ValoTracker is always running quietly in the background and ready the moment you queue.
- **Quiet mode** — set both `check_updates = false` and `notifications = false` if you prefer ValoTracker to make no noise and no network calls beyond the match data fetch.
