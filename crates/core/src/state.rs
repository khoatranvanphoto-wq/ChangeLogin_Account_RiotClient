//! Runtime state remembered across restarts: which account is currently signed
//! in on each launcher (as far as this app knows).

use crate::model::Launcher;
use crate::paths;
use crate::store::write_atomic;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ActiveState {
    pub riot: Option<String>,
    pub garena: Option<String>,
}

impl ActiveState {
    pub fn get(&self, launcher: Launcher) -> Option<&str> {
        match launcher {
            Launcher::Riot => self.riot.as_deref(),
            Launcher::Garena => self.garena.as_deref(),
        }
    }

    pub fn set(&mut self, launcher: Launcher, id: Option<String>) {
        match launcher {
            Launcher::Riot => self.riot = id,
            Launcher::Garena => self.garena = id,
        }
    }

    /// Forget an account that was deleted.
    pub fn forget(&mut self, id: &str) {
        if self.riot.as_deref() == Some(id) {
            self.riot = None;
        }
        if self.garena.as_deref() == Some(id) {
            self.garena = None;
        }
    }
}

pub fn load() -> ActiveState {
    std::fs::read_to_string(paths::state_file())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(state: &ActiveState) -> Result<(), String> {
    let json = serde_json::to_string(state).map_err(|e| e.to_string())?;
    write_atomic(&paths::state_file(), json.as_bytes(), false)
}
