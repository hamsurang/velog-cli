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
**Sections enhanced:** 6 (Phase 0–5, originally 5 phases merged to 4, then 2 new phases added)
**Research conducted:** GraphQL introspection, clap try_parse patterns, validation ordering, repo conventions, clap requires+conflicts_with interaction, exit code conventions

### Key Improvements from Research
1. **Phase 0 — API 스키마 확정:** `searchPosts`가 `SearchResult { count: Int!, posts: [Post!]! }`를 반환함을 introspection으로 확인.
2. **Phase 2 — validate_slug 호환성 위험 발견:** 기존 `validate_slug()`는 ASCII만 허용하지만 velog slug에 한글이 포함될 수 있음. `validate_slug_nonempty()` 사용 권장.
3. **Phase 1 — `e.render()` 안전성:** `e.use_stderr()`로 충분함을 확인.
4. **Phase 4 — clap `requires` + `conflicts_with` 상호작용 버그 발견:** `--period`에 `requires = "trending"`이 있지만, `--recent --period day` 조합에서 clap이 requires 체크를 건너뜀 → `conflicts_with_all` 추가로 해결.
5. **Phase 5 — exit code 관례 확인:** POSIX/GNU 관례에서 exit 2는 "잘못된 사용법", exit 1은 "일반 에러". `AuthError`는 런타임 조건이므로 exit 1이 적절.

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

CLI UX E2E 테스트 (`.ux-test-results/report.md`)에서 발견된 2 Critical, 3 Fail, 5 Warning 이슈를 수정한다. search 명령어 복구, compact 모드 에러 포맷 통일, 유효성 검증 순서 정비, help 텍스트 개선, **플래그 조용한 실패 방지, 인증 에러 exit code 분리**를 포함한다.

## Problem Statement / Motivation

- `velog search`가 GraphQL 스키마 불일치로 **완전히 미작동** (Critical)
- compact/silent 모드에서 clap 에러가 plain text로 출력되어 **머신 파싱 실패** (Fail)
- 빈 slug 입력 시 validation 대신 auth 에러가 표시되어 **사용자 혼란** (Fail)
- 상호 배타 플래그가 help에 미표시, 복잡 명령어에 Examples 없음 (Warning/New)

## Proposed Solution

6개 Phase로 나누어 순차적으로 수정한다. 각 Phase는 독립적이며 개별 커밋 가능. (Phase 0→1→2 순서 권장: compact 에러 래핑이 validation 에러에도 적용되도록. Phase 4, 5는 독립적으로 진행 가능)

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

---

### Phase 4: 플래그 조용한 실패 방지 [Critical — UX E2E 테스트에서 신규 발견]

**파일:** `src/cli.rs`

**현재 상태:** `--period`와 `--offset`에 `requires = "trending"`이 설정되어 있지만, `--recent`나 `--username`이 함께 사용되면 clap이 requires 체크를 건너뛰고 해당 플래그가 **에러 없이 무시**됨.

**재현:**
```bash
$ velog post list --recent --period day
# → exit 0, --period 완전히 무시. 사용자는 "오늘의 최근 글"을 받았다고 착각

$ velog post list --username testuser --period day
# → exit 0, --period 무시. "No posts found"가 period 때문인지 알 수 없음

$ velog post list --recent --offset 10
# → exit 0, --offset 무시. 페이지네이션이 동작하지 않는데 에러 없음
```

**Root Cause 분석:**
clap의 `requires = "trending"` 제약은 "이 플래그가 있으면 `--trending`도 있어야 한다"를 의미한다. 하지만 `--recent`가 함께 있으면 `--trending`과 `--recent`의 `conflicts_with_all` 관계가 먼저 평가되어, `requires` 체크가 우회되는 것으로 보인다. 단독으로 `--period day`만 사용하면 requires 에러가 정상 발생하지만, conflicting 플래그가 함께 있으면 동작하지 않는다.

**해결:** `conflicts_with_all`을 `--period`와 `--offset`에 명시적으로 추가하여, 지원하지 않는 모드에서 사용 시 clap이 에러를 생성하도록 한다.

```rust
// src/cli.rs — PostCommands::List 수정

// 기존:
#[arg(long, value_enum, requires = "trending")]
period: Option<Period>,

#[arg(long, requires = "trending")]
offset: Option<u32>,

// 변경:
/// Time period for trending (day, week, month, year)
#[arg(long, value_enum, requires = "trending", conflicts_with_all = ["recent", "username", "drafts"])]
period: Option<Period>,

/// Offset for pagination (trending only)
#[arg(long, requires = "trending", conflicts_with_all = ["recent", "username", "drafts"])]
offset: Option<u32>,
```

**기대 동작 (수정 후):**
```bash
$ velog post list --recent --period day
error: the argument '--period <PERIOD>' cannot be used with '--recent'

Usage: velog post list --trending --period <PERIOD>

For more information, try '--help'.
$ echo $?
2

$ velog post list --recent --offset 10
error: the argument '--offset <OFFSET>' cannot be used with '--recent'
$ echo $?
2
```

**Edge Cases:**
- `--period day` (단독): 기존 requires 에러 유지 ("--trending required") ✓
- `--trending --period day`: 정상 동작 ✓
- `--trending --offset 20`: 정상 동작 ✓
- `--tag rust --period day`: 에러 — tag은 trending이 아님 (tag은 conflicts_with_all에 포함하지 않음. tag 모드에서 period가 의미 있을 수 있는지 검토 필요. 현재 API가 지원하지 않으므로 conflicts_with에 포함하지 않되, requires = "trending"이 방어)

**검증:**
- [ ] `velog post list --recent --period day` → exit 2, 에러 메시지 (조용한 실패 해소)
- [ ] `velog post list --username test --period day` → exit 2, 에러 메시지
- [ ] `velog post list --drafts --period day` → exit 2, 에러 메시지
- [ ] `velog post list --recent --offset 10` → exit 2, 에러 메시지
- [ ] `velog post list --trending --period day` → exit 0, 정상 동작 (기존 동작 유지)
- [ ] `velog post list --period day` → exit 2, requires 에러 (기존 동작 유지)
- [ ] `velog post list --trending --offset 20` → exit 0, 정상 동작

---

### Phase 5: 인증 에러 Exit Code 분리 [Fail — UX E2E 테스트에서 신규 발견]

**파일:** `src/main.rs`, `src/models/mod.rs` (테스트)

**현재 상태:** `exit_code()` 함수(main.rs:272)가 `AuthError`에 대해 exit code 2를 반환한다. clap 인자 에러도 exit code 2이므로, 스크립트에서 "사용법 에러"와 "인증 필요"를 구분할 수 없다.

```rust
// 현재 (main.rs:272-279)
fn exit_code(err: &anyhow::Error) -> i32 {
    for cause in err.chain() {
        if cause.downcast_ref::<auth::AuthError>().is_some() {
            return 2;  // ← 인증 에러도 2
        }
    }
    1
}
```

**해결:** `AuthError`의 exit code를 1로 변경한다.

```rust
// 수정 후
fn exit_code(err: &anyhow::Error) -> i32 {
    // AuthError는 런타임 조건 에러이므로 exit code 1 (일반 에러).
    // exit code 2는 clap 인자 에러(잘못된 사용법)에만 사용.
    // POSIX/GNU 관례: 0=성공, 1=일반 에러, 2=사용법 에러
    for cause in err.chain() {
        if cause.downcast_ref::<auth::AuthError>().is_some() {
            return 1;
        }
    }
    1
}
```

**참고:** 수정 후 `exit_code()` 함수는 항상 1을 반환하게 되므로, AuthError 분기가 의미적 문서화 역할만 한다. 향후 다른 에러 유형에 대해 별도 exit code가 필요할 수 있으므로 함수 구조는 유지한다.

**추가 수정 — `emit_error` 호출부:**

`src/output.rs`의 `emit_error(format, msg, exit_code)` 호출에서 인증 에러에 exit_code 2를 하드코딩하는 곳이 있으면 1로 변경:

```rust
// src/models/mod.rs 테스트 수정 (line 181-186)
// 기존:
let err = ErrorResponse {
    error: "Not authenticated".to_string(),
    exit_code: 2,
};
assert!(json.contains(r#""exit_code":2"#));

// 수정:
let err = ErrorResponse {
    error: "Not authenticated".to_string(),
    exit_code: 1,
};
assert!(json.contains(r#""exit_code":1"#));
```

**기대 동작 (수정 후):**
```bash
$ velog post list          # 비로그인
$ echo $?
1   # 일반 에러 (인증 실패)

$ velog post show          # 필수 인자 누락
$ echo $?
2   # 사용법 에러 (clap)

# 스크립트에서 구분 가능:
velog post list 2>/dev/null
case $? in
  0) echo "success" ;;
  1) echo "runtime error (maybe auth)" ;;
  2) echo "wrong usage" ;;
esac
```

**검증:**
- [ ] `velog post list` (비로그인) → exit code 1 (기존 2에서 변경)
- [ ] `velog post list --format compact` (비로그인) → JSON에 `"exit_code":1`
- [ ] `velog post show` (인자 누락) → exit code 2 (변경 없음)
- [ ] `velog --format invalid` → exit code 2 (변경 없음)
- [ ] 기존 테스트 업데이트 후 전부 통과

## Acceptance Criteria

- [ ] `velog search "rust" --format compact` 정상 작동 (유효한 JSON, `total_count` 포함)
- [ ] compact/silent 모드에서 모든 에러가 JSON 포맷 (`{"error":"...","exit_code":N}`)
- [ ] 빈 slug 입력 시 auth 에러가 아닌 validation 에러 표시
- [ ] 한글 slug 포스트 접근이 거부되지 않음
- [ ] `velog post list --help`에 상호 배타 플래그 문서 표시
- [ ] 복잡한 명령어(post create, search)의 `--help`에 Examples 섹션 표시
- [ ] `velog post list --recent --period day` → exit 2, 에러 메시지 (조용한 실패 해소)
- [ ] `velog post list --recent --offset 10` → exit 2, 에러 메시지 (조용한 실패 해소)
- [ ] `velog post list` (비로그인) → exit code 1 (인증 에러와 사용법 에러 구분)
- [ ] 기존 146개 테스트 전부 통과 (exit code 관련 테스트 업데이트 후)
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
| clap `requires` + `conflicts_with` 상호작용 | `--period`/`--offset` 조용한 실패 | `conflicts_with_all` 명시적 추가로 방어 |
| exit code 변경 시 기존 스크립트 영향 | AuthError exit 2→1 변경 | 하위 호환성 이슈 가능하나, 기존 exit 2가 관례에 맞지 않으므로 변경이 올바름 |

## Sources & References

- UX Test Report: `.ux-test-results/report.md`
- UX Test Recommendations: `.ux-test-results/recommendations.md`
- Output Format Brainstorm: `docs/brainstorms/2026-03-11-output-format-brainstorm.md` — compact stderr = JSON 설계 결정 (Decision #4)
- CLI UX Principles: `.claude/references/cli-ux-principles.md` — §2 에러 포맷, §8 검증 순서
- API Schema Verification: GraphQL introspection on `v2.velog.io/graphql` and `v3.velog.io/graphql` (2026-03-13)
- Repo Research: `.omc/research/RESEARCH_SUMMARY.md` — conventions, error patterns, validation ordering
- UX E2E Test (Phase 4-5 근거): `.ux-test-results/test-results.md` — [EDGE-08], [EDGE-09], [EDGE-11], [EXIT-04] 시나리오
- CLI UX Principles §8: `.claude/references/cli-ux-principles.md` — 조용한 실패 방지, 검증 순서, exit code 관례
