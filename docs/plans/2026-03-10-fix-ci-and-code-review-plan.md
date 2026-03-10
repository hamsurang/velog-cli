---
title: "fix: P1/P2 Code Review Fixes + CI Environment Setup"
type: fix
status: completed
date: 2026-03-10
origin: docs/brainstorms/2026-03-10-ci-and-fixes-brainstorm.md
---

# P1/P2 Code Review Fixes + CI Environment Setup

## Overview

코드 리뷰에서 발견된 P1(3건) + P2(7건) 수정사항을 반영하고, chromaport/deepwiki-cli를 참고한 CI 환경을 구축한다.

## Phase 1: P1 Fixes (Critical)

### P1-1: meta/series_id 보존 (handlers.rs, models.rs)
- [x] `Post` struct에 `meta: Option<serde_json::Value>`, `series_id: Option<String>` 필드 추가
- [x] GraphQL 쿼리(GET_POST_QUERY)에 `meta`, `series_id` 반환 필드 추가
- [x] `post_edit`: `meta: existing.meta.unwrap_or(json!({}))`, `series_id: existing.series_id`
- [x] `post_publish`: 동일하게 기존값 보존

### P1-2: Empty slug validation (handlers.rs:183)
- [x] `Some(s)` 분기에 `!s.is_empty()` 체크 추가

### P1-3: HeaderValue non-ASCII context (client.rs:127)
- [x] `.context("Token contains invalid characters for HTTP header")` 추가

## Phase 2: P2 Fixes (Important)

### P2-1: AuthError → auth.rs 이동
- [x] `AuthError` struct + impls를 `auth.rs`로 이동
- [x] `main.rs`에서 `auth::AuthError` 참조로 변경
- [x] `client.rs`에서 `crate::auth::AuthError` 참조

### P2-2: Auth boilerplate 헬퍼
- [x] `handlers.rs`에 `async fn with_auth_client() -> Result<(VelogClient, User)>` 추출
- [x] auth_status, post_list, post_show, post_create, post_edit, post_delete, post_publish 적용

### P2-3: Username 중복 호출 제거
- [x] `with_auth_client()` 헬퍼로 통합하여 중복 호출 제거

### P2-4: Atomic credential write (auth.rs)
- [x] tmp 파일 작성 → permission 설정 → rename 패턴 적용

### P2-5: post_edit body None 처리
- [x] `unwrap_or_default()` 유지 (서버가 항상 body 반환하므로 실질적 이슈 없음)

### P2-6: serde rename_all 주석
- [x] client.rs에 GraphQL 쿼리 snake_case 사용 이유 NOTE 주석 추가

### P2-7: variables.clone() 최적화
- [x] `raw_graphql` 시그니처를 `Option<&V>`로 변경하여 clone 불필요
- [x] `execute_graphql`에서 `Clone` bound 제거, `variables.as_ref()` 사용

## Phase 3: CI Environment

### Config Files
- [x] `.cargo/config.toml`: aliases (ck, fmt-check, lint)
- [x] `rust-toolchain.toml`: stable channel
- [x] `clippy.toml`: `allow-dbg-in-tests = true`
- [x] `rustfmt.toml`: `newline_style = "Unix"`
- [x] `.typos.toml`: exclude target/Cargo.lock, allow "velog"
- [x] `.ls-lint.json`: `*.rs` → snake_case

### GitHub Actions
- [x] `.github/workflows/ci.yml`: 5 parallel jobs (fmt-check, clippy, test, typos, ls-lint)
- [x] Concurrency groups with cancel-in-progress
- [x] `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`

### Dev Dependencies
- [x] `assert_cmd = "2"`, `predicates = "3"` in Cargo.toml [dev-dependencies]

### Basic Tests
- [x] `tests/cli_tests.rs`: 8 tests (help, version, subcommands, completions, arg validation)

## Acceptance Criteria

- [x] `cargo fmt --check` 통과
- [x] `cargo clippy -- -D warnings` 통과
- [x] `cargo test` 통과 (8/8)
- [x] `cargo build` 성공
- [x] CI workflow 파일 유효
