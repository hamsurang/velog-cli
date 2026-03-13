use anyhow::Context;
use colored::Colorize;

use crate::auth::{self, Credentials};
use crate::cli::Format;
use crate::client::VelogClient;
use crate::models::CompactAuthStatus;
use crate::output;

use super::require_auth;

// ---- Auth handlers ----

pub async fn auth_login(format: Format) -> anyhow::Result<()> {
    if format != Format::Pretty {
        anyhow::bail!("auth login requires --format pretty (interactive mode)");
    }
    eprintln!("Paste your velog tokens (hidden input).");
    eprintln!("Find them in browser DevTools → Application → Cookies → velog.io");

    eprint!("access_token: ");
    let access_token = rpassword::read_password().context("Failed to read access_token")?;
    anyhow::ensure!(
        !access_token.trim().is_empty(),
        "access_token cannot be empty"
    );
    for w in auth::validate_velog_jwt(access_token.trim(), "access_token")? {
        eprintln!("{}", w);
    }

    eprint!("refresh_token: ");
    let refresh_token = rpassword::read_password().context("Failed to read refresh_token")?;
    anyhow::ensure!(
        !refresh_token.trim().is_empty(),
        "refresh_token cannot be empty"
    );
    for w in auth::validate_velog_jwt(refresh_token.trim(), "refresh_token")? {
        eprintln!("{}", w);
    }

    let creds = Credentials {
        access_token: access_token.trim().to_string(),
        refresh_token: refresh_token.trim().to_string(),
        username: None,
    };

    // 실제 API 호출로 토큰 유효성 최종 확인
    let mut client = VelogClient::new(creds.clone())?;
    let (user, new_creds) = client
        .current_user()
        .await
        .context("Token validation failed. The tokens may be expired.")?;
    let mut creds = new_creds.unwrap_or(creds);
    creds.username = Some(user.username.clone());
    auth::save_credentials(&creds)?;

    eprintln!("{} Logged in as {}", "✓".green(), user.username.bold());
    if let Ok(path) = auth::credentials_path() {
        eprintln!("  Credentials saved to {}", path.display());
    }
    Ok(())
}

pub async fn auth_status(format: Format) -> anyhow::Result<()> {
    // auth_status는 email 표시를 위해 currentUser API 호출 유지
    let creds = require_auth()?;
    let mut client = VelogClient::new(creds.clone())?;
    let (user, new_creds) = client.current_user().await?;
    // username 캐싱 (refresh 된 credentials 또는 기존 credentials에 username 저장)
    let mut save_creds = new_creds.unwrap_or(creds);
    if save_creds.username.is_none() {
        save_creds.username = Some(user.username.clone());
        auth::save_credentials(&save_creds)?;
    }

    match format {
        Format::Pretty => {
            eprintln!("{} Logged in as {}", "✓".green(), user.username.bold());
            if let Some(email) = &user.email {
                eprintln!("  Email: {}", email);
            }
        }
        Format::Compact | Format::Silent => {
            let status = CompactAuthStatus {
                logged_in: true,
                username: user.username,
            };
            output::emit_data(format, &status);
        }
    }
    Ok(())
}

pub fn auth_logout(format: Format) -> anyhow::Result<()> {
    auth::delete_credentials()?;
    match format {
        Format::Pretty => {
            eprintln!("{} Logged out.", "✓".green());
        }
        Format::Compact | Format::Silent => {
            output::emit_ok(format, "Logged out");
        }
    }
    Ok(())
}
