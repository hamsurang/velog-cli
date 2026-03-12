use std::io::Read as _;
use std::path::Path;

use crate::auth::{self, AuthError, Credentials};
use crate::client::VelogClient;

mod auth_handlers;
mod post;

pub use auth_handlers::*;
pub use post::*;

// ---- Helper functions ----

/// credentials 로드 실패 시 AuthError 반환 (exit code 2)
pub(crate) fn require_auth() -> anyhow::Result<Credentials> {
    auth::load_credentials()?.ok_or_else(|| {
        anyhow::Error::new(AuthError).context("Not logged in. Run `velog auth login` first.")
    })
}

/// Option<Credentials>가 Some이면 디스크에 저장
pub(crate) fn maybe_save_creds(creds: Option<Credentials>) -> anyhow::Result<()> {
    if let Some(c) = creds {
        auth::save_credentials(&c)?;
    }
    Ok(())
}

/// 인증 + 클라이언트 생성 + username 확보 (캐시 우선, 미스 시 API 호출)
pub(crate) async fn with_auth_client() -> anyhow::Result<(VelogClient, String)> {
    let creds = require_auth()?;
    let mut client = VelogClient::new(creds.clone())?;

    let username = if let Some(u) = creds.username {
        u
    } else {
        // 캐시 미스: API 호출 후 username 저장
        let (user, new_creds) = client.current_user().await?;
        let mut save_creds = new_creds.unwrap_or(creds);
        save_creds.username = Some(user.username.clone());
        auth::save_credentials(&save_creds)?;
        user.username
    };

    Ok((client, username))
}

/// 쉼표 구분 태그 문자열 파싱 + 중복 제거
pub(crate) fn parse_tags(tags: &str) -> Vec<String> {
    let mut result = Vec::new();
    for raw in tags.split(',') {
        let t = raw.trim().to_string();
        if !t.is_empty() && !result.contains(&t) {
            result.push(t);
        }
    }
    result
}

/// 사용자 지정 slug 유효성 검증
pub(crate) fn validate_slug(s: &str) -> anyhow::Result<()> {
    anyhow::ensure!(!s.is_empty(), "Slug cannot be empty");
    anyhow::ensure!(s.len() <= 255, "Slug too long (max 255 chars)");
    anyhow::ensure!(
        s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && !s.starts_with('-')
            && !s.ends_with('-')
            && !s.contains("--"),
        "Invalid slug: only lowercase alphanumeric and hyphens allowed (e.g. 'my-first-post')"
    );
    Ok(())
}

pub(crate) fn validate_username(u: &str) -> anyhow::Result<()> {
    anyhow::ensure!(!u.is_empty(), "Username cannot be empty");
    anyhow::ensure!(u.len() <= 64, "Username too long (max 64 chars)");
    anyhow::ensure!(
        u.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "Invalid username: only alphanumeric, hyphens, and underscores allowed"
    );
    Ok(())
}

pub(crate) fn validate_cursor(c: &str) -> anyhow::Result<()> {
    anyhow::ensure!(!c.is_empty(), "Cursor cannot be empty");
    anyhow::ensure!(c.len() <= 128, "Cursor too long (max 128 chars)");
    anyhow::ensure!(c.is_ascii(), "Cursor must be ASCII");
    Ok(())
}

/// 파일 경로 또는 stdin에서 마크다운 본문 읽기
pub(crate) fn read_body(file: Option<&Path>) -> anyhow::Result<String> {
    use anyhow::Context;
    use std::io::IsTerminal;
    match file {
        Some(p) if p == Path::new("-") => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
        Some(p) => {
            std::fs::read_to_string(p).with_context(|| format!("Cannot read file: {}", p.display()))
        }
        None if !std::io::stdin().is_terminal() => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
        None => {
            anyhow::bail!(
                "No content source. Provide --file <path>, --file - for stdin, or pipe content."
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- parse_tags tests ----

    #[test]
    fn parse_tags_basic() {
        assert_eq!(parse_tags("rust,cli,blog"), vec!["rust", "cli", "blog"]);
    }

    #[test]
    fn parse_tags_trims_whitespace() {
        assert_eq!(
            parse_tags(" rust , cli , blog "),
            vec!["rust", "cli", "blog"]
        );
    }

    #[test]
    fn parse_tags_deduplicates() {
        assert_eq!(
            parse_tags("rust,cli,rust,blog,cli"),
            vec!["rust", "cli", "blog"]
        );
    }

    #[test]
    fn parse_tags_empty_string() {
        assert!(parse_tags("").is_empty());
    }

    #[test]
    fn parse_tags_only_commas() {
        assert!(parse_tags(",,,").is_empty());
    }

    #[test]
    fn parse_tags_single() {
        assert_eq!(parse_tags("rust"), vec!["rust"]);
    }

    // ---- validate_slug tests ----

    #[test]
    fn validate_slug_valid() {
        assert!(validate_slug("my-first-post").is_ok());
        assert!(validate_slug("hello123").is_ok());
        assert!(validate_slug("a").is_ok());
    }

    #[test]
    fn validate_slug_empty() {
        assert!(validate_slug("").is_err());
    }

    #[test]
    fn validate_slug_uppercase() {
        assert!(validate_slug("My-Post").is_err());
    }

    #[test]
    fn validate_slug_double_hyphen() {
        assert!(validate_slug("my--post").is_err());
    }

    #[test]
    fn validate_slug_leading_hyphen() {
        assert!(validate_slug("-my-post").is_err());
    }

    #[test]
    fn validate_slug_trailing_hyphen() {
        assert!(validate_slug("my-post-").is_err());
    }

    #[test]
    fn validate_slug_special_chars() {
        assert!(validate_slug("my_post").is_err());
        assert!(validate_slug("my post").is_err());
        assert!(validate_slug("my.post").is_err());
    }
}
