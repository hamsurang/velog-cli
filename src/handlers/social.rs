use colored::Colorize;

use crate::cli::Format;
use crate::output;

use super::post::{emit_public_posts, PagingHint};
use super::{maybe_save_creds, validate_username, with_auth_client};

// ---- Like / Unlike ----

pub async fn post_like(slug: &str, username: Option<&str>, format: Format) -> anyhow::Result<()> {
    super::validate_slug_nonempty(slug)?;
    if let Some(u) = username {
        validate_username(u)?;
    }

    let (mut client, my_username) = with_auth_client().await?;
    let target_username = username.unwrap_or(&my_username);

    let (post, creds1) = client.get_post(target_username, slug).await?;
    maybe_save_creds(creds1)?;

    let (liked_post, creds2) = client.like_post(&post.id).await?;
    maybe_save_creds(creds2)?;

    match format {
        Format::Pretty => {
            eprintln!(
                "{} Liked '{}' ({} likes)",
                "♥".red(),
                slug,
                liked_post.likes
            );
        }
        Format::Compact | Format::Silent => {
            let compact = serde_json::json!({
                "action": "liked",
                "slug": slug,
                "likes": liked_post.likes,
            });
            output::emit_data(format, &compact);
        }
    }
    Ok(())
}

pub async fn post_unlike(slug: &str, username: Option<&str>, format: Format) -> anyhow::Result<()> {
    super::validate_slug_nonempty(slug)?;
    if let Some(u) = username {
        validate_username(u)?;
    }

    let (mut client, my_username) = with_auth_client().await?;
    let target_username = username.unwrap_or(&my_username);

    let (post, creds1) = client.get_post(target_username, slug).await?;
    maybe_save_creds(creds1)?;

    let (unliked_post, creds2) = client.unlike_post(&post.id).await?;
    maybe_save_creds(creds2)?;

    match format {
        Format::Pretty => {
            eprintln!(
                "{} Unliked '{}' ({} likes)",
                "♡".dimmed(),
                slug,
                unliked_post.likes
            );
        }
        Format::Compact | Format::Silent => {
            let compact = serde_json::json!({
                "action": "unliked",
                "slug": slug,
                "likes": unliked_post.likes,
            });
            output::emit_data(format, &compact);
        }
    }
    Ok(())
}

// ---- Follow / Unfollow ----

pub async fn follow(username: &str, format: Format) -> anyhow::Result<()> {
    validate_username(username)?;

    let (mut client, _my_username) = with_auth_client().await?;
    let user = client.get_user(username).await?;

    let (_ok, creds) = client.follow_user(&user.id).await?;
    maybe_save_creds(creds)?;

    match format {
        Format::Pretty => {
            eprintln!("{} Now following @{}", "✓".green(), username);
        }
        Format::Compact | Format::Silent => {
            let compact = serde_json::json!({
                "action": "followed",
                "username": username,
            });
            output::emit_data(format, &compact);
        }
    }
    Ok(())
}

pub async fn unfollow(username: &str, format: Format) -> anyhow::Result<()> {
    validate_username(username)?;

    let (mut client, _my_username) = with_auth_client().await?;
    let user = client.get_user(username).await?;

    let (_ok, creds) = client.unfollow_user(&user.id).await?;
    maybe_save_creds(creds)?;

    match format {
        Format::Pretty => {
            eprintln!("{} Unfollowed @{}", "✓".green(), username);
        }
        Format::Compact | Format::Silent => {
            let compact = serde_json::json!({
                "action": "unfollowed",
                "username": username,
            });
            output::emit_data(format, &compact);
        }
    }
    Ok(())
}

// ---- Reading List ----

pub async fn reading_list(
    list_type: &str,
    limit: u32,
    cursor: Option<&str>,
    format: Format,
) -> anyhow::Result<()> {
    let (mut client, _username) = with_auth_client().await?;

    let (posts, creds) = client.get_reading_list(list_type, limit, cursor).await?;
    maybe_save_creds(creds)?;

    emit_public_posts(&posts, format, PagingHint::Cursor)
}
