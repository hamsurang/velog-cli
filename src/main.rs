use clap::{CommandFactory, Parser};
use velog_cli::auth;
use velog_cli::cli::{AuthCommands, Cli, CommentCommands, Commands, Format, PostCommands, SeriesCommands, TagCommands};
use velog_cli::handlers;
use velog_cli::output;

// NOTE: `colored` 크레이트는 NO_COLOR, CLICOLOR 환경변수를 자동 인식.
// 모든 사용자 메시지는 eprintln! (stderr), 데이터 출력만 println! (stdout).

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = Cli::parse();
    let format = cli.format;

    let result = match cli.command {
        Commands::Completions { shell } => {
            // Completions are excluded from --format flag
            clap_complete::generate(shell, &mut Cli::command(), "velog", &mut std::io::stdout());
            return;
        }
        Commands::Auth { command } => match command {
            AuthCommands::Login => handlers::auth_login(format).await,
            AuthCommands::Status => handlers::auth_status(format).await,
            AuthCommands::Logout => handlers::auth_logout(format),
        },
        Commands::Search {
            keyword,
            username,
            limit,
            offset,
        } => handlers::search(&keyword, username.as_deref(), limit, offset, format).await,
        Commands::Tags { command } => match command {
            TagCommands::List {
                sort,
                username,
                limit,
                cursor,
            } => {
                handlers::tags_list(&sort, username.as_deref(), limit, cursor.as_deref(), format)
                    .await
            }
            TagCommands::Posts {
                tag,
                username,
                limit,
                cursor,
            } => {
                handlers::post_list_by_tag(&tag, username.as_deref(), limit, cursor.as_deref(), format)
                    .await
            }
        },
        Commands::Comment { command } => match command {
            CommentCommands::List {
                post_slug,
                username,
                limit,
            } => handlers::comment_list(&post_slug, username.as_deref(), limit, format).await,
            CommentCommands::Write {
                post_slug,
                text,
                file,
            } => handlers::comment_write(&post_slug, text.as_deref(), file.as_deref(), format).await,
            CommentCommands::Reply {
                post_slug,
                number,
                text,
                file,
            } => {
                handlers::comment_reply(&post_slug, &number, text.as_deref(), file.as_deref(), format)
                    .await
            }
            CommentCommands::Edit {
                post_slug,
                number,
                text,
                file,
            } => {
                handlers::comment_edit(&post_slug, &number, text.as_deref(), file.as_deref(), format)
                    .await
            }
            CommentCommands::Delete {
                post_slug,
                number,
                yes,
            } => handlers::comment_delete(&post_slug, &number, yes, format).await,
        },
        Commands::Stats { slug } => handlers::stats(&slug, format).await,
        Commands::Series { command } => match command {
            SeriesCommands::List { username } => {
                handlers::series_list(username.as_deref(), format).await
            }
            SeriesCommands::Show { slug, username } => {
                handlers::series_show(&slug, username.as_deref(), format).await
            }
            SeriesCommands::Create { name, slug } => {
                handlers::series_create(&name, slug.as_deref(), format).await
            }
            SeriesCommands::Edit { slug, name, order } => {
                handlers::series_edit(&slug, name.as_deref(), order.as_deref(), format).await
            }
            SeriesCommands::Delete { slug, yes } => {
                handlers::series_delete(&slug, yes, format).await
            }
        },
        Commands::Post { command } => match command {
            PostCommands::List {
                drafts,
                trending,
                recent,
                username,
                tag,
                limit,
                period,
                cursor,
                offset,
            } => {
                if let Some(t) = tag.as_deref() {
                    handlers::post_list_by_tag(t, username.as_deref(), limit, cursor.as_deref(), format)
                        .await
                } else {
                    handlers::post_list(
                        drafts,
                        trending,
                        recent,
                        username.as_deref(),
                        limit,
                        period,
                        cursor.as_deref(),
                        offset,
                        format,
                    )
                    .await
                }
            }
            PostCommands::Show { slug, username } => {
                handlers::post_show(&slug, username.as_deref(), format).await
            }
            PostCommands::Create {
                file,
                title,
                tags,
                slug,
                publish,
                private,
            } => {
                handlers::post_create(
                    file.as_deref(),
                    &title,
                    &tags,
                    slug.as_deref(),
                    publish,
                    private,
                    format,
                )
                .await
            }
            PostCommands::Edit {
                slug,
                file,
                title,
                tags,
            } => {
                handlers::post_edit(
                    &slug,
                    file.as_deref(),
                    title.as_deref(),
                    tags.as_deref(),
                    format,
                )
                .await
            }
            PostCommands::Delete { slug, yes } => handlers::post_delete(&slug, yes, format).await,
            PostCommands::Publish { slug } => handlers::post_publish(&slug, format).await,
            PostCommands::Like { slug, username } => {
                handlers::post_like(&slug, username.as_deref(), format).await
            }
            PostCommands::Unlike { slug, username } => {
                handlers::post_unlike(&slug, username.as_deref(), format).await
            }
        },
        Commands::Follow { username } => handlers::follow(&username, format).await,
        Commands::Unfollow { username } => handlers::unfollow(&username, format).await,
        Commands::ReadingList {
            list_type,
            limit,
            cursor,
        } => {
            handlers::reading_list(&list_type.to_string(), limit, cursor.as_deref(), format).await
        }
    };

    if let Err(e) = result {
        let code = exit_code(&e);
        match format {
            Format::Pretty => {
                eprintln!("{}: {:#}", colored::Colorize::red("error"), e);
            }
            Format::Compact | Format::Silent => {
                output::emit_error(format, &format!("{:#}", e), code);
            }
        }
        std::process::exit(code);
    }
}

/// .context() 래핑 후에도 AuthError를 찾으려면 chain() 순회 필요
fn exit_code(err: &anyhow::Error) -> i32 {
    for cause in err.chain() {
        if cause.downcast_ref::<auth::AuthError>().is_some() {
            return 2;
        }
    }
    1
}
