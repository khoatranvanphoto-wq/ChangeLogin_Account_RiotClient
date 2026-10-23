//! Game Account Switcher core.
//!
//! Everything that is not UI lives here: the account model, encrypted storage,
//! settings, and the per-launcher switching flows (Riot Client, Garena).
//! OS-specific work goes through [`os`], which is a thin facade over the
//! Windows implementation in [`win`] and harmless stubs elsewhere, so the
//! crate builds and its pure logic is testable on any platform.

pub mod backup;
pub mod crypto;
pub mod fields;
pub mod garena;
pub mod geometry;
pub mod legacy;
pub mod model;
pub mod os;
pub mod paths;
pub mod riot;
pub mod settings;
pub mod state;
pub mod store;
pub mod switcher;

#[cfg(windows)]
mod win;

pub use model::{Account, AccountInput, AccountView, Game, Launcher};
pub use settings::Settings;
