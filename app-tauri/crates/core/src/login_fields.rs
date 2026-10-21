//! Pick username/password edit controls from an accessibility tree.
//! Kept OS-free so field-selection logic can be unit-tested on CI (Linux).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginEdit {
    pub name: String,
    pub is_password: bool,
    pub offscreen: bool,
    pub enabled: bool,
}

/// Returns `(username_index, password_index)` into `edits`, or `None` if the
/// tree does not contain a usable pair of visible fields.
pub fn pick_login_fields(edits: &[LoginEdit]) -> Option<(usize, usize)> {
    let candidates: Vec<usize> = edits
        .iter()
        .enumerate()
        .filter(|(_, e)| e.enabled && !e.offscreen)
        .map(|(i, _)| i)
        .collect();
    if candidates.len() < 2 {
        return None;
    }

    let is_pwd = |i: usize| edits[i].is_password || name_looks_password(&edits[i].name);
    let is_user = |i: usize| !is_pwd(i) && name_looks_username(&edits[i].name);

    if let Some(pwd) = candidates.iter().copied().find(|&i| is_pwd(i)) {
        let user = candidates
            .iter()
            .copied()
            .find(|&i| is_user(i) && i != pwd)
            .or_else(|| {
                candidates
                    .iter()
                    .copied()
                    .find(|&i| i != pwd && !is_pwd(i))
            })?;
        return Some((user, pwd));
    }

    if let Some(user) = candidates.iter().copied().find(|&i| is_user(i)) {
        let pwd = candidates.iter().copied().find(|&i| i != user)?;
        return Some((user, pwd));
    }

    // Unlabelled pair: Riot login is username then password.
    if candidates.len() == 2 {
        return Some((candidates[0], candidates[1]));
    }
    None
}

fn name_looks_password(name: &str) -> bool {
    let n = normalize(name);
    n.contains("password")
        || n.contains("passwd")
        || n.contains("mat khau")
        || n.contains("mot de passe")
        || n.contains("passwort")
        || n.contains("senha")
}

fn name_looks_username(name: &str) -> bool {
    let n = normalize(name);
    n.contains("username")
        || n.contains("user name")
        || n.contains("email")
        || n.contains("e-mail")
        || n.contains("riot id")
        || n.contains("account")
        || n.contains("ten dang nhap")
        || n.contains("tai khoan")
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .replace('đ', "d")
        .replace('ă', "a")
        .replace('â', "a")
        .replace('ê', "e")
        .replace('ô', "o")
        .replace('ơ', "o")
        .replace('ư', "u")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(name: &str, is_password: bool) -> LoginEdit {
        LoginEdit {
            name: name.into(),
            is_password,
            offscreen: false,
            enabled: true,
        }
    }

    #[test]
    fn two_unlabelled_edits_are_user_then_password() {
        let edits = vec![edit("", false), edit("", true)];
        assert_eq!(pick_login_fields(&edits), Some((0, 1)));
    }

    #[test]
    fn named_fields_win_over_order() {
        let edits = vec![
            edit("Password", true),
            edit("Username", false),
            edit("search", false),
        ];
        assert_eq!(pick_login_fields(&edits), Some((1, 0)));
    }

    #[test]
    fn vietnamese_names() {
        let edits = vec![
            edit("Tên đăng nhập", false),
            edit("Mật khẩu", false),
        ];
        assert_eq!(pick_login_fields(&edits), Some((0, 1)));
    }

    #[test]
    fn offscreen_and_disabled_are_ignored() {
        let edits = vec![
            LoginEdit {
                name: "hidden".into(),
                is_password: false,
                offscreen: true,
                enabled: true,
            },
            edit("Username", false),
            LoginEdit {
                name: "dead".into(),
                is_password: true,
                offscreen: false,
                enabled: false,
            },
            edit("Password", true),
        ];
        assert_eq!(pick_login_fields(&edits), Some((1, 3)));
    }

    #[test]
    fn two_generic_visible_edits() {
        let edits = vec![edit("", false), edit("", false)];
        assert_eq!(pick_login_fields(&edits), Some((0, 1)));
    }

    #[test]
    fn three_unlabelled_non_password_is_ambiguous() {
        let edits = vec![edit("", false), edit("", false), edit("", false)];
        assert_eq!(pick_login_fields(&edits), None);
    }

    #[test]
    fn single_field_is_not_enough() {
        let edits = vec![edit("Username", false)];
        assert_eq!(pick_login_fields(&edits), None);
    }
}
