use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};

use crate::cli::Format;
use crate::models::CompactStats;
use crate::output;

use super::{maybe_save_creds, with_auth_client};

// ---- Stats handler ----

pub async fn stats(slug: &str, format: Format) -> anyhow::Result<()> {
    super::validate_slug_nonempty(slug)?;
    let (mut client, username) = with_auth_client().await?;

    // slug → post ID 변환
    let (post, creds1) = client.get_post(&username, slug).await?;
    maybe_save_creds(creds1)?;
    let post_id = &post.id;

    let (stats, creds2) = client.get_stats(post_id).await?;
    maybe_save_creds(creds2)?;

    let today = today_str();
    let yesterday = yesterday_str();
    let compact = CompactStats::from_stats(&stats, &today, &yesterday);

    match format {
        Format::Pretty => {
            println!("{}", format!("Stats for '{}'", slug).bold());
            println!();
            println!("  Total views:     {}", format_number(compact.total).cyan());
            println!(
                "  Today:           {}",
                format_number(compact.today).green()
            );
            println!("  Yesterday:       {}", format_number(compact.yesterday));
            println!();

            if !compact.daily.is_empty() {
                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL)
                    .set_content_arrangement(ContentArrangement::Dynamic)
                    .set_header(vec!["Date", "Views"]);
                for day in &compact.daily {
                    table.add_row(vec![&day.date, &format_number(day.views)]);
                }
                println!("{table}");
            }
        }
        Format::Compact | Format::Silent => {
            output::emit_data(format, &compact);
        }
    }
    Ok(())
}

/// 천 단위 구분자 포맷
fn format_number(n: i32) -> String {
    if n < 0 {
        return format!("-{}", format_number(-n));
    }
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn today_str() -> String {
    date_offset_days(0)
}

fn yesterday_str() -> String {
    date_offset_days(-1)
}

/// 현재 UTC 기준 offset일 후의 YYYY-MM-DD 문자열
fn date_offset_days(offset: i64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        + offset * 86400;
    let days = secs / 86400;
    // Civil date from days since epoch (algorithm from Howard Hinnant)
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_number_basic() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1234567), "1,234,567");
    }

    #[test]
    fn format_number_negative() {
        assert_eq!(format_number(-1234), "-1,234");
    }

    #[test]
    fn today_and_yesterday_format() {
        let today = today_str();
        let yesterday = yesterday_str();
        // YYYY-MM-DD 형식 확인
        assert_eq!(today.len(), 10);
        assert_eq!(yesterday.len(), 10);
        assert!(today.starts_with("20"));
        assert!(yesterday.starts_with("20"));
    }
}
