use std::path::Path;

use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};

use crate::auth;
use crate::cli::{Format, Period};
use crate::client::VelogClient;
use crate::models::{CompactPost, Post, WritePostInput};
use crate::output;

use super::{
    maybe_save_creds, parse_tags, read_body, validate_cursor, validate_slug, validate_username,
    with_auth_client,
};

// ---- Post handlers ----

#[allow(clippy::too_many_arguments)]
pub async fn post_list(
    drafts: bool,
    trending: bool,
    recent: bool,
    username: Option<&str>,
    limit: u32,
    period: Option<Period>,
    cursor: Option<&str>,
    offset: Option<u32>,
    format: Format,
) -> anyhow::Result<()> {
    // clap conflicts_with handles mutual exclusion — dispatch by mode
    if trending {
        return post_list_trending(limit, offset.unwrap_or(0), period, format).await;
    }
    if recent {
        if let Some(c) = cursor {
            validate_cursor(c)?;
        }
        return post_list_recent(limit, cursor, format).await;
    }
    if let Some(uname) = username {
        validate_username(uname)?;
        if let Some(c) = cursor {
            validate_cursor(c)?;
        }
        return post_list_user(uname, limit, cursor, format).await;
    }
    post_list_mine(drafts, format).await
}

async fn post_list_trending(
    limit: u32,
    offset: u32,
    period: Option<Period>,
    format: Format,
) -> anyhow::Result<()> {
    let client = VelogClient::anonymous()?;
    let timeframe = period.unwrap_or(Period::Week).to_string();
    let posts = client.get_trending_posts(limit, offset, &timeframe).await?;
    let next_offset = offset + posts.len() as u32;
    emit_public_posts(&posts, format, PagingHint::Offset(next_offset))
}

async fn post_list_recent(limit: u32, cursor: Option<&str>, format: Format) -> anyhow::Result<()> {
    let client = VelogClient::anonymous()?;
    let posts = client.get_recent_posts(limit, cursor).await?;
    emit_public_posts(&posts, format, PagingHint::Cursor)
}

async fn post_list_user(
    username: &str,
    limit: u32,
    cursor: Option<&str>,
    format: Format,
) -> anyhow::Result<()> {
    let client = VelogClient::anonymous()?;
    let posts = client.get_user_posts(username, limit, cursor).await?;
    emit_public_posts(&posts, format, PagingHint::Cursor)
}

async fn post_list_mine(drafts: bool, format: Format) -> anyhow::Result<()> {
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

pub(crate) enum PagingHint {
    Cursor,
    Offset(u32),
}

pub(crate) fn emit_public_posts(
    posts: &[Post],
    format: Format,
    paging: PagingHint,
) -> anyhow::Result<()> {
    match format {
        Format::Pretty => {
            if posts.is_empty() {
                eprintln!("{}", "No posts found.".yellow());
                return Ok(());
            }
            print_public_posts_table(posts);
            match paging {
                PagingHint::Cursor => {
                    if let Some(last) = posts.last() {
                        eprintln!("Next page: --cursor {}", last.id);
                    }
                }
                PagingHint::Offset(next) => {
                    eprintln!("Next page: --offset {}", next);
                }
            }
        }
        Format::Compact | Format::Silent => {
            let compact: Vec<CompactPost> = posts.iter().map(CompactPost::from).collect();
            let output_val = match paging {
                PagingHint::Cursor => {
                    let next_cursor = posts.last().map(|p| p.id.as_str());
                    serde_json::json!({ "posts": compact, "next_cursor": next_cursor })
                }
                PagingHint::Offset(next) => {
                    serde_json::json!({ "posts": compact, "next_offset": next })
                }
            };
            output::emit_data(format, &output_val);
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

fn date_or_dash(post: &Post) -> String {
    let d = post.date_short();
    if d.is_empty() {
        "-".to_string()
    } else {
        d
    }
}

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

        let date = date_or_dash(post);

        table.add_row(vec![&post.title, &post.url_slug, &status, &tags, &date]);
    }

    println!("{table}");
}

fn print_public_posts_table(posts: &[Post]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Title", "Author", "Slug", "Likes", "Date"]);

    for post in posts {
        let author = post
            .user
            .as_ref()
            .map(|u| u.username.as_str())
            .unwrap_or("-");
        let likes = post.likes.to_string();
        let date = date_or_dash(post);

        table.add_row(vec![&post.title, author, &post.url_slug, &likes, &date]);
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
