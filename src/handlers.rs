use std::io::Read as _;
use std::path::Path;

use anyhow::Context;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};

use crate::auth::{self, AuthError, Credentials};
use crate::client::VelogClient;
use crate::models::{EditPostInput, Post, User, WritePostInput};

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

/// 인증 + 클라이언트 생성 + 현재 유저 조회를 한번에 수행
async fn with_auth_client() -> anyhow::Result<(VelogClient, User)> {
    let creds = require_auth()?;
    let mut client = VelogClient::new(creds)?;
    let (user, new_creds) = client.current_user().await?;
    maybe_save_creds(new_creds)?;
    Ok((client, user))
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

pub async fn auth_login() -> anyhow::Result<()> {
    eprintln!("Paste your velog tokens (hidden input).");
    eprintln!("Find them in browser DevTools → Application → Cookies → velog.io");

    eprint!("access_token: ");
    let access_token = rpassword::read_password().context("Failed to read access_token")?;
    anyhow::ensure!(
        !access_token.trim().is_empty(),
        "access_token cannot be empty"
    );
    auth::validate_velog_jwt(access_token.trim(), "access_token")?;

    eprint!("refresh_token: ");
    let refresh_token = rpassword::read_password().context("Failed to read refresh_token")?;
    anyhow::ensure!(
        !refresh_token.trim().is_empty(),
        "refresh_token cannot be empty"
    );
    auth::validate_velog_jwt(refresh_token.trim(), "refresh_token")?;

    let creds = Credentials {
        access_token: access_token.trim().to_string(),
        refresh_token: refresh_token.trim().to_string(),
    };

    // 실제 API 호출로 토큰 유효성 최종 확인
    let mut client = VelogClient::new(creds.clone())?;
    let (user, new_creds) = client
        .current_user()
        .await
        .context("Token validation failed. The tokens may be expired.")?;
    let creds = new_creds.unwrap_or(creds);
    auth::save_credentials(&creds)?;

    eprintln!("{} Logged in as {}", "✓".green(), user.username.bold());
    Ok(())
}

pub async fn auth_status() -> anyhow::Result<()> {
    let (_client, user) = with_auth_client().await?;

    eprintln!("{} Logged in as {}", "✓".green(), user.username.bold());
    if let Some(email) = &user.email {
        eprintln!("  Email: {}", email);
    }
    Ok(())
}

pub fn auth_logout() -> anyhow::Result<()> {
    auth::delete_credentials()?;
    eprintln!("{} Logged out.", "✓".green());
    Ok(())
}

// ---- Post handlers ----

pub async fn post_list(drafts: bool) -> anyhow::Result<()> {
    let (mut client, user) = with_auth_client().await?;
    let (posts, new_creds) = client.get_posts(&user.username, drafts).await?;
    maybe_save_creds(new_creds)?;

    if posts.is_empty() {
        eprintln!("{}", "No posts found.".yellow());
        return Ok(());
    }
    print_posts_table(&posts);
    Ok(())
}

pub async fn post_show(slug: &str, username: Option<&str>) -> anyhow::Result<()> {
    let (post, new_creds) = if let Some(uname) = username {
        let mut client = match auth::load_credentials()? {
            Some(c) => VelogClient::new(c)?,
            None => VelogClient::anonymous()?,
        };
        client.get_post(uname, slug).await?
    } else {
        let (mut client, user) = with_auth_client().await?;
        client.get_post(&user.username, slug).await?
    };
    maybe_save_creds(new_creds)?;
    print_post_detail(&post);
    Ok(())
}

pub async fn post_create(
    file: Option<&Path>,
    title: &str,
    tags: &str,
    slug_override: Option<&str>,
    publish: bool,
    private: bool,
) -> anyhow::Result<()> {
    let (mut client, user) = with_auth_client().await?;

    let body = read_body(file)?;
    anyhow::ensure!(!body.trim().is_empty(), "Post body is empty");

    // slug 생성
    let url_slug = match slug_override {
        Some(s) => {
            anyhow::ensure!(!s.is_empty(), "Slug cannot be empty");
            anyhow::ensure!(s.len() <= 255, "Slug too long (max 255 chars)");
            anyhow::ensure!(
                s.bytes().all(|b| b.is_ascii_lowercase()
                    || b.is_ascii_digit()
                    || b == b'-')
                    && !s.starts_with('-')
                    && !s.ends_with('-')
                    && !s.contains("--"),
                "Invalid slug: only lowercase alphanumeric and hyphens allowed (e.g. 'my-first-post')"
            );
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
    let tag_list: Vec<String> = tags
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

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

    let status = if publish {
        "Published"
    } else {
        "Saved as draft"
    };
    eprintln!("{}: {}", status.green(), title);
    println!("https://velog.io/@{}/{}", user.username, url_slug);
    Ok(())
}

pub async fn post_edit(
    slug: &str,
    file: Option<&Path>,
    title: Option<&str>,
    tags: Option<&str>,
) -> anyhow::Result<()> {
    let (mut client, user) = with_auth_client().await?;
    let (existing, new_creds) = client.get_post(&user.username, slug).await?;
    maybe_save_creds(new_creds)?;

    let new_body = match file {
        Some(p) => read_body(Some(p))?,
        None => existing.body.unwrap_or_default(),
    };
    let new_title = title.map(String::from).unwrap_or(existing.title);
    let new_tags = match tags {
        Some(t) => t
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        None => existing.tags.unwrap_or_default(),
    };

    let input = EditPostInput {
        id: existing.id,
        title: new_title,
        body: new_body,
        tags: new_tags,
        is_markdown: true,
        is_temp: existing.is_temp,
        is_private: existing.is_private,
        url_slug: existing.url_slug.clone(),
        thumbnail: existing.thumbnail,
        meta: existing.meta.unwrap_or_else(|| serde_json::json!({})),
        series_id: existing.series_id,
    };
    let (_post, new_creds) = client.edit_post(input).await?;
    maybe_save_creds(new_creds)?;

    eprintln!("{}", "Post updated.".green());
    println!("https://velog.io/@{}/{}", user.username, existing.url_slug);
    Ok(())
}

pub async fn post_delete(slug: &str, yes: bool) -> anyhow::Result<()> {
    let (mut client, user) = with_auth_client().await?;
    let (post, new_creds) = client.get_post(&user.username, slug).await?;
    maybe_save_creds(new_creds)?;

    if !yes {
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
    eprintln!("{}", "Post deleted.".green());
    Ok(())
}

pub async fn post_publish(slug: &str) -> anyhow::Result<()> {
    let (mut client, user) = with_auth_client().await?;
    let (existing, new_creds) = client.get_post(&user.username, slug).await?;
    maybe_save_creds(new_creds)?;

    if !existing.is_temp {
        eprintln!("{}", "Post is already published.".yellow());
        return Ok(());
    }

    let input = EditPostInput {
        id: existing.id,
        title: existing.title,
        body: existing.body.unwrap_or_default(),
        tags: existing.tags.unwrap_or_default(),
        is_markdown: true,
        is_temp: false,
        is_private: existing.is_private,
        url_slug: existing.url_slug.clone(),
        thumbnail: existing.thumbnail,
        meta: existing.meta.unwrap_or_else(|| serde_json::json!({})),
        series_id: existing.series_id,
    };
    let (_post, new_creds) = client.edit_post(input).await?;
    maybe_save_creds(new_creds)?;

    eprintln!("{} Post published.", "✓".green());
    println!("https://velog.io/@{}/{}", user.username, existing.url_slug);
    Ok(())
}

// ---- Display functions ----

fn print_posts_table(posts: &[Post]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Title", "Status", "Tags", "Date"]);

    for post in posts {
        let status = if post.is_temp {
            "draft".yellow().to_string()
        } else if post.is_private {
            "private".red().to_string()
        } else {
            "published".green().to_string()
        };

        let tags = post.tags.as_ref().map(|t| t.join(", ")).unwrap_or_default();

        let date = post
            .released_at
            .as_deref()
            .or(post.updated_at.as_deref())
            .unwrap_or("-")
            .chars()
            .take(10)
            .collect::<String>();

        table.add_row(vec![&post.title, &status, &tags, &date]);
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
