use serde::{Deserialize, Serialize};

/// Which launcher an account belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Launcher {
    Riot,
    Garena,
}

impl Launcher {
    pub fn display_name(self) -> &'static str {
        match self {
            Launcher::Riot => "Riot Client",
            Launcher::Garena => "Garena",
        }
    }
}

/// A game that can be started after signing in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Game {
    League,
    Valorant,
    DeltaForce,
}

impl Game {
    pub fn launcher(self) -> Launcher {
        match self {
            Game::League | Game::Valorant => Launcher::Riot,
            Game::DeltaForce => Launcher::Garena,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Game::League => "League of Legends",
            Game::Valorant => "VALORANT",
            Game::DeltaForce => "Delta Force",
        }
    }

    /// `--launch-product` value understood by RiotClientServices.exe.
    pub fn riot_product(self) -> Option<&'static str> {
        match self {
            Game::League => Some("league_of_legends"),
            Game::Valorant => Some("valorant"),
            Game::DeltaForce => None,
        }
    }
}

/// A stored account. Persisted encrypted; the password never leaves the backend
/// except when typed into the launcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub launcher: Launcher,
    #[serde(default)]
    pub label: String,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub note: String,
    /// Game started after login when the user doesn't pick one (tray, hotkeys,
    /// or "remember my choice").
    #[serde(default)]
    pub default_game: Option<Game>,
    /// Unix seconds of the last successful switch.
    #[serde(default)]
    pub last_used: Option<i64>,
}

/// What the UI sees: everything except the password.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountView {
    pub id: String,
    pub launcher: Launcher,
    pub label: String,
    pub username: String,
    pub note: String,
    pub default_game: Option<Game>,
    pub last_used: Option<i64>,
    pub has_password: bool,
}

impl From<&Account> for AccountView {
    fn from(a: &Account) -> Self {
        AccountView {
            id: a.id.clone(),
            launcher: a.launcher,
            label: a.label.clone(),
            username: a.username.clone(),
            note: a.note.clone(),
            default_game: a.default_game,
            last_used: a.last_used,
            has_password: !a.password.is_empty(),
        }
    }
}

/// Add / edit payload from the UI. `id: None` creates a new account.
/// An empty `password` on edit keeps the stored one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInput {
    #[serde(default)]
    pub id: Option<String>,
    pub launcher: Launcher,
    #[serde(default)]
    pub label: String,
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub default_game: Option<Game>,
}

impl AccountInput {
    pub fn validate(&self) -> Result<(), String> {
        if self.username.trim().is_empty() {
            return Err("Tên đăng nhập không được để trống".into());
        }
        if self.id.is_none() && self.password.is_empty() {
            return Err("Mật khẩu không được để trống".into());
        }
        if let Some(g) = self.default_game {
            if g.launcher() != self.launcher {
                return Err(format!(
                    "{} không chạy trên {}",
                    g.display_name(),
                    self.launcher.display_name()
                ));
            }
        }
        Ok(())
    }
}

/// Random 128-bit hex id.
pub fn new_id() -> String {
    let mut b = [0u8; 16];
    if getrandom::getrandom(&mut b).is_err() {
        let t = now_unix_nanos();
        b.copy_from_slice(&t.to_le_bytes());
    }
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn now_unix_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_names_are_snake_case() {
        assert_eq!(serde_json::to_string(&Game::DeltaForce).unwrap(), "\"delta_force\"");
        assert_eq!(serde_json::to_string(&Launcher::Garena).unwrap(), "\"garena\"");
    }

    #[test]
    fn validate_rejects_game_on_wrong_launcher() {
        let input = AccountInput {
            id: None,
            launcher: Launcher::Garena,
            label: String::new(),
            username: "u".into(),
            password: "p".into(),
            note: String::new(),
            default_game: Some(Game::Valorant),
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn ids_are_unique_hex() {
        let a = new_id();
        let b = new_id();
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
