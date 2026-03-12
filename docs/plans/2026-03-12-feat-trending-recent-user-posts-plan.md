---
title: "feat: Add trending, recent, and user post listing"
type: feat
status: completed
date: 2026-03-12
deepened: 2026-03-12
origin: docs/brainstorms/2026-03-12-feed-trending-brainstorm.md
---

# feat: Add trending, recent, and user post listing

## Enhancement Summary

**Deepened on:** 2026-03-12
**Research sources:** API introspection (live curl), velog-server source, institutional learnings, architecture/security/performance/simplicity reviews

### Key Improvements
1. v3 GraphQL `input` wrapper 변수 타입 불확실성 발견 → 안전한 쿼리 패턴 제시
2. `Post.is_temp`/`is_private`에 `#[serde(default)]` 필요 (v3 응답에서 누락 가능)
3. Clap `conflicts_with` 활용하여 상호 배타 검증을 CLI 레벨로 이동
4. User posts에 별도 메서드 대신 기존 `get_posts` 확장 (v2 API가 `limit`/`cursor` 지원 확인)
5. Anonymous 메서드는 `&self` (not `&mut self`) — credential 변경 없음

### Institutional Learnings Applied
- Pattern #2: 새 쿼리마다 `*Data` wrapper 필수 (see `docs/solutions/logic-errors/plan-review-checklist-rust-cli.md`)
- Pattern #3: `raw_graphql`은 `query: &'static str` — 새 쿼리는 반드시 `const`
- Pattern #6: API 파라미터를 실제 스키마와 대조 검증 완료

---

## Overview

`velog post list`를 확장하여 트렌딩, 최신, 타 유저 포스트 목록 조회 기능을 추가한다.
브레인스톰에서 논의된 feed/curated 기능은 velog GraphQL API에 해당 엔드포인트가 없으므로 이번 범위에서 제외한다.

## Problem Statement / Motivation

현재 `velog post list`는 **자신의 포스트만** 조회할 수 있다.
velog.io의 핵심 기능인 트렌딩/최신 포스트 탐색이나 타 유저의 글 목록 확인이 CLI에서 불가능하다.

## Proposed Solution

기존 `PostCommands::List`에 플래그를 추가하여 4가지 모드를 지원한다:

```bash
# 1. 기존 (내 글)
velog post list              # 내 발행 글
velog post list --drafts     # 내 임시 글

# 2. Trending (비로그인 가능)
velog post list --trending                    # 주간 인기 (기본)
velog post list --trending --period day       # 일간
velog post list --trending --period month     # 월간
velog post list --trending --period year      # 연간

# 3. Recent (비로그인 가능)
velog post list --recent

# 4. User posts (비로그인 가능)
velog post list -u <username>

# 공통 옵션
--limit <n>    # 결과 개수 (기본 20)
```

## Technical Considerations

### API 스키마 (실제 검증 완료)

velog GraphQL API는 v2와 v3 두 버전이 있으며, trending/recent는 **v3만 동작**한다:

| 쿼리 | 엔드포인트 | 시그니처 | 페이지네이션 | 검증 |
|------|-----------|---------|-------------|------|
| trendingPosts | v3 | `trendingPosts(input: { limit, offset, timeframe })` | offset 기반 | ✅ 실제 데이터 반환 확인 |
| recentPosts | v3 | `recentPosts(input: { limit, cursor })` | cursor 기반 | ✅ 실제 데이터 반환 확인 |
| posts (user) | v2 | `posts(username, limit, cursor, temp_only, tag)` | cursor 기반 | ✅ limit + cursor 동작 확인 |

#### API 검증 결과 (2026-03-12 실제 호출)

```bash
# v3 trending — 정상 동작 (input wrapper 패턴)
curl -X POST https://v3.velog.io/graphql \
  -d '{"query":"query { trendingPosts(input: { limit: 3, offset: 0, timeframe: \"month\" }) { id title url_slug user { username } } }"}'
# → {"data":{"trendingPosts":[{"id":"...","title":"...","user":{"username":"teo"}}, ...]}}

# v3 recent — 정상 동작
curl -X POST https://v3.velog.io/graphql \
  -d '{"query":"query { recentPosts(input: { limit: 3, cursor: null }) { id title url_slug user { username } released_at } }"}'
# → {"data":{"recentPosts":[...]}}

# v2 user posts with limit — 정상 동작
curl -X POST https://v2.velog.io/graphql \
  -d '{"query":"query { posts(username: \"teo\", limit: 3) { id title url_slug } }"}'
# → {"data":{"posts":[...]}}

# v2 cursor pagination — 정상 동작
curl -X POST https://v2.velog.io/graphql \
  -d '{"query":"query { posts(username: \"teo\", limit: 2, cursor: \"<id>\") { id title } }"}'
# → 다음 페이지 결과 반환 확인

# v2 trending — 빈 배열 반환 (v2에서는 deprecated)
# v3에서만 동작하므로 trending/recent는 반드시 v3 사용
```

#### v3 `input` wrapper 변수 타입 주의사항

v3 API는 `input` 객체 패턴을 사용하지만, **introspection이 비활성화**되어 정확한 input type 이름(`TrendingPostsInput` 등)을 확인할 수 없다.

**안전한 접근법 — inline query 사용:**

```rust
// ✅ 안전: inline 값으로 검증 완료된 패턴
const GET_TRENDING_POSTS_QUERY: &str = r#"
    query ($limit: Int, $offset: Int, $timeframe: String) {
        trendingPosts(input: { limit: $limit, offset: $offset, timeframe: $timeframe }) {
            id title short_description thumbnail
            likes url_slug released_at updated_at tags
            user { username }
        }
    }
"#;

// ⚠️ 위험: input type 이름이 정확한지 확인 불가
const GET_TRENDING_POSTS_QUERY: &str = r#"
    query ($input: TrendingPostsInput!) {
        trendingPosts(input: $input) { ... }
    }
"#;
```

구현 시 inline 스칼라 변수 방식을 먼저 시도하고, v3 API가 이를 거부하면 named input type 방식으로 전환한다.

**페이지네이션 설계 변경 (brainstorm 결정 수정):**

브레인스톰에서는 `--limit` + `--page`를 결정했으나, API 조사 결과:
- `trendingPosts`만 offset 기반 (page 매핑 가능)
- `recentPosts`와 `posts`는 cursor 기반 (page 매핑 불가)

따라서 **`--limit`만 지원**하고, cursor 기반 페이지네이션은 compact 모드에서 `next_cursor`를 반환하여 `--cursor` 옵션으로 다음 페이지를 요청하는 방식으로 구현한다. `--offset` 옵션은 trending 전용으로 추가한다.

```bash
# Trending: offset 기반
velog post list --trending --limit 20 --offset 40

# Recent / User posts: cursor 기반
velog post list --recent --limit 20
velog post list --recent --limit 20 --cursor <id>
velog post list -u teo --cursor <id>
```

#### Cursor 페이지네이션 UX

compact/silent 모드에서 결과의 마지막 포스트 ID를 `next_cursor`로 반환한다:
```json
{"posts": [...], "next_cursor": "4df00736-6cef-4a9b-9446-072581a71f4b"}
```

pretty 모드에서는 결과가 있으면 하단에 안내 메시지 표시:
```
Showing 20 posts. Next page: velog post list --recent --cursor 4df00736...
```

### API 미지원 기능 (범위 제외)

- **feed (following)**: `followingPosts` 쿼리 없음 → 제외
- **curated**: 큐레이션 쿼리 없음 → 제외
- 향후 velog API에 추가되면 별도 이슈로 구현

### 상호 배타 플래그

`--trending`, `--recent`, `--drafts`는 상호 배타적. 동시 사용 시 에러.
`-u <username>`은 `--trending`/`--recent`/`--drafts`와 상호 배타.
아무 플래그도 없으면 기존 동작 (내 글 목록).

**Clap `conflicts_with` 활용 (개선):** 핸들러 runtime 검증 대신 clap의 `conflicts_with` 속성을 사용하면 자동으로 에러 메시지와 help text가 생성된다.

### 인증 정책 (see brainstorm)

- **trending / recent**: `VelogClient::anonymous()` 사용 (비로그인)
- **user posts (-u)**: `VelogClient::anonymous()` 사용 (비로그인)
- **기본 (내 글)**: 기존 `with_auth_client()` (로그인 필수)

#### 보안 고려사항

- Anonymous 클라이언트는 `credentials: None`으로 생성 → 토큰 유출 불가
- username, cursor, offset은 GraphQL 변수로 전달 (query string 아님) → injection 안전
- `raw_graphql`에서 `credentials`가 `None`이면 Cookie 헤더 자체가 전송되지 않음 (`src/client.rs:132-140`)

## Acceptance Criteria

### Functional

- [x] `velog post list --trending` — v3 API로 트렌딩 포스트 조회
- [x] `--period day|week|month|year` — 기간 필터 (기본 week)
- [x] `velog post list --recent` — v3 API로 최신 포스트 조회
- [x] `velog post list -u <username>` — 특정 유저 포스트 목록 조회
- [x] `--limit <n>` — 결과 개수 제한 (기본 20)
- [x] `--cursor <id>` — cursor 기반 페이지네이션 (recent, user posts)
- [x] `--offset <n>` — offset 기반 페이지네이션 (trending)
- [x] 상호 배타 플래그 동시 사용 시 명확한 에러 메시지 (clap 레벨)
- [x] trending/recent/user posts는 비로그인으로 동작
- [x] pretty/compact/silent 3가지 포맷 모두 정상 출력
- [x] compact 모드에서 cursor 기반 결과 시 `next_cursor` 반환

### Non-Functional

- [x] 기존 `post list` / `post list --drafts` 동작 변경 없음
- [x] 단위 테스트 추가 (플래그 검증, 모델 변환)
- [x] 통합 테스트 추가 (CLI argument parsing)
- [x] README 업데이트

## Implementation Phases

### Phase 1: CLI + 플래그 검증 (`src/cli.rs`, `src/main.rs`)

`PostCommands::List` 확장. **Clap `conflicts_with`로 상호 배타 검증:**

```rust
// src/cli.rs
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
pub enum PostCommands {
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
        /// Maximum number of posts to show
        #[arg(long, default_value_t = 20)]
        limit: u32,
        /// Time period for trending (day, week, month, year)
        #[arg(long, requires = "trending")]
        period: Option<Period>,
        /// Cursor for pagination (recent, user posts)
        #[arg(long, conflicts_with_all = ["trending", "drafts"])]
        cursor: Option<String>,
        /// Offset for pagination (trending only)
        #[arg(long, requires = "trending")]
        offset: Option<u32>,
    },
    // ... 기존 커맨드 (Show, Create, Edit, Delete, Publish)
}
```

**`conflicts_with` 장점:**
- Clap이 자동으로 `error: the argument '--trending' cannot be used with '--recent'` 메시지 생성
- `--help` 출력에도 제약 조건 반영
- 핸들러에서 별도 검증 코드 불필요

`main.rs` dispatch 확장:

```rust
PostCommands::List { drafts, trending, recent, username, limit, period, cursor, offset } => {
    handlers::post_list(drafts, trending, recent, username.as_deref(), limit, period, cursor.as_deref(), offset, format).await
}
```

### Phase 2: GraphQL 쿼리 + 클라이언트 메서드 (`src/client.rs`)

새 쿼리 상수 추가 (**`const` — Pattern #3: `raw_graphql`은 `&'static str` 필요**):

```rust
// v3 API용 trending 쿼리 (inline 변수 — input type 이름 불확실성 회피)
const GET_TRENDING_POSTS_QUERY: &str = r#"
    query ($limit: Int, $offset: Int, $timeframe: String) {
        trendingPosts(input: { limit: $limit, offset: $offset, timeframe: $timeframe }) {
            id title short_description thumbnail
            likes url_slug released_at updated_at tags
            user { username }
        }
    }
"#;

// v3 API용 recent 쿼리
const GET_RECENT_POSTS_QUERY: &str = r#"
    query ($limit: Int, $cursor: ID) {
        recentPosts(input: { limit: $limit, cursor: $cursor }) {
            id title short_description thumbnail
            likes url_slug released_at updated_at tags
            user { username }
        }
    }
"#;

// v2 user posts 쿼리 (기존 GET_POSTS_QUERY에 limit/cursor 추가)
const GET_USER_POSTS_QUERY: &str = r#"
    query Posts($username: String!, $limit: Int, $cursor: ID) {
        posts(username: $username, limit: $limit, cursor: $cursor) {
            id title short_description thumbnail
            likes is_private is_temp url_slug
            released_at updated_at tags
            user { username }
        }
    }
"#;
```

클라이언트 메서드 추가 (`VelogClient` impl).
**Anonymous 메서드는 `&self` (not `&mut self`) — credential 변경이 없으므로:**

```rust
/// 트렌딩 포스트 (anonymous, v3 API)
/// Pattern #2: raw_graphql → into_result, execute_graphql 미사용
pub async fn get_trending_posts(
    &self,
    limit: u32,
    offset: u32,
    timeframe: &str,
) -> anyhow::Result<Vec<Post>> {
    let vars = serde_json::json!({
        "limit": limit,
        "offset": offset,
        "timeframe": timeframe,
    });
    let resp: GraphQLResponse<TrendingPostsData> =
        self.raw_graphql(API_V3, GET_TRENDING_POSTS_QUERY, Some(&vars)).await?;
    Ok(resp.into_result()?.trending_posts)
}

/// 최신 포스트 (anonymous, v3 API)
pub async fn get_recent_posts(
    &self,
    limit: u32,
    cursor: Option<&str>,
) -> anyhow::Result<Vec<Post>> {
    let vars = serde_json::json!({
        "limit": limit,
        "cursor": cursor,
    });
    let resp: GraphQLResponse<RecentPostsData> =
        self.raw_graphql(API_V3, GET_RECENT_POSTS_QUERY, Some(&vars)).await?;
    Ok(resp.into_result()?.recent_posts)
}

/// 특정 유저의 포스트 (anonymous 가능, v2 API)
pub async fn get_user_posts(
    &self,
    username: &str,
    limit: u32,
    cursor: Option<&str>,
) -> anyhow::Result<Vec<Post>> {
    let vars = serde_json::json!({
        "username": username,
        "limit": limit,
        "cursor": cursor,
    });
    let resp: GraphQLResponse<PostsData> =
        self.raw_graphql(API_V2, GET_USER_POSTS_QUERY, Some(&vars)).await?;
    Ok(resp.into_result()?.posts)
}
```

**기존 `get_posts` 변경 없음** — 인증된 자신의 글 조회용으로 유지.

### Phase 3: 응답 모델 (`src/models.rs`)

**Pattern #2: 새 쿼리마다 `*Data` wrapper 필수:**

```rust
#[derive(Deserialize)]
pub struct TrendingPostsData {
    #[serde(rename = "trendingPosts")]
    pub trending_posts: Vec<Post>,
}

#[derive(Deserialize)]
pub struct RecentPostsData {
    #[serde(rename = "recentPosts")]
    pub recent_posts: Vec<Post>,
}
```

**`Post` 구조체 — `is_temp`/`is_private`에 `#[serde(default)]` 추가:**

v3 trending/recent 응답에서 `is_temp`과 `is_private`이 반환되지 않을 수 있다 (공개 포스트만 반환되므로). 역직렬화 실패를 방지:

```rust
pub struct Post {
    // ... 기존 필드
    #[serde(default)]
    pub is_private: bool,
    #[serde(default)]
    pub is_temp: bool,
    // ...
}
```

**`CompactPost`에 `user` 필드 추가:**

```rust
pub struct CompactPost {
    pub title: String,
    pub slug: String,
    pub status: CompactStatus,
    pub tags: Vec<String>,
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,  // username — trending/recent/user posts에서만 포함
}
```

`From<&Post>` 구현에서 `post.user`가 있으면 username 추출:

```rust
impl From<&Post> for CompactPost {
    fn from(post: &Post) -> Self {
        CompactPost {
            // ... 기존 필드
            user: post.user.as_ref().map(|u| u.username.clone()),
        }
    }
}
```

### Phase 4: 핸들러 (`src/handlers.rs`)

`post_list` 시그니처 확장 및 모드별 분기:

```rust
pub async fn post_list(
    drafts: bool,
    trending: bool,
    recent: bool,
    username: Option<&str>,
    limit: u32,
    period: Option<Period>,
    cursor: Option<&str>,
    offset: Option<u32>,
    format: Format,
) -> anyhow::Result<()> {
    // clap conflicts_with가 상호 배타를 처리하므로 여기서는 모드 분기만
    if trending {
        return post_list_trending(limit, offset.unwrap_or(0), period, format).await;
    }
    if recent {
        return post_list_recent(limit, cursor, format).await;
    }
    if let Some(uname) = username {
        return post_list_user(uname, limit, cursor, format).await;
    }
    // 기존: 내 글 목록
    post_list_mine(drafts, format).await
}
```

**각 모드별 핸들러:**

```rust
async fn post_list_trending(
    limit: u32, offset: u32, period: Option<Period>, format: Format,
) -> anyhow::Result<()> {
    let client = VelogClient::anonymous()?;
    let timeframe = period.unwrap_or(Period::Week).to_string();
    let posts = client.get_trending_posts(limit, offset, &timeframe).await?;
    emit_public_posts(&posts, format)
}

async fn post_list_recent(
    limit: u32, cursor: Option<&str>, format: Format,
) -> anyhow::Result<()> {
    let client = VelogClient::anonymous()?;
    let posts = client.get_recent_posts(limit, cursor).await?;
    emit_public_posts(&posts, format)
}

async fn post_list_user(
    username: &str, limit: u32, cursor: Option<&str>, format: Format,
) -> anyhow::Result<()> {
    let client = VelogClient::anonymous()?;
    let posts = client.get_user_posts(username, limit, cursor).await?;
    emit_public_posts(&posts, format)
}

async fn post_list_mine(drafts: bool, format: Format) -> anyhow::Result<()> {
    // 기존 post_list 로직 그대로 이동
    let (mut client, username) = with_auth_client().await?;
    let (posts, new_creds) = client.get_posts(&username, drafts).await?;
    maybe_save_creds(new_creds)?;
    match format {
        Format::Pretty => { /* 기존 테이블 */ }
        Format::Compact | Format::Silent => { /* 기존 compact */ }
    }
    Ok(())
}
```

**`emit_public_posts` 공통 출력 헬퍼:**

```rust
fn emit_public_posts(posts: &[Post], format: Format) -> anyhow::Result<()> {
    match format {
        Format::Pretty => {
            if posts.is_empty() {
                eprintln!("{}", "No posts found.".yellow());
                return Ok(());
            }
            print_public_posts_table(posts);
            // cursor hint: 마지막 post ID 안내
            if let Some(last) = posts.last() {
                eprintln!("Next page: --cursor {}", last.id);
            }
        }
        Format::Compact | Format::Silent => {
            let compact: Vec<CompactPost> = posts.iter().map(CompactPost::from).collect();
            // next_cursor 포함
            let next_cursor = posts.last().map(|p| p.id.as_str());
            let output = serde_json::json!({
                "posts": compact,
                "next_cursor": next_cursor,
            });
            output::emit_data(format, &output);
        }
    }
    Ok(())
}
```

pretty 모드 테이블에 `Author` 컬럼 추가:

```rust
fn print_public_posts_table(posts: &[Post]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Title", "Author", "Slug", "Likes", "Date"]);

    for post in posts {
        let author = post.user.as_ref().map(|u| u.username.as_str()).unwrap_or("-");
        let likes = post.likes.to_string();
        let date = post.date_short();
        table.add_row(vec![&post.title, author, &post.url_slug, &likes, &date]);
    }
    println!("{table}");
}
```

### Phase 5: 테스트

**단위 테스트** (`src/models.rs` 내 `mod tests`):

```rust
#[test]
fn trending_posts_data_deserializes() {
    let json = r#"{"trendingPosts":[{"id":"abc","title":"Test","short_description":null,"body":null,"thumbnail":null,"likes":10,"url_slug":"test","released_at":null,"updated_at":null,"tags":["rust"],"meta":null,"user":{"username":"teo"}}]}"#;
    let data: TrendingPostsData = serde_json::from_str(json).unwrap();
    assert_eq!(data.trending_posts.len(), 1);
    assert_eq!(data.trending_posts[0].likes, 10);
}

#[test]
fn post_deserializes_without_is_temp() {
    // v3 응답에서 is_temp/is_private 누락 시 기본값 false
    let json = r#"{"id":"1","title":"T","short_description":null,"body":null,"thumbnail":null,"likes":0,"url_slug":"t","released_at":null,"updated_at":null,"tags":null,"meta":null,"user":null}"#;
    let post: Post = serde_json::from_str(json).unwrap();
    assert!(!post.is_temp);
    assert!(!post.is_private);
}

#[test]
fn compact_post_includes_user_when_present() {
    let mut post = make_test_post(false, false);
    post.user = Some(PostUser { username: "teo".into() });
    let compact = CompactPost::from(&post);
    assert_eq!(compact.user.as_deref(), Some("teo"));
}

#[test]
fn compact_post_omits_user_when_absent() {
    let mut post = make_test_post(false, false);
    post.user = None;
    let compact = CompactPost::from(&post);
    assert!(compact.user.is_none());
    let json = serde_json::to_string(&compact).unwrap();
    assert!(!json.contains("user"));
}
```

**통합 테스트** (`tests/cli_tests.rs`):

```rust
#[test]
fn conflicting_flags_trending_recent() {
    Command::cargo_bin("velog").unwrap()
        .args(["post", "list", "--trending", "--recent"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot be used with"));
}

#[test]
fn conflicting_flags_trending_drafts() {
    Command::cargo_bin("velog").unwrap()
        .args(["post", "list", "--trending", "--drafts"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot be used with"));
}

#[test]
fn conflicting_flags_username_recent() {
    Command::cargo_bin("velog").unwrap()
        .args(["post", "list", "-u", "teo", "--recent"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot be used with"));
}

#[test]
fn period_requires_trending() {
    Command::cargo_bin("velog").unwrap()
        .args(["post", "list", "--period", "week"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("--trending"));
}

#[test]
fn offset_requires_trending() {
    Command::cargo_bin("velog").unwrap()
        .args(["post", "list", "--offset", "10"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("--trending"));
}

#[test]
fn valid_trending_flags_parse() {
    // 파싱만 검증 — API 호출 없이 (auth 없으면 바로 실패하므로 exit code만 확인하지 않음)
    Command::cargo_bin("velog").unwrap()
        .args(["post", "list", "--trending", "--period", "month", "--limit", "5"])
        .assert(); // 파싱 성공 확인 (API 에러는 OK)
}
```

### Phase 6: README 업데이트

Commands 테이블에 새 옵션 추가, 한국어 가이드 섹션도 동기화.

```markdown
| `velog post list --trending` | Trending posts (`--period day/week/month/year`) |
| `velog post list --recent` | Recent posts |
| `velog post list -u <user>` | Posts by a specific user |
```

## Edge Cases

- **빈 결과**: trending에 데이터가 없을 수 있음 (timeframe이 짧을 때). pretty 모드에서 "No posts found." 표시.
- **잘못된 cursor**: API가 빈 배열 반환. 에러가 아닌 빈 결과로 처리.
- **존재하지 않는 username**: API가 빈 배열 반환. 동일 처리.
- **limit 0 또는 매우 큰 값**: API 기본 동작에 위임. CLI에서 별도 검증하지 않음.
- **네트워크 에러**: 기존 `raw_graphql`의 에러 처리 재사용.

## Dependencies & Risks

| 리스크 | 영향 | 완화 방안 |
|--------|------|-----------|
| v3 API `input` 변수 타입 이름 불확실 | 쿼리 실패 가능 | inline 스칼라 변수 방식 사용 (검증 완료) |
| `trendingPosts` timeframe 유효 값 | 잘못된 값에 빈 배열 반환 | day/week/month/year만 CLI enum으로 허용 |
| v3 응답에서 `is_temp`/`is_private` 누락 | 역직렬화 실패 | `#[serde(default)]` 추가 |
| v3와 v2 혼용 | 인증 쿠키 처리 차이 | anonymous 쿼리는 쿠키 불필요, 기존 v2 코드 변경 없음 |
| feed/curated API 부재 | 브레인스톰 범위 축소 | 명시적으로 범위 제외, 향후 별도 이슈 |

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-12-feed-trending-brainstorm.md](docs/brainstorms/2026-03-12-feed-trending-brainstorm.md) — 주요 결정: post list 확장 방식, 비로그인 허용, 기간 필터 day/week/month/year

### Institutional Learnings

- **Plan review checklist:** [docs/solutions/logic-errors/plan-review-checklist-rust-cli.md](docs/solutions/logic-errors/plan-review-checklist-rust-cli.md) — Pattern #2 (GraphQL wrapper), #3 (`&'static str`), #6 (API params)
- **Learnings audit:** [docs/solutions/logic-errors/learnings-audit.md](docs/solutions/logic-errors/learnings-audit.md) — 전체 패턴 준수 확인

### Internal References

- CLI 구조: `src/cli.rs:61-125`
- GraphQL 클라이언트: `src/client.rs:88-271`
- 핸들러 패턴: `src/handlers.rs:199-218` (기존 post_list)
- Anonymous 클라이언트: `src/client.rs:103-108`
- 출력 포맷: `src/output.rs`
- 기존 테스트: `src/models.rs:326-588`, `tests/cli_tests.rs`

### External References

- velog-server GraphQL schema: `github.com/velopert/velog-server/src/graphql/post.ts`
- v2 schema: `posts(cursor: ID, limit: Int, username: String, temp_only: Boolean, tag: String)`
- v2 schema: `recentPosts(cursor: ID, limit: Int)`, `trendingPosts(offset: Int, limit: Int, timeframe: String)`
- v3 API: `input` wrapper 패턴 사용 확인 (2026-03-12 실제 호출 검증)
