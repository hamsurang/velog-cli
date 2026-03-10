use clap::{CommandFactory, Parser};
use velog_cli::auth;
use velog_cli::cli::{AuthCommands, Cli, Commands, PostCommands};
use velog_cli::handlers;

// NOTE: `colored` 크레이트는 NO_COLOR, CLICOLOR 환경변수를 자동 인식.
// 모든 사용자 메시지는 eprintln! (stderr), 데이터 출력만 println! (stdout).

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "velog", &mut std::io::stdout());
            return;
        }
        Commands::Auth { command } => match command {
            AuthCommands::Login => handlers::auth_login().await,
            AuthCommands::Status => handlers::auth_status().await,
            AuthCommands::Logout => handlers::auth_logout(),
        },
        Commands::Post { command } => match command {
            PostCommands::List { drafts } => handlers::post_list(drafts).await,
            PostCommands::Show { slug, username } => {
                handlers::post_show(&slug, username.as_deref()).await
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
                )
                .await
            }
            PostCommands::Edit {
                slug,
                file,
                title,
                tags,
            } => {
                handlers::post_edit(&slug, file.as_deref(), title.as_deref(), tags.as_deref()).await
            }
            PostCommands::Delete { slug, yes } => handlers::post_delete(&slug, yes).await,
            PostCommands::Publish { slug } => handlers::post_publish(&slug).await,
        },
    };

    if let Err(e) = result {
        eprintln!("{}: {:#}", colored::Colorize::red("error"), e);
        let code = exit_code(&e);
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
