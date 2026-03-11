# velog-cli

[![CI](https://github.com/hamsurang/velog-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/hamsurang/velog-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A command-line interface for [velog.io](https://velog.io). Manage your blog posts from the terminal.

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

## Output Formats

Use `--format` to control output style:

| Format | Description | Use case |
|--------|-------------|----------|
| `pretty` | Tables, colors, markdown rendering (default) | Interactive terminal use |
| `compact` | Minified JSON | AI agents, scripts, pipelines |
| `silent` | Queries emit JSON, mutations emit nothing | CI/CD, exit-code-only checks |

```bash
# Human-friendly (default)
velog post list

# Machine-readable JSON
velog --format compact post list

# Silent mode (data on stdout, nothing on stderr)
velog --format silent post create --title "Post" --file post.md --publish
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

### Development Setup

```bash
# Install lefthook (git hooks manager)
brew install lefthook

# Install typos (spell checker)
brew install typos-cli

# Activate pre-commit hooks
lefthook install
```

Pre-commit hooks automatically run `cargo fmt-check`, `clippy`, `typos`, and `cargo test` on every commit.

> **Note:** `audit` and `ls-lint` are CI-only checks. To run ls-lint locally: `npx @ls-lint/ls-lint`

See [CONTRIBUTING.md](CONTRIBUTING.md) for full guidelines.

## License

[MIT](LICENSE)

---

## 한국어 가이드

[velog.io](https://velog.io) 블로그를 터미널에서 관리하는 CLI 도구입니다.

### 설치

```bash
# Homebrew (macOS)
brew tap hamsurang/velog-cli
brew install velog-cli

# Cargo
cargo install velog-cli
```

### 출력 형식

`--format` 옵션으로 출력 스타일을 선택할 수 있습니다:

| 형식 | 설명 | 용도 |
|------|------|------|
| `pretty` | 테이블, 색상, 마크다운 렌더링 (기본값) | 터미널에서 직접 사용 |
| `compact` | 압축 JSON | AI 에이전트, 스크립트, 파이프라인 |
| `silent` | 쿼리는 JSON, 뮤테이션은 출력 없음 | CI/CD, 종료 코드만 필요한 경우 |

```bash
# 사람이 읽기 쉬운 형태 (기본값)
velog post list

# 기계 판독용 JSON
velog --format compact post list
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

### 기여하기

```bash
# lefthook 설치 (git hooks 매니저)
brew install lefthook

# typos 설치 (맞춤법 검사)
brew install typos-cli

# pre-commit hooks 활성화
lefthook install
```

커밋 시 `cargo fmt-check`, `clippy`, `typos`, `cargo test`가 자동으로 실행됩니다.

> **참고:** `audit`과 `ls-lint`은 CI에서만 실행됩니다. ls-lint를 로컬에서 실행하려면: `npx @ls-lint/ls-lint`

자세한 내용은 [CONTRIBUTING.md](CONTRIBUTING.md)를 참고하세요.

### 토큰 획득 방법

1. [velog.io](https://velog.io)에 로그인
2. 브라우저 DevTools 열기 (F12)
3. Application → Cookies → `velog.io`
4. `access_token`과 `refresh_token` 값 복사
5. `velog auth login` 실행 후 붙여넣기
