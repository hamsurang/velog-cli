use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};

use crate::cli::Format;
use crate::models::{CompactSeries, CompactSeriesDetail};
use crate::output;

use super::{
    confirm_destructive, maybe_save_creds, resolve_client, validate_username, with_auth_client,
};

// ---- Series handlers ----

/// 시리즈 목록 조회
pub async fn series_list(username: Option<&str>, format: Format) -> anyhow::Result<()> {
    if let Some(u) = username {
        validate_username(u)?;
    }

    let (client, resolved_username) = resolve_client(username).await?;
    let series_list = client.get_series_list(&resolved_username).await?;

    match format {
        Format::Pretty => {
            if series_list.is_empty() {
                eprintln!("{}", "No series found.".yellow());
                return Ok(());
            }
            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL)
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(vec!["Name", "Slug", "Posts"]);
            for s in &series_list {
                let name = s.name.as_deref().unwrap_or("-");
                let slug = s.url_slug.as_deref().unwrap_or("-");
                let count = s
                    .series_posts
                    .as_ref()
                    .map(|p| p.len().to_string())
                    .unwrap_or_else(|| "0".to_string());
                table.add_row(vec![name, slug, &count]);
            }
            println!("{table}");
        }
        Format::Compact | Format::Silent => {
            let compact: Vec<CompactSeries> = series_list.iter().map(CompactSeries::from).collect();
            output::emit_data(format, &compact);
        }
    }
    Ok(())
}

/// 시리즈 상세 조회
pub async fn series_show(slug: &str, username: Option<&str>, format: Format) -> anyhow::Result<()> {
    if let Some(u) = username {
        validate_username(u)?;
    }

    let (client, resolved_username) = resolve_client(username).await?;
    let series = client.get_series(&resolved_username, slug).await?;

    match format {
        Format::Pretty => {
            let name = series.name.as_deref().unwrap_or("-");
            let desc = series.description.as_deref().unwrap_or("");
            println!("{}", name.bold());
            if !desc.is_empty() {
                println!("{}", desc.dimmed());
            }
            println!();

            if let Some(posts) = &series.series_posts {
                if posts.is_empty() {
                    eprintln!("{}", "No posts in this series.".yellow());
                } else {
                    let mut table = Table::new();
                    table
                        .load_preset(UTF8_FULL)
                        .set_content_arrangement(ContentArrangement::Dynamic)
                        .set_header(vec!["#", "Title", "Slug"]);
                    for sp in posts {
                        let idx = sp.index.map(|i| i.to_string()).unwrap_or_default();
                        let (title, post_slug) = sp
                            .post
                            .as_ref()
                            .map(|p| (p.title.as_str(), p.url_slug.as_str()))
                            .unwrap_or(("-", "-"));
                        table.add_row(vec![&idx, title, post_slug]);
                    }
                    println!("{table}");
                }
            }
        }
        Format::Compact | Format::Silent => {
            let detail = CompactSeriesDetail::from(&series);
            output::emit_data(format, &detail);
        }
    }
    Ok(())
}

/// 시리즈 생성
pub async fn series_create(name: &str, slug: Option<&str>, format: Format) -> anyhow::Result<()> {
    validate_series_name(name)?;

    let (mut client, _username) = with_auth_client().await?;
    let url_slug = slug
        .map(String::from)
        .unwrap_or_else(|| generate_slug(name));

    let (series, creds) = client.create_series(name, &url_slug).await?;
    maybe_save_creds(creds)?;

    let created_slug = series.url_slug.as_deref().unwrap_or(&url_slug);
    match format {
        Format::Pretty => {
            eprintln!("{} Series '{}' created.", "✓".green(), name);
            eprintln!("Slug: {}", created_slug);
        }
        Format::Compact | Format::Silent => {
            let compact = CompactSeries::from(&series);
            output::emit_data(format, &compact);
        }
    }
    Ok(())
}

/// 시리즈 수정
pub async fn series_edit(
    slug: &str,
    name: Option<&str>,
    order: Option<&str>,
    format: Format,
) -> anyhow::Result<()> {
    if name.is_none() && order.is_none() {
        anyhow::bail!("Nothing to edit. Provide --name or --order.");
    }
    if let Some(n) = name {
        validate_series_name(n)?;
    }

    let (mut client, username) = with_auth_client().await?;

    // 기존 시리즈를 조회하여 ID와 현재 이름 획득
    let existing = client.get_series(&username, slug).await?;
    let series_id = existing
        .id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Series has no ID"))?;
    let current_name = existing.name.as_deref().unwrap_or("");

    // --order: slug 목록 → series_post ID 목록으로 변환
    let series_order = if let Some(order_str) = order {
        let slugs: Vec<&str> = order_str.split(',').map(|s| s.trim()).collect();
        let posts = existing.series_posts.as_deref().unwrap_or(&[]);

        let mut ids = Vec::with_capacity(slugs.len());
        for s in &slugs {
            let found = posts
                .iter()
                .find(|sp| sp.post.as_ref().is_some_and(|p| p.url_slug == *s));
            match found {
                Some(sp) => {
                    let sp_id = sp
                        .id
                        .as_deref()
                        .ok_or_else(|| anyhow::anyhow!("SeriesPost for '{}' has no ID", s))?;
                    ids.push(sp_id.to_string());
                }
                None => {
                    anyhow::bail!(
                        "Post '{}' not found in series '{}'. Available: {}",
                        s,
                        slug,
                        posts
                            .iter()
                            .filter_map(|sp| sp.post.as_ref().map(|p| p.url_slug.as_str()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
            }
        }
        Some(ids)
    } else {
        None
    };

    let final_name = name.unwrap_or(current_name);
    let (series, creds) = client
        .edit_series(series_id, final_name, series_order)
        .await?;
    maybe_save_creds(creds)?;

    match format {
        Format::Pretty => {
            eprintln!(
                "{} Series '{}' updated.",
                "✓".green(),
                series.name.as_deref().unwrap_or(slug)
            );
        }
        Format::Compact | Format::Silent => {
            let compact = CompactSeries::from(&series);
            output::emit_data(format, &compact);
        }
    }
    Ok(())
}

/// 시리즈 삭제
pub async fn series_delete(slug: &str, yes: bool, format: Format) -> anyhow::Result<()> {
    let (mut client, username) = with_auth_client().await?;

    let series = client.get_series(&username, slug).await?;
    let series_id = series
        .id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Series has no ID"))?;
    let name = series.name.as_deref().unwrap_or(slug);
    let post_count = series.series_posts.as_ref().map(|p| p.len()).unwrap_or(0);

    let msg = format!(
        "Delete series '{}' (contains {} posts)? Posts will NOT be deleted.",
        name, post_count
    );
    if !confirm_destructive(&msg, yes)? {
        eprintln!("Aborted.");
        return Ok(());
    }

    let (_ok, creds) = client.remove_series(series_id).await?;
    maybe_save_creds(creds)?;

    match format {
        Format::Pretty => {
            eprintln!("{} Series '{}' deleted.", "✓".green(), name);
        }
        Format::Compact | Format::Silent => {
            let msg = crate::models::CompactMessage {
                status: "ok".to_string(),
                msg: format!("Series '{}' deleted", name),
            };
            output::emit_data(format, &msg);
        }
    }
    Ok(())
}

// ---- Helpers ----

fn validate_series_name(name: &str) -> anyhow::Result<()> {
    anyhow::ensure!(!name.trim().is_empty(), "Series name cannot be empty");
    anyhow::ensure!(name.len() <= 255, "Series name too long (max 255 chars)");
    Ok(())
}

/// 이름에서 URL slug 생성 (간단한 변환)
fn generate_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_series_name_valid() {
        assert!(validate_series_name("Rust 입문").is_ok());
        assert!(validate_series_name("A").is_ok());
    }

    #[test]
    fn validate_series_name_empty() {
        assert!(validate_series_name("").is_err());
        assert!(validate_series_name("   ").is_err());
    }

    #[test]
    fn validate_series_name_too_long() {
        let long = "a".repeat(256);
        assert!(validate_series_name(&long).is_err());
    }

    #[test]
    fn validate_series_name_max_length() {
        let max = "a".repeat(255);
        assert!(validate_series_name(&max).is_ok());
    }

    #[test]
    fn generate_slug_basic() {
        assert_eq!(generate_slug("Hello World"), "hello-world");
    }

    #[test]
    fn generate_slug_korean() {
        // 한글은 모두 하이픈으로 변환되어 빈 결과가 될 수 있음
        let slug = generate_slug("Rust 입문 시리즈");
        assert!(slug.starts_with("rust"));
    }

    #[test]
    fn generate_slug_special_chars() {
        assert_eq!(generate_slug("C++ & Rust!"), "c-rust");
    }

    #[test]
    fn confirm_destructive_yes_flag() {
        assert!(confirm_destructive("test?", true).unwrap());
    }
}
