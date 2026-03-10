---
title: "Multi-Agent Code Review: 8 Fixes (P1+P2) Before Open Source Release"
category: logic-errors
tags:
  - rust
  - code-review
  - utf8-safety
  - toctou
  - token-refresh
  - error-handling
  - defensive-programming
module: velog-cli (client, auth, handlers)
symptom: "6-agent code review discovered 1 P1 crash bug (UTF-8 byte-slice panic), 7 P2 issues (silent error loss, cache invalidation, race condition, info disclosure, no-op guard, stale URL output, missing cache write)"
root_cause: Multiple — see individual findings below
severity: high
date_discovered: 2026-03-11
---

# Multi-Agent Code Review: 8 Fixes Before Open Source Release

## Context

Before publishing `velog-cli` as open source at `hamsurang/velog-cli`, a comprehensive code review was run using 6 parallel agents: direct review, simplicity reviewer, architecture strategist, performance oracle, security sentinel, and quality reviewer.

The review discovered 1 P1 (runtime crash), 7 P2 (important logic/security issues), and 6 P3 (nice-to-have). All P1+P2 were fixed in a single commit. 39 tests pass, clippy clean.

**Related documents:**
- Open source plan: `docs/plans/2026-03-11-feat-open-source-release-readiness-plan.md`
- Previous plan review: `docs/solutions/logic-errors/plan-review-checklist-rust-cli.md`

---

## Findings and Fixes

### 1. UTF-8 Byte-Slice Panic on Non-ASCII API Responses (P1)

**Symptom:** Runtime panic when velog API returns Korean error messages.

**Root cause:** `&body[..body.len().min(500)]` slices a `String` by byte offset. If byte 500 falls inside a multi-byte UTF-8 character (Korean is 3 bytes per char), Rust panics with `byte index N is not a char boundary`.

**Fix:**
```rust
// WRONG: panics on non-ASCII
&body[..body.len().min(500)]

// CORRECT: char-boundary safe
let preview: String = body.chars().take(200).collect();
```

**Prevention:** Never use byte-index slicing (`&s[..n]`) on strings that may contain non-ASCII. Always use `.chars().take(n)` or `.char_indices()`. This applies to any string from external sources (API responses, user input, file content).

---

### 2. Token Refresh Silently Discards Original Error (P2)

**Symptom:** User sees only "Token refresh failed. Run `velog auth login` again." with no diagnostic info, even when the root cause is a network timeout or JSON parse error.

**Root cause:** `map_err(|_| ...)` — the closure parameter `_` discards the original error.

**Fix:**
```rust
// WRONG: original error lost
.map_err(|_| { anyhow::Error::new(AuthError).context("Token refresh failed...") })

// CORRECT: original error preserved as context
.map_err(|e| { anyhow::Error::new(AuthError).context(format!("Token refresh failed: {e:#}...")) })
```

**Prevention:** Never use `|_|` in `map_err` unless the error is genuinely uninformative. Always capture and include the original error.

---

### 3. Token Refresh Loses Cached Username (P2)

**Symptom:** After token refresh, every subsequent CLI invocation makes an unnecessary `currentUser` API call even though the username was previously cached.

**Root cause:** `From<UserToken> for Credentials` sets `username: None`. When `execute_graphql` refreshes tokens, the new credentials overwrite the disk file without the cached username.

**Fix:** In `execute_graphql`, after `restore_token()`, carry over the existing username:
```rust
let mut new_creds = self.restore_token().await?;
new_creds.username = self.credentials.as_ref().and_then(|c| c.username.clone());
```

**Prevention:** When replacing a struct that has cached/derived fields, explicitly copy those fields from the old value. Token refresh should only change token fields, not auxiliary caches.

---

### 4. TOCTOU Race in `load_credentials` (P2)

**Symptom:** Misleading "Cannot read credentials" error if `velog auth logout` runs concurrently.

**Root cause:** `path.exists()` check followed by `read_to_string()` — between the two syscalls, the file can be deleted.

**Fix:**
```rust
// WRONG: TOCTOU race
if !path.exists() { return Ok(None); }
let content = std::fs::read_to_string(&path)?;

// CORRECT: atomic check via error kind
match std::fs::read_to_string(&path) {
    Ok(c) => { /* parse */ }
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
    Err(e) => Err(anyhow::Error::new(e).context("Cannot read credentials")),
}
```

**Prevention:** Never use `path.exists()` + `read/write`. Instead, attempt the operation and handle `NotFound` in the error path. This is a general Rust and systems programming pattern — "ask forgiveness, not permission."

---

### 5. API Error Response Body Information Disclosure (P2)

**Symptom:** Raw server response body (up to 500 chars) printed to user's terminal, potentially including internal error details, stack traces, or debug info.

**Fix:** Reduced preview to 200 chars and switched to char-safe truncation (combined with P1 fix). For higher-security contexts, consider logging raw bodies only at debug level.

**Prevention:** Never include raw external API responses in user-facing error messages verbatim. Truncate, sanitize, or log at debug level.

---

### 6. `post_edit` No-Op Not Guarded (P2)

**Symptom:** `velog post edit my-slug` (no `--file`, `--title`, or `--tags`) silently makes an API call and prints "Post updated." even though nothing changed.

**Fix:**
```rust
anyhow::ensure!(
    file.is_some() || title.is_some() || tags.is_some(),
    "Nothing to edit. Provide --file, --title, or --tags."
);
```

**Prevention:** For update/edit commands with optional fields, always validate that at least one field was provided before making the API call.

---

### 7. Mutation Response `url_slug` Discarded (P2)

**Symptom:** After editing or publishing a post, the CLI prints a URL using the *input* slug, not the server-returned canonical slug. If the server normalizes the slug, the printed URL is wrong.

**Fix:** Use `post.url_slug` from the mutation response instead of the locally-captured `url_slug`:
```rust
// WRONG: uses pre-mutation slug
let url_slug = existing.url_slug.clone();
let (_post, new_creds) = client.edit_post(input).await?;
println!("https://velog.io/@{}/{}", username, url_slug);

// CORRECT: uses server-canonical slug
let (post, new_creds) = client.edit_post(input).await?;
println!("https://velog.io/@{}/{}", username, post.url_slug);
```

**Prevention:** When an API returns a canonical version of an input field (URL, slug, ID), always use the returned value in subsequent operations and display.

---

### 8. `auth_status` Doesn't Cache Username (P2)

**Symptom:** Running `velog auth status` (which calls `currentUser`) never saves the username to the credentials file, so subsequent commands still hit the `currentUser` API unnecessarily.

**Fix:** After the `currentUser` call in `auth_status`, save the username if not already cached:
```rust
let mut save_creds = new_creds.unwrap_or(creds);
if save_creds.username.is_none() {
    save_creds.username = Some(user.username.clone());
    auth::save_credentials(&save_creds)?;
}
```

**Prevention:** Any handler that resolves the username via API should check if the credentials file already has it cached, and save it if not. This is a "cache on every successful resolution" pattern.

---

## Prevention Checklists

### Rust String Safety Checklist
- [ ] No byte-index slicing (`&s[..n]`) on strings from external sources
- [ ] `.chars().take(n)` or `.char_indices()` used for truncation
- [ ] Date/time string slicing uses `.chars().take(10)` not `&date[..10]`

### Error Handling Checklist
- [ ] `map_err` closures capture the original error (no `|_|`)
- [ ] Error messages do not expose raw external API response bodies
- [ ] File operations use `ErrorKind::NotFound` matching, not `path.exists()` guards
- [ ] Token refresh error includes the underlying failure cause

### Cache Consistency Checklist
- [ ] Token refresh preserves cached/derived fields (username, preferences)
- [ ] Every API call that resolves a cacheable value writes it back to disk
- [ ] `From`/`Into` conversions explicitly handle auxiliary fields

### Command Input Validation Checklist
- [ ] Edit/update commands require at least one field to be provided
- [ ] Server-returned canonical values (slug, ID) used over client-side values
- [ ] Interactive prompts have TTY guards + `--yes` flags
