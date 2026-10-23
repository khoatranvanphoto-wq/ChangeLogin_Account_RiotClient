//! At-rest protection for `accounts.dat`.
//!
//! On Windows the JSON is sealed with DPAPI (`CryptProtectData`, current-user
//! scope): no key lives in the app, and only the same Windows user on the same
//! machine can open it. Format: `GAS1:` + Base64(blob).
//!
//! Other platforms are development-only; they store `DEV0:` + Base64(plain JSON)
//! so the round-trip can be exercised in tests. That format is not secret, and
//! the Windows build refuses to write it.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

const DPAPI_PREFIX: &str = "GAS1:";
const DEV_PREFIX: &str = "DEV0:";

pub fn protect(plaintext: &str) -> Result<String, String> {
    #[cfg(windows)]
    {
        let blob = dpapi::protect(plaintext.as_bytes())?;
        return Ok(format!("{DPAPI_PREFIX}{}", B64.encode(blob)));
    }
    #[cfg(not(windows))]
    {
        return Ok(format!("{DEV_PREFIX}{}", B64.encode(plaintext.as_bytes())));
    }
}

pub fn unprotect(data: &str) -> Result<String, String> {
    let data = data.trim();
    if data.is_empty() {
        return Ok(String::new());
    }
    if let Some(payload) = data.strip_prefix(DPAPI_PREFIX) {
        let blob = B64
            .decode(payload.as_bytes())
            .map_err(|e| format!("Dữ liệu hỏng (base64): {e}"))?;
        #[cfg(windows)]
        {
            let plain = dpapi::unprotect(&blob)?;
            return String::from_utf8(plain).map_err(|e| format!("Dữ liệu hỏng (utf8): {e}"));
        }
        #[cfg(not(windows))]
        {
            let _ = blob;
            return Err("Dữ liệu DPAPI chỉ đọc được trên Windows".into());
        }
    }
    if let Some(payload) = data.strip_prefix(DEV_PREFIX) {
        let raw = B64
            .decode(payload.as_bytes())
            .map_err(|e| format!("Dữ liệu hỏng (base64): {e}"))?;
        return String::from_utf8(raw).map_err(|e| format!("Dữ liệu hỏng (utf8): {e}"));
    }
    Err("Không nhận ra định dạng file dữ liệu".into())
}

/// DPAPI decrypt of a raw blob (used to read v3 data too).
pub(crate) fn dpapi_unprotect(blob: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(windows)]
    {
        return dpapi::unprotect(blob);
    }
    #[cfg(not(windows))]
    {
        let _ = blob;
        return Err("Dữ liệu DPAPI chỉ đọc được trên Windows".into());
    }
}

#[cfg(windows)]
mod dpapi {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB,
    };

    pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
        unsafe {
            let mut input = CRYPT_INTEGER_BLOB {
                cbData: data.len() as u32,
                pbData: data.as_ptr() as *mut u8,
            };
            let mut output = CRYPT_INTEGER_BLOB::default();
            CryptProtectData(&mut input, None, None, None, None, 0, &mut output)
                .map_err(|e| format!("CryptProtectData lỗi: {e}"))?;
            let out = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
            let _ = LocalFree(HLOCAL(output.pbData as *mut _));
            Ok(out)
        }
    }

    pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
        unsafe {
            let mut input = CRYPT_INTEGER_BLOB {
                cbData: data.len() as u32,
                pbData: data.as_ptr() as *mut u8,
            };
            let mut output = CRYPT_INTEGER_BLOB::default();
            CryptUnprotectData(&mut input, None, None, None, None, 0, &mut output).map_err(
                |e| {
                    format!(
                        "Không giải mã được dữ liệu (file tạo bởi user/máy khác?): {e}"
                    )
                },
            )?;
            let out = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
            let _ = LocalFree(HLOCAL(output.pbData as *mut _));
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let json = r#"[{"id":"1","username":"tèst","password":"p@ss"}]"#;
        let sealed = protect(json).unwrap();
        assert!(!sealed.contains("p@ss"));
        assert_eq!(unprotect(&sealed).unwrap(), json);
    }

    #[test]
    fn rejects_unknown_format() {
        assert!(unprotect("hello").is_err());
        assert_eq!(unprotect("   ").unwrap(), "");
    }
}
