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
├── valotracker-core/   # Engine — all API calls, data models, SQLite history
├── valotracker-tui/    # ratatui terminal UI
├── valotracker-gui/    # egui desktop GUI (cargo build --features gui)
└── valotracker-py/     # PyO3 Python bindings
python/        # Pure-Python wrapper package
```

**The golden rule:** `valotracker-core` must never import from any UI crate. Keep the engine clean.

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

## What to Discuss First

Open an issue before starting work on:
- New crates or major dependencies
- Changes to the SQLite schema (needs a migration strategy)
- Anything that touches the Riot API in a way that could increase request frequency

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
