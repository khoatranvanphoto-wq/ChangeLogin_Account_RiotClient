//! Choosing the username / password inputs out of an accessibility tree.
//! Kept free of OS types so it can be unit-tested anywhere.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditInfo {
    pub name: String,
    pub is_password: bool,
    pub visible: bool,
}

/// Returns `(username_index, password_index)`.
pub fn pick_login_fields(edits: &[EditInfo]) -> Option<(usize, usize)> {
    let visible: Vec<usize> = (0..edits.len()).filter(|&i| edits[i].visible).collect();
    if visible.len() < 2 {
        return None;
    }
    let is_pwd = |i: usize| edits[i].is_password || looks_like_password(&edits[i].name);

    let pwd = visible.iter().copied().find(|&i| is_pwd(i));
    match pwd {
        Some(p) => {
            // Prefer a labelled username field, otherwise the nearest non-password
            // input that comes before the password.
            let user = visible
                .iter()
                .copied()
                .find(|&i| i != p && !is_pwd(i) && looks_like_username(&edits[i].name))
                .or_else(|| visible.iter().copied().rev().find(|&i| i < p && !is_pwd(i)))
                .or_else(|| visible.iter().copied().find(|&i| i != p && !is_pwd(i)))?;
            Some((user, p))
        }
        None if visible.len() == 2 => Some((visible[0], visible[1])),
        None => None,
    }
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ả' | 'ã' | 'ạ' | 'ă' | 'ắ' | 'ằ' | 'ẳ' | 'ẵ' | 'ặ' | 'â' | 'ấ' | 'ầ'
            | 'ẩ' | 'ẫ' | 'ậ' => 'a',
            'é' | 'è' | 'ẻ' | 'ẽ' | 'ẹ' | 'ê' | 'ế' | 'ề' | 'ể' | 'ễ' | 'ệ' => 'e',
            'í' | 'ì' | 'ỉ' | 'ĩ' | 'ị' => 'i',
            'ó' | 'ò' | 'ỏ' | 'õ' | 'ọ' | 'ô' | 'ố' | 'ồ' | 'ổ' | 'ỗ' | 'ộ' | 'ơ' | 'ớ' | 'ờ'
            | 'ở' | 'ỡ' | 'ợ' => 'o',
            'ú' | 'ù' | 'ủ' | 'ũ' | 'ụ' | 'ư' | 'ứ' | 'ừ' | 'ử' | 'ữ' | 'ự' => 'u',
            'ý' | 'ỳ' | 'ỷ' | 'ỹ' | 'ỵ' => 'y',
            'đ' => 'd',
            other => other,
        })
        .collect()
}

fn looks_like_password(name: &str) -> bool {
    let n = normalize(name);
    ["password", "passwd", "mat khau", "mật khẩu"].iter().any(|k| n.contains(k))
}

fn looks_like_username(name: &str) -> bool {
    let n = normalize(name);
    [
        "username",
        "user name",
        "account",
        "email",
        "ten dang nhap",
        "tai khoan",
        "so dien thoai",
        "phone",
        "login",
    ]
    .iter()
    .any(|k| n.contains(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(name: &str, pwd: bool) -> EditInfo {
        EditInfo { name: name.into(), is_password: pwd, visible: true }
    }

    #[test]
    fn labelled_fields() {
        let edits = [e("Search", false), e("Tên đăng nhập", false), e("Mật khẩu", false)];
        assert_eq!(pick_login_fields(&edits), Some((1, 2)));
    }

    #[test]
    fn password_flag_wins() {
        let edits = [e("", false), e("", true)];
        assert_eq!(pick_login_fields(&edits), Some((0, 1)));
    }

    #[test]
    fn unlabelled_pair() {
        assert_eq!(pick_login_fields(&[e("", false), e("", false)]), Some((0, 1)));
        assert_eq!(pick_login_fields(&[e("", false), e("", false), e("", false)]), None);
    }

    #[test]
    fn ignores_hidden() {
        let mut hidden = e("Username", false);
        hidden.visible = false;
        assert_eq!(pick_login_fields(&[hidden, e("x", true)]), None);
    }
}
