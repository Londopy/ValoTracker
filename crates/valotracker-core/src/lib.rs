//! # valotracker-core
//!
//! The engine behind the ValoTracker tool.
//!
//! ## Architecture
//!
//! ```text
//! lockfile  →  client  →  auth
//!                           │
//!                    ┌──────┼──────┐
//!                 presence  │   coregame/pregame
//!                    │    names       │
//!                  state    │       rank + stats
//!                    │   party        │
//!                    └──────┴────────►  models::ResolvedPlayer
//! ```
//!
//! The `engine` module orchestrates all of the above into a single
//! `MatchSnapshot` value that the UI crates consume.

pub mod auth;
pub mod client;
pub mod coregame;
pub mod config;
pub mod error;
pub mod lockfile;
pub mod models;
pub mod names;
pub mod party;
pub mod pregame;
pub mod presence;
pub mod rank;
pub mod state;
pub mod stats;
pub mod websocket;
pub mod engine;
pub mod history;

// Convenient re-exports for UI crates
pub use auth::Auth;
pub use client::{build_local_client, build_remote_client};
pub use config::Config;
pub use error::ValoTrackerError;
pub use lockfile::Lockfile;
pub use models::{match_data::MatchSnapshot, player::ResolvedPlayer};
pub use rank::{tier_to_color, tier_to_name, tier_to_short};
pub use state::GameState;
