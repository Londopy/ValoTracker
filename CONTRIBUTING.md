# Contributing to ValoTracker

Thanks for your interest in contributing! Here's everything you need to get started.

---

## Getting Started

```powershell
git clone https://github.com/Londopy/ValoTracker.git
cd ValoTracker
cargo build -p valotracker-tui
```

VALORANT must be running on Windows to test any live match features. For everything else (history, config, UI layout) you can work offline.

---

## Project Structure

```
crates/
├── valotracker-core/        # Engine — API calls, data models, SQLite history
│   ├── src/updater.rs       # Silent background auto-updater
│   ├── src/notifications.rs # Windows toast notifications (winrt-notification)
│   └── src/discord.rs       # Discord Rich Presence (discord-presence, feature-gated)
├── valotracker-tui/         # ratatui terminal UI
├── valotracker-gui/         # egui desktop GUI (cargo build --features gui)
├── valotracker-py/          # PyO3 Python bindings
└── valotracker-installer/   # Packaging-only crate (cargo-wix MSI target)
python/                      # Python package (pip install valotracker)
├── valotracker/launcher.py  # Console script launchers (valotracker, valotracker-gui)
└── valotracker/bin/         # Pre-compiled binaries staged here by CI
installer/                   # Inno Setup script (per-user, no UAC)
scoop/                       # Scoop bucket manifest
```

**The golden rule:** `valotracker-core` must never import from any UI crate. Keep the engine clean.

The `discord` feature in `valotracker-core` is enabled by `valotracker-gui` via `features = ["discord"]` in its dependency declaration. Do not enable it globally to avoid pulling in the IPC client for the TUI or Python bindings.

---

## Before Submitting a PR

Run the full check suite:

```powershell
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

All three must pass with zero errors/warnings before opening a PR.

---

## Code Style

- Rust edition 2021 throughout
- Use `tracing` for logging in library code — no bare `println!` in `valotracker-core`
- All public API items in `valotracker-core` should have `///` doc comments
- Prefer `thiserror`-derived errors; avoid `unwrap()` in library code
- Keep async code in `valotracker-core`; UI crates drive it via `tokio::runtime`

---

## What's Welcome

- Bug fixes — always welcome, please include a description of what broke and how you confirmed it's fixed
- New agent UUIDs — Riot adds agents regularly; PRs updating `engine.rs` with new UUIDs are always merged quickly
- Performance improvements to the history queries
- New columns or stats in the TUI/GUI table (add a config toggle too)
- Python binding improvements
- Improvements to the idle waiting screen, toast notifications, or Discord presence

## What to Discuss First

Open an issue before starting work on:
- New crates or major dependencies
- Changes to the SQLite schema (needs a migration strategy)
- Anything that touches the Riot API in a way that could increase request frequency
- Changes to the auto-updater strategy (e.g., adding a self_update crate, changing the GitHub release format)

## New Feature Guidelines

**Auto-updater:** All network I/O in `updater.rs` must use the blocking `reqwest` client with a hard 3-second timeout. Never add async to the updater — it runs on a dedicated OS thread. Failures must be silently logged, never surfaced to the user.

**Toast notifications:** All calls to `notifications::notify()` must pass the `notifications_enabled` flag from config. Never skip the flag check.

**Discord RPC:** The `discord` feature must remain optional. Code that uses `valotracker_core::discord` must be inside a `#[cfg(feature = "discord")]` block or only in crates that explicitly opt in (currently only `valotracker-gui`).

**MSI / Scoop:** The WiX descriptor at `crates/valotracker-installer/wix/main.wxs` uses per-user install scope. Do not change the `InstallScope` or `UpgradeCode` after the first public release.

---

## Reporting Bugs

Please include:
- Windows version
- VALORANT version / patch
- The exact error message or screenshot
- Steps to reproduce

---

## License

By contributing you agree that your changes will be licensed under the [MIT License](LICENSE).
