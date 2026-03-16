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
#[command(name = "velog", about = "Unofficial CLI client for velog.io — not affiliated with velog.io or Chaf Inc.", version)]
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
    /// Search posts
    #[command(after_long_help = "\
Examples:
  velog search rust                         # Search all posts
  velog search rust --username velopert      # Search within a user's posts
  velog search \"async await\" --limit 5      # Limit results
  velog search rust --offset 20             # Paginate results")]
    Search {
        /// Search keyword
        keyword: String,
        /// Filter by username
        #[arg(short, long)]
        username: Option<String>,
        /// Maximum number of results (1–100)
        #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..=100))]
        limit: u32,
        /// Offset for pagination
        #[arg(long, default_value_t = 0)]
        offset: u32,
    },
    /// Tag commands
    Tags {
        #[command(subcommand)]
        command: TagCommands,
    },
    /// Comment commands
    Comment {
        #[command(subcommand)]
        command: CommentCommands,
    },
    /// View post statistics (views)
    Stats {
        /// Post URL slug
        slug: String,
    },
    /// Series management commands
    Series {
        #[command(subcommand)]
        command: SeriesCommands,
    },
    /// Follow a user
    Follow {
        /// Username to follow
        username: String,
    },
    /// Unfollow a user
    Unfollow {
        /// Username to unfollow
        username: String,
    },
    /// View your reading list (liked/read posts)
    #[command(name = "reading-list")]
    ReadingList {
        /// List type
        #[arg(long, value_enum, default_value_t = ReadingListType::Liked)]
        list_type: ReadingListType,
        /// Maximum number of posts (1–100)
        #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..=100))]
        limit: u32,
        /// Cursor for pagination
        #[arg(long)]
        cursor: Option<String>,
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

/// Time period for trending posts
#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum Period {
    Day,
    Week,
    Month,
    Year,
}

impl std::fmt::Display for Period {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Period::Day => write!(f, "day"),
            Period::Week => write!(f, "week"),
            Period::Month => write!(f, "month"),
            Period::Year => write!(f, "year"),
        }
    }
}

#[derive(Subcommand)]
pub enum TagCommands {
    /// List tags (global or by user)
    List {
        /// Sort order (trending, alphabetical)
        #[arg(long, default_value = "trending")]
        sort: String,
        /// Filter tags by username
        #[arg(short, long)]
        username: Option<String>,
        /// Maximum number of tags (1–100)
        #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..=100))]
        limit: u32,
        /// Cursor for pagination
        #[arg(long)]
        cursor: Option<String>,
    },
    /// List posts with a specific tag
    Posts {
        /// Tag name
        tag: String,
        /// Filter by username
        #[arg(short, long)]
        username: Option<String>,
        /// Maximum number of posts (1–100)
        #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..=100))]
        limit: u32,
        /// Cursor for pagination
        #[arg(long)]
        cursor: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum SeriesCommands {
    /// List series for a user
    List {
        /// Username (defaults to logged-in user)
        #[arg(short, long)]
        username: Option<String>,
    },
    /// Show series detail with post list
    Show {
        /// Series URL slug
        slug: String,
        /// Username (defaults to logged-in user)
        #[arg(short, long)]
        username: Option<String>,
    },
    /// Create a new series
    Create {
        /// Series name
        name: String,
        /// Custom URL slug (auto-generated from name if omitted)
        #[arg(long)]
        slug: Option<String>,
    },
    /// Edit a series (name or post order)
    Edit {
        /// Series URL slug
        slug: String,
        /// New series name
        #[arg(long)]
        name: Option<String>,
        /// Reorder posts by comma-separated post slugs
        #[arg(long)]
        order: Option<String>,
    },
    /// Delete a series (posts are NOT deleted)
    Delete {
        /// Series URL slug
        slug: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum CommentCommands {
    /// List comments on a post
    List {
        /// Post URL slug
        post_slug: String,
        /// Username (defaults to logged-in user)
        #[arg(short, long)]
        username: Option<String>,
        /// Maximum number of comments to show
        #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u32).range(1..=500))]
        limit: u32,
    },
    /// Write a comment on a post
    Write {
        /// Post URL slug
        post_slug: String,
        /// Comment text (or use --file)
        text: Option<String>,
        /// Read comment from file (use "-" for stdin)
        #[arg(short, long, conflicts_with = "text")]
        file: Option<PathBuf>,
    },
    /// Reply to a comment
    #[command(after_long_help = "\
Comment Numbering:
  Top-level comments are numbered 1, 2, 3, ...
  Replies use dot notation: 1.1, 1.2, 2.1, ...
  Use `velog comment list <slug>` to see comment numbers.

Examples:
  velog comment reply my-post 1 \"Great post!\"
  velog comment reply my-post 1.2 --file reply.md")]
    Reply {
        /// Post URL slug
        post_slug: String,
        /// Comment number to reply to (e.g. "1", "1.2")
        number: String,
        /// Reply text (or use --file)
        text: Option<String>,
        /// Read reply from file (use "-" for stdin)
        #[arg(short, long, conflicts_with = "text")]
        file: Option<PathBuf>,
    },
    /// Edit a comment
    Edit {
        /// Post URL slug
        post_slug: String,
        /// Comment number to edit (e.g. "1", "1.2")
        number: String,
        /// New text (or use --file)
        text: Option<String>,
        /// Read new text from file (use "-" for stdin)
        #[arg(short, long, conflicts_with = "text")]
        file: Option<PathBuf>,
    },
    /// Delete a comment
    Delete {
        /// Post URL slug
        post_slug: String,
        /// Comment number to delete (e.g. "1", "1.2")
        number: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum PostCommands {
    /// List posts (yours, trending, recent, or by user)
    #[command(after_long_help = "\
Listing Modes:
  (default)    Your own posts (requires login)
  --drafts     Your draft posts only
  --trending   Trending posts across velog (public, no login needed)
  --recent     Recent posts across velog (public, no login needed)
  --username   Posts by a specific user (public, no login needed)
  --tag        Posts with a specific tag

Examples:
  velog post list                          # Your published posts
  velog post list --drafts                 # Your drafts
  velog post list --trending --period day  # Today's trending
  velog post list --username velopert      # Posts by velopert
  velog post list --tag rust --limit 10    # Rust-tagged posts")]
    List {
        /// Show draft (temporary) posts instead of published
        #[arg(long, conflicts_with_all = ["trending", "recent", "username"])]
        drafts: bool,
        /// Show trending posts
        #[arg(long, conflicts_with_all = ["drafts", "recent", "username"])]
        trending: bool,
        /// Show recent posts
        #[arg(long, conflicts_with_all = ["drafts", "trending", "username"])]
        recent: bool,
        /// Show posts by a specific user
        #[arg(short, long, conflicts_with_all = ["drafts", "trending", "recent"])]
        username: Option<String>,
        /// Filter by tag name
        #[arg(long, conflicts_with_all = ["drafts", "trending", "recent"])]
        tag: Option<String>,
        /// Maximum number of posts to show (1–100)
        #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..=100))]
        limit: u32,
        /// Time period for trending (day, week, month, year)
        #[arg(long, value_enum, requires = "trending", conflicts_with_all = ["recent", "username", "drafts"])]
        period: Option<Period>,
        /// Cursor for pagination (recent, user posts)
        #[arg(long, conflicts_with_all = ["trending", "drafts"])]
        cursor: Option<String>,
        /// Offset for pagination (trending only)
        #[arg(long, requires = "trending", conflicts_with_all = ["recent", "username", "drafts"])]
        offset: Option<u32>,
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
    #[command(after_long_help = "\
Examples:
  velog post create --title \"My Post\" --file post.md --tags rust,cli
  velog post create --title \"Draft\" --file post.md
  velog post create --title \"Published\" --file post.md --publish
  cat post.md | velog post create --title \"Piped\" --publish")]
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
    /// Like a post
    Like {
        /// Post URL slug
        slug: String,
        /// Post author username (defaults to logged-in user)
        #[arg(short, long)]
        username: Option<String>,
    },
    /// Unlike a post
    Unlike {
        /// Post URL slug
        slug: String,
        /// Post author username (defaults to logged-in user)
        #[arg(short, long)]
        username: Option<String>,
    },
}

/// Reading list type
#[derive(ValueEnum, Clone, Copy, Debug, Default)]
pub enum ReadingListType {
    /// Liked posts
    #[default]
    Liked,
    /// Read posts
    Read,
}

impl std::fmt::Display for ReadingListType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadingListType::Liked => write!(f, "LIKED"),
            ReadingListType::Read => write!(f, "READ"),
        }
    }
}
