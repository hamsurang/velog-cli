use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};

use crate::cli::Format;
use crate::client::VelogClient;
use crate::models::CompactTag;
use crate::output;

use super::{validate_cursor, validate_username};
use super::post::{emit_public_posts, PagingHint};

// ---- Tag handlers ----

pub async fn tags_list(
    sort: &str,
    username: Option<&str>,
    limit: u32,
    cursor: Option<&str>,
    format: Format,
) -> anyhow::Result<()> {
    if let Some(u) = username {
        validate_username(u)?;
    }
    if let Some(c) = cursor {
        validate_cursor(c)?;
    }

    let client = VelogClient::anonymous()?;

    // username 지정 시 userTags 사용, 아니면 글로벌 tags
    if let Some(uname) = username {
        let user_tags = client.get_user_tags(uname).await?;
        match format {
            Format::Pretty => {
                if user_tags.is_empty() {
                    eprintln!("{}", "No tags found.".yellow());
                    return Ok(());
                }
                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL)
                    .set_content_arrangement(ContentArrangement::Dynamic)
                    .set_header(vec!["Tag", "Posts"]);
                for ut in &user_tags {
                    let name = ut.tag.name.as_deref().unwrap_or("-");
                    table.add_row(vec![name, &ut.posts_count.to_string()]);
                }
                println!("{table}");
            }
            Format::Compact | Format::Silent => {
                let compact: Vec<CompactTag> =
                    user_tags.iter().map(CompactTag::from).collect();
                output::emit_data(format, &compact);
            }
        }
    } else {
        let tags = client.get_tags(sort, limit, cursor).await?;
        match format {
            Format::Pretty => {
                if tags.is_empty() {
                    eprintln!("{}", "No tags found.".yellow());
                    return Ok(());
                }
                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL)
                    .set_content_arrangement(ContentArrangement::Dynamic)
                    .set_header(vec!["Tag", "Posts"]);
                for tag in &tags {
                    let name = tag.name.as_deref().unwrap_or("-");
                    let count = tag.posts_count.map(|c| c.to_string()).unwrap_or_else(|| "-".to_string());
                    table.add_row(vec![name, &count]);
                }
                println!("{table}");
                if let Some(last) = tags.last() {
                    if let Some(id) = &last.id {
                        eprintln!("Next page: --cursor {}", id);
                    }
                }
            }
            Format::Compact | Format::Silent => {
                let compact: Vec<CompactTag> = tags.iter().map(CompactTag::from).collect();
                let next_cursor = tags.last().and_then(|t| t.id.as_deref());
                let output_val = serde_json::json!({
                    "tags": compact,
                    "next_cursor": next_cursor,
                });
                output::emit_data(format, &output_val);
            }
        }
    }
    Ok(())
}

/// 태그별 포스트 목록
pub async fn post_list_by_tag(
    tag: &str,
    username: Option<&str>,
    limit: u32,
    cursor: Option<&str>,
    format: Format,
) -> anyhow::Result<()> {
    validate_tag_name(tag)?;
    if let Some(u) = username {
        validate_username(u)?;
    }
    if let Some(c) = cursor {
        validate_cursor(c)?;
    }

    let client = VelogClient::anonymous()?;
    let posts = client.get_posts_by_tag(tag, username, limit, cursor).await?;
    emit_public_posts(&posts, format, PagingHint::Cursor)
}

fn validate_tag_name(tag: &str) -> anyhow::Result<()> {
    anyhow::ensure!(!tag.trim().is_empty(), "Tag name cannot be empty");
    anyhow::ensure!(tag.len() <= 100, "Tag name too long (max 100 chars)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_tag_name_valid() {
        assert!(validate_tag_name("rust").is_ok());
        assert!(validate_tag_name("한글태그").is_ok());
        assert!(validate_tag_name("C++").is_ok());
    }

    #[test]
    fn validate_tag_name_empty() {
        assert!(validate_tag_name("").is_err());
        assert!(validate_tag_name("   ").is_err());
    }

    #[test]
    fn validate_tag_name_too_long() {
        let long = "a".repeat(101);
        assert!(validate_tag_name(&long).is_err());
    }
}
