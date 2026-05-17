# ValoTracker — Code Complexity & Quality Audit

**Date:** 2026-05-17  
**Scope:** All Rust source files across 5 workspace crates + Python layer  
**Auditor:** Static analysis of source code (no tooling — manual review)  
**Total Rust LOC analyzed:** ~3,800 lines across 23 `.rs` files

---

## Table of Contents

1. [Cyclomatic Complexity](#1-cyclomatic-complexity)
2. [Cognitive Complexity](#2-cognitive-complexity)
3. [Lines of Code Metrics](#3-lines-of-code-metrics)
4. [Coupling Metrics](#4-coupling-metrics)
5. [Cohesion Analysis](#5-cohesion-analysis)
6. [Security Findings](#6-security-findings)
7. [Summary Table](#7-summary-table)

---

## Scoring Key

| Score | Meaning |
|-------|---------|
| 9–10 | Drop everything — blocks safe development |
| 7–8 | Fix before next major feature lands |
| 5–6 | Fix in next regular sprint |
| 3–4 | Address when touching the file |
| 1–2 | Cosmetic / informational |

---

## 1. Cyclomatic Complexity

> Cyclomatic complexity (CC) = number of linearly independent paths through a function.  
> Threshold used: CC > 10 = high, CC > 15 = critical.

---

### F-CC-01 · `draw_match_view` — CC ≈ 20 · Importance: **8/10**

**Location:** `crates/valotracker-gui/src/app.rs:769–1068`  
**Lines:** 299

CC breakdown:
- 3 early-return guard branches (`!valorant_detected`, `bg.loading && snapshot.is_none()`, `bg.snapshot.is_none()`)
- 1 `for player in players` loop
- 1 `if let Some(la) = last_ally { if la != player.is_ally }` team-divider branch
- 12 column render blocks, each with 1–2 config-conditional branches (`show_hs`, `show_wr`, `show_kd`, `show_rr_delta`, `show_level`, `show_party_size`, `highlight_enemy_parties`, `short_ranks`, `times_seen`, etc.)
- Estimated unique paths: > 2,048 (2^11 config combinations × 3 state branches)

**Remediation:** Extract each column into a named helper and pull the per-row logic into a `draw_player_row` function:

```rust
// Before (inside nested lambdas):
let hs_str = if config.display.show_hs {
    format!("{:.0}%", hs * 100.0)
} else { "—".into() };
ui.add_sized([W[6], 20.0], egui::Label::new(...));

// After:
fn draw_player_row(
    ui: &mut egui::Ui,
    player: &ResolvedPlayer,
    config: &Config,
    open_enc: &mut Option<(String, String)>,
) {
    draw_col_party(ui, player, config);
    draw_col_agent(ui, player);
    draw_col_name(ui, player, config, open_enc);
    draw_col_rank(ui, player, config);
    draw_col_hs(ui, player, config);
    draw_col_wr(ui, player, config);
    draw_col_kd(ui, player, config);
    draw_col_level(ui, player, config);
    draw_col_rr_delta(ui, player, config);
    draw_col_met(ui, player);
}
```

---

### F-CC-02 · `GuiApp::update` — CC ≈ 14 · Importance: **8/10**

**Location:** `crates/valotracker-gui/src/app.rs:392–568`  
**Lines:** 177

This is the egui `update()` frame callback — the application's heart — doing too many things at once. It handles:

1. Tray icon / close event handling
2. Auto-updater result polling
3. Toast expiry
4. Status message expiry
5. Escape key → close encounter panel
6. Lock + clone of `BgState`
7. Encounter side-panel rendering
8. Top bar rendering (dispatching 4 bool flags)
9. Status bar rendering
10. Central panel (Tab::Match / Tab::History switch)
11. Deferred action dispatch (refresh, save, history, settings, encounter, delete)
12. Settings modal
13. Toast overlay rendering

CC paths: ~14 from 7 `if let` / `if` guards + 2 match arms + 3 deferred-action flags.

**Remediation:** Extract into `handle_events()`, `expire_transient_state()`, `dispatch_deferred_actions()`, and keep `update()` as a thin coordinator:

```rust
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    self.handle_events(ctx);
    self.expire_transient_state();
    let bg = self.bg.lock().unwrap().clone();
    let actions = self.draw_panels(ctx, &bg);
    self.dispatch_actions(ctx, actions, &bg);
}
```

---

### F-CC-03 · `bg_thread` — CC ≈ 11 · Importance: **6/10**

**Location:** `crates/valotracker-gui/src/app.rs:250–387`  
**Lines:** 137

A free function with two nested infinite loops and 3 match arms inside the inner loop, each with 1–2 discord RPC branches:

```
loop {                          // retry-until-VALORANT
    loop {                      // main refresh
        match build_snapshot {
            Ok(snap) => { if was_none { ... } if let Some(rpc) ... }
            Err(NotInMatch) => { if had_snapshot { if let Some(rpc) ... } }
            Err(e) => { if let Some(rpc) ... }
        }
        loop { /* polling wait */ }
    }
}
```

**Remediation:** Extract the inner snapshot-handling into `handle_snapshot_result(result, rpc, bg, ctx)`, and the polling wait into `wait_or_refresh(rf, deadline)`. Also move this function to a dedicated `bg.rs` module (see F-COH-01).

---

### F-CC-04 · `Engine::build_snapshot` — CC ≈ 12 · Importance: **6/10**

**Location:** `crates/valotracker-core/src/engine.rs:52–217`  
**Lines:** 165

Seven sequential pipeline steps in a single `async fn`, with a `match game_state` (3 arms), a `for` loop with a cache branch, an `Err` arm inside the loop, and a nested closure for player assembly:

```
build_snapshot()
  → get_presences()
  → match game_state { Pregame | Ingame | _ }
  → fetch_names()
  → for puuid { if cached { ... } else { sleep; match rank { Ok | Err } } }
  → join_all(stats_futures)
  → build_party_map()
  → for raw_player { find party; assemble ResolvedPlayer }
```

**Remediation:** Extract the rank-fetching loop into a private method, and the player assembly closure into `assemble_player`:

```rust
// New private method on Engine:
async fn fetch_ranks(
    &mut self,
    puuids: &[String],
    match_id: &str,
) -> HashMap<String, PlayerRank> { ... }

// New free function:
fn assemble_player(raw: &RawPlayer, ...) -> ResolvedPlayer { ... }
```

---

## 2. Cognitive Complexity

> Cognitive complexity measures how hard code is to *read*, not just how many paths it has.  
> Key contributors: nesting depth, out-of-order logic, mixed abstraction levels.

---

### F-COG-01 · Six-level nesting in `draw_match_view` · Importance: **8/10**

**Location:** `crates/valotracker-gui/src/app.rs:804–1066`

The player grid reaches **6 levels of lambda nesting**:

```
egui::ScrollArea::vertical().show(ui, |ui| {         // Level 1
    egui::Grid::new("match_grid").show(ui, |ui| {    // Level 2
        for player in players.iter() {               // Level 3
            if let Some(la) = last_ally {            // Level 4
                if la != player.is_ally { ... }      // Level 5
            }
            if player.times_seen > 0 {               // Level 4
                if resp.clicked() { ... }            // Level 5
            }
        }
    });
});
```

At level 6, the borrow checker context is completely unclear without mental stack-tracing. Any future maintainer touching column rendering must understand all six enclosing scopes.

**Remediation:** Same as F-CC-01. Moving `draw_player_row` out of the closure reduces nesting to 3 levels.

---

### F-COG-02 · Mixed abstraction levels in `GuiApp::update` · Importance: **7/10**

**Location:** `crates/valotracker-gui/src/app.rs:392–568`

Business logic and pixel-level rendering are interleaved:

```rust
// High level: business routing
if let Some((puuid, name)) = open_enc {
    self.open_encounter(&puuid, &name);   // DB call
}

// Low level: pixel math on the same page
egui::Area::new("update_toast".into())
    .fixed_pos(egui::pos2(screen.max.x - 360.0, screen.max.y - 52.0))
```

A reader switching mental gears between DB calls and pixel offsets in the same function carries heavy cognitive load.

**Remediation:** Enforce the rule that `update()` dispatches only. All egui draw calls belong in `draw_*` functions.

---

### F-COG-03 · Tilt-point logic in `build_session` · Importance: **4/10**

**Location:** `crates/valotracker-core/src/history.rs:916–933`

Three interacting `Option<bool>` variables with a non-obvious streak-flip condition:

```rust
let mut last_won: Option<bool> = None;
let mut streak_start_won: Option<bool> = None;
for (i, m) in matches.iter().enumerate() {
    let w = m.won.unwrap_or(false);
    if streak_start_won.is_none() {
        streak_start_won = Some(w);
        last_won = Some(w);
    } else if Some(w) != last_won {
        if Some(w) != streak_start_won {   // ← this condition is non-obvious
            tilt_point = Some(i as u32 + 1);
            break;
        }
        last_won = Some(w);
    }
}
```

The intent (find the first flip back to the starting result) is not explained by the variable names. `streak_start_won` and `last_won` are almost synonymous at first glance.

**Remediation:** Replace with an extracted helper and a comment:

```rust
/// Returns the 1-based game index where the session's win/loss direction
/// first reversed — i.e., went W→L→W or L→W→L. Returns None if the
/// session was a clean streak in one direction.
fn find_tilt_point(matches: &[MyMatchResult]) -> Option<u32> {
    let first = matches.first()?.won.unwrap_or(false);
    let mut current = first;
    for (i, m) in matches.iter().enumerate().skip(1) {
        let w = m.won.unwrap_or(false);
        if w != current {
            if w == first { continue; }  // bounced back to start — that's the tilt
            current = w;
        } else if w == first && current != first {
            return Some(i as u32 + 1);
        }
    }
    None
}
```

---

## 3. Lines of Code Metrics

---

### F-LOC-01 · `crates/valotracker-gui/src/app.rs` — **1,458 lines** · Importance: **9/10**

This is the largest file in the codebase by a factor of 1.5×. It should be the primary structural refactoring target. See F-COH-01 for the split plan.

---

### F-LOC-02 · `draw_match_view` — **299 lines** (single function) · Importance: **8/10**

A function longer than most entire modules in the codebase. Cited in F-CC-01, F-COG-01. No function should exceed ~60 lines in a UI layer.

---

### F-LOC-03 · `crates/valotracker-core/src/history.rs` — **945 lines** · Importance: **5/10**

Contains 8 data structs, the DB schema constant, a migration helper, 3 write methods, 6 read methods, 6 analytics methods, a free function, and a local trait. The analytics section (lines 693–907) was added as "Phase 6" and never split out. See F-COH-02.

---

### F-LOC-04 · `Engine::build_snapshot` — **165 lines** (single async fn) · Importance: **6/10**

Cited under F-CC-04. Target: ≤ 60 lines after extracting `fetch_ranks` and `assemble_player`.

---

### F-LOC-05 · Functions over 50 lines (summary)

| Function | File | Lines |
|---|---|---|
| `draw_match_view` | gui/app.rs | 299 |
| `GuiApp::update` | gui/app.rs | 177 |
| `bg_thread` | gui/app.rs | 137 |
| `draw_history_view` | gui/app.rs | 138 |
| `Engine::build_snapshot` | engine.rs | 165 |
| `summarize_encounters` | history.rs | 73 |
| `MatchHistory::save_match` | history.rs | 63 |
| `MatchHistory::my_match_history` | history.rs | 54 |

---

## 4. Coupling Metrics

---

### F-CUP-01 · `engine.rs` — Efferent coupling = 13 · Importance: **5/10**

**Location:** `crates/valotracker-core/src/engine.rs:1–16`

```rust
use crate::{
    auth::Auth,
    coregame,
    error::ValoTrackerError,
    lockfile::Lockfile,
    models::{match_data::MatchSnapshot, player::ResolvedPlayer},
    names, party, pregame, presence,
    rank::{self, RankCache},
    state::GameState,
    stats,
};
```

13 intra-crate imports. `engine.rs` is the orchestrator so this is partly intentional, but instability is high:

```
I = Ce / (Ca + Ce) = 13 / (13 + 1) ≈ 0.93
```

(Afferent Ca ≈ 1: only `app.rs` in the GUI/TUI imports `Engine` directly.)

**Note:** `resolve_agent_name` (see F-COH-03) adds an unnecessary dependency on a hardcoded data table that should live in a separate `agents.rs` module, which would reduce Ce by 0.

---

### F-CUP-02 · `MatchHistory::open()` — 7 call sites, no connection pooling · Importance: **6/10**

`MatchHistory::open()` creates a fresh `rusqlite::Connection` every time it is called. Call sites identified:

| Location | Method |
|---|---|
| `gui/app.rs:184` | `save_match_action` |
| `gui/app.rs:202` | `open_history` |
| `gui/app.rs:214` | `open_encounter` |
| `gui/app.rs:527` | delete handler |
| `tui/app.rs:186` | `save_current_match` |
| `tui/app.rs:174` | `open_history` |
| `tui/app.rs:208` | `open_encounter` |

SQLite connection creation is cheap but non-zero. More importantly, each open leaves the connection in auto-commit mode with no transaction batching.

**Remediation:** Add an `Arc<Mutex<MatchHistory>>` to `GuiApp` and `App` and open once at startup:

```rust
// In GuiApp / App:
history_db: Arc<Mutex<MatchHistory>>,

// In new():
history_db: Arc::new(Mutex::new(
    MatchHistory::open().expect("history DB")
)),
```

---

### F-CUP-03 · `valotracker-gui/src/app.rs` instability = 1.0 · Importance: **3/10**

Pure leaf node — nothing depends on `valotracker-gui`. All coupling flows inward. This is structurally correct for a UI binary but means any breaking change in any dependency (egui API, valotracker-core, tray-icon) cascades directly here. This is amplified by the monolithic file size — see F-COH-01.

No remediation required for instability; addressed by splitting the file.

---

## 5. Cohesion Analysis

---

### F-COH-01 · `gui/app.rs` — 12 concerns in one file · Importance: **9/10**

This is the primary structural debt in the codebase. The file contains:

| # | Concern | Lines (approx) |
|---|---|---|
| 1 | `BgState` struct | 30–55 |
| 2 | `Tab` enum | 57–64 |
| 3 | `TrayState` struct | 66–73 |
| 4 | `GuiApp` struct + 10 methods | 76–246 |
| 5 | `bg_thread` (background engine) | 250–387 |
| 6 | `eframe::App::update` | 392–568 |
| 7 | `draw_topbar` | 573–653 |
| 8 | `draw_statusbar` | 657–683 |
| 9 | `draw_idle_screen` | 689–765 |
| 10 | `draw_match_view` | 769–1068 |
| 11 | `draw_history_view` | 1072–1210 |
| 12 | `draw_encounter_panel` | 1214–1348 |
| 13 | `build_tray` | 1354–1389 |
| 14 | `draw_settings_modal` | 1395–1457 |

**Recommended split:**

```
crates/valotracker-gui/src/
    app.rs          ← GuiApp struct, new(), update(), dispatch logic only (~150 lines)
    bg.rs           ← BgState, bg_thread
    tray.rs         ← TrayState, build_tray
    ui/
        mod.rs
        match_view.rs   ← draw_match_view, draw_idle_screen, draw_player_row
        history_view.rs ← draw_history_view
        encounter.rs    ← draw_encounter_panel
        topbar.rs       ← draw_topbar
        statusbar.rs    ← draw_statusbar
        settings.rs     ← draw_settings_modal
```

---

### F-COH-02 · `history.rs` — 5 concerns, 945 lines · Importance: **5/10**

**Location:** `crates/valotracker-core/src/history.rs`

The "Phase 6" analytics block was appended without splitting the file:

| Concern | Lines |
|---|---|
| Data model structs (8 structs) | 14–128 |
| Schema + migration | 130–203 |
| Write operations (`save_match`, `delete_match`) | 205–281 |
| Read operations (5 methods) | 283–500 |
| Smurf flag check | 455–500 |
| Analytics operations (6 methods: agent stats, map stats, nemesis, sessions, hourly) | 502–907 |
| `summarize_encounters` free function | 602–675 |
| `OptionalExt` trait | 678–691 |

**Recommended split:**

```
crates/valotracker-core/src/history/
    mod.rs        ← re-exports, MatchHistory struct, open/migrate
    models.rs     ← all data structs
    write.rs      ← save_match, delete_match
    read.rs       ← list_matches, get_match_players, times_played_with, …
    analytics.rs  ← agent_stats, map_stats, nemesis, sessions, hourly, summarize_encounters
```

---

### F-COH-03 · `resolve_agent_name` misplaced in `engine.rs` · Importance: **4/10**

**Location:** `crates/valotracker-core/src/engine.rs:236–275`

A 40-line static lookup table for agent UUIDs lives inside the orchestration module. It has nothing to do with building a snapshot — it's a data table. It also contains a bug (see F-SEC-04).

**Remediation:** Move to a dedicated module:

```rust
// crates/valotracker-core/src/agents.rs
pub const AGENT_TABLE: &[(&str, &str)] = &[
    ("e370fa57-...", "Gekko"),
    // ...
];

pub fn resolve_agent_name(character_id: &str) -> &'static str {
    let id = character_id.to_ascii_lowercase();
    AGENT_TABLE.iter()
        .find(|(uuid, _)| *uuid == id)
        .map(|(_, name)| *name)
        .unwrap_or("Unknown")
}
```

Note the return type change from `String` to `&'static str` — no allocation needed since all values are `'static`.

---

### F-COH-04 · `save_match` suppresses argument-count warning · Importance: **3/10**

**Location:** `crates/valotracker-core/src/history.rs:209`

```rust
#[allow(clippy::too_many_arguments)]
pub fn save_match(
    &self,
    match_id: &str,
    map: &str,
    queue: &str,
    server: &str,
    players: &[ResolvedPlayer],
    my_puuid: &str,
    won: Option<bool>,
) -> Result<(), ValoTrackerError>
```

7 arguments. The lint suppression is a signal to introduce a parameter object:

```rust
pub struct SaveMatchParams<'a> {
    pub match_id: &'a str,
    pub map: &'a str,
    pub queue: &'a str,
    pub server: &'a str,
    pub players: &'a [ResolvedPlayer],
    pub my_puuid: &'a str,
    pub won: Option<bool>,
}

pub fn save_match(&self, params: &SaveMatchParams) -> Result<(), ValoTrackerError>
```

---

## 6. Security Findings

---

### F-SEC-01 · Auto-update binary replacement — no checksum verification · Importance: **7/10**

**Location:** `crates/valotracker-core/src/updater.rs:159–174`

```rust
let bytes = match client.get(&download_url).send().and_then(|r| r.bytes()) {
    Ok(b) => b,
    Err(e) => { ... return UpdateResult::Skipped; }
};
let tmp_path = current_exe.with_extension("exe.vtupdate");
if let Err(e) = std::fs::write(&tmp_path, &bytes) { ... }
```

The downloaded binary is written directly to disk with no integrity check. `checksums.txt` is already published alongside every release but is not fetched or verified here.

An attacker who can influence the download (compromised GitHub CDN, network MITM with a stolen cert) could replace the running binary. The risk is low in practice but the mitigation is trivial.

**Remediation:**

```rust
// After fetching bytes, also fetch checksums.txt and verify:
fn verify_sha256(bytes: &[u8], expected_hex: &str) -> bool {
    use std::fmt::Write;
    // sha2 crate or windows BCryptHashData — add sha2 = "0.10" as a dependency
    let hash = sha2::Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for b in hash { write!(hex, "{b:02x}").ok(); }
    hex == expected_hex
}
```

Add `sha2 = "0.10"` to `valotracker-core/Cargo.toml` and call before `fs::write`.

---

### F-SEC-02 · `MoveFileExW` return value silently discarded · Importance: **5/10**

**Location:** `crates/valotracker-core/src/updater.rs:214–219`

```rust
unsafe {
    MoveFileExW(
        dest_wide.as_ptr(),
        std::ptr::null(),
        MOVEFILE_DELAY_UNTIL_REBOOT,
    );
    // ← return value (BOOL) is dropped here
}
```

`MoveFileExW` returns 0 on failure. If it fails (e.g., insufficient permissions), the old binary is not scheduled for deletion at reboot but `fs::copy` still proceeds. On the next run, two copies of the binary will coexist and the old inode might prevent cleanup.

**Remediation:**

```rust
let ok = unsafe {
    MoveFileExW(
        dest_wide.as_ptr(),
        std::ptr::null(),
        MOVEFILE_DELAY_UNTIL_REBOOT,
    )
};
if ok == 0 {
    // Non-fatal: log and continue — the copy will still work on most systems
    tracing::warn!(
        "updater: MoveFileExW schedule-delete failed (err={}); \
         old binary may persist until manual cleanup",
        unsafe { windows_sys::Win32::Foundation::GetLastError() }
    );
}
```

---

### F-SEC-03 · `AcceptAnyCert` — blanket TLS certificate bypass · Importance: **5/10**

**Location:** `crates/valotracker-core/src/websocket.rs:107–144`

```rust
impl rustls::client::danger::ServerCertVerifier for AcceptAnyCert {
    fn verify_server_cert(&self, ..) -> Result<ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(&self, ..) -> Result<HandshakeSignatureValid, _> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(&self, ..) -> Result<HandshakeSignatureValid, _> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
```

All three verification methods return success unconditionally. This is intentional — Riot uses a self-signed cert on `127.0.0.1` — but the struct name `AcceptAnyCert` does not document _why_ this is safe. Any future developer or security scanner will flag this as a vulnerability.

The actual risk is negligible (localhost only, no external network), but the code lacks a rationale comment.

**Remediation:** Pin to the known Riot certificate thumbprint if available; otherwise add a prominent comment:

```rust
/// Riot Games' local client API uses a self-signed certificate on 127.0.0.1.
/// We cannot use the system trust store because Riot does not install their CA.
/// This verifier is intentionally permissive and safe ONLY because:
///   1. The connection is always to 127.0.0.1 (localhost).
///   2. The lockfile password acts as a second layer of authentication.
///   3. No sensitive data is transmitted from ValoTracker → Riot over this socket.
/// DO NOT use this verifier for any non-localhost connection.
#[derive(Debug)]
struct AcceptRiotLocalCert;
```

Rename `AcceptAnyCert` → `AcceptRiotLocalCert` to make the intent explicit.

---

### F-SEC-04 · Duplicate "Viper" UUID in agent table — data correctness bug · Importance: **5/10**

**Location:** `crates/valotracker-core/src/engine.rs:252, 263`

```rust
("7f94d92c-4234-0a36-9646-3a87eb8b5edc", "Viper"),  // line 252
// ... 10 other agents ...
("707eab51-4836-f488-046a-cda6bf494859", "Viper"),  // line 263
```

Two different UUIDs are both labelled `"Viper"`. One of these is certainly a different agent (the second UUID does not match any known Viper UUID in the public Valorant API). Players on that agent will be shown as "Viper" in both the TUI and GUI, corrupting displayed data.

**Remediation:** Verify against the public API (`https://valorant-api.com/v1/agents`) and correct the wrong entry. Also add a compile-time uniqueness assertion:

```rust
// In a test:
#[test]
fn agent_table_no_duplicate_uuids() {
    use std::collections::HashSet;
    let uuids: HashSet<&str> = AGENT_TABLE.iter().map(|(u, _)| *u).collect();
    assert_eq!(uuids.len(), AGENT_TABLE.len(), "duplicate UUID in AGENT_TABLE");
}
```

---

### F-SEC-05 · `resolve_agent_name` allocates on every call · Importance: **3/10**

**Location:** `crates/valotracker-core/src/engine.rs:270–275`

```rust
    .map(|(_, name)| name.to_string())   // ← heap allocation
    .unwrap_or_else(|| "Unknown".to_owned())  // ← heap allocation
```

Called once per player per snapshot build (~10 players × every 30 s). While not a performance concern at this scale, it is avoidable. See F-COH-03 for the `&'static str` refactor.

---

### F-SEC-06 · Mutex poisoning not handled in `bg_thread` · Importance: **3/10**

**Location:** `crates/valotracker-gui/src/app.rs:277, 287, 295, 316, 347, 362`

```rust
let mut s = bg.lock().unwrap();   // ← repeated 8 times
```

If the UI thread panics while holding the `bg` mutex, subsequent `lock().unwrap()` calls in `bg_thread` will panic on the poisoned mutex, silently killing the background thread. The UI will then freeze on the last rendered state with no error.

**Remediation:**

```rust
// Helper to recover from a poisoned mutex gracefully:
fn lock_bg(bg: &Mutex<BgState>) -> std::sync::MutexGuard<BgState> {
    bg.lock().unwrap_or_else(|poisoned| {
        tracing::error!("bg mutex was poisoned — recovering inner value");
        poisoned.into_inner()
    })
}
```

---

### F-SEC-07 · `#![allow(dead_code)]` masks real unused code in `rank.rs` · Importance: **3/10**

**Location:** `crates/valotracker-core/src/rank.rs:1`

```rust
#![allow(dead_code)]
```

`SeasonalInfo` has 14 deserialized fields, of which only `competitive_tier`, `number_of_wins`, and `number_of_games` are referenced in the file. The blanket suppression prevents the compiler from warning when further dead fields accumulate. The annotation was originally added because the struct mirrors an external API shape, but it now masks real structural waste.

**Remediation:** Remove `#![allow(dead_code)]` and annotate only the specific fields that must be deserialized but not read:

```rust
#[derive(Deserialize)]
struct SeasonalInfo {
    #[serde(rename = "CompetitiveTier")]
    competitive_tier: u8,
    #[serde(rename = "NumberOfWins")]
    number_of_wins: u32,
    #[serde(rename = "NumberOfGames")]
    number_of_games: u32,
    // Fields below are deserialized for API compatibility but not yet used:
    #[allow(dead_code)]
    #[serde(rename = "LeaderboardRank")]
    leaderboard_rank: u32,
    // ... etc
}
```

---

### F-SEC-08 · GitHub owner/repo hardcoded in two locations · Importance: **2/10**

**Location:**  
- `crates/valotracker-core/src/updater.rs:33–34`

```rust
const GITHUB_OWNER: &str = "Londopy";
const GITHUB_REPO:  &str = "ValoTracker";
```

These constants exist only in `updater.rs`. The same owner/repo appears in `release.yml`, `scoop/valotracker.json`, and wiki links. If the GitHub account or repository name changes, the auto-updater silently fails (returns `Skipped` on 404) while the other locations still work.

Not a security vulnerability. The practical risk is a broken auto-updater after a repo rename.

**Remediation:** Centralise in `crates/valotracker-core/src/lib.rs`:

```rust
pub const GITHUB_OWNER: &str = "Londopy";
pub const GITHUB_REPO:  &str = "ValoTracker";
```

And import in `updater.rs`:
```rust
use crate::{GITHUB_OWNER, GITHUB_REPO};
```

---

## 7. Summary Table

| ID | Category | Location | Finding | Importance |
|---|---|---|---|---|
| F-CC-01 | Cyclomatic | gui/app.rs:769 | `draw_match_view` CC ≈ 20 | **8/10** |
| F-CC-02 | Cyclomatic | gui/app.rs:392 | `GuiApp::update` CC ≈ 14, 13 concerns | **8/10** |
| F-CC-03 | Cyclomatic | gui/app.rs:250 | `bg_thread` CC ≈ 11, nested loops | **6/10** |
| F-CC-04 | Cyclomatic | engine.rs:52 | `build_snapshot` CC ≈ 12, 165 lines | **6/10** |
| F-COG-01 | Cognitive | gui/app.rs:804 | 6-level lambda nesting in match grid | **8/10** |
| F-COG-02 | Cognitive | gui/app.rs:392 | Mixed abstraction in `update()` | **7/10** |
| F-COG-03 | Cognitive | history.rs:916 | Opaque tilt-point logic, 3 interacting Options | **4/10** |
| F-LOC-01 | LOC | gui/app.rs | 1,458-line monolithic file | **9/10** |
| F-LOC-02 | LOC | gui/app.rs:769 | 299-line single function | **8/10** |
| F-LOC-03 | LOC | history.rs | 945-line file, 5 concerns | **5/10** |
| F-LOC-04 | LOC | engine.rs:52 | 165-line `build_snapshot` | **6/10** |
| F-CUP-01 | Coupling | engine.rs:1 | 13 efferent imports, instability 0.93 | **5/10** |
| F-CUP-02 | Coupling | 7 call sites | `MatchHistory::open()` called 7×, no pooling | **6/10** |
| F-CUP-03 | Coupling | gui/app.rs | instability = 1.0 (leaf), amplified by monolith | **3/10** |
| F-COH-01 | Cohesion | gui/app.rs | 14 distinct concerns in one file | **9/10** |
| F-COH-02 | Cohesion | history.rs | 5 concerns; analytics appended in-place | **5/10** |
| F-COH-03 | Cohesion | engine.rs:236 | `resolve_agent_name` misplaced; unnecessary allocations | **4/10** |
| F-COH-04 | Cohesion | history.rs:209 | `save_match` 7-arg signature, lint suppressed | **3/10** |
| F-SEC-01 | Security | updater.rs:159 | No SHA256 verification on downloaded binary | **7/10** |
| F-SEC-02 | Security | updater.rs:214 | `MoveFileExW` return value discarded | **5/10** |
| F-SEC-03 | Security | websocket.rs:107 | `AcceptAnyCert` undocumented; misleading name | **5/10** |
| F-SEC-04 | Security | engine.rs:252,263 | Duplicate "Viper" UUID — wrong agent shown | **5/10** |
| F-SEC-05 | Security | engine.rs:270 | `resolve_agent_name` heap-allocates every call | **3/10** |
| F-SEC-06 | Security | gui/app.rs:277+ | Mutex `.unwrap()` × 8 — no poisoning recovery | **3/10** |
| F-SEC-07 | Security | rank.rs:1 | `#![allow(dead_code)]` masks growing dead fields | **3/10** |
| F-SEC-08 | Security | updater.rs:33 | `GITHUB_OWNER`/`GITHUB_REPO` in two places | **2/10** |

---

## Recommended Refactoring Priority

### Sprint 1 (before next feature)
1. **F-LOC-01 / F-COH-01** — Split `gui/app.rs` into 8 files. This unblocks everything else in the GUI.
2. **F-LOC-02 / F-CC-01 / F-COG-01** — Extract `draw_player_row` out of `draw_match_view`.
3. **F-CC-02 / F-COG-02** — Introduce `handle_events`, `expire_transient_state`, `dispatch_actions` in `GuiApp::update`.
4. **F-SEC-04** — Fix the duplicate Viper UUID and add the test.

### Sprint 2 (next regular sprint)
5. **F-SEC-01** — Add SHA256 verification to the auto-updater.
6. **F-CC-04** — Extract `fetch_ranks` and `assemble_player` from `build_snapshot`.
7. **F-CUP-02** — Open `MatchHistory` once at startup; inject via `Arc<Mutex<...>>`.
8. **F-COH-03** — Move `resolve_agent_name` (and the table) to `agents.rs`.

### When touching the file
9. **F-SEC-02** — Check `MoveFileExW` return value.
10. **F-SEC-03** — Rename `AcceptAnyCert` → `AcceptRiotLocalCert`; add rationale comment.
11. **F-SEC-06** — Introduce `lock_bg` helper for poisoning recovery.
12. **F-SEC-07** — Remove blanket `#![allow(dead_code)]` from `rank.rs`.
13. **F-COH-02** — Split `history.rs` into `history/` submodule.
14. **F-COG-03** — Extract `find_tilt_point` with a clear doc comment.
