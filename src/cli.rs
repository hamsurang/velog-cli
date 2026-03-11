use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use std::path::PathBuf;

// SECURITY: Token values MUST NOT be accepted as CLI arguments or env vars.
// They must only be collected via rpassword's hidden prompt.
// Do NOT add #[arg(env = "...")] to any token-related fields.

/// Output format: pretty (human-friendly, default), compact (JSON), silent (minimal)
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Format {
    /// Machine-readable minified JSON (optimized for AI agents and pipelines)
    Compact,
    /// Human-friendly output with tables, colors, and markdown rendering (default)
    #[default]
    Pretty,
    /// Minimal output: queries emit JSON, mutations emit nothing (exit code only)
    Silent,
}

#[derive(Parser)]
#[command(name = "velog", about = "CLI client for velog.io", version)]
pub struct Cli {
    /// Output format
    #[arg(long, global = true, value_enum, default_value_t = Format::Pretty)]
    pub format: Format,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Authentication commands
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    /// Post management commands
    Post {
        #[command(subcommand)]
        command: PostCommands,
    },
    /// Generate shell completions
    Completions {
        /// Shell type (bash, zsh, fish, elvish, powershell)
        shell: Shell,
    },
}

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Log in by pasting access_token and refresh_token
    Login,
    /// Show current authentication status
    Status,
    /// Log out and remove stored credentials
    Logout,
}

#[derive(Subcommand)]
pub enum PostCommands {
    /// List your posts
    List {
        /// Show draft (temporary) posts instead of published
        #[arg(long)]
        drafts: bool,
    },
    /// Show a specific post by slug
    Show {
        /// Post URL slug (e.g. "my-first-post")
        slug: String,
        /// Username (required if not logged in)
        #[arg(short, long)]
        username: Option<String>,
    },
    /// Create a new post from a markdown file (or stdin)
    Create {
        /// Path to markdown file (use "-" for stdin, omit to read piped stdin)
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Post title
        #[arg(short, long)]
        title: String,
        /// Comma-separated tags
        #[arg(long, default_value = "")]
        tags: String,
        /// Custom URL slug (auto-generated from title if omitted)
        #[arg(long)]
        slug: Option<String>,
        /// Publish immediately instead of saving as draft
        #[arg(long)]
        publish: bool,
        /// Make the post private
        #[arg(long)]
        private: bool,
    },
    /// Edit an existing post
    Edit {
        /// Post URL slug
        slug: String,
        /// Path to updated markdown file
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Update title
        #[arg(short, long)]
        title: Option<String>,
        /// Update tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,
    },
    /// Delete a post
    Delete {
        /// Post URL slug
        slug: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
    /// Publish a draft post
    Publish {
        /// Post URL slug of the draft to publish
        slug: String,
    },
}
