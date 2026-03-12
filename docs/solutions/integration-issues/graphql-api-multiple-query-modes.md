---
title: "velog GraphQL API: Multiple Query Modes with Mixed Pagination"
category: integration-issues
tags: [graphql, rust, clap, serde, pagination, velog-api, input-validation]
module: client, handlers, cli, models
symptom: "v3 API uses input wrapper pattern; introspection disabled; responses omit boolean fields; mixed offset/cursor pagination"
root_cause: "v3 and v2 APIs have different query patterns, response shapes, and pagination strategies"
date: 2026-03-12
---

# velog GraphQL API: Multiple Query Modes with Mixed Pagination

## Problem

Extending `velog post list` to support trending, recent, and user post listing required integrating with two different API versions (v2 and v3) that have incompatible patterns:

1. **v3 `input` wrapper**: `trendingPosts` and `recentPosts` wrap arguments in an `input` object, but introspection is disabled so the exact input type name is unknown.
2. **Missing boolean fields**: v3 responses omit `is_temp` and `is_private`, causing serde deserialization failures.
3. **Mixed pagination**: trending uses offset-based, recent/user use cursor-based.
4. **Mutual exclusion**: 4 list modes must be mutually exclusive with dependent flags.

## Investigation

- Tested v2 API with `trendingPosts` query → empty results (v2 doesn't serve trending)
- Tested v3 API with named input type `TrendingPostsInput!` → unknown type (introspection disabled)
- Tested v3 with inline scalar variables → **works**
- Deserialization of v3 responses failed on `is_temp` field → v3 doesn't return it

## Solution

### 1. v3 GraphQL `input` Wrapper — Inline Scalar Variables

When the API uses an `input` object wrapper but introspection is disabled, declare scalar variables at the query level and pass them inside the inline `input: { ... }` object:

```rust
// src/client.rs — v3 trending (offset-based)
const GET_TRENDING_POSTS_QUERY: &str = r#"
    query ($limit: Int, $offset: Int, $timeframe: String) {
        trendingPosts(input: { limit: $limit, offset: $offset, timeframe: $timeframe }) {
            id title short_description thumbnail
            likes url_slug released_at updated_at tags
            user { username }
        }
    }
"#;

// v3 recent (cursor-based)
const GET_RECENT_POSTS_QUERY: &str = r#"
    query ($limit: Int, $cursor: ID) {
        recentPosts(input: { limit: $limit, cursor: $cursor }) {
            id title short_description thumbnail
            likes url_slug released_at updated_at tags
            user { username }
        }
    }
"#;
```

The v2 `posts` query does NOT use an `input` wrapper — it accepts args directly.

### 2. `#[serde(default)]` for Optional Booleans

v3 API responses omit `is_temp` and `is_private` entirely. `bool`'s `Default` returns `false`:

```rust
// src/models.rs
pub struct Post {
    // ...
    #[serde(default)]
    pub is_private: bool,
    #[serde(default)]
    pub is_temp: bool,
    // ...
}
```

### 3. `PagingHint` Enum for Mixed Pagination

A single `emit_public_posts` function handles both pagination styles via an enum:

```rust
// src/handlers.rs
enum PagingHint {
    Cursor,
    Offset(u32),
}
```

- Pretty mode: `Cursor` prints `--cursor <id>`, `Offset` prints `--offset N`
- Compact mode: `Cursor` emits `{"posts": [...], "next_cursor": "..."}`, `Offset` emits `{"posts": [...], "next_offset": N}`

**Bug caught in code review**: Original code always showed `--cursor` hint even for offset-based trending. The `PagingHint` enum was the fix.

### 4. Clap Mutual Exclusion with `conflicts_with_all`

```rust
// src/cli.rs
#[arg(long, conflicts_with_all = ["trending", "recent", "username"])]
drafts: bool,
#[arg(long, conflicts_with_all = ["drafts", "recent", "username"])]
trending: bool,
// ... etc.

// Dependent flags use `requires`
#[arg(long, value_enum, requires = "trending")]
period: Option<Period>,
#[arg(long, requires = "trending")]
offset: Option<u32>,
```

### 5. Input Validation

```rust
// src/cli.rs — limit bounded at parse time
#[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..=100))]
limit: u32,

// src/handlers.rs — string validation
fn validate_username(u: &str) -> anyhow::Result<()> { /* alphanumeric, -, _, max 64 */ }
fn validate_cursor(c: &str) -> anyhow::Result<()> { /* ASCII, max 128 */ }
```

## Endpoint Routing Summary

| Mode | API | Auth | Pagination |
|------|-----|------|------------|
| `--trending` | v3 (`trendingPosts`) | Anonymous | Offset |
| `--recent` | v3 (`recentPosts`) | Anonymous | Cursor |
| `-u <username>` | v2 (`posts`) | Anonymous | Cursor |
| default / `--drafts` | v2 (`posts`) | Required | None (full fetch) |

## Prevention Strategies

1. **Always curl-test GraphQL queries before coding.** Especially for APIs with disabled introspection — verify the exact query pattern works.
2. **Use `#[serde(default)]` for all booleans that might be absent** in any API version. Add deserialization tests with minimal JSON (missing optional fields).
3. **Make mode-specific behavior explicit via enum parameters.** When a helper handles multiple modes with different behaviors (pagination style, output format), use an enum — not a one-size-fits-all approach.
4. **Validate every user-facing input at the boundary.** Use clap `value_parser` for numbers, `validate_*` functions for strings. Define constraints once and apply everywhere.
5. **For 3+ mutually exclusive flags, consider clap ArgGroup** instead of N-way `conflicts_with_all` to prevent the conflict graph from drifting on additions.

## Testing Recommendations

- **Deserialization tests**: Minimal JSON payloads missing optional fields (e.g., no `is_temp`, no `user`)
- **CLI integration tests**: All mutual exclusion combinations, `requires` dependency violations, invalid value rejection
- **Pagination mode tests**: Verify correct hint type per mode (cursor vs offset)

## Related

- `docs/plans/2026-03-12-feat-trending-recent-user-posts-plan.md`
- `docs/brainstorms/2026-03-12-feed-trending-brainstorm.md`
- `docs/solutions/logic-errors/plan-review-checklist-rust-cli.md` (Pattern #2: `*Data` wrappers, Pattern #3: `&'static str` queries)
