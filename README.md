# velog-cli

[![CI](https://github.com/hamsurang/velog-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/hamsurang/velog-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A command-line interface for [velog.io](https://velog.io). Manage your blog posts from the terminal.

## Installation

### Homebrew (macOS)

```bash
brew install hamsurang/velog-cli/velog-cli
```

### Cargo

```bash
cargo install velog-cli
```

### GitHub Releases

Download pre-built binaries from the [Releases](https://github.com/hamsurang/velog-cli/releases) page.

## Quick Start

### 1. Authenticate

Get your tokens from browser DevTools → Application → Cookies → `velog.io`, then:

```bash
velog auth login
# Paste access_token (hidden)
# Paste refresh_token (hidden)
```

### 2. List your posts

```bash
velog post list
velog post list --drafts
```

### 3. Create a post

```bash
velog post create --title "My Post" --file post.md --tags "rust,cli" --publish
```

### 4. Edit a post

```bash
velog post edit my-post-slug --file updated.md --title "Updated Title"
```

## Commands

| Command | Description |
|---------|-------------|
| `velog auth login` | Authenticate with velog.io tokens |
| `velog auth status` | Show current login status |
| `velog auth logout` | Remove stored credentials |
| `velog post list` | List your posts (`--drafts` for drafts) |
| `velog post show <slug>` | Show a post (`-u <user>` for others' posts) |
| `velog post create` | Create a new post from markdown |
| `velog post edit <slug>` | Edit an existing post |
| `velog post delete <slug>` | Delete a post (`-y` to skip confirmation) |
| `velog post publish <slug>` | Publish a draft post |
| `velog completions <shell>` | Generate shell completions (bash/zsh/fish) |

### Post Create Options

```
--file, -f <path>   Markdown file (use "-" for stdin)
--title, -t <text>  Post title (required)
--tags <list>       Comma-separated tags
--slug <text>       Custom URL slug (auto-generated if omitted)
--publish           Publish immediately (default: save as draft)
--private           Make the post private
```

### Shell Completions

```bash
# Bash
velog completions bash > ~/.bash_completion.d/velog

# Zsh
velog completions zsh > ~/.zfunc/_velog

# Fish
velog completions fish > ~/.config/fish/completions/velog.fish
```

## How It Works

velog-cli communicates with velog.io's GraphQL API. Tokens are stored securely in `~/.config/velog-cli/credentials.json` with `0600` permissions. Expired tokens are refreshed automatically.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

[MIT](LICENSE)

---

## 한국어 가이드

[velog.io](https://velog.io) 블로그를 터미널에서 관리하는 CLI 도구입니다.

### 설치

```bash
# Homebrew (macOS)
brew install hamsurang/velog-cli/velog-cli

# Cargo
cargo install velog-cli
```

### 사용법

```bash
# 1. 로그인 (브라우저 DevTools → Application → Cookies에서 토큰 복사)
velog auth login

# 2. 글 목록 확인
velog post list

# 3. 새 글 작성
velog post create --title "제목" --file post.md --tags "태그1,태그2" --publish

# 4. 글 수정
velog post edit my-slug --file updated.md

# 5. 글 삭제
velog post delete my-slug

# 6. 임시 글 발행
velog post publish my-slug
```

### 토큰 획득 방법

1. [velog.io](https://velog.io)에 로그인
2. 브라우저 DevTools 열기 (F12)
3. Application → Cookies → `velog.io`
4. `access_token`과 `refresh_token` 값 복사
5. `velog auth login` 실행 후 붙여넣기
