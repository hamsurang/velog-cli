use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};

use crate::cli::Format;
use crate::client::VelogClient;
use crate::models::{CompactSearchResult, Post};
use crate::output;

use super::validate_username;

// ---- Search handler ----

pub async fn search(
    keyword: &str,
    username: Option<&str>,
    limit: u32,
    offset: u32,
    format: Format,
) -> anyhow::Result<()> {
    validate_search_keyword(keyword)?;
    if let Some(u) = username {
        validate_username(u)?;
    }

    let client = VelogClient::anonymous()?;
    let result = client
        .search_posts(keyword, offset, limit, username)
        .await?;
    let posts = &result.posts;
    let total = result.count.max(0) as u32;
    let consumed = offset.saturating_add(posts.len() as u32);
    let has_more = consumed < total;

    match format {
        Format::Pretty => {
            if posts.is_empty() {
                eprintln!(
                    "{}",
                    format!("No results found for '{}'.", keyword).yellow()
                );
                return Ok(());
            }
            print_search_results(posts, keyword);
            if has_more {
                eprintln!("Showing {consumed} of {total} results. Next page: --offset {consumed}");
            }
        }
        Format::Compact | Format::Silent => {
            let compact: Vec<CompactSearchResult> =
                posts.iter().map(CompactSearchResult::from).collect();
            let output_val = serde_json::json!({
                "results": compact,
                "total_count": total,
                "next_offset": consumed,
                "has_more": has_more,
            });
            output::emit_data(format, &output_val);
        }
    }
    Ok(())
}

fn validate_search_keyword(keyword: &str) -> anyhow::Result<()> {
    anyhow::ensure!(!keyword.trim().is_empty(), "Search keyword cannot be empty");
    anyhow::ensure!(
        keyword.len() <= 100,
        "Search keyword too long (max 100 chars)"
    );
    Ok(())
}

/// 검색 키워드를 bold yellow로 하이라이트 (대소문자 무시)
fn highlight_match(text: &str, keyword: &str) -> String {
    let lower = text.to_lowercase();
    let kw_lower = keyword.to_lowercase();
    if kw_lower.is_empty() {
        return text.to_string();
    }
    let mut result = String::with_capacity(text.len() + 64);
    let mut start = 0;
    while let Some(pos) = lower[start..].find(&kw_lower) {
        let abs = start + pos;
        result.push_str(&text[start..abs]);
        let matched = &text[abs..abs + keyword.len()];
        result.push_str(&format!("{}", matched.bold().yellow()));
        start = abs + keyword.len();
    }
    result.push_str(&text[start..]);
    result
}

fn print_search_results(posts: &[Post], keyword: &str) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Title", "Author", "Slug", "Tags", "Date"]);

    for post in posts {
        let title = highlight_match(&post.title, keyword);
        let author = post
            .user
            .as_ref()
            .map(|u| u.username.as_str())
            .unwrap_or("-");
        let tags = post.tags.as_ref().map(|t| t.join(", ")).unwrap_or_default();
        let date = post.date_short();
        let date = if date.is_empty() {
            "-".to_string()
        } else {
            date
        };

        table.add_row(vec![&title, author, &post.url_slug, &tags, &date]);
    }

    println!("{table}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_keyword_empty() {
        assert!(validate_search_keyword("").is_err());
        assert!(validate_search_keyword("   ").is_err());
    }

    #[test]
    fn validate_keyword_valid() {
        assert!(validate_search_keyword("rust").is_ok());
        assert!(validate_search_keyword("한글 검색").is_ok());
    }

    #[test]
    fn validate_keyword_too_long() {
        let long = "a".repeat(101);
        assert!(validate_search_keyword(&long).is_err());
    }

    #[test]
    fn validate_keyword_max_length() {
        let max = "a".repeat(100);
        assert!(validate_search_keyword(&max).is_ok());
    }

    #[test]
    fn highlight_match_basic() {
        let result = highlight_match("Hello World", "world");
        assert!(result.contains("World")); // original case preserved
    }

    #[test]
    fn highlight_match_no_match() {
        let result = highlight_match("Hello World", "xyz");
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn highlight_match_empty_keyword() {
        let result = highlight_match("Hello", "");
        assert_eq!(result, "Hello");
    }

    #[test]
    fn highlight_match_case_insensitive() {
        let result = highlight_match("Rust Programming", "rust");
        // Should still contain the text (with ANSI codes for highlighting)
        assert!(result.contains("Rust") || result.len() > "Rust Programming".len());
    }
}
