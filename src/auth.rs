use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("access_token", &"[REDACTED]")
            .field("refresh_token", &"[REDACTED]")
            .field("username", &self.username)
            .finish()
    }
}

/// 인증 필요 에러 마커 (exit code 2)
pub struct AuthError;

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "authentication required")
    }
}

impl std::fmt::Debug for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for AuthError {}

/// XDG config directory: ~/.config/velog-cli/
fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("Cannot determine config directory")?
        .join("velog-cli");
    Ok(dir)
}

pub fn credentials_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("credentials.json"))
}

/// Atomic write: tmp 파일에 쓰고 permission 설정 후 rename
pub fn save_credentials(creds: &Credentials) -> Result<()> {
    let dir = config_dir()?;
    std::fs::create_dir_all(&dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    }

    let path = credentials_path()?;
    let tmp = path.with_extension("json.tmp");
    let content = serde_json::to_string_pretty(creds)?;
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(&tmp, &content)?;
    }
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn load_credentials() -> Result<Option<Credentials>> {
    let path = credentials_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path).context("Cannot read credentials")?;
    let creds = serde_json::from_str(&content)
        .context("Credentials file is corrupt. Run `velog auth login` again.")?;
    Ok(Some(creds))
}

pub fn delete_credentials() -> Result<()> {
    let path = credentials_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    let tmp = path.with_extension("json.tmp");
    if tmp.exists() {
        std::fs::remove_file(&tmp)?;
    }
    Ok(())
}

/// Validate that a token is a velog.io JWT with the expected `sub` claim.
/// Does NOT verify the signature — only checks `iss` and `sub` in the payload.
pub fn validate_velog_jwt(token: &str, expected_sub: &str) -> Result<()> {
    let parts: Vec<&str> = token.splitn(3, '.').collect();
    if parts.len() != 3 {
        bail!("Invalid token format (not a JWT)");
    }
    let payload = URL_SAFE_NO_PAD
        .decode(parts[1])
        .context("Failed to decode JWT payload")?;
    let claims: serde_json::Value = serde_json::from_slice(&payload)?;
    if claims["iss"].as_str() != Some("velog.io") {
        bail!("Token is not from velog.io (unexpected issuer)");
    }
    if claims["sub"].as_str() != Some(expected_sub) {
        bail!(
            "Expected {} token, but got a different sub claim",
            expected_sub
        );
    }
    // exp 만료 검증 (경고만 — refresh로 복구 가능)
    if let Some(exp) = claims["exp"].as_u64() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if exp < now {
            eprintln!(
                "Warning: {} token is expired. It will be refreshed automatically.",
                expected_sub
            );
        } else if exp < now + 300 {
            eprintln!(
                "Warning: {} token expires in less than 5 minutes.",
                expected_sub
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;

    /// 테스트용 JWT 생성 헬퍼
    fn make_jwt(claims: &serde_json::Value) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(claims.to_string());
        format!("{}.{}.test_signature", header, payload)
    }

    #[test]
    fn validate_jwt_valid_access_token() {
        let claims = serde_json::json!({
            "iss": "velog.io",
            "sub": "access_token",
            "exp": 9999999999u64,
        });
        assert!(validate_velog_jwt(&make_jwt(&claims), "access_token").is_ok());
    }

    #[test]
    fn validate_jwt_valid_refresh_token() {
        let claims = serde_json::json!({
            "iss": "velog.io",
            "sub": "refresh_token",
            "exp": 9999999999u64,
        });
        assert!(validate_velog_jwt(&make_jwt(&claims), "refresh_token").is_ok());
    }

    #[test]
    fn validate_jwt_not_a_jwt() {
        assert!(validate_velog_jwt("not-a-jwt", "access_token").is_err());
    }

    #[test]
    fn validate_jwt_wrong_issuer() {
        let claims = serde_json::json!({
            "iss": "other.io",
            "sub": "access_token",
        });
        let err = validate_velog_jwt(&make_jwt(&claims), "access_token").unwrap_err();
        assert!(err.to_string().contains("not from velog.io"));
    }

    #[test]
    fn validate_jwt_wrong_sub() {
        let claims = serde_json::json!({
            "iss": "velog.io",
            "sub": "refresh_token",
        });
        let err = validate_velog_jwt(&make_jwt(&claims), "access_token").unwrap_err();
        assert!(err.to_string().contains("different sub claim"));
    }

    #[test]
    fn validate_jwt_expired_still_ok_with_warning() {
        // exp 만료는 경고만 출력, 에러 아님
        let claims = serde_json::json!({
            "iss": "velog.io",
            "sub": "access_token",
            "exp": 1000000000u64,
        });
        assert!(validate_velog_jwt(&make_jwt(&claims), "access_token").is_ok());
    }

    #[test]
    fn credentials_serde_without_username() {
        let json = r#"{"access_token":"a","refresh_token":"r"}"#;
        let creds: Credentials = serde_json::from_str(json).unwrap();
        assert!(creds.username.is_none());
    }

    #[test]
    fn credentials_serde_with_username() {
        let json = r#"{"access_token":"a","refresh_token":"r","username":"testuser"}"#;
        let creds: Credentials = serde_json::from_str(json).unwrap();
        assert_eq!(creds.username.as_deref(), Some("testuser"));
    }

    #[test]
    fn credentials_debug_redacts_tokens() {
        let creds = Credentials {
            access_token: "secret".into(),
            refresh_token: "secret".into(),
            username: Some("user".into()),
        };
        let debug = format!("{:?}", creds);
        assert!(!debug.contains("secret"));
        assert!(debug.contains("[REDACTED]"));
    }
}
