---
title: "fix: CLI UX improvements from E2E test report"
type: fix
status: active
date: 2026-03-13
deepened: 2026-03-13
---

# fix: CLI UX Improvements from E2E Test Report

## Enhancement Summary

**Deepened on:** 2026-03-13
**Technical review:** 2026-03-13 (Architecture, Simplicity, Security — 3 reviewers)
**Sections enhanced:** 4 (Phase 0–3, originally 5 phases merged to 4)
**Research conducted:** GraphQL introspection, clap try_parse patterns, validation ordering, repo conventions

### Key Improvements from Research
1. **Phase 0 — API 스키마 확정:** `searchPosts`가 `SearchResult { count: Int!, posts: [Post!]! }`를 반환함을 introspection으로 확인.
2. **Phase 2 — validate_slug 호환성 위험 발견:** 기존 `validate_slug()`는 ASCII만 허용하지만 velog slug에 한글이 포함될 수 있음. `validate_slug_nonempty()` 사용 권장.
3. **Phase 1 — `e.render()` 안전성:** `e.use_stderr()`로 충분함을 확인.

### Technical Review Findings (반영 완료)
1. **[Security HIGH] Phase 1 false positive 수정:** `args.any(|a| a == "compact")` → `detect_format_from_raw_args()` 함수로 `--format` + 값 쌍만 스캔. `velog search "compact"` 같은 키워드가 매칭되는 버그 방지.
2. **[Architecture] Phase 2 누락 핸들러 추가:** `post_like`/`post_unlike`에도 `validate_slug_nonempty` 추가.
3. **[Security MEDIUM] Phase 0 정수 안전성:** `has_more` 계산에 `count.max(0)` + `saturating_add` 사용.
4. **[Simplicity] Phase 3+4 병합:** 동일 파일(`cli.rs`) 수정이므로 단일 "Help text improvements" phase로 통합. `after_long_help`만 사용하여 중복 제거.
5. **[Security LOW] Phase 2 whitespace 검증:** `validate_slug_nonempty`에 `s.trim() == s` 검증 추가.

### Out of Scope (별도 추적)
- `highlight_match` Unicode byte-index panic (기존 버그, Phase 0과 별도)
- `emit_error` `.expect()` → fallback 패턴 (낮은 우선순위)
- `cargo audit` CI 통합

---

## Overview

CLI UX E2E 테스트 (`.ux-test-results/report.md`)에서 발견된 1 Critical, 2 Fail, 4 Warning 이슈를 수정한다. search 명령어 복구, compact 모드 에러 포맷 통일, 유효성 검증 순서 정비, help 텍스트 개선을 포함한다.

## Problem Statement / Motivation

- `velog search`가 GraphQL 스키마 불일치로 **완전히 미작동** (Critical)
- compact/silent 모드에서 clap 에러가 plain text로 출력되어 **머신 파싱 실패** (Fail)
- 빈 slug 입력 시 validation 대신 auth 에러가 표시되어 **사용자 혼란** (Fail)
- 상호 배타 플래그가 help에 미표시, 복잡 명령어에 Examples 없음 (Warning/New)

## Proposed Solution

4개 Phase로 나누어 순차적으로 수정한다. 각 Phase는 독립적이며 개별 커밋 가능. (Phase 0→1→2 순서 권장: compact 에러 래핑이 validation 에러에도 적용되도록)

## Technical Approach

### Phase 0: Search GraphQL 스키마 수정 [Critical]

**파일:** `src/client/search.rs`, `src/models/search.rs`, `src/handlers/search.rs`

**현재 상태:** `SEARCH_POSTS_QUERY`가 `searchPosts`에서 Post 필드를 직접 요청하지만, `searchPosts`의 반환 타입은 `SearchResult`이며 Post 필드를 직접 지원하지 않는다.

**작업:**
1. `SEARCH_POSTS_QUERY`에 `count`와 `posts { ... }` 래퍼 추가
2. `SearchPostsData` 모델에 중간 `SearchResultWrapper` 타입 추가
3. `search_posts()` 반환 타입을 `(Vec<Post>, i32)` (posts + count)로 변경
4. handler에서 `count` 활용하여 `has_more` 계산 개선
5. 기존 단위 테스트 업데이트 + 실제 API 호출 smoke test

#### Research Insights

**API 스키마 확정 (introspection으로 확인):**

```graphql
# v2.velog.io/graphql & v3.velog.io/graphql 동일
type SearchResult {
  count: Int!       # 전체 검색 결과 수
  posts: [Post!]!   # 현재 페이지의 포스트 목록
}
```

**검증 완료된 쿼리 (실제 API 호출 성공):**

```graphql
query ($keyword: String!, $offset: Int, $limit: Int, $username: String) {
    searchPosts(keyword: $keyword, offset: $offset, limit: $limit, username: $username) {
        count
        posts {
            id title short_description thumbnail
            likes url_slug released_at updated_at tags
            user { username }
        }
    }
}
```

**응답 예시 (실제 API에서 확인):**
```json
{
  "data": {
    "searchPosts": {
      "count": 4087,
      "posts": [{
        "id": "7b303c28-...",
        "title": "2020년 상반기. 양질의 기술 아티클 모음",
        "short_description": "카카오 100일 프로젝트는...",
        "thumbnail": "https://images.velog.io/...",
        "likes": 179,
        "url_slug": "2020년-상반기-양질의-기술-아티클-모음집",
        "released_at": "2020-06-20T14:06:02.265Z",
        "updated_at": null,
        "tags": ["TIL", "기술아티클"],
        "user": {"username": "rkdrhksdn"}
      }]
    }
  }
}
```

**구체적 코드 변경:**

```rust
// src/client/search.rs — 쿼리 수정
const SEARCH_POSTS_QUERY: &str = r#"
    query ($keyword: String!, $offset: Int, $limit: Int, $username: String) {
        searchPosts(keyword: $keyword, offset: $offset, limit: $limit, username: $username) {
            count
            posts {
                id title short_description thumbnail
                likes url_slug released_at updated_at tags
                user { username }
            }
        }
    }
"#;

// 반환 타입 변경: Vec<Post> → SearchResult (count 포함)
pub async fn search_posts(...) -> anyhow::Result<crate::models::SearchResult> {
    // ...
    Ok(resp.into_result()?.search_posts)
}
```

```rust
// src/models/search.rs — SearchResult 래퍼 추가
#[derive(Deserialize, Serialize)]
pub struct SearchResult {
    pub count: i32,
    pub posts: Vec<Post>,
}

#[derive(Deserialize)]
pub struct SearchPostsData {
    #[serde(rename = "searchPosts")]
    pub search_posts: SearchResult,
}
```

```rust
// src/handlers/search.rs — count 활용 (정수 안전성 확보)
let result = client.search_posts(keyword, offset, limit, username).await?;
// count가 음수일 수 있으므로 max(0) + saturating_add 사용
let total = result.count.max(0) as u32;
let consumed = offset.saturating_add(result.posts.len() as u32);
let has_more = consumed < total;
```

**Edge Cases:**
- `count`가 0 또는 음수일 때: `.max(0)`으로 방어 → `has_more = false`
- `count`와 `posts.len()`이 다를 수 있음 (마지막 페이지) → `has_more` 계산에 `count` 사용
- `offset + posts.len()` 오버플로우: `saturating_add`로 방어

**검증:**
- [ ] `velog search "rust" --format compact` 가 유효한 JSON 반환
- [ ] `velog search "한글" --format compact` 유니코드 검색 작동
- [ ] `velog search "rust" --format pretty` 테이블 출력
- [ ] compact 출력에 `total_count` 필드 포함
- [ ] 기존 search 관련 단위 테스트 통과

---

### Phase 1: Clap 에러 JSON 래핑 [Fail]

**파일:** `src/main.rs`

**현재 상태:** `Cli::parse()` (line 14)가 인자 에러 시 직접 stderr에 plain text를 출력하고 exit(2)한다. 이후의 `format` 기반 에러 핸들링(line 218–229)에 도달하지 못한다.

**작업:**
1. `Cli::parse()` → `Cli::try_parse()`로 변경
2. `Err(e)` 분기에서 `--format` 값을 raw args에서 추출
3. compact/silent이면 `output::emit_error()` 사용, 아니면 기존 `e.exit()` 호출

```rust
// src/main.rs 수정
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // help/version은 그대로 출력 (stdout로 나가는 것들)
            if e.use_stderr() {
                let format = detect_format_from_raw_args();
                if matches!(format, Format::Compact | Format::Silent) {
                    output::emit_error(
                        format,
                        &e.render().to_string(),
                        2,
                    );
                    std::process::exit(2);
                }
            }
            e.exit();
        }
    };
    // ... 이하 동일
}

/// clap 파싱 실패 시 --format 값을 raw args에서 추출.
/// positional 값이 아닌 --format 플래그의 값만 매칭하여 false positive 방지.
fn detect_format_from_raw_args() -> Format {
    let args: Vec<String> = std::env::args().collect();
    // --format=compact 형태
    for arg in &args {
        if let Some(val) = arg.strip_prefix("--format=") {
            return match val {
                "compact" => Format::Compact,
                "silent" => Format::Silent,
                _ => Format::Pretty,
            };
        }
    }
    // --format compact 형태 (space-separated)
    for w in args.windows(2) {
        if w[0] == "--format" {
            return match w[1].as_str() {
                "compact" => Format::Compact,
                "silent" => Format::Silent,
                _ => Format::Pretty,
            };
        }
    }
    Format::Pretty
}
```

#### Research Insights

**clap try_parse 패턴 — Best Practices:**

- `e.use_stderr()` 분기 하나로 충분. `ErrorKind`별 세분화는 불필요한 복잡성.
- `e.render().to_string()`은 ANSI 코드 없이 순수 텍스트를 반환하므로 JSON 래핑에 안전.
- `e.exit()`는 help/version 시 exit(0), 에러 시 exit(2)를 자동 처리.

**`--format` 감지 전략 (Security Review 반영):**
- ~~`std::env::args().any(|a| a == "compact")`~~ — `velog search "compact"` 시 키워드가 매칭되는 false positive 버그
- `detect_format_from_raw_args()`: `--format=compact` 또는 `--format` + `compact` 쌍만 스캔하여 positional 값 오매칭 방지
- `--format` 자체가 잘못된 경우 (예: `--format invalid`): `Format::Pretty`로 fall through

**Edge Cases:**
- `velog --format compact --help`: `e.use_stderr()` = false → 정상 help 출력 (correct)
- `velog --format compact --invalid-flag`: `e.use_stderr()` = true + format = Compact → JSON 에러 (correct)
- `velog --format invalid`: `detect_format_from_raw_args()` returns Pretty → plain text 에러 (correct)
- `velog search "compact"`: "compact"은 positional 값이므로 `detect_format_from_raw_args()`가 무시 → plain text (correct, **false positive 수정됨**)
- `velog post show compact --format pretty`: slug "compact"은 positional → format = Pretty (correct)

**주의:**
- `e.use_stderr()`로 help/version 요청(stdout 출력)과 실제 에러(stderr 출력)를 구분
- `--format` 플래그 자체가 잘못된 경우(`--format invalid`)도 처리해야 함

**검증:**
- [ ] `velog post list --format compact --limit abc` → stderr에 JSON 출력
- [ ] `velog post list --format pretty --limit abc` → stderr에 기존 plain text
- [ ] `velog --help` → 정상 stdout 출력 (변화 없음)
- [ ] `velog --version` → 정상 출력
- [ ] `velog post list --format invalid` → JSON이 아닌 plain text (format 자체 에러)
- [ ] `velog --format compact --help` → 정상 help 출력 (JSON이 아님)

---

### Phase 2: 입력 유효성 검증 순서 정비 [Fail]

**파일:** `src/handlers/mod.rs`, `src/handlers/post.rs`, `src/handlers/stats.rs`, `src/handlers/comment.rs`, `src/handlers/social.rs`

**현재 상태:** 일부 핸들러(`series_edit`, `search`)는 인증 전 validation이 작동하지만, `post_show`, `stats`, `comment_write` 등은 `with_auth_client()` 후에 validation이 발생한다.

**원칙:** 인자/입력 에러(사용자가 즉시 수정 가능) > 인증 에러(별도 action 필요) > API 에러

#### Research Insights

**Critical 발견 — `validate_slug()` 한글 slug 호환성 문제:**

기존 `validate_slug()` (`handlers/mod.rs:74`)는 ASCII 소문자+숫자+하이픈만 허용한다:
```rust
s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
```

그러나 실제 velog slug에는 한글이 포함될 수 있다:
```
2020년-상반기-양질의-기술-아티클-모음집
```

**따라서 Phase 2에서는 `validate_slug()` 대신 새로운 `validate_slug_nonempty()` 함수를 사용해야 한다:**

```rust
// src/handlers/mod.rs — 새 함수 추가
/// slug가 비어있지 않은지만 검증 (읽기/수정 작업용).
/// 기존 validate_slug()는 생성 시 ASCII slug 형식만 허용하므로,
/// 한글 등 비ASCII slug를 가진 기존 포스트 접근이 불가능해지는 문제가 있다.
pub(crate) fn validate_slug_nonempty(s: &str) -> anyhow::Result<()> {
    anyhow::ensure!(!s.trim().is_empty(), "Slug cannot be empty");
    anyhow::ensure!(s.trim() == s, "Slug must not have leading or trailing whitespace");
    Ok(())
}
```

**작업:**

대상 핸들러별 수정 — `validate_slug_nonempty()` 사용:

| Handler | 현재 | 수정 |
|---------|------|------|
| `post_show` (line 147) | auth/anonymous → slug 사용 | `validate_slug_nonempty(slug)?;` 추가 (최상단) |
| `post_edit` (line 245) | auth → slug 사용 | `validate_slug_nonempty(slug)?;` 추가 |
| `post_delete` (line 288) | `with_auth_client()` 먼저 | `validate_slug_nonempty(slug)?;` 추가 |
| `post_publish` (line 327) | `with_auth_client()` 먼저 | `validate_slug_nonempty(slug)?;` 추가 |
| `stats` (line 12) | `with_auth_client()` 먼저 | `validate_slug_nonempty(slug)?;` 추가 |
| `comment_write` (line 72) | body validation은 auth 전 | `validate_slug_nonempty(post_slug)?;` 추가 |
| `comment_reply` (line 105) | body validation은 auth 전 | `validate_slug_nonempty(post_slug)?;` 추가 |
| `comment_edit` (line 146) | body validation은 auth 전 | `validate_slug_nonempty(post_slug)?;` 추가 |
| `comment_delete` (line 184) | `with_auth_client()` 먼저 | `validate_slug_nonempty(post_slug)?;` 추가 |
| `post_like` (social.rs) | username validation은 auth 전, slug 미검증 | `validate_slug_nonempty(slug)?;` 추가 (최상단) |
| `post_unlike` (social.rs) | username validation은 auth 전, slug 미검증 | `validate_slug_nonempty(slug)?;` 추가 (최상단) |

```rust
// 패턴 예시: src/handlers/stats.rs
pub async fn stats(slug: &str, format: Format) -> anyhow::Result<()> {
    validate_slug_nonempty(slug)?;  // Phase 2: auth 전 validation
    let (mut client, username) = with_auth_client().await?;
    // ...
}
```

**`post_show` 특수 케이스:**
`post_show`는 `--username` 제공 시 anonymous 클라이언트를 사용하므로 auth 에러가 발생하지 않을 수 있다. 하지만 빈 slug는 어느 경로든 API 에러를 발생시키므로, 함수 최상단에서 검증하는 것이 올바르다:

```rust
pub async fn post_show(slug: &str, username: Option<&str>, format: Format) -> anyhow::Result<()> {
    validate_slug_nonempty(slug)?;  // 모든 경로 공통
    let (post, new_creds) = if let Some(uname) = username {
        // anonymous path...
    } else {
        // auth path...
    };
    // ...
}
```

**검증:**
- [ ] `velog post show "" --format compact` → `{"error":"Slug cannot be empty",...}` (auth 에러가 아님)
- [ ] `velog stats "" --format compact` → slug validation 에러
- [ ] `velog comment delete "" 1 --format compact` → slug validation 에러
- [ ] `velog post show "한글-slug" --username someone --format compact` → 정상 동작 (거부되지 않음)
- [ ] 기존 통과하던 테스트 모두 유지

---

### Phase 3: Help 텍스트 개선 — 상호 배타 플래그 문서화 + Examples [Warning/New]

**파일:** `src/cli.rs`

**현재 상태:** `PostCommands::List`의 `--drafts`, `--trending`, `--recent`, `--username`이 `conflicts_with_all`로 상호 배타이지만 help 텍스트에 이 관계가 표시되지 않음. 복잡한 명령어에 Examples 없음.

**작업:** `after_long_help`를 사용하여 상호 배타 문서와 examples를 통합. `after_long_help`만 사용하여 중복 제거 (Simplicity Review 반영).

> **설계 결정:** `after_help`와 `after_long_help`를 동시에 사용하면 `--help` 시 `after_long_help`가 `after_help`를 **대체**하여 내용 중복이 필요하다. `after_long_help`만 사용하면 `--help`에서만 표시되고 `-h`에서는 숨겨지지만, 중복 없이 깔끔하다. Examples와 listing modes 모두 `--help`에서 표시하는 것이 적합.

```rust
// PostCommands::List — listing modes + examples
#[command(after_long_help = "\
Listing modes (pick one):
  --trending   Trending posts (combine with --period, --offset)
  --recent     Recent posts (combine with --cursor)
  -u <USER>    Posts by a specific user (combine with --cursor)
  --tag <TAG>  Posts by tag (combine with --username, --cursor)
  --drafts     Your draft posts (requires auth)
  (default)    Your published posts (requires auth)

These modes are mutually exclusive.

Examples:
  velog post list --trending --period week --limit 10
  velog post list --recent --limit 5
  velog post list -u author --limit 20
  velog post list --drafts
  velog post list --tag rust -u author")]
List { ... }

// PostCommands::Create
#[command(after_long_help = "\
Examples:
  velog post create -t \"My Post\" -f content.md --publish
  velog post create -t \"Draft\" -f draft.md
  cat content.md | velog post create -t \"Piped Post\" --tags \"rust,cli\"
  echo \"# Hello\" | velog post create -t \"Quick\" -f - --publish")]
Create { ... }

// Commands::Search
#[command(after_long_help = "\
Examples:
  velog search \"rust\"
  velog search \"한글\" --limit 5
  velog search \"tutorial\" -u author --offset 10
  velog search \"cli\" --format compact | jq '.results[].title'")]
Search { ... }

// CommentCommands::Reply
#[command(after_long_help = "\
Comment numbering:
  Top-level comments are numbered 1, 2, 3, ...
  Replies use dot notation: 1.1, 1.2, 2.1, ...

Examples:
  velog comment reply my-post 1 \"Great post!\"
  velog comment reply my-post 1.2 -f reply.md")]
Reply { ... }
```

**검증:**
- [ ] `velog post list --help` 출력에 listing modes + examples 표시
- [ ] `velog post list -h` 에는 after_long_help 미표시 (간결)
- [ ] `velog post create --help` 출력에 Examples 표시
- [ ] `velog search --help` 출력에 Examples 표시
- [ ] `velog comment reply --help` 출력에 번호 체계 + Examples 표시
- [ ] 기존 `conflicts_with_all` 런타임 에러 동작 유지

## Acceptance Criteria

- [ ] `velog search "rust" --format compact` 정상 작동 (유효한 JSON, `total_count` 포함)
- [ ] compact/silent 모드에서 모든 에러가 JSON 포맷 (`{"error":"...","exit_code":N}`)
- [ ] 빈 slug 입력 시 auth 에러가 아닌 validation 에러 표시
- [ ] 한글 slug 포스트 접근이 거부되지 않음
- [ ] `velog post list --help`에 상호 배타 플래그 문서 표시
- [ ] 복잡한 명령어(post create, search)의 `--help`에 Examples 섹션 표시
- [ ] 기존 146개 테스트 전부 통과
- [ ] `cargo clippy -- -D warnings` 경고 0개

## Success Metrics

- UX 테스트 재실행 시 CRITICAL 0, FAIL 0
- compact 모드에서 모든 에러가 `jq` 파싱 가능

## Dependencies & Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| ~~Search API 스키마 불명확~~ | ~~Phase 0 블로커~~ | **해결됨:** introspection으로 `SearchResult { count, posts }` 확인 완료 |
| `try_parse()` 전환 시 help/version 동작 변경 | 기능 regression | `e.use_stderr()` 분기로 help/version 보존 |
| ~~raw args "compact" false positive~~ | ~~JSON 오출력~~ | **해결됨:** `detect_format_from_raw_args()`로 `--format` + 값 쌍만 스캔 |
| `validate_slug()` 한글 slug 거부 | 기존 포스트 접근 불가 | `validate_slug_nonempty()` 신규 함수 사용 (빈/whitespace만 차단) |
| `count` 음수 시 `has_more` 오동작 | 무한 pagination | `count.max(0)` + `saturating_add`로 방어 |

## Sources & References

- UX Test Report: `.ux-test-results/report.md`
- UX Test Recommendations: `.ux-test-results/recommendations.md`
- Output Format Brainstorm: `docs/brainstorms/2026-03-11-output-format-brainstorm.md` — compact stderr = JSON 설계 결정 (Decision #4)
- CLI UX Principles: `.claude/references/cli-ux-principles.md` — §2 에러 포맷, §8 검증 순서
- API Schema Verification: GraphQL introspection on `v2.velog.io/graphql` and `v3.velog.io/graphql` (2026-03-13)
- Repo Research: `.omc/research/RESEARCH_SUMMARY.md` — conventions, error patterns, validation ordering
