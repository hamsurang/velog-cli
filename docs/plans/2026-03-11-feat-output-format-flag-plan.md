---
title: "feat: Add --format compact|pretty|silent global flag"
type: feat
status: completed
date: 2026-03-11
origin: docs/brainstorms/2026-03-11-output-format-brainstorm.md
---

# feat: Add --format compact|pretty|silent global flag

## Overview

velog CLI에 `--format` 글로벌 플래그를 추가하여 AI 에이전트 및 스크립트 파이프라인에서 토큰 효율적이고 파싱 가능한 출력을 제공한다. 현재 human-friendly 출력(테이블, 색상, 마크다운 렌더링)은 `--format pretty`로 유지한다.

(see brainstorm: docs/brainstorms/2026-03-11-output-format-brainstorm.md)

## Problem Statement

현재 velog CLI 출력은 comfy-table, colored, termimad를 통한 human-friendly 형태만 지원. AI 에이전트가 이 출력을 파싱하려면 불필요한 토큰을 소모하고 파싱 오류 가능성이 높다.

## Proposed Solution

3가지 출력 모드를 글로벌 플래그로 제공:

| 모드 | 기본값 | stdout | stderr |
|---|---|---|---|
| `compact` | **예** | minified JSON | JSON |
| `pretty` | | 테이블/색상/마크다운 (현재) | 색상 텍스트 (현재) |
| `silent` | | 조회=compact, mutation=없음 | 에러만 JSON |

## Technical Approach

### 파일 변경 맵

| 파일 | 변경 내용 |
|---|---|
| `src/cli.rs` | `Format` enum 추가, `Cli`에 `--format` 글로벌 플래그 추가 |
| `src/output.rs` | **신규** — format-aware 출력 헬퍼 함수 |
| `src/models.rs` | `Post`, `PostUser`, `User`에 `Serialize` derive 추가 |
| `src/handlers.rs` | 모든 핸들러에 `format: Format` 파라미터 추가, 출력 분기 |
| `src/main.rs` | `cli.format` 추출 → 핸들러 전달, 에러 핸들러 format-aware |
| `src/auth.rs` | JWT 경고 메시지 format-aware |
| `src/lib.rs` | `pub mod output;` 추가 |
| `tests/cli_tests.rs` | compact/silent 통합 테스트 추가 |

### Compact 출력 스키마

**post list (stdout):**
```json
[{"title":"My Post","slug":"my-post","status":"pub","tags":["rust"],"date":"2026-03-10"}]
```
- JSON 배열 (minified). 빈 리스트 = `[]`
- `status` 값: `"pub"` | `"draft"` | `"priv"`
- `date`: ISO 날짜 10자리 (YYYY-MM-DD)

**post show (stdout):**
```json
{"title":"My Post","slug":"my-post","status":"pub","tags":["rust"],"date":"2026-03-10","body":"markdown content..."}
```
- body 포함 (명령의 주 목적)

**mutation 성공 — create/edit/publish (stdout):**
```json
{"url":"https://velog.io/@username/my-post"}
```

**mutation 성공 (stderr):**
```json
{"status":"ok","msg":"Post created"}
```

**post delete:** compact/silent 모드에서 `--yes` 자동 적용 (대화형 프롬프트 없음)

**auth status (stdout):**
```json
{"logged_in":true,"username":"gimminsu"}
```

**auth login:** compact/silent 모드 시 에러 반환 (대화형 필수)
```json
{"error":"auth login requires --format pretty (interactive mode)","exit_code":1}
```

**auth logout (stderr):**
```json
{"status":"ok","msg":"Logged out"}
```

**에러 (stderr, compact+silent 공통):**
```json
{"error":"Not authenticated","exit_code":2}
```

**JWT 경고 (stderr, compact만. silent에서는 제거):**
```json
{"warning":"access_token token is expired. It will be refreshed automatically."}
```

### Silent 모드 동작 요약

- **조회 명령** (post list, post show, auth status): compact와 동일
- **mutation 명령** (create, edit, delete, publish, auth logout): stdout 없음, exit code만
- **에러**: stderr JSON 유지 (compact과 동일)
- **JWT 경고**: 제거

### 제외 명령

`velog completions <shell>` — format 플래그 영향 없음 (main.rs에서 early return 전에 이미 처리)

## Implementation Phases

### Phase 1: Foundation (`src/cli.rs`, `src/models.rs`, `src/lib.rs`)

**`src/cli.rs`:**
```rust
#[derive(clap::ValueEnum, Clone, Copy, Debug, Default, PartialEq)]
pub enum Format {
    #[default]
    Compact,
    Pretty,
    Silent,
}

#[derive(Parser)]
#[command(name = "velog", about = "...")]
pub struct Cli {
    #[arg(long, global = true, value_enum, default_value_t = Format::Compact)]
    pub format: Format,

    #[command(subcommand)]
    pub command: Commands,
}
```

**`src/models.rs`:**

compact 출력 전용 타입 추가. `Post`의 `is_temp: bool` + `is_private: bool` → `CompactStatus` 단일 필드로 합성:

```rust
// --- Compact 출력 전용 타입 ---

#[derive(Serialize)]
pub enum CompactStatus {
    #[serde(rename = "pub")]
    Published,
    #[serde(rename = "draft")]
    Draft,
    #[serde(rename = "priv")]
    Private,
}

#[derive(Serialize)]
pub struct CompactPost {
    pub title: String,
    pub slug: String,
    pub status: CompactStatus,
    pub tags: Vec<String>,
    pub date: String,
}

#[derive(Serialize)]
pub struct CompactPostDetail {
    #[serde(flatten)]
    pub post: CompactPost,
    pub body: String,
}

#[derive(Serialize)]
pub struct CompactAuthStatus {
    pub logged_in: bool,
    pub username: String,
}

#[derive(Serialize)]
pub struct CompactMutationResult {
    pub url: String,
}

#[derive(Serialize)]
pub struct CompactMessage {
    pub status: String,
    pub msg: String,
}

#[derive(Serialize)]
pub struct CompactError {
    pub error: String,
    pub exit_code: i32,
}

#[derive(Serialize)]
pub struct CompactWarning {
    pub warning: String,
}

impl From<&Post> for CompactPost {
    fn from(post: &Post) -> Self {
        let status = if post.is_temp {
            CompactStatus::Draft
        } else if post.is_private {
            CompactStatus::Private
        } else {
            CompactStatus::Published
        };
        CompactPost {
            title: post.title.clone(),
            slug: post.url_slug.clone(),
            status,
            tags: post.tags.as_ref().map(|t| /* tag names */).unwrap_or_default(),
            date: post.released_at.as_deref().map(|d| d[..10].to_string()).unwrap_or_default(),
        }
    }
}
```

Note: `date` 필드에서 `&d[..10]`은 ISO 날짜 문자열이므로 ASCII-safe. institutional learnings의 UTF-8 바이트 슬라이싱 경고는 해당 없음 (날짜는 항상 ASCII).

**`src/lib.rs`:**
- `pub mod output;` 추가

### Phase 2: Output 헬퍼 (`src/output.rs` — 신규)

compact/silent 모드의 JSON 출력만 담당. pretty 출력은 handlers.rs의 기존 `print_posts_table`, `print_post_detail` 함수를 그대로 유지:

```rust
// src/output.rs — compact/silent 전용
use crate::cli::Format;
use crate::models::*;

/// compact/silent 조회 데이터를 stdout JSON으로 출력
pub fn emit_data<T: Serialize>(format: Format, value: &T) { ... }

/// compact mutation 결과를 stdout JSON으로 출력. silent이면 출력 없음
pub fn emit_mutation_result(format: Format, url: &str) { ... }

/// compact 성공 메시지를 stderr JSON으로 출력. silent이면 출력 없음
pub fn emit_ok(format: Format, msg: &str) { ... }

/// compact 경고를 stderr JSON으로 출력. silent이면 출력 없음
pub fn emit_warning(format: Format, msg: &str) { ... }

/// compact/silent 에러를 stderr JSON으로 출력 (main.rs에서 사용)
pub fn emit_error(format: Format, msg: &str, exit_code: i32) { ... }
```

핸들러에서의 사용 패턴:
```rust
// handlers.rs
match format {
    Format::Pretty => {
        print_posts_table(&posts); // 기존 함수 그대로
    }
    _ => {
        let compact: Vec<CompactPost> = posts.iter().map(CompactPost::from).collect();
        output::emit_data(format, &compact);
    }
}
```

### Phase 3: 핸들러 마이그레이션 (`src/handlers.rs`)

모든 public 핸들러 시그니처에 `format: Format` 추가:

```rust
pub async fn post_list(client: &VelogClient, drafts: bool, format: Format) -> Result<()>
pub async fn post_show(client: &VelogClient, slug: &str, username: Option<&str>, format: Format) -> Result<()>
pub async fn post_create(client: &VelogClient, args: ..., format: Format) -> Result<()>
pub async fn post_edit(client: &VelogClient, args: ..., format: Format) -> Result<()>
pub async fn post_delete(client: &VelogClient, slug: &str, yes: bool, format: Format) -> Result<()>
pub async fn post_publish(client: &VelogClient, slug: &str, format: Format) -> Result<()>
pub async fn auth_login(format: Format) -> Result<()>
pub async fn auth_status(client: &VelogClient, format: Format) -> Result<()>
pub async fn auth_logout(format: Format) -> Result<()>
```

**핸들러별 변경:**

| 핸들러 | compact 변경 | silent 변경 |
|---|---|---|
| `post_list` | `emit_data(format, json_array)` | compact과 동일 |
| `post_show` | `emit_data(format, json_obj)` | compact과 동일 |
| `post_create` | `emit_mutation_result` + `emit_ok` | stdout/stderr 없음 |
| `post_edit` | `emit_mutation_result` + `emit_ok` | stdout/stderr 없음 |
| `post_delete` | `--yes` 자동 적용, 프롬프트 없음 | stdout/stderr 없음 |
| `post_publish` | `emit_mutation_result` + `emit_ok` | stdout/stderr 없음 |
| `auth_login` | 에러 반환 (interactive 필수) | 에러 반환 |
| `auth_status` | `emit_data(format, json)` | compact과 동일 |
| `auth_logout` | `emit_ok` | stdout/stderr 없음 |

**`post_list` 빈 결과 처리:**
- compact/silent: `[]` 출력 (stdout), 경고 메시지 없음
- pretty: `"No posts found."` (stderr, 현재 동작 유지)

**`post_delete` compact 동작:**
- `format != Format::Pretty`일 때 `yes = true` 강제 적용
- 확인 프롬프트 건너뛰고 바로 삭제

**`post_publish` "already published" 처리:**
- compact: `emit_mutation_result` + `emit_ok("Already published")`. exit code 0
- silent: 출력 없음, exit code 0

### Phase 4: 에러 핸들링 (`src/main.rs`, `src/auth.rs`)

**`src/main.rs`:**
```rust
let format = cli.format;
// ... 기존 dispatch에 format 전달 ...

// 에러 핸들러 변경 (line 58-62)
Err(e) => {
    let exit_code = if e.chain().any(|c| c.downcast_ref::<AuthError>().is_some()) { 2 } else { 1 };
    output::emit_error(format, &format!("{:#}", e), exit_code);
    std::process::exit(exit_code);
}
```

**`src/auth.rs` JWT 경고:**
- `validate_velog_jwt` 함수에 `format: Format` 파라미터 추가
- 또는 `emit_warning` 호출을 위해 format을 전달받는 wrapper 사용
- silent: 경고 제거, compact: JSON stderr, pretty: 현재 eprintln! 유지

### Phase 5: 테스트 (`tests/cli_tests.rs`)

통합 테스트 추가:

```rust
// compact 기본 동작 확인
#[test] fn compact_post_list_outputs_json_array() { ... }
#[test] fn compact_post_list_empty_outputs_empty_array() { ... }
#[test] fn compact_post_show_outputs_json_object() { ... }
#[test] fn compact_mutation_outputs_url_json() { ... }

// pretty 모드 확인 (기존 동작 보존)
#[test] fn pretty_post_list_outputs_table() { ... }

// silent 모드 확인
#[test] fn silent_mutation_no_stdout() { ... }
#[test] fn silent_query_same_as_compact() { ... }

// 엣지 케이스
#[test] fn compact_auth_login_returns_error() { ... }
#[test] fn compact_delete_auto_yes() { ... }
#[test] fn format_flag_ignored_for_completions() { ... }
```

## System-Wide Impact

- **Error propagation**: 에러 체인 구조 변경 없음. 마지막 단계(main.rs)에서만 출력 형태 변경
- **API surface parity**: 모든 명령에 동일하게 적용 (completions 제외)
- **State lifecycle**: 상태 변경 없음. 순수 출력 레이어 변경
- **Backward compatibility**: 기본값이 `compact`로 변경되므로 **breaking change**. 기존 사용자는 `--format pretty` 또는 alias 설정 필요

## Acceptance Criteria

- [x] `--format compact`가 기본값으로 동작하며 모든 stdout이 valid JSON
- [x] `--format pretty`가 현재 동작과 100% 동일
- [x] `--format silent`에서 mutation stdout이 비어있고 exit code로만 결과 전달
- [x] `--format silent`에서 조회 명령은 compact과 동일한 JSON 출력
- [x] compact 에러 출력이 `{"error":"...","exit_code":N}` 형태
- [x] `post list` 빈 결과가 compact에서 `[]` 출력
- [x] `post delete --format compact`에서 `--yes` 없이도 프롬프트 없이 삭제
- [x] `auth login --format compact`에서 에러 반환
- [x] `completions` 명령이 format 플래그에 영향받지 않음
- [x] `cargo test` 전체 통과 (46 유닛 + 20 통합)
- [x] `cargo clippy` 경고 없음

## Dependencies & Risks

- **Breaking change**: 기본값 변경 (human-friendly → compact). README에 마이그레이션 가이드 필요
- **serde_json**: 이미 의존성에 존재 (`Cargo.toml` line 33). 추가 의존성 없음
- **auth.rs 수정**: JWT 경고에 format 전달 시 `validate_velog_jwt` 시그니처 변경 필요. 호출 체인 확인 필요

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-11-output-format-brainstorm.md](docs/brainstorms/2026-03-11-output-format-brainstorm.md)
  - Key decisions: compact 기본값, JSON 배열 형식, 축약 값(status→pub/draft/priv), silent=조회 compact/mutation 무출력

### Internal References

- CLI 구조: `src/cli.rs:9-14` (Cli struct)
- 출력 함수: `src/handlers.rs:348-402` (print_posts_table, print_post_detail)
- 에러 핸들링: `src/main.rs:58-62`
- JWT 경고: `src/auth.rs:139-148`
- 모델: `src/models.rs` (Post, PostUser, User)

### Institutional Learnings

- **UTF-8 안전**: `.chars().take(n)` 사용, 바이트 슬라이싱 금지 (docs/solutions/logic-errors/multi-agent-code-review-p1-p2-fixes.md)
- **에러 컨텍스트 보존**: `map_err(|_|)` 금지, 원본 에러 포함 필수
- **TTY 가드**: stdin 프롬프트에 `IsTerminal` 체크 — compact 모드에서는 프롬프트 자체를 건너뜀
- **정보 노출 방지**: 외부 API 응답을 사용자 메시지에 그대로 포함하지 않기 (200자 제한)
