use std::path::Path;

use colored::Colorize;

use crate::cli::Format;
use crate::client::VelogClient;
use crate::models::{assign_comment_numbers, CompactComment, NumberedComment};
use crate::output;

use super::{maybe_save_creds, validate_username, with_auth_client};
use super::series::confirm_destructive;

// ---- Comment handlers ----

/// 댓글 목록 조회
pub async fn comment_list(
    post_slug: &str,
    username: Option<&str>,
    limit: u32,
    format: Format,
) -> anyhow::Result<()> {
    if let Some(u) = username {
        validate_username(u)?;
    }

    let (client, resolved_username) = resolve_comment_client(username).await?;

    // slug → post ID (anonymous 조회)
    let post = client.get_post_anonymous(&resolved_username, post_slug).await?;
    let post_id = &post.id;

    let comments = client.get_comments(post_id).await?;
    let numbered = assign_comment_numbers(&comments);

    let display: Vec<&NumberedComment> = if limit > 0 && (limit as usize) < numbered.len() {
        numbered.iter().take(limit as usize).collect()
    } else {
        numbered.iter().collect()
    };

    match format {
        Format::Pretty => {
            if display.is_empty() {
                eprintln!("{}", "No comments.".yellow());
                return Ok(());
            }
            print_comment_tree(&display);
            if (limit as usize) < numbered.len() {
                eprintln!(
                    "\nShowing {} of {} comments. Use --limit to see more.",
                    display.len(),
                    numbered.len()
                );
            }
        }
        Format::Compact | Format::Silent => {
            let compact: Vec<CompactComment> = display.iter().map(|nc| nc.to_compact()).collect();
            let output_val = serde_json::json!({
                "comments": compact,
                "total": numbered.len(),
            });
            output::emit_data(format, &output_val);
        }
    }
    Ok(())
}

/// 댓글 작성
pub async fn comment_write(
    post_slug: &str,
    text: Option<&str>,
    file: Option<&Path>,
    format: Format,
) -> anyhow::Result<()> {
    let content = read_comment_text(text, file)?;
    validate_comment_text(&content)?;

    let (mut client, username) = with_auth_client().await?;
    let (post, creds1) = client.get_post(&username, post_slug).await?;
    maybe_save_creds(creds1)?;

    let (comment, creds2) = client.write_comment(&post.id, &content, None).await?;
    maybe_save_creds(creds2)?;

    match format {
        Format::Pretty => {
            eprintln!(
                "{} Comment posted on '{}'.",
                "✓".green(),
                post_slug
            );
        }
        Format::Compact | Format::Silent => {
            let compact = serde_json::json!({
                "id": comment.id,
                "text": comment.text,
                "post_slug": post_slug,
            });
            output::emit_data(format, &compact);
        }
    }
    Ok(())
}

/// 댓글에 답글
pub async fn comment_reply(
    post_slug: &str,
    number: &str,
    text: Option<&str>,
    file: Option<&Path>,
    format: Format,
) -> anyhow::Result<()> {
    let content = read_comment_text(text, file)?;
    validate_comment_text(&content)?;

    let (mut client, username) = with_auth_client().await?;
    let (post_id, comment_id) = resolve_comment_id(&username, post_slug, number).await?;

    let (comment, creds) = client
        .write_comment(&post_id, &content, Some(&comment_id))
        .await?;
    maybe_save_creds(creds)?;

    match format {
        Format::Pretty => {
            eprintln!(
                "{} Reply posted to #{} on '{}'.",
                "✓".green(),
                number,
                post_slug
            );
        }
        Format::Compact | Format::Silent => {
            let compact = serde_json::json!({
                "id": comment.id,
                "text": comment.text,
                "reply_to": number,
                "post_slug": post_slug,
            });
            output::emit_data(format, &compact);
        }
    }
    Ok(())
}

/// 댓글 수정
pub async fn comment_edit(
    post_slug: &str,
    number: &str,
    text: Option<&str>,
    file: Option<&Path>,
    format: Format,
) -> anyhow::Result<()> {
    let content = read_comment_text(text, file)?;
    validate_comment_text(&content)?;

    let (mut client, username) = with_auth_client().await?;
    let (_post_id, comment_id) = resolve_comment_id(&username, post_slug, number).await?;

    let (_comment, creds) = client.edit_comment(&comment_id, &content).await?;
    maybe_save_creds(creds)?;

    match format {
        Format::Pretty => {
            eprintln!(
                "{} Comment #{} updated on '{}'.",
                "✓".green(),
                number,
                post_slug
            );
        }
        Format::Compact | Format::Silent => {
            let compact = serde_json::json!({
                "id": comment_id,
                "number": number,
                "status": "updated",
            });
            output::emit_data(format, &compact);
        }
    }
    Ok(())
}

/// 댓글 삭제
pub async fn comment_delete(
    post_slug: &str,
    number: &str,
    yes: bool,
    format: Format,
) -> anyhow::Result<()> {
    let (mut client, username) = with_auth_client().await?;
    let (_post_id, comment_id) = resolve_comment_id(&username, post_slug, number).await?;

    let msg = format!("Delete comment #{} on '{}'?", number, post_slug);
    if !confirm_destructive(&msg, yes)? {
        eprintln!("Aborted.");
        return Ok(());
    }

    let (_ok, creds) = client.remove_comment(&comment_id).await?;
    maybe_save_creds(creds)?;

    match format {
        Format::Pretty => {
            eprintln!("{} Comment #{} deleted.", "✓".green(), number);
        }
        Format::Compact | Format::Silent => {
            let compact = crate::models::CompactMessage {
                status: "ok".to_string(),
                msg: format!("Comment #{} deleted", number),
            };
            output::emit_data(format, &compact);
        }
    }
    Ok(())
}

// ---- Helpers ----

fn print_comment_tree(comments: &[&NumberedComment]) {
    for nc in comments {
        let indent = "  ".repeat(nc.depth);
        let id_display = format!("[{}]", nc.id_short()).dimmed();
        let author = nc.author.bold();
        let date = if nc.date.is_empty() {
            String::new()
        } else {
            format!(" · {}", nc.date)
        };

        if nc.deleted {
            println!(
                "{}#{} {} {} {}",
                indent,
                nc.number.dimmed(),
                id_display,
                "(deleted)".dimmed(),
                date.dimmed()
            );
        } else {
            println!(
                "{}#{} {} {} {}",
                indent, nc.number, id_display, author, date
            );
            // 텍스트를 들여쓰기하여 표시
            for line in nc.text.lines() {
                println!("{}  {}", indent, line);
            }
        }
        println!();
    }
}

/// 댓글 번호(e.g. "1.2") → comment_id 변환
async fn resolve_comment_id(
    username: &str,
    post_slug: &str,
    number: &str,
) -> anyhow::Result<(String, String)> {
    let anon = VelogClient::anonymous()?;
    let post = anon.get_post_anonymous(username, post_slug).await?;
    let post_id = post.id.clone();

    let comments = anon.get_comments(&post_id).await?;
    let numbered = assign_comment_numbers(&comments);

    let found = numbered
        .iter()
        .find(|nc| nc.number == number)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Comment #{} not found on '{}'. Use `velog comment list {}` to see available comments.",
                number,
                post_slug,
                post_slug
            )
        })?;

    Ok((post_id, found.id.clone()))
}

/// username이 있으면 anonymous, 없으면 인증 클라이언트
async fn resolve_comment_client(
    username: Option<&str>,
) -> anyhow::Result<(VelogClient, String)> {
    if let Some(u) = username {
        let client = VelogClient::anonymous()?;
        Ok((client, u.to_string()))
    } else {
        with_auth_client().await
    }
}

fn read_comment_text(text: Option<&str>, file: Option<&Path>) -> anyhow::Result<String> {
    use std::io::{IsTerminal, Read};
    match (text, file) {
        (Some(t), _) => Ok(t.to_string()),
        (None, Some(p)) if p == Path::new("-") => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
        (None, Some(p)) => {
            let meta = std::fs::metadata(p)?;
            anyhow::ensure!(
                meta.len() <= 1_048_576,
                "Comment file too large (max 1MB)"
            );
            std::fs::read_to_string(p)
                .map_err(|e| anyhow::anyhow!("Cannot read file '{}': {}", p.display(), e))
        }
        (None, None) if !std::io::stdin().is_terminal() => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
        (None, None) => {
            anyhow::bail!(
                "No comment text provided. Use positional text, --file, or pipe stdin."
            )
        }
    }
}

fn validate_comment_text(text: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        !text.trim().is_empty(),
        "Comment text cannot be empty"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_comment_text_valid() {
        assert!(validate_comment_text("hello").is_ok());
    }

    #[test]
    fn validate_comment_text_empty() {
        assert!(validate_comment_text("").is_err());
        assert!(validate_comment_text("   ").is_err());
    }

    #[test]
    fn read_comment_text_from_string() {
        let result = read_comment_text(Some("hello"), None).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn read_comment_text_string_overrides_file() {
        let result = read_comment_text(Some("hello"), Some(Path::new("nonexistent.md"))).unwrap();
        assert_eq!(result, "hello");
    }
}
