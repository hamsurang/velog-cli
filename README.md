# velog-cli

> An unofficial command-line interface for [velog.io](https://velog.io) — manage your blog from the terminal

[![CI](https://github.com/hamsurang/velog-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/hamsurang/velog-cli/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/velog-cli.svg)](https://crates.io/crates/velog-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

[한국어](./README.ko.md)

---

## Installation

### Homebrew (macOS)

```bash
brew tap hamsurang/velog-cli
brew install velog-cli
```

### Cargo

```bash
cargo install velog-cli
```

### GitHub Releases

Download pre-built binaries from the [Releases](https://github.com/hamsurang/velog-cli/releases) page.

---

## Quick Start

### Step 1: Authenticate

Get your tokens from browser DevTools → Application → Cookies → `velog.io`, then:

```bash
velog auth login
# Paste access_token (hidden)
# Paste refresh_token (hidden)
```

### Step 2: Browse posts

```bash
velog post list                          # Your posts
velog post list --drafts                 # Your drafts
velog post list --trending               # Trending posts
velog post list --trending --period week # Trending this week
velog post list --recent                 # Latest posts
velog post list -u <username>            # A user's posts
```

### Step 3: Create a post

```bash
velog post create --title "My Post" --file post.md --tags "rust,cli" --publish
```

### Step 4: Edit a post

```bash
velog post edit my-post-slug --file updated.md --title "Updated Title"
```

---

## Output Formats

Use `--format` to control output style:

| Format | Description | Use case |
|--------|-------------|----------|
| `pretty` | Tables, colors, markdown rendering (default) | Interactive terminal use |
| `compact` | Minified JSON | AI agents, scripts, pipelines |
| `silent` | Queries emit JSON, mutations emit nothing | CI/CD, exit-code-only checks |

```bash
velog post list                        # Human-friendly (default)
velog --format compact post list       # Machine-readable JSON
velog --format silent post create ...  # Silent mode (exit-code only)
```

---

## Commands

### Auth

| Command | Description |
|---------|-------------|
| `velog auth login` | Authenticate with velog.io tokens |
| `velog auth status` | Show current login status |
| `velog auth logout` | Remove stored credentials |

### Posts

| Command | Description |
|---------|-------------|
| `velog post list` | List posts (yours, trending, recent, or by user) |
| `velog post show <slug>` | Show a post (`-u <user>` for others' posts) |
| `velog post create` | Create a new post from markdown |
| `velog post edit <slug>` | Edit an existing post |
| `velog post delete <slug>` | Delete a post (`-y` to skip confirmation) |
| `velog post publish <slug>` | Publish a draft post |
| `velog post like <slug>` | Like a post |
| `velog post unlike <slug>` | Unlike a post |

### Search & Tags

| Command | Description |
|---------|-------------|
| `velog search <keyword>` | Search posts globally or by username |
| `velog tags list` | List tags (trending or alphabetical) |
| `velog tags posts <tag>` | List posts with a specific tag |

### Series

| Command | Description |
|---------|-------------|
| `velog series list` | List your series |
| `velog series show <name>` | Show posts in a series |
| `velog series create <name>` | Create a new series |
| `velog series delete <name>` | Delete a series |
| `velog series edit <name>` | Edit series name/description |
| `velog series add <name> <slug>` | Add a post to a series |
| `velog series remove <name> <slug>` | Remove a post from a series |
| `velog series reorder <name>` | Reorder posts in a series |

### Comments

| Command | Description |
|---------|-------------|
| `velog comment list <slug>` | List comments on a post |
| `velog comment write <slug>` | Write a comment |
| `velog comment reply <comment-id>` | Reply to a comment |
| `velog comment edit <comment-id>` | Edit a comment |
| `velog comment delete <comment-id>` | Delete a comment |

### Social & Stats

| Command | Description |
|---------|-------------|
| `velog stats <slug>` | View post statistics |
| `velog follow <username>` | Follow a user |
| `velog unfollow <username>` | Unfollow a user |
| `velog reading-list` | Access liked and read posts |

### Utilities

| Command | Description |
|---------|-------------|
| `velog completions <shell>` | Generate shell completions (bash/zsh/fish) |

---

## Shell Completions

```bash
# Bash
velog completions bash > ~/.bash_completion.d/velog

# Zsh
velog completions zsh > ~/.zfunc/_velog

# Fish
velog completions fish > ~/.config/fish/completions/velog.fish
```

---

## How It Works

velog-cli communicates with velog.io's GraphQL API. Tokens are stored securely in `~/.config/velog-cli/credentials.json` with `0600` permissions. Expired tokens are refreshed automatically.

---

## Contributing

```bash
brew install lefthook typos-cli
lefthook install
```

Pre-commit hooks run `cargo fmt-check`, `clippy`, `typos`, and `cargo test` on every commit.

> `audit` and `ls-lint` are CI-only. To run ls-lint locally: `npx @ls-lint/ls-lint`

See [CONTRIBUTING.md](CONTRIBUTING.md) for full guidelines.

---

## Disclaimer

This is an **unofficial** tool and is not affiliated with, endorsed by, or associated with [velog.io](https://velog.io) or Chaf Inc. "velog" is a trademark of Chaf Inc. All product names, logos, and brands are property of their respective owners.

## License

MIT — see [LICENSE](./LICENSE)
