use std::io::Read as _;
use std::path::Path;

use anyhow::Context;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};

use crate::auth::{self, AuthError, Credentials};
use crate::cli::Format;
use crate::client::VelogClient;
use crate::models::{CompactAuthStatus, CompactPost, Post, WritePostInput};
use crate::output;

// ---- Helper functions ----

/// credentials 로드 실패 시 AuthError 반환 (exit code 2)
fn require_auth() -> anyhow::Result<Credentials> {
    auth::load_credentials()?.ok_or_else(|| {
        anyhow::Error::new(AuthError).context("Not logged in. Run `velog auth login` first.")
    })
}

/// Option<Credentials>가 Some이면 디스크에 저장
fn maybe_save_creds(creds: Option<Credentials>) -> anyhow::Result<()> {
    if let Some(c) = creds {
        auth::save_credentials(&c)?;
    }
    Ok(())
}

/// 인증 + 클라이언트 생성 + username 확보 (캐시 우선, 미스 시 API 호출)
async fn with_auth_client() -> anyhow::Result<(VelogClient, String)> {
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
fn parse_tags(tags: &str) -> Vec<String> {
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
fn validate_slug(s: &str) -> anyhow::Result<()> {
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

/// 파일 경로 또는 stdin에서 마크다운 본문 읽기
fn read_body(file: Option<&Path>) -> anyhow::Result<String> {
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

// ---- Post handlers ----

pub async fn post_list(drafts: bool, format: Format) -> anyhow::Result<()> {
    let (mut client, username) = with_auth_client().await?;
    let (posts, new_creds) = client.get_posts(&username, drafts).await?;
    maybe_save_creds(new_creds)?;

    match format {
        Format::Pretty => {
            if posts.is_empty() {
                eprintln!("{}", "No posts found.".yellow());
                return Ok(());
            }
            print_posts_table(&posts);
        }
        Format::Compact | Format::Silent => {
            let compact: Vec<CompactPost> = posts.iter().map(CompactPost::from).collect();
            output::emit_data(format, &compact);
        }
    }
    Ok(())
}

pub async fn post_show(slug: &str, username: Option<&str>, format: Format) -> anyhow::Result<()> {
    let (post, new_creds) = if let Some(uname) = username {
        let mut client = match auth::load_credentials()? {
            Some(c) => VelogClient::new(c)?,
            None => VelogClient::anonymous()?,
        };
        client.get_post(uname, slug).await?
    } else {
        let (mut client, username) = with_auth_client().await?;
        client.get_post(&username, slug).await?
    };
    maybe_save_creds(new_creds)?;

    match format {
        Format::Pretty => {
            print_post_detail(&post);
        }
        Format::Compact | Format::Silent => {
            output::emit_data(format, &CompactPost::detail(&post));
        }
    }
    Ok(())
}

pub async fn post_create(
    file: Option<&Path>,
    title: &str,
    tags: &str,
    slug_override: Option<&str>,
    publish: bool,
    private: bool,
    format: Format,
) -> anyhow::Result<()> {
    let (mut client, username) = with_auth_client().await?;

    let body = read_body(file)?;
    anyhow::ensure!(!body.trim().is_empty(), "Post body is empty");

    // slug 생성
    let url_slug = match slug_override {
        Some(s) => {
            validate_slug(s)?;
            s.to_string()
        }
        None => {
            let s = slug::slugify(title);
            if s.is_empty() {
                format!(
                    "post-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs()
                )
            } else {
                s
            }
        }
    };

    // tags 파싱
    let tag_list = parse_tags(tags);

    let input = WritePostInput {
        title: title.to_string(),
        body,
        tags: tag_list,
        is_markdown: true,
        is_temp: !publish,
        is_private: private,
        url_slug: url_slug.clone(),
        thumbnail: None,
        meta: serde_json::json!({}),
        series_id: None,
    };

    let (_post, new_creds) = client.write_post(input).await?;
    maybe_save_creds(new_creds)?;

    let url = format!("https://velog.io/@{}/{}", username, url_slug);
    let status_msg = if publish {
        "Published"
    } else {
        "Saved as draft"
    };

    match format {
        Format::Pretty => {
            eprintln!("{}: {}", status_msg.green(), title);
            println!("{url}");
        }
        Format::Compact | Format::Silent => {
            output::emit_mutation_result(format, &url);
            output::emit_ok(format, &format!("{}: {}", status_msg, title));
        }
    }
    Ok(())
}

pub async fn post_edit(
    slug: &str,
    file: Option<&Path>,
    title: Option<&str>,
    tags: Option<&str>,
    format: Format,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        file.is_some() || title.is_some() || tags.is_some(),
        "Nothing to edit. Provide --file, --title, or --tags."
    );
    let (mut client, username) = with_auth_client().await?;
    let (existing, new_creds) = client.get_post(&username, slug).await?;
    maybe_save_creds(new_creds)?;

    let mut input = existing.into_edit_input();
    if let Some(p) = file {
        input.body = read_body(Some(p))?;
    }
    if let Some(t) = title {
        input.title = t.to_string();
    }
    if let Some(t) = tags {
        input.tags = parse_tags(t);
    }

    let (post, new_creds) = client.edit_post(input).await?;
    maybe_save_creds(new_creds)?;

    let url = format!("https://velog.io/@{}/{}", username, post.url_slug);
    match format {
        Format::Pretty => {
            eprintln!("{}", "Post updated.".green());
            println!("{url}");
        }
        Format::Compact | Format::Silent => {
            output::emit_mutation_result(format, &url);
            output::emit_ok(format, "Post updated");
        }
    }
    Ok(())
}

pub async fn post_delete(slug: &str, yes: bool, format: Format) -> anyhow::Result<()> {
    let (mut client, username) = with_auth_client().await?;
    let (post, new_creds) = client.get_post(&username, slug).await?;
    maybe_save_creds(new_creds)?;

    // In compact/silent mode, auto-confirm deletion (no interactive prompt)
    let effective_yes = yes || format != Format::Pretty;

    if !effective_yes {
        use std::io::IsTerminal;
        if !std::io::stdin().is_terminal() {
            anyhow::bail!("Refusing to delete in non-interactive mode. Use --yes to confirm.");
        }
        eprint!(
            "Delete post '{}'? This cannot be undone. [y/N] ",
            post.title
        );
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if answer.trim().to_lowercase() != "y" {
            eprintln!("Aborted.");
            return Ok(());
        }
    }

    let (_ok, new_creds) = client.remove_post(&post.id).await?;
    maybe_save_creds(new_creds)?;

    match format {
        Format::Pretty => {
            eprintln!("{}", "Post deleted.".green());
        }
        Format::Compact | Format::Silent => {
            output::emit_ok(format, "Post deleted");
        }
    }
    Ok(())
}

pub async fn post_publish(slug: &str, format: Format) -> anyhow::Result<()> {
    let (mut client, username) = with_auth_client().await?;
    let (existing, new_creds) = client.get_post(&username, slug).await?;
    maybe_save_creds(new_creds)?;

    if !existing.is_temp {
        let url = format!("https://velog.io/@{}/{}", username, existing.url_slug);
        match format {
            Format::Pretty => {
                eprintln!("{}", "Post is already published.".yellow());
            }
            Format::Compact | Format::Silent => {
                output::emit_mutation_result(format, &url);
                output::emit_ok(format, "Already published");
            }
        }
        return Ok(());
    }

    let mut input = existing.into_edit_input();
    input.is_temp = false;

    let (post, new_creds) = client.edit_post(input).await?;
    maybe_save_creds(new_creds)?;

    let url = format!("https://velog.io/@{}/{}", username, post.url_slug);
    match format {
        Format::Pretty => {
            eprintln!("{} Post published.", "✓".green());
            println!("{url}");
        }
        Format::Compact | Format::Silent => {
            output::emit_mutation_result(format, &url);
            output::emit_ok(format, "Post published");
        }
    }
    Ok(())
}

// ---- Display functions ----

fn print_posts_table(posts: &[Post]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Title", "Slug", "Status", "Tags", "Date"]);

    for post in posts {
        let status = if post.is_temp {
            "draft".yellow().to_string()
        } else if post.is_private {
            "private".red().to_string()
        } else {
            "published".green().to_string()
        };

        let tags = post.tags.as_ref().map(|t| t.join(", ")).unwrap_or_default();

        let date = post.date_short();
        let date = if date.is_empty() {
            "-".to_string()
        } else {
            date
        };

        table.add_row(vec![&post.title, &post.url_slug, &status, &tags, &date]);
    }

    println!("{table}");
}

fn print_post_detail(post: &Post) {
    println!("{}", post.title.bold());
    if let Some(tags) = &post.tags {
        if !tags.is_empty() {
            println!("Tags: {}", tags.join(", ").cyan());
        }
    }

    let status = if post.is_temp { "draft" } else { "published" };
    let visibility = if post.is_private { "private" } else { "public" };
    println!("Status: {} | {}", status, visibility);

    if let Some(date) = &post.released_at {
        println!("Published: {}", &date[..10.min(date.len())]);
    }
    println!();

    if let Some(body) = &post.body {
        let skin = termimad::MadSkin::default();
        skin.print_text(body);
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
