//! Core logic for Riot Account Manager — intentionally free of any GUI/Tauri/WebKit
//! dependency so it can be unit-tested on any platform (including CI on Linux).

pub mod config;
pub mod crypto;
pub mod login_fields;
pub mod model;
pub mod paths;
pub mod process;
#[cfg(windows)]
mod riot_windows;
pub mod riot;
pub mod settings;
pub mod startup;
pub mod store;
