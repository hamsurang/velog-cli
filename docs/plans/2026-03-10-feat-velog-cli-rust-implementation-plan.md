---
title: "feat: Velog CLI - Rust Implementation"
type: feat
status: active
date: 2026-03-10
deepened: 2026-03-10
origin: docs/brainstorms/2026-03-10-velog-cli-brainstorm.md
---

# feat: Velog CLI - Rust Implementation

## Enhancement Summary

**Deepened on:** 2026-03-10
**Research agents used:** Security Sentinel, Architecture Strategist, Performance Oracle, Code Simplicity Reviewer, Best Practices Researcher, Context7 (clap, reqwest)

### Key Improvements
1. **구조 단순화**: 12 파일 → 6 파일. YAGNI 원칙 적용 (lib.rs, config.rs, display.rs, commands/mod.rs 제거)
2. **보안 강화**: `https_only(true)`, `rustls-tls`, JWT claim 검증, slug 입력값 검증, 민감 헤더 마킹
3. **성능 최적화**: `tokio current_thread`, `reqwest::Client` 단일 생성, release 프로필 최적화
4. **아키텍처 개선**: client-auth 분리 (client는 credentials를 쓰지 않고 반환), 2-layer GraphQL 호출로 무한 갱신 방지
5. **새 의존성**: `slug` (한글 slug 생성), `termimad` (마크다운 터미널 렌더링)
6. **수정된 결정**: `post edit/delete/publish`가 ID 대신 slug 기반으로 변경 (velog API에 ID 직접 조회 쿼리 없음)

### New Considerations Discovered
- `Cargo.lock`은 바이너리 크레이트에서 반드시 커밋해야 함 (.gitignore에서 제거)
- `rpassword` v5는 구버전 → v7 사용
- velog GraphQL API는 HTTP 200 + errors 배열로 에러 반환 (HTTP status로 구분 불가)
- `post` 쿼리는 `username + url_slug`만 지원, ID 직접 조회 없음

### Review Round 2-3 Additions
- `AuthError` 마커 타입으로 exit code 구별 (문자열 매칭 제거)
- `base64` 크레이트 추가 (JWT payload 디코딩)
- `clap_complete` 추가 → `velog completions <shell>` 서브커맨드
- stdin pipe 지원 (`--file -` 또는 non-TTY stdin)
- stderr/stdout 분리 원칙 + `NO_COLOR` 자동 지원 (colored 크레이트 내장)
- Post-MVP 로드맵 대폭 확장 (참조 CLI 7종 + Rust CLI 모범 사례 비교)
- VelogClient: `default_headers` → per-request Cookie 생성 (갱신 후 새 토큰 즉시 반영)
- `execute_graphql`: 인증 에러만 재시도 (네트워크 에러 즉시 전파), `&mut self`로 변경
- `base64_url_decode` → `base64::engine::URL_SAFE_NO_PAD.decode()` 실제 API 사용

### Review Round 7 Fixes (12 issues)
- **CRITICAL**: `exit_code()` — `downcast_ref` → `err.chain()` 순회 (`.context()` 이후에도 AuthError 탐색)
- **CRITICAL**: `post_delete` — piped stdin + no `--yes` 시 무한 대기 → TTY 가드 추가
- **MAJOR**: `raw_graphql` → `GraphQLResponse<T>` 반환, `execute_graphql`이 `is_auth_error()` 를 `into_result()` 전에 체크 (string matching 제거)
- **MAJOR**: `query: &str` → `&'static str` (GraphQLRequest 필드 타입과 일치)
- **MAJOR**: `From<UserToken> for Credentials` 변환 추가
- **MAJOR**: `WritePostInput`/`EditPostInput` 구조체 추가 (models.rs)
- **MAJOR**: `get_posts` — velog API에 없는 `limit` 파라미터 제거
- **MAJOR**: `restore_token()` 실패 시 `AuthError` 마커 부착 → exit code 2 보장
- **HIGH**: `post_show` — `--username` 항상 우선, handler 코드 추가
- **HIGH**: `post_create` handler 스켈레톤 (4-way content source 분기)
- **HIGH**: `post_edit` merge 로직 (None vs Some("") tags 구분)
- **HIGH**: `auth_login` handler + JWT sub claim 값 문서화 (`"access_token"`, `"refresh_token"`)

### Review Round 8-9 Fixes (4 issues)
- **MAJOR**: GraphQL 응답 래퍼 타입 7종 추가 (`CurrentUserData`, `RestoreTokenData`, `PostsData` 등) — `GraphQLResponse<T>`의 `T`가 JSON `data` 구조와 일치해야 역직렬화 성공
- **MINOR**: handlers.rs에 `use anyhow::Context;` 추가 (`.context()` 컴파일에 필요)
- **MINOR**: `execute_graphql` — `resp.data.is_none()` 가드 추가 (partial success 시 불필요 retry 방지)
- **MINOR**: `post_create` slug 검증 — prose에만 있던 정규식 검증을 실제 코드에 추가

---

## Overview

Rust로 만드는 velog.io CLI 클라이언트. 사용자의 JWT 쿠키(access_token, refresh_token)를 통해 터미널에서 velog 포스트의 CRUD와 목록 조회를 수행하는 도구.

핵심 유즈케이스: 글 작성, 수정, 삭제, 조회, 목록, 임시저장 관리 (see brainstorm: docs/brainstorms/2026-03-10-velog-cli-brainstorm.md)

## Problem Statement / Motivation

velog.io는 웹 브라우저에서만 포스트 관리가 가능하다. CLI 도구가 있으면:
- 터미널 워크플로우에서 벗어나지 않고 블로그 포스트를 관리할 수 있음
- 로컬 마크다운 파일을 직접 발행할 수 있음 (에디터 자유도)
- 스크립트/자동화 파이프라인과 연동 가능

## Proposed Solution

수동 쿠키 입력으로 인증하고, velog의 GraphQL API(`v3.velog.io/graphql`)에 직접 요청을 보내는 CLI 도구.

```
velog auth login / status / logout
velog post list / show / create / edit / delete / publish
velog completions <shell>
```

## Technical Approach

### Architecture

```
┌─────────────┐    ┌───────────────┐    ┌──────────────────┐
│   CLI Layer  │───▶│  Client Layer │───▶│ v3.velog.io/graphql│
│  (clap)      │    │  (reqwest)    │    │  GraphQL API      │
└─────────────┘    └───────────────┘    └──────────────────┘
       │                   │
       ▼                   ▼
┌─────────────┐    ┌───────────────┐
│  Handlers   │    │  Auth         │
│  (display   │    │  (XDG + JSON) │
│   inline)   │    │               │
└─────────────┘    └───────────────┘
```

### Research Insights — Architecture

**Client-Auth 분리 원칙 (Architecture Review):**
- `client.rs`는 credentials를 디스크에 직접 저장하지 않는다
- `execute_graphql(&mut self)` → `Result<(T, Option<Credentials>)>` 반환
- 갱신 시 `self.credentials`를 내부 교체 + `Option<Credentials>` 반환
- `Option<Credentials>`가 `Some`이면 핸들러에서 `auth::save_credentials()` 호출
- Cookie는 `default_headers`가 아닌 매 요청마다 `self.credentials`에서 동적 생성
- 이로써 client.rs는 파일시스템 의존 없이 테스트 가능하며, 토큰 갱신 후 재시도도 정상 동작

**2-Layer GraphQL 호출 (Architecture + Performance Review):**

> **핵심 설계:** `raw_graphql`은 `GraphQLResponse`를 반환하고, `execute_graphql`이 `is_auth_error()`를 **`into_result()` 호출 전에** 체크. 이로써 string matching 없이 구조화된 에러 감지 가능.

```rust
/// raw_graphql: HTTP 요청 → GraphQLResponse 그대로 반환 (into_result 미호출)
async fn raw_graphql<V: Serialize, T: DeserializeOwned>(
    &self, query: &'static str, variables: Option<V>,
) -> anyhow::Result<GraphQLResponse<T>> { ... }

/// execute_graphql: raw_graphql + 인증 에러 시 1회 갱신 재시도
/// &mut self — 갱신 성공 시 self.credentials를 새 토큰으로 교체
pub async fn execute_graphql<V, T>(
    &mut self, query: &'static str, variables: Option<V>,
) -> anyhow::Result<(T, Option<Credentials>)>
where V: Serialize + Clone, T: DeserializeOwned
{
    let resp: GraphQLResponse<T> = self.raw_graphql(query, variables.clone()).await?;

    // data가 없고 인증 에러인 경우에만 재시도 (partial success 시 불필요 retry 방지)
    if resp.data.is_none() && resp.is_auth_error() && self.credentials.is_some() {
        let new_creds = self.restore_token().await
            .map_err(|_| anyhow::Error::new(crate::AuthError)
                .context("Token refresh failed. Run `velog auth login` again."))?;
        self.credentials = Some(new_creds.clone());
        let retry_resp: GraphQLResponse<T> = self.raw_graphql(query, variables).await?;
        let data = retry_resp.into_result()?;
        return Ok((data, Some(new_creds)));
    }

    let data = resp.into_result()?;
    Ok((data, None))
}
```

> **핵심 변경 (Review Round 5 + 7):**
> - `&self` → `&mut self`: 갱신 시 `self.credentials` 교체 필요
> - Cookie를 `default_headers`에 넣지 않고 `raw_graphql()`에서 매 요청마다 동적 생성
> - `raw_graphql`이 `GraphQLResponse<T>`를 반환 → `is_auth_error()`로 구조화된 인증 에러 감지 (string matching 제거)
> - `restore_token()` 실패 시 `AuthError` 마커 부착 → exit code 2 보장
> - `V: Clone` — 재시도 시 variables를 다시 사용해야 하므로 Clone 필요
> - `query: &'static str` — 모든 쿼리가 `const` 문자열이므로 `'static` 보장

### Project Structure (Simplified — 6 files)

```
velog-cli/
├── Cargo.toml
├── Cargo.lock              # 반드시 커밋 (바이너리 크레이트)
├── .gitignore
├── src/
│   ├── main.rs             # 엔트리포인트 + Cli::parse() + dispatch + exit code
│   ├── cli.rs              # clap derive 커맨드 정의
│   ├── client.rs           # VelogClient: GraphQL 요청 + 토큰 갱신 (디스크 미접근)
│   ├── auth.rs             # credentials CRUD + XDG 경로 (config.rs 흡수)
│   ├── models.rs           # Post, User, GraphQL envelope 타입
│   └── handlers.rs         # 모든 커맨드 핸들러 + display 함수 인라인
└── tests/                  # MVP 이후 추가
```

### Research Insights — Simplification

**왜 6 파일인가 (Simplicity Review):**

| 제거된 파일 | 사유 |
|-------------|------|
| `lib.rs` | integration test가 MVP에 없음. 필요 시 추가 |
| `config.rs` | XDG 경로 계산은 3줄. `auth.rs`에 private fn으로 흡수 |
| `display.rs` | 각 함수가 1회만 호출됨. `handlers.rs` 내 private fn으로 인라인 |
| `commands/mod.rs` | 디스패처 boilerplate. `main.rs` match에서 직접 호출 |
| `commands/auth.rs` | `handlers.rs`로 통합 |
| `commands/post.rs` | `handlers.rs`로 통합 |
| `error.rs` | `anyhow` 단독 사용. 종료 코드용 최소 enum만 `main.rs`에 정의 |

### Key Design Decisions (from brainstorm)

| 결정 사항 | 선택 | 근거 |
|-----------|------|------|
| 인증 | 수동 쿠키 입력 | GitHub OAuth 사용자 포함 모든 사용자 커버 |
| API 방식 | 수동 GraphQL + serde | 쿼리 <10개, 코드젠 불필요 |
| 출력 | comfy-table + colored + termimad | Rich 터미널 스타일 + 마크다운 렌더링 |
| 메타데이터 입력 | CLI 플래그 | frontmatter 미사용 |
| 설정 저장 | XDG 규약 | ~/.config/velog-cli/ |

### SpecFlow 분석 + 리뷰에서 도출된 추가 결정사항

| 결정 사항 | 선택 | 근거 |
|-----------|------|------|
| `post create` 기본값 | `is_temp=true` (임시저장) | 실수로 발행 방지. `--publish` 플래그로 즉시 발행 |
| `is_private` 기본값 | `false` (공개) | velog 웹 UI 기본값과 동일. `--private` 플래그 제공 |
| `url_slug` 생성 | `slug` 크레이트 (deunicode 기반) + `--slug` 오버라이드 | 한글→로마자 자동 변환 |
| `post edit/delete/publish` 식별자 | **slug 기반** (ID 아님) | velog API에 ID 직접 조회 없음. `post(username, url_slug)`만 지원 |
| `post edit` 필드 보존 | 기존 포스트 fetch 후 merge | editPost는 모든 필드 필수 |
| `post show` username | 인증 시 `current_user()` API 호출로 획득, 미인증 시 `--username` 필수 | |
| 토큰 갱신 후 저장 | client가 `Option<Credentials>` 반환 → 핸들러에서 저장 | client-auth 분리 |
| 삭제 확인 | `y/N` 프롬프트 + `--yes` 플래그 | 스크립트 호환 |
| 토큰 입력 | rpassword v7 숨김 입력 | 터미널 스크롤백 노출 방지 |
| 종료 코드 | 0=성공, 1=일반오류, 2=인증오류 | 스크립트 오류 구분 |
| 에러 처리 | `anyhow` 단독 (thiserror 제거) | 바이너리 크레이트에서 타입 에러 불필요 |
| 마크다운 렌더링 | `termimad` | 터미널 폭 인식, 코드 블록 하이라이팅 |

---

## Implementation Phases

### Phase 1: 프로젝트 기반 + 인증 (Foundation)

프로젝트 초기화와 인증 레이어 구현. CLI의 가장 기본적인 뼈대.

#### 1.1 프로젝트 초기화

**파일: `Cargo.toml`**
```toml
[package]
name = "velog-cli"
version = "0.1.0"
edition = "2021"
description = "CLI client for velog.io"

[dependencies]
# CLI
clap = { version = "4", features = ["derive"] }
clap_complete = "4"

# Async (minimal features for CLI)
tokio = { version = "1", features = ["rt", "macros"] }

# HTTP (explicit TLS backend, no system openssl dependency)
reqwest = { version = "0.12", default-features = false, features = [
    "json", "cookies", "rustls-tls"
] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling (anyhow only — binary crate, no need for thiserror)
anyhow = "1"

# Directories
dirs = "5"

# Terminal output
comfy-table = "7"
colored = "2"
termimad = "0.29"

# Input
rpassword = "7"

# Slug generation (Korean → romanized ASCII via deunicode)
slug = "0.1"

# JWT payload decoding (base64url, signature verification 없이 claim 검사용)
base64 = "0.22"

[profile.release]
strip = true
opt-level = "z"
lto = true
codegen-units = 1
```

### Research Insights — Dependencies

**변경 사항 (vs 원본 plan):**

| 의존성 | 변경 | 사유 |
|--------|------|------|
| `tokio` | `"full"` → `["rt", "macros"]` | CLI는 순차 HTTP만 필요. 바이너리 ~120KB 절감 (Performance) |
| `reqwest` | `rustls-tls` 명시 + `default-features = false` | 시스템 openssl 의존 제거, TLS 백엔드 고정 (Security) |
| `thiserror` | 제거 | 바이너리 크레이트, `anyhow`만으로 충분 (Simplicity) |
| `rpassword` | v5 → v7 | v5는 구버전, 일부 터미널 호환 문제 (Architecture) |
| `slug` | 추가 | 한글 제목 → ASCII slug 변환. `deunicode` 기반 (Best Practices) |
| `termimad` | 추가 | `post show`에서 마크다운 렌더링 (Best Practices) |
| `base64` | 추가 | JWT payload 디코딩 (validate_velog_jwt에서 사용) (Security Review) |

**파일: `.gitignore`**
```
/target
```

> **중요**: `Cargo.lock`은 `.gitignore`에 넣지 않는다. 바이너리 크레이트는 재현 가능한 빌드를 위해 반드시 커밋. (Architecture Review)

- [ ] `cargo init --name velog-cli` 실행
- [ ] `Cargo.toml` 의존성 설정
- [ ] `.gitignore` 작성 (`Cargo.lock` 포함하지 않음)

#### 1.2 인증 레이어

**파일: `src/auth.rs`** (config.rs 흡수)

```rust
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub access_token: String,
    pub refresh_token: String,
}

/// XDG config directory: ~/.config/velog-cli/
fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("Cannot determine config directory")?
        .join("velog-cli");
    Ok(dir)
}

fn credentials_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("credentials.json"))
}

pub fn save_credentials(creds: &Credentials) -> Result<()> {
    let dir = config_dir()?;
    std::fs::create_dir_all(&dir)?;
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    }

    let path = credentials_path()?;
    let content = serde_json::to_string_pretty(creds)?;
    std::fs::write(&path, &content)?;
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub fn load_credentials() -> Result<Option<Credentials>> {
    let path = credentials_path()?;
    if !path.exists() { return Ok(None); }
    let content = std::fs::read_to_string(&path)
        .context("Cannot read credentials")?;
    let creds = serde_json::from_str(&content)
        .context("Credentials file is corrupt. Run `velog auth login` again.")?;
    Ok(Some(creds))
}

pub fn delete_credentials() -> Result<()> {
    let path = credentials_path()?;
    // 무조건 삭제 시도 (corrupt 상태에서도 logout 가능)
    if path.exists() { std::fs::remove_file(&path)?; }
    // temp 파일도 정리
    let tmp = path.with_extension("json.tmp");
    if tmp.exists() { std::fs::remove_file(&tmp)?; }
    Ok(())
}
```

### Research Insights — Auth Security

**JWT Claim 검증 (Security Review):**
로그인 시 토큰의 `iss`와 `sub` claim을 디코딩(서명 검증 없이)하여 올바른 velog 토큰인지 확인:

```rust
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

pub fn validate_velog_jwt(token: &str, expected_sub: &str) -> Result<()> {
    let parts: Vec<&str> = token.splitn(3, '.').collect();
    if parts.len() != 3 { bail!("Invalid token format (not a JWT)"); }
    let payload = URL_SAFE_NO_PAD.decode(parts[1])
        .context("Failed to decode JWT payload")?;
    let claims: serde_json::Value = serde_json::from_slice(&payload)?;
    if claims["iss"].as_str() != Some("velog.io") {
        bail!("Token is not from velog.io (unexpected issuer)");
    }
    if claims["sub"].as_str() != Some(expected_sub) {
        bail!("Expected {} token, got {:?}", expected_sub, claims["sub"]);
    }
    Ok(())
}
```

**JWT sub claim 값 (brainstorm 참조):**
- `access_token` JWT: `{ user_id, iss: "velog.io", sub: "access_token" }` — 1시간 만료
- `refresh_token` JWT: `{ user_id, token_id, iss: "velog.io", sub: "refresh_token" }` — 30일 만료
- `validate_velog_jwt(token, "access_token")` / `validate_velog_jwt(token, "refresh_token")` 으로 호출

**보안 정책 (Security Review):**
- 토큰은 절대 CLI 인자로 받지 않음 (rpassword prompt만 사용)
- `#[arg(env = "...")]` 사용 금지 — cli.rs에 주석으로 명시
- logout은 무조건 삭제 (파싱 실패해도 삭제 가능)

**auth_login handler (handlers.rs):**
```rust
pub async fn auth_login() -> anyhow::Result<()> {
    eprintln!("Paste your velog tokens (hidden input).");
    eprintln!("Find them in browser DevTools → Application → Cookies → velog.io");

    eprint!("access_token: ");
    let access_token = rpassword::read_password()
        .context("Failed to read access_token")?;
    anyhow::ensure!(!access_token.trim().is_empty(), "access_token cannot be empty");
    auth::validate_velog_jwt(access_token.trim(), "access_token")?;

    eprint!("refresh_token: ");
    let refresh_token = rpassword::read_password()
        .context("Failed to read refresh_token")?;
    anyhow::ensure!(!refresh_token.trim().is_empty(), "refresh_token cannot be empty");
    auth::validate_velog_jwt(refresh_token.trim(), "refresh_token")?;

    let creds = Credentials {
        access_token: access_token.trim().to_string(),
        refresh_token: refresh_token.trim().to_string(),
    };

    // 실제 API 호출로 토큰 유효성 최종 확인
    let mut client = VelogClient::new(creds.clone())?;
    let (user, new_creds) = client.current_user().await
        .context("Token validation failed. The tokens may be expired.")?;
    let creds = new_creds.unwrap_or(creds);
    auth::save_credentials(&creds)?;

    eprintln!("{} Logged in as {}", "✓".green(), user.username.bold());
    Ok(())
}
```

- [ ] `src/auth.rs` 작성 — credentials CRUD + XDG 경로 + JWT 검증

#### 1.3 CLI 커맨드 정의

**파일: `src/cli.rs`**
```rust
use clap::{Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

// SECURITY: Token values MUST NOT be accepted as CLI arguments or env vars.
// They must only be collected via rpassword's hidden prompt.
// Do NOT add #[arg(env = "...")] to any token-related fields.

#[derive(Parser)]
#[command(name = "velog", about = "CLI client for velog.io", version)]
pub struct Cli {
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
```

### Research Insights — CLI

**slug 기반 식별자 (Architecture Review):**
velog API의 `post` 쿼리는 `username + url_slug`만 지원하고, ID 직접 조회가 없다. 따라서 `edit`, `delete`, `publish` 커맨드 모두 slug를 식별자로 사용:
```bash
velog post edit my-first-post --file ./updated.md
velog post delete my-first-post
velog post publish my-first-post
```
이 방식이 사용자 관점에서도 직관적 (ID는 기억하기 어려움).

**slug 입력값 검증 (Security Review):**
`--slug` 값에 대해 `^[a-z0-9]+(?:-[a-z0-9]+)*$` 정규식 검증. 길이 255자 제한.

- [ ] `src/cli.rs` 작성 — clap derive 전체 커맨드 트리

#### 1.4 엔트리포인트

**파일: `src/main.rs`**
```rust
mod auth;
mod cli;
mod client;
mod handlers;
mod models;

use clap::{CommandFactory, Parser};
use cli::{Cli, Commands, AuthCommands, PostCommands};

// NOTE: `colored` 크레이트는 NO_COLOR, CLICOLOR 환경변수를 자동 인식.
// 추가 코드 없이 NO_COLOR=1 설정 시 색상 비활성화됨.
// 모든 사용자 메시지는 eprintln! (stderr), 데이터 출력만 println! (stdout).

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Completions { shell } => {
            clap_complete::generate(
                shell,
                &mut Cli::command(),
                "velog",
                &mut std::io::stdout(),
            );
            return;  // completions은 에러 불가
        }
        Commands::Auth { command } => match command {
            AuthCommands::Login => handlers::auth_login().await,
            AuthCommands::Status => handlers::auth_status().await,
            AuthCommands::Logout => handlers::auth_logout(),
        },
        Commands::Post { command } => match command {
            PostCommands::List { drafts } =>
                handlers::post_list(drafts).await,
            PostCommands::Show { slug, username } =>
                handlers::post_show(&slug, username.as_deref()).await,
            PostCommands::Create { file, title, tags, slug, publish, private } =>
                handlers::post_create(file.as_deref(), &title, &tags, slug.as_deref(), publish, private).await,
            PostCommands::Edit { slug, file, title, tags } =>
                handlers::post_edit(&slug, file.as_deref(), title.as_deref(), tags.as_deref()).await,
            PostCommands::Delete { slug, yes } =>
                handlers::post_delete(&slug, yes).await,
            PostCommands::Publish { slug } =>
                handlers::post_publish(&slug).await,
        },
    };

    if let Err(e) = result {
        eprintln!("{}: {:#}", colored::Colorize::red("error"), e);
        let code = exit_code(&e);
        std::process::exit(code);
    }
}

/// 에러 체인에서 AuthError 마커를 찾아 종료 코드 결정
/// handlers.rs에서 crate::AuthError로 참조
pub(crate) struct AuthError;
impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "authentication required")
    }
}
impl std::fmt::Debug for AuthError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { std::fmt::Display::fmt(self, f) } }
impl std::error::Error for AuthError {}

/// .context() 래핑 후에도 AuthError를 찾으려면 chain() 순회 필요
/// downcast_ref는 outermost 타입만 검사 → .context() 후 AuthError를 못 찾음
fn exit_code(err: &anyhow::Error) -> i32 {
    for cause in err.chain() {
        if cause.downcast_ref::<AuthError>().is_some() {
            return 2;
        }
    }
    1
}

// 핸들러에서 인증 에러 생성 시:
// Err(anyhow::Error::new(AuthError).context("Not logged in. Run `velog auth login` first."))
```

### Research Insights — Performance

**`current_thread` 런타임 (Performance Review):**
- CLI는 순차 HTTP 요청만 수행 → 멀티스레드 스케줄러 불필요
- `#[tokio::main(flavor = "current_thread")]` → 바이너리 ~120KB 절감, 시작 ~2-5ms 개선

- [ ] `src/main.rs` 작성 — parse + dispatch + exit code

#### Phase 1 완료 기준
- [ ] `cargo build` 성공
- [ ] `velog auth login` → 숨김 입력 → JWT 검증 → credentials.json 저장 (0600)
- [ ] `velog auth status` → currentUser 쿼리 → 사용자명 출력
- [ ] `velog auth logout` → credentials + temp 파일 삭제

---

### Phase 2: GraphQL 클라이언트 + 모델 (Core)

API 통신 레이어. 2-layer 호출 구조 + 토큰 갱신.

#### 2.1 데이터 모델

**파일: `src/models.rs`**
```rust
use serde::{Deserialize, Serialize};

// ---- GraphQL Envelope ----
#[derive(Serialize)]
pub struct GraphQLRequest<V: Serialize> {
    pub query: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<V>,
}

#[derive(Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize, Debug)]
pub struct GraphQLError {
    pub message: String,
    pub extensions: Option<serde_json::Value>,
}

impl<T> GraphQLResponse<T> {
    /// data가 있으면 반환, errors만 있으면 에러, 둘 다 없으면 에러
    pub fn into_result(self) -> anyhow::Result<T> {
        match (self.data, self.errors) {
            (Some(data), _) => Ok(data),  // partial errors 시에도 data 우선
            (None, Some(errs)) => {
                let msg = errs.first()
                    .map(|e| e.message.clone())
                    .unwrap_or_else(|| "Unknown GraphQL error".into());
                anyhow::bail!("GraphQL error: {}", msg)
            }
            (None, None) => anyhow::bail!("Empty GraphQL response"),
        }
    }

    /// 에러 배열에서 인증 관련 에러인지 확인
    pub fn is_auth_error(&self) -> bool {
        self.errors.as_ref().map_or(false, |errs| {
            errs.iter().any(|e|
                e.message.contains("not logged in")
                || e.message.contains("Unauthorized")
                || e.extensions.as_ref()
                    .and_then(|ext| ext.get("code"))
                    .and_then(|v| v.as_str())
                    .map(|c| c == "UNAUTHENTICATED")
                    .unwrap_or(false)
            )
        })
    }
}

// ---- Domain Models ----
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub id: String,
    pub title: String,
    pub short_description: Option<String>,
    pub body: Option<String>,
    pub thumbnail: Option<String>,
    pub likes: i32,
    pub is_private: bool,
    pub is_temp: bool,
    pub url_slug: String,
    pub released_at: Option<String>,
    pub updated_at: Option<String>,
    pub tags: Option<Vec<String>>,
    pub user: Option<PostUser>,
}

#[derive(Deserialize, Debug)]
pub struct PostUser {
    pub username: String,
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserToken {
    pub access_token: String,
    pub refresh_token: String,
}

/// restoreToken 응답(UserToken) → 디스크 저장용(Credentials) 변환
impl From<UserToken> for crate::auth::Credentials {
    fn from(t: UserToken) -> Self {
        Self { access_token: t.access_token, refresh_token: t.refresh_token }
    }
}

// ---- GraphQL Response Wrappers ----
// GraphQLResponse<T>의 T는 JSON "data" 필드의 구조와 일치해야 한다.
// 예: { "data": { "currentUser": { ... } } } → T = CurrentUserData

#[derive(Deserialize)]
pub struct CurrentUserData {
    #[serde(rename = "currentUser")]
    pub current_user: User,
}

#[derive(Deserialize)]
pub struct RestoreTokenData {
    #[serde(rename = "restoreToken")]
    pub restore_token: UserToken,
}

#[derive(Deserialize)]
pub struct PostsData {
    pub posts: Vec<Post>,
}

#[derive(Deserialize)]
pub struct PostData {
    pub post: Post,
}

#[derive(Deserialize)]
pub struct WritePostData {
    #[serde(rename = "writePost")]
    pub write_post: Post,
}

#[derive(Deserialize)]
pub struct EditPostData {
    #[serde(rename = "editPost")]
    pub edit_post: Post,
}

#[derive(Deserialize)]
pub struct RemovePostData {
    #[serde(rename = "removePost")]
    pub remove_post: bool,
}

// ---- Mutation Input Types ----
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WritePostInput {
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub is_markdown: bool,     // 항상 true
    pub is_temp: bool,
    pub is_private: bool,
    pub url_slug: String,
    pub thumbnail: Option<String>,
    pub meta: serde_json::Value, // 기본: {}
    pub series_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditPostInput {
    pub id: String,            // 수정 대상 포스트 ID (get_post에서 획득)
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub is_markdown: bool,
    pub is_temp: bool,
    pub is_private: bool,
    pub url_slug: String,
    pub thumbnail: Option<String>,
    pub meta: serde_json::Value,
    pub series_id: Option<String>,
}
```

### Research Insights — GraphQL Error Handling

**HTTP 200 + errors 배열 (Best Practices):**
velog GraphQL API는 인증 에러도 HTTP 200으로 반환한다. `GraphQLResponse::is_auth_error()`로 에러 메시지 + extensions.code를 확인하여 인증 실패를 감지.

- [ ] `src/models.rs` 작성 — GraphQL envelope (into_result, is_auth_error) + 도메인 모델

#### 2.2 GraphQL 클라이언트

**파일: `src/client.rs`**

핵심 설계:
- `VelogClient` struct: `reqwest::Client` (1회 생성, Cookie 미포함) + `Option<Credentials>` (None=미인증, Some=갱신 가능)
- **2-layer 구조**: `raw_graphql(&self)` (단순 요청) + `execute_graphql(&mut self)` (인증 에러 시 1회 갱신 재시도)
- Cookie 헤더는 매 요청마다 `self.credentials`에서 동적 생성 (갱신 후 새 토큰 즉시 반영)
- 갱신 성공 시 `self.credentials` 내부 교체 + `Option<Credentials>` 반환 (디스크 저장은 핸들러 책임)

```rust
use std::time::Duration;
use reqwest::header::{HeaderValue, COOKIE};
use serde::Serialize;
use serde::de::DeserializeOwned;
use crate::auth::Credentials;
use crate::models::{
    GraphQLRequest, GraphQLResponse,
    CurrentUserData, RestoreTokenData, PostsData, PostData,
    WritePostData, EditPostData, RemovePostData,
};

pub struct VelogClient {
    http: reqwest::Client,  // Cookie를 default_headers에 넣지 않음
    credentials: Option<Credentials>,  // None이면 미인증 (public 쿼리만 가능)
}

impl VelogClient {
    /// 인증된 클라이언트 (CRUD + 토큰 갱신)
    pub fn new(credentials: Credentials) -> anyhow::Result<Self> {
        Ok(Self { http: Self::build_http()?, credentials: Some(credentials) })
    }

    /// 미인증 클라이언트 (post show --username 등 public 쿼리 전용)
    pub fn anonymous() -> anyhow::Result<Self> {
        Ok(Self { http: Self::build_http()?, credentials: None })
    }

    fn build_http() -> anyhow::Result<reqwest::Client> {
        reqwest::Client::builder()
            .https_only(true)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("velog-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(Into::into)
    }

    /// raw_graphql: 단순 HTTP 요청 → GraphQLResponse 그대로 반환 (into_result 미호출)
    /// credentials가 있으면 Cookie 헤더 추가, 없으면 미인증 요청
    /// execute_graphql이 is_auth_error()를 into_result() 전에 체크하기 위해
    /// GraphQLResponse를 그대로 반환한다.
    async fn raw_graphql<V: Serialize, T: DeserializeOwned>(
        &self, query: &'static str, variables: Option<V>,
    ) -> anyhow::Result<GraphQLResponse<T>> {
        let mut req = self.http.post("https://v3.velog.io/graphql")
            .json(&GraphQLRequest { query, variables });

        if let Some(creds) = &self.credentials {
            let mut cookie = HeaderValue::from_str(&format!(
                "access_token={}; refresh_token={}",
                creds.access_token, creds.refresh_token
            ))?;
            cookie.set_sensitive(true);
            req = req.header(COOKIE, cookie);
        }

        let resp = req.send().await?
            .json::<GraphQLResponse<T>>().await?;
        Ok(resp)
    }
}
```

### Research Insights — HTTP Security

**`https_only(true)` (Security Review):**
velog.io에서 HTTP 리다이렉트가 발생하면 reqwest가 따라가면서 Cookie 헤더(JWT 토큰)를 평문으로 전송할 수 있다. `https_only(true)`로 차단.

**`set_sensitive(true)` (Context7 — reqwest):**
Cookie 헤더에 `set_sensitive(true)` 설정 → reqwest 디버그 로그에서 토큰 값 마스킹.

**타임아웃 (Performance + Security):**
- `connect_timeout(10s)`: 네트워크 연결 실패 시 빠른 피드백
- `timeout(30s)`: API 서버 무응답 시 무한 대기 방지

GraphQL 쿼리 상수 (변경 없음 — 원본 plan과 동일):
```rust
const CURRENT_USER_QUERY: &str = r#"{ currentUser { id username email } }"#;
const RESTORE_TOKEN_QUERY: &str = r#"{ restoreToken { accessToken refreshToken } }"#;
const GET_POSTS_QUERY: &str = r#"..."#;   // body 필드 미포함 (성능)
const GET_POST_QUERY: &str = r#"..."#;    // body 필드 포함
const WRITE_POST_MUTATION: &str = r#"..."#;
const EDIT_POST_MUTATION: &str = r#"..."#;
const REMOVE_POST_MUTATION: &str = r#"..."#;
```

> **주의 (Performance Review)**: `GET_POSTS_QUERY`에 `body` 필드를 절대 포함하지 않는다. 포스트당 50-200KB → 20개 리스트에서 1-4MB 불필요한 전송 발생.

API 메서드 (모든 인증 메서드는 `execute_graphql` 경유 → 갱신 시 `Option<Credentials>` 반환):

> **`execute_graphql`의 `T` 파라미터는 `*Data` 래퍼 타입**이다. 각 메서드가 내부에서 래퍼를 unwrap해서 도메인 타입으로 반환.

- `current_user()` → `Result<(User, Option<Credentials>)>`
  - `execute_graphql::<_, CurrentUserData>(CURRENT_USER_QUERY, None)` → `.current_user` 추출
- `restore_token()` → `Result<Credentials>`
  - `raw_graphql::<_, RestoreTokenData>` 직접 호출 → `.into_result()?.restore_token.into()` 변환 (execute_graphql 미경유)
- `get_posts(username, temp_only)` → `Result<(Vec<Post>, Option<Credentials>)>`
  - `execute_graphql::<_, PostsData>` → `.posts` 추출. velog API에 limit 파라미터 없음. cursor 페이지네이션은 Post-MVP
- `get_post(username, url_slug)` → `Result<(Post, Option<Credentials>)>`
  - `execute_graphql::<_, PostData>` → `.post` 추출
- `write_post(input: WritePostInput)` → `Result<(Post, Option<Credentials>)>`
  - `execute_graphql::<_, WritePostData>` → `.write_post` 추출
- `edit_post(input: EditPostInput)` → `Result<(Post, Option<Credentials>)>`
  - `execute_graphql::<_, EditPostData>` → `.edit_post` 추출
- `remove_post(id: &str)` → `Result<(bool, Option<Credentials>)>`
  - `execute_graphql::<_, RemovePostData>` → `.remove_post` 추출. `removePost(id: ID!)` mutation

- [ ] `src/client.rs` 작성 — VelogClient + 2-layer GraphQL + 보안 설정
- [ ] `velog auth status`가 실제 API 호출로 동작 확인

#### Phase 2 완료 기준
- [ ] currentUser 쿼리 성공
- [ ] 만료된 토큰으로 요청 → restoreToken → 재시도 성공 → 핸들러에서 새 토큰 저장 확인
- [ ] posts/post 쿼리 성공

---

### Phase 3: Post 커맨드 (Features)

실제 사용자 기능 구현. 모든 핸들러는 `src/handlers.rs`에 위치.

**파일 상단 imports (handlers.rs):**
```rust
use std::io::Read as _;
use std::path::Path;
use anyhow::Context;  // .context(), .with_context() 메서드 사용에 필요
use colored::Colorize;
use crate::{auth, AuthError};
use crate::auth::Credentials;
use crate::client::VelogClient;
use crate::models::{Post, WritePostInput, EditPostInput};
```

**공통 헬퍼 함수 (handlers.rs 내 private):**
```rust
/// credentials 로드 실패 시 AuthError 반환 (exit code 2)
fn require_auth() -> anyhow::Result<Credentials> {
    auth::load_credentials()?
        .ok_or_else(|| anyhow::Error::new(AuthError).context("Not logged in. Run `velog auth login` first."))
}

/// Option<Credentials>가 Some이면 디스크에 저장
fn maybe_save_creds(creds: Option<Credentials>) -> anyhow::Result<()> {
    if let Some(c) = creds { auth::save_credentials(&c)?; }
    Ok(())
}

/// 파일 경로 또는 stdin에서 마크다운 본문 읽기
fn read_body(file: Option<&Path>) -> anyhow::Result<String> {
    use std::io::IsTerminal;
    match file {
        Some(p) if p == Path::new("-") => {
            // --file - : 명시적 stdin 읽기
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
        Some(p) => {
            // --file <path> : 파일 읽기
            std::fs::read_to_string(p)
                .with_context(|| format!("Cannot read file: {}", p.display()))
        }
        None if !std::io::stdin().is_terminal() => {
            // --file 미지정 + stdin이 pipe → stdin에서 읽기
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
        None => {
            // --file 미지정 + interactive TTY → 에러
            anyhow::bail!("No content source. Provide --file <path>, --file - for stdin, or pipe content.")
        }
    }
}
```

#### 3.1 Post List

```rust
pub async fn post_list(drafts: bool) -> anyhow::Result<()> {
    let creds = require_auth()?;
    let mut client = VelogClient::new(creds)?;
    let (user, new_creds) = client.current_user().await?;
    maybe_save_creds(new_creds)?;
    let (posts, new_creds) = client.get_posts(&user.username, drafts).await?;
    maybe_save_creds(new_creds)?;

    if posts.is_empty() {
        eprintln!("{}", "No posts found.".yellow());  // 메시지 → stderr
        return Ok(());
    }
    print_posts_table(&posts);  // 인라인 display 함수
    Ok(())
}
```

테이블 컬럼: `Title | Status | Tags | Date` (comfy-table + UTF8_FULL + ROUND_CORNERS)

- [ ] `post list` 구현 + 테이블 출력
- [ ] `post list --drafts` 구현

#### 3.2 Post Show

- **username 우선순위**: `--username` 플래그가 있으면 항상 우선 (인증 상태 무관). 없으면 인증된 사용자의 username 사용. 둘 다 없으면 에러.
- `--username` 제공 시: 인증 불필요, `VelogClient::anonymous()` 사용 가능
- 인증된 상태에서 `--username` 없으면: `current_user()` → username 획득 후 `get_post`
- 미인증 + `--username` 없으면 에러: "Username required. Use --username or log in first."

```rust
pub async fn post_show(slug: &str, username: Option<&str>) -> anyhow::Result<()> {
    let (post, new_creds) = if let Some(uname) = username {
        // --username 제공 → 항상 이 username 사용 (인증 불필요)
        let mut client = match auth::load_credentials()? {
            Some(c) => VelogClient::new(c)?,
            None => VelogClient::anonymous()?,
        };
        let (post, creds) = client.get_post(uname, slug).await?;
        (post, creds)
    } else {
        // --username 없음 → 인증 필수
        let creds = require_auth()?;
        let mut client = VelogClient::new(creds)?;
        let (user, new_creds) = client.current_user().await?;
        maybe_save_creds(new_creds)?;
        let (post, new_creds) = client.get_post(&user.username, slug).await?;
        (post, new_creds)
    };
    maybe_save_creds(new_creds)?;
    print_post_detail(&post);
    Ok(())
}
```

### Research Insights — Markdown Rendering

**termimad (Best Practices):**
```rust
use termimad::MadSkin;

fn print_post_detail(post: &Post) {
    // 메타 정보 (colored)
    println!("{}", post.title.bold());
    if let Some(tags) = &post.tags {
        println!("Tags: {}", tags.join(", ").cyan());
    }
    println!();
    // 본문 (termimad — 터미널 폭 자동 인식)
    if let Some(body) = &post.body {
        let skin = MadSkin::default();
        skin.print_text(body);
    }
}
```

- [ ] `post show <slug>` 구현 (termimad 마크다운 렌더링)
- [ ] `--username` 플래그로 타인 포스트 조회

#### 3.3 Post Create

- 파일 읽기: `std::fs::read_to_string` (동기 — 블로그 포스트 크기에서 async 불필요)
- **stdin pipe 지원**: `--file -` 또는 `--file` 미지정 + stdin이 TTY가 아닌 경우 stdin에서 읽기 (`std::io::IsTerminal`, Rust 1.70+)
  - 예: `cat post.md | velog post create --title "T"` 또는 `velog post create --title "T" --file -`
- **파일 경로 검증 (Security)**: canonicalize + is_file 확인, .md 확장자 경고
- url_slug 생성: `slug::slugify(&title)` (한글 → 로마자 자동 변환)
  - 결과가 비어있으면 `post-{unix_timestamp}` 폴백
  - `--slug` 오버라이드 시 정규식 검증: `^[a-z0-9]+(?:-[a-z0-9]+)*$`
- tags 파싱: 쉼표 구분, trim, 빈 문자열 필터
- writePost 호출: `is_temp = !publish`, `is_private = private`, `meta = {}`
- 성공 시 포스트 URL + slug 출력

### Research Insights — Slug Generation

**`slug` 크레이트 (Best Practices):**
```rust
use slug::slugify;

let title = "Rust로 velog CLI 만들기";
let s = slugify(title);  // → "reoseuteulo-velog-cli-mandeulgi"

let english = "My First Blog Post!";
let s = slugify(english);  // → "my-first-blog-post"
```

`slug` 크레이트는 내부적으로 `deunicode`를 사용하여 한글/중국어/일본어 등을 로마자로 변환. velog 자체가 ASCII slug를 사용하므로 적합.

```rust
pub async fn post_create(
    file: Option<&Path>, title: &str, tags: &str,
    slug_override: Option<&str>, publish: bool, private: bool,
) -> anyhow::Result<()> {
    let creds = require_auth()?;
    let mut client = VelogClient::new(creds)?;

    let body = read_body(file)?;  // 공통 헬퍼: 4-way 분기 (파일/stdin/-/에러)
    anyhow::ensure!(!body.trim().is_empty(), "Post body is empty");

    // slug 생성
    let url_slug = match slug_override {
        Some(s) => {
            // 정규식 검증 + 길이 제한
            anyhow::ensure!(s.len() <= 255, "Slug too long (max 255 chars)");
            anyhow::ensure!(
                s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                    && !s.starts_with('-') && !s.ends_with('-') && !s.contains("--"),
                "Invalid slug: only lowercase alphanumeric and hyphens allowed (e.g. 'my-first-post')"
            );
            s.to_string()
        }
        None => {
            let s = slug::slugify(title);
            if s.is_empty() {
                format!("post-{}", std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?.as_secs())
            } else { s }
        }
    };

    // tags 파싱: 쉼표 구분, trim, 빈 문자열 필터
    let tag_list: Vec<String> = tags.split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    let input = WritePostInput {
        title: title.to_string(), body, tags: tag_list,
        is_markdown: true, is_temp: !publish, is_private: private,
        url_slug: url_slug.clone(), thumbnail: None,
        meta: serde_json::json!({}), series_id: None,
    };

    let (user, new_creds) = client.current_user().await?;
    maybe_save_creds(new_creds)?;
    let (_post, new_creds) = client.write_post(input).await?;
    maybe_save_creds(new_creds)?;

    let status = if publish { "Published" } else { "Saved as draft" };
    eprintln!("{}: {}", status.green(), title);
    println!("https://velog.io/@{}/{}", user.username, url_slug);
    Ok(())
}
```

- [ ] `post create` 구현 (파일 검증 + slug 생성 + writePost)
- [ ] 성공 출력: `https://velog.io/@{username}/{slug}` + post slug

#### 3.4 Post Edit

- `current_user()` → username 획득 (velog API가 `post(username, url_slug)` 형식이므로 필수)
- `get_post(username, slug)` → 기존 포스트 fetch
- CLI 플래그로 전달된 필드만 오버라이드, 나머지는 서버 값 유지
- `--file` 있으면 body 교체, 없으면 기존 body 유지
- **tags merge**: `None` = 서버값 유지, `Some("")` = 태그 전부 삭제, `Some("a,b")` = 교체
- editPost 호출 (모든 필드 전달 필수)
- 성공 시 포스트 URL 출력

```rust
pub async fn post_edit(
    slug: &str, file: Option<&Path>, title: Option<&str>, tags: Option<&str>,
) -> anyhow::Result<()> {
    let creds = require_auth()?;
    let mut client = VelogClient::new(creds)?;
    let (user, new_creds) = client.current_user().await?;
    maybe_save_creds(new_creds)?;
    let (existing, new_creds) = client.get_post(&user.username, slug).await?;
    maybe_save_creds(new_creds)?;

    // merge: CLI 플래그 Some → 오버라이드, None → 서버값 유지
    let new_body = match file {
        Some(p) => read_body(Some(p))?,
        None => existing.body.unwrap_or_default(),
    };
    let new_title = title.map(String::from)
        .unwrap_or(existing.title);
    // tags: None → 서버값 유지, Some("") → 빈 배열, Some("a,b") → 파싱
    let new_tags = match tags {
        Some(t) => t.split(',').map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()).collect(),
        None => existing.tags.unwrap_or_default(),
    };

    let input = EditPostInput {
        id: existing.id,
        title: new_title, body: new_body, tags: new_tags,
        is_markdown: true, is_temp: existing.is_temp,
        is_private: existing.is_private, url_slug: existing.url_slug.clone(),
        thumbnail: existing.thumbnail, meta: serde_json::json!({}),
        series_id: None,
    };
    let (_post, new_creds) = client.edit_post(input).await?;
    maybe_save_creds(new_creds)?;

    eprintln!("{}", "Post updated.".green());
    println!("https://velog.io/@{}/{}", user.username, existing.url_slug);
    Ok(())
}
```

- [ ] `post edit <slug>` 구현 (기존 포스트 fetch → merge → editPost)

#### 3.5 Post Delete

- `current_user()` → username 획득
- `get_post(username, slug)` → 제목 확인
- **TTY 가드**: `--yes` 없이 stdin이 pipe면 즉시 에러 반환 (무한 대기 방지)
- `--yes` 없으면 확인 프롬프트: `"Delete post '{title}'? This cannot be undone. [y/N]"`
- removePost 호출 (Post ID 사용 — get_post에서 획득)
- 성공 시 "Post deleted." 메시지

```rust
pub async fn post_delete(slug: &str, yes: bool) -> anyhow::Result<()> {
    let creds = require_auth()?;
    let mut client = VelogClient::new(creds)?;
    let (user, new_creds) = client.current_user().await?;
    maybe_save_creds(new_creds)?;
    let (post, new_creds) = client.get_post(&user.username, slug).await?;
    maybe_save_creds(new_creds)?;

    if !yes {
        use std::io::IsTerminal;
        if !std::io::stdin().is_terminal() {
            anyhow::bail!("Refusing to delete in non-interactive mode. Use --yes to confirm.");
        }
        eprint!("Delete post '{}'? This cannot be undone. [y/N] ", post.title);
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if answer.trim().to_lowercase() != "y" {
            eprintln!("Aborted.");
            return Ok(());
        }
    }

    let (_ok, new_creds) = client.remove_post(&post.id).await?;
    maybe_save_creds(new_creds)?;
    eprintln!("{}", "Post deleted.".green());
    Ok(())
}
```

- [ ] `post delete <slug>` 구현 (TTY 가드 + 확인 프롬프트 + removePost)
- [ ] `--yes` 플래그로 프롬프트 생략

#### 3.6 Post Publish

- `current_user()` → username 획득
- `get_post(username, slug)` → `is_temp` 확인
- 이미 발행된 포스트면 "Post is already published." + exit 0
- editPost(is_temp=false) 호출 (기존 필드 모두 유지)
- 성공 시 포스트 URL 출력

- [ ] `post publish <slug>` 구현 (draft → published 전환)

#### Phase 3 완료 기준
- [ ] 모든 CRUD 커맨드가 실제 velog API와 동작
- [ ] 에러 메시지가 사용자 친화적 (다음 행동 안내 포함)
- [ ] slug 생성이 한글/영문 제목 모두에서 동작

---

### Phase 4: 마무리 (Polish)

#### 4.1 최종 테스트 + 빌드 확인

- [ ] `cargo clippy` 경고 0개
- [ ] `cargo build --release` 성공
- [ ] 모든 커맨드 수동 테스트 (실제 velog 계정)
- [ ] 만료 토큰 → 자동 갱신 → 재시도 흐름 확인
- [ ] 네트워크 에러 시 사용자 메시지 확인

#### Phase 4 완료 기준
- [ ] 깔끔한 터미널 출력 (테이블, 색상, 마크다운 렌더링)
- [ ] clippy 경고 없음
- [ ] release 빌드 성공

---

## Alternative Approaches Considered

| 접근 방식 | 기각 사유 |
|-----------|-----------|
| graphql_client 코드젠 | 쿼리 <10개, 빌드 복잡도 대비 이점 없음 (see brainstorm) |
| 이메일 매직링크 인증 | GitHub OAuth 전용 사용자 미지원 가능성 (see brainstorm) |
| YAML frontmatter | 순수 마크다운 파일 유지 선호 (see brainstorm) |
| $EDITOR 통합 | 파일 경로 지정이 스크립트 연동에 유리 (see brainstorm) |
| thiserror + anyhow 혼용 | 바이너리 크레이트에 타입 에러 불필요 (Simplicity Review) |
| tokio "full" | CLI에 멀티스레드/fs/net 불필요 (Performance Review) |
| `post edit <id>` | velog API에 ID 직접 조회 없음 → slug 기반 (Architecture Review) |

## Acceptance Criteria

### Functional Requirements

- [ ] `velog auth login` — 숨김 입력 + JWT claim 검증 + 안전하게 저장
- [ ] `velog auth status` — 현재 사용자 정보 (username) 출력
- [ ] `velog auth logout` — 저장된 credentials + temp 파일 삭제
- [ ] `velog post list` — 발행된 포스트 테이블 출력 (body 미포함)
- [ ] `velog post list --drafts` — 임시저장 포스트 테이블 출력
- [ ] `velog post show <slug>` — termimad로 마크다운 렌더링 출력
- [ ] `velog post create --file <path> --title "T" --tags "a,b"` — 포스트 생성 (기본: 임시저장)
- [ ] `velog post create --file <path> --title "T" --publish` — 포스트 즉시 발행
- [ ] `velog post edit <slug> --file <path>` — 포스트 수정 (미지정 필드 서버값 유지)
- [ ] `velog post delete <slug>` — 확인 후 삭제
- [ ] `velog post publish <slug>` — 임시저장 → 발행 전환
- [ ] 토큰 만료 시 restoreToken → 자동 갱신 → 핸들러에서 디스크 저장
- [ ] credentials.json 파일 권한 0600, 디렉토리 0700
- [ ] `velog completions bash/zsh/fish` — shell 자동완성 스크립트 생성
- [ ] `cat post.md | velog post create --title "T"` — stdin pipe로 포스트 생성

### Non-Functional Requirements

- [ ] `cargo clippy` 경고 0개
- [ ] `cargo build --release` 성공
- [ ] 에러 메시지가 사용자 친화적 (다음 행동 안내 포함)
- [ ] 종료 코드 규약 준수 (0=성공, 1=일반, 2=인증)
- [ ] `https_only(true)` + `rustls-tls` + 타임아웃 설정
- [ ] stderr/stdout 분리 (메시지→stderr, 데이터→stdout)
- [ ] `NO_COLOR=1` 설정 시 색상 비활성화

## Dependencies & Prerequisites

- Rust toolchain (stable, 2021 edition)
- velog.io 계정 + 브라우저에서 쿠키 추출 가능
- 인터넷 연결 (v3.velog.io 접근)

## Risk Analysis & Mitigation

| 리스크 | 확률 | 영향 | 완화 |
|--------|------|------|------|
| velog API 스키마 변경 | 중 | 높음 | 수동 쿼리로 빠른 대응 가능 |
| v3.velog.io rate limiting | 낮 | 중 | CLI 특성상 요청 빈도 낮음 |
| 한글 slug 생성 품질 | 중 | 낮 | `slug` 크레이트(deunicode) + `--slug` 오버라이드 |
| Cloudflare 방어 차단 | 낮 | 높음 | User-Agent 설정, 필요 시 조사 |
| GraphQL 인증 에러 감지 | 중 | 중 | is_auth_error()로 메시지+extensions 확인 |
| 토큰 갱신 무한 루프 | 낮 | 높음 | 2-layer 구조: restore_token()은 raw_graphql만 호출 |

## Post-MVP Enhancements

리뷰와 참조 CLI 비교에서 발견된 추가 개선사항 (MVP 이후):

**P0 — 첫 번째 업데이트에 포함:**

| 개선 | 사유 | 출처 |
|------|------|------|
| `velog pull [slug]` | 원격 포스트를 로컬 .md 파일로 다운로드. 기존 글 관리의 시작점 | blogsync, devto-cli |
| `--json` 출력 모드 | `velog post list --json \| jq ...` 스크립트 연동 필수 | gh CLI, devto-cli, clig.dev |
| `--dry-run` (create/edit) | 실제 API 호출 없이 변경 내용 미리보기. 사용자 신뢰도 | devto-cli, wp-cli, clig.dev |

**P1 — 기능 확장:**

| 개선 | 사유 | 출처 |
|------|------|------|
| `velog new "Title"` | 프론트매터 + 템플릿으로 .md 파일 scaffold | devto-cli, jekyll-compose |
| `-v`/`-vv`/`-q` 로깅 | `clap-verbosity-flag` + stderr. 디버깅/스크립트 필수 | Rust CLI 표준, twitter-cli |
| `velog open <slug>` | 시스템 브라우저에서 포스트 열기 (`open` / `xdg-open`) | gh CLI `--web`, medium-cli |
| TOML config 파일 | `~/.config/velog-cli/config.toml` 기본값 설정 (기본 태그, 색상 등) | blogsync, clig.dev |
| `--version` git hash | `shadow-rs` 크레이트로 빌드 시 commit hash 포함 | Rust CLI 표준 |
| Ctrl+C 시그널 처리 | `tokio::signal::ctrl_c()` + exit code 130 | Rust CLI Book |

**P2 — 보안 강화 + 안정성:**

| 개선 | 사유 | 출처 |
|------|------|------|
| `zeroize` 크레이트 | 프로세스 메모리에서 토큰 제로화 | Security Review |
| `fd-lock` 파일 락 | 동시 실행 시 토큰 갱신 경쟁 방지 | Security Review |
| `tempfile` 크레이트 | atomic write + 자동 정리 | Security Review |
| `keyring` 크레이트 | OS 네이티브 키체인 연동 | Best Practices |
| 업데이트 알림 | `update-informer` 크레이트 — 24h 캐시 | Rust CLI 표준 |
| man page 생성 | `clap_mangen` — 패키징 시 필요 | Rust CLI 표준 |

**P3 — 장기 로드맵:**

| 개선 | 사유 | 출처 |
|------|------|------|
| 이메일 매직링크 로그인 | 브라우저 불필요한 인증 | Brainstorm |
| 시리즈 관리 | series_id 지정/목록 | Brainstorm |
| 이미지 업로드 | /api/v2/files/upload 연동 | Brainstorm |
| `velog search <keyword>` | 내 포스트 키워드 검색 | medium-cli |
| cursor 페이지네이션 | `--cursor` / `--page` 플래그로 다음 페이지 | blogsync, twitter-cli |

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-10-velog-cli-brainstorm.md](docs/brainstorms/2026-03-10-velog-cli-brainstorm.md)
  - Key decisions: 수동 쿠키 인증, 수동 GraphQL 쿼리 + serde, CLI 플래그 기반 메타데이터, XDG 규약

### Research Sources (Deepen)

- **Architecture Review:** client-auth 분리, Cargo.lock 커밋, rpassword v7, error flow 설계
- **Security Review:** https_only, rustls-tls, JWT claim 검증, slug 검증, 무조건 logout
- **Performance Review:** tokio current_thread, Client 재사용, body 미포함 list, release 프로필
- **Code Simplicity Review:** 12→6 파일, thiserror 제거, lib.rs/config.rs/display.rs 흡수
- **Best Practices Research:** slug 크레이트, termimad, GraphQL envelope 패턴, into_result()
- **Context7 (clap):** derive subcommand 패턴, custom value parser, flatten
- **Context7 (reqwest):** default_headers, set_sensitive, cookies feature

### External References

- [velog-server](https://github.com/velopert/velog-server) — API 구현 참고
- [velog-client](https://github.com/velopert/velog-client) — GraphQL 쿼리 패턴 참고
- [twitter-cli](https://github.com/jackwener/twitter-cli) — CLI 아키텍처 패턴 참고 (--json, --verbose, stderr/stdout 분리)
- GraphQL endpoint: `POST https://v3.velog.io/graphql`
- REST auth: `https://api.velog.io/api/v2/auth/`

### CLI 비교 분석 (Review Round 3)

- [blogsync](https://github.com/x-motemen/blogsync) — 양방향 pull/push 모델, YAML config
- [devto-cli](https://github.com/sinedied/devto-cli) — --dry-run, --reconcile, --pull, stats
- [jekyll-compose](https://github.com/jekyll/jekyll-compose) — new/publish/unpublish 워크플로우
- [gh CLI](https://cli.github.com/manual/) — --json, --jq, --web, completions (gold standard)
- [wp-cli](https://developer.wordpress.org/cli/commands/post/) — --format=table|json|csv, bulk ops
- [clig.dev](https://clig.dev/) — CLI UX 가이드라인 (stderr/stdout, NO_COLOR, --dry-run)
- [Rust CLI Book](https://rust-cli.github.io/book/) — signal handling, exit codes, error context
- [Rain's Rust CLI Recommendations](https://rust-cli-recommendations.sunshowers.io/) — NO_COLOR, dirs, config hierarchy
