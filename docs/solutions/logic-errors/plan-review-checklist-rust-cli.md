---
title: "Rust CLI Implementation Plan Review - 44 Issues from 10 Rounds"
category: logic-errors
tags:
  - rust
  - plan-review
  - type-system
  - graphql-integration
  - api-contract
  - auth-handling
  - anyhow
  - serde
module: velog-cli (multi-module: client, auth, handlers, models)
symptom: Implementation plan contained 44+ issues across type mismatches, missing types, API contract violations, auth error handling gaps, and incomplete handler logic that would have caused compilation failures and runtime bugs
root_cause: Specification gap between GraphQL API contract and Rust type system; insufficient validation of plan code snippets against actual compile/runtime behavior
severity: critical
date_discovered: 2026-03-10
---

# Rust CLI Plan Review: 44 Issues in 10 Rounds

## Context

A Rust CLI implementation plan (`velog-cli`) was the source of truth for code generation. Over 10 review rounds, 44+ issues were discovered — from CRITICAL (code that would never compile) to MINOR (missing imports). This document captures the patterns for reuse in future plan reviews.

**Related documents:**
- Plan: `docs/plans/2026-03-10-feat-velog-cli-rust-implementation-plan.md`
- Brainstorm: `docs/brainstorms/2026-03-10-velog-cli-brainstorm.md`

## Issue Distribution

| Round | CRITICAL | MAJOR | HIGH | MINOR | Total |
|-------|----------|-------|------|-------|-------|
| 1-6   | 4        | 8     | 6    | 6     | 24    |
| 7     | 2        | 6     | 4    | 0     | 12    |
| 8     | 0        | 0     | 0    | 3     | 3     |
| 9     | 0        | 1     | 0    | 0     | 1     |
| 10    | 0        | 0     | 0    | 1     | 1     |

Convergence: rounds 8-10 found only MINOR/single MAJOR, confirming maturity.

---

## Top Issue Patterns (Reusable Checklist)

### 1. `anyhow::Error.downcast_ref` Fails After `.context()`

**Symptom:** Exit code 2 for auth errors never triggers.

**Root cause:** `.context("msg")` wraps the original error. `downcast_ref::<AuthError>()` only checks the outermost type (the context string), not the inner `AuthError`.

**Fix:**
```rust
// WRONG: only checks outermost
fn exit_code(err: &anyhow::Error) -> i32 {
    if err.downcast_ref::<AuthError>().is_some() { 2 } else { 1 }
}

// CORRECT: traverses full error chain
fn exit_code(err: &anyhow::Error) -> i32 {
    for cause in err.chain() {
        if cause.downcast_ref::<AuthError>().is_some() {
            return 2;
        }
    }
    1
}
```

**Prevention:** Any time you use `downcast_ref` with `anyhow`, check if `.context()` is called anywhere in the error's creation path. If so, use `err.chain()`.

---

### 2. GraphQL Response Wrapper Types Missing

**Symptom:** All API calls fail at runtime with serde deserialization errors.

**Root cause:** `GraphQLResponse<T>` requires `T` to match the JSON `data` field structure. The velog API returns:
```json
{ "data": { "currentUser": { "id": "...", "username": "..." } } }
```
Using `T = User` expects `{ "id": "...", "username": "..." }` directly — missing the `currentUser` nesting level.

**Fix:** Define wrapper types for each query:
```rust
#[derive(Deserialize)]
pub struct CurrentUserData {
    #[serde(rename = "currentUser")]
    pub current_user: User,
}
// T = CurrentUserData, not User
```

**Prevention:** For every GraphQL query/mutation, define a `*Data` wrapper type matching the JSON `data` field structure. One wrapper per operation.

---

### 3. `&str` vs `&'static str` in Generic Structs

**Symptom:** Compilation error when passing `&str` parameter to a struct field typed `&'static str`.

**Root cause:** `GraphQLRequest.query: &'static str` but `raw_graphql(query: &str)` has a non-static lifetime.

**Fix:** Since all GraphQL queries are `const` string literals, change the function parameter to `&'static str`:
```rust
async fn raw_graphql<V, T>(&self, query: &'static str, variables: Option<V>)
```

**Prevention:** When a struct field is `&'static str`, all functions that construct that struct must also accept `&'static str`.

---

### 4. Auth Token in `default_headers` Prevents Refresh

**Symptom:** Token refresh succeeds but retry still sends the old (expired) token.

**Root cause:** `reqwest::Client::default_headers` bakes headers at construction time. After refreshing `self.credentials`, the `Client` still sends the old Cookie.

**Fix:** Generate Cookie header per-request from `self.credentials`:
```rust
// Per-request: reads current credentials
if let Some(creds) = &self.credentials {
    let mut cookie = HeaderValue::from_str(&format!(
        "access_token={}; refresh_token={}", creds.access_token, creds.refresh_token
    ))?;
    cookie.set_sensitive(true);
    req = req.header(COOKIE, cookie);
}
```

**Prevention:** Never put auth tokens in `default_headers` if credentials can change during the client's lifetime.

---

### 5. String Matching for Error Classification

**Symptom:** Auth error detection works "by accident" but is fragile.

**Root cause:** `is_auth_err` checked `format!("{:#}", e).contains("not logged in")` — works only because `into_result()` preserves the substring. Any format change silently breaks retry.

**Fix:** Check `GraphQLResponse::is_auth_error()` on the structured response BEFORE calling `into_result()`:
```rust
let resp: GraphQLResponse<T> = self.raw_graphql(query, variables).await?;
if resp.data.is_none() && resp.is_auth_error() && self.credentials.is_some() {
    // retry with refreshed token
}
let data = resp.into_result()?; // string conversion happens here, after check
```

**Prevention:** Always check structured error data before converting to string-based `anyhow::Error`.

---

### 6. API Parameters That Don't Exist

**Symptom:** Runtime API error or silently ignored parameter.

**Root cause:** Plan included `limit` parameter for `get_posts` but the velog GraphQL API only supports `cursor`, `username`, `temp_only`, `tag`.

**Prevention:** Cross-reference every API parameter against the actual API documentation or schema. Don't assume standard parameters (like `limit`) exist.

---

### 7. TTY Guard for Interactive Prompts

**Symptom:** `post_delete` confirmation prompt hangs indefinitely when stdin is piped.

**Fix:**
```rust
if !yes {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        anyhow::bail!("Refusing to delete in non-interactive mode. Use --yes to confirm.");
    }
    // prompt...
}
```

**Prevention:** Every `stdin` prompt must check `IsTerminal` first. Provide `--yes`/`-y` flag for non-interactive use.

---

### 8. Missing `From` Impl Between Similar Types

**Symptom:** Compilation error when assigning API response type to credentials field.

**Root cause:** `UserToken` (API response, camelCase serde) and `Credentials` (disk storage, snake_case) have identical fields but different types. No `From` conversion defined.

**Fix:**
```rust
impl From<UserToken> for Credentials {
    fn from(t: UserToken) -> Self {
        Self { access_token: t.access_token, refresh_token: t.refresh_token }
    }
}
```

**Prevention:** When two types have overlapping fields for different contexts (API vs storage), define `From` conversions explicitly.

---

## Prevention Checklists

### Rust Plan Review Checklist

- [ ] All struct field types match function parameter types (especially lifetimes)
- [ ] `use` imports shown for every type referenced across module boundaries
- [ ] `anyhow` error chain traversal (`err.chain()`) used instead of `downcast_ref` when `.context()` is involved
- [ ] `From`/`Into` conversions defined between similar types (API ↔ storage)
- [ ] Handler function signatures documented (not just prose)
- [ ] Interactive prompts have TTY guard + non-interactive flag

### GraphQL Integration Checklist

- [ ] Response wrapper type defined for each query/mutation (`*Data` struct)
- [ ] Wrapper fields use `#[serde(rename = "camelCase")]` or `#[serde(rename_all)]` matching API response
- [ ] `GraphQLResponse<T>` uses wrapper as `T`, not raw domain type
- [ ] Mutation input types (`*Input` struct) defined with all required fields
- [ ] Auth error checked on structured `GraphQLResponse` before `into_result()`
- [ ] API parameters verified against actual API schema

### Auth Token Checklist

- [ ] Tokens NOT in `default_headers` (per-request injection)
- [ ] `&mut self` on methods that modify credentials
- [ ] `restore_token()` failure wraps with `AuthError` marker for exit code
- [ ] Token refresh retry uses new credentials immediately
- [ ] `V: Clone` bound on retry-capable methods (variables reused)
