use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub access_token: String,
    pub refresh_token: String,
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

fn credentials_path() -> Result<PathBuf> {
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
    std::fs::write(&tmp, &content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
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
        bail!("Expected {} token, got {:?}", expected_sub, claims["sub"]);
    }
    Ok(())
}
