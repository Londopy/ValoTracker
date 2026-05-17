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
gui                          = false
smurf_flag_threshold_tiers   = 8
smurf_flag_threshold_days    = 30
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
| `discord_rpc` | bool | `false` | Enable Discord Rich Presence integration (shows match info in your Discord status) |
| `gui` | bool | `false` | Launch the egui GUI window instead of the TUI on startup |
| `smurf_flag_threshold_tiers` | int | `8` | Flag a player as a potential smurf if they climbed this many rank tiers… |
| `smurf_flag_threshold_days` | int | `30` | …within this many days. Players meeting both thresholds get a smurf indicator. |
| `minimize_to_tray` | bool | `false` | **GUI only.** Hide the window to the system tray when you click the close button. Double-click the tray icon to restore, or right-click for Open / Quit. |
| `run_on_startup` | bool | `false` | **GUI only.** Add ValoTracker to the Windows startup registry (`HKCU\...\Run`) so it launches automatically at login. Starts hidden in the tray. |

---

## Live Config Editor (TUI)

Press `c` while ValoTracker is running to open the built-in config editor. Toggle any display option on or off and the table updates immediately — no restart required. Changes are saved to `config.toml` automatically when you exit the editor.

## Settings Panel (GUI)

Click the **⚙** button in the top-right corner of the GUI window to open the Settings panel. From here you can toggle **Minimize to tray** and **Run on startup** with checkboxes — changes are written to `config.toml` and (for startup) the Windows registry immediately.

---

## Tips

- **Hide columns you don't care about** — set `show_kd`, `show_hs`, etc. to `false` for a more compact table.
- **Smurf detection** — the defaults (`8` tiers in `30` days) flag someone who went from, say, Silver to Diamond in a month. Tune these to be more or less aggressive.
- **Short ranks** — useful if you have a narrow terminal; `D2 100RR` takes less space than `Diamond 2 100RR`.
- **Tray + startup combo** — enable both `minimize_to_tray` and `run_on_startup` so ValoTracker is always running quietly in the background and ready the moment you queue.
