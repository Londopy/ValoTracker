//! View modules — pure egui draw functions for each UI section.
//! Each module is a self-contained panel/widget with no access to `GuiApp` state
//! beyond what it receives as parameters.

pub mod encounter;
pub mod history;
pub mod idle;
pub mod match_view;
pub mod settings;
pub mod topbar;
