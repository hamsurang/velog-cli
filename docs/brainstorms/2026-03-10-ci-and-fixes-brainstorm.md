# Brainstorm: CI Environment + P1/P2 Code Review Fixes

**Date**: 2026-03-10
**Status**: Complete
**Origin**: Code review findings (19 findings: 3 P1, 7 P2, 9 P3)

---

## What We're Building

1. **P1/P2 코드 리뷰 수정사항 반영** (10건)
2. **CI 환경 구축** (chromaport/deepwiki-cli 참고)

---

## P1 Fixes (3건 - Critical)

### P1-1: meta/series_id overwrite on edit/publish
- `handlers.rs:278,346` — edit/publish 시 `meta: json!({})`, `series_id: None`으로 덮어씀
- 기존 포스트의 meta/series_id를 보존해야 함
- **Fix**: Post 모델에 `meta`, `series_id` 필드 추가, edit/publish에서 기존 값 사용

### P1-2: Empty slug validation gap
- `handlers.rs:183` — slug에 빈 문자열 `""` 통과 가능
- **Fix**: `!s.is_empty()` 체크 추가

### P1-3: HeaderValue non-ASCII error context
- `client.rs:127` — `HeaderValue::from_str` 실패 시 에러 메시지 불충분
- **Fix**: `.context()` 추가

## P2 Fixes (7건 - Important)

### P2-1: AuthError location
- `main.rs:85-96` — AuthError가 main.rs에 있음
- **Fix**: `auth.rs`로 이동

### P2-2: Repeated auth boilerplate
- handlers의 모든 함수에서 `require_auth → client → current_user → maybe_save_creds` 반복
- **Fix**: `with_auth_client` 헬퍼 추출

### P2-3: Cache username
- `current_user()` 중복 호출 (post_create에서 write_post 전후로 호출)
- **Fix**: 불필요한 중복 호출 제거 (이미 username이 있으면 재호출 안 함)

### P2-4: TOCTOU in save_credentials
- `auth.rs:34-36` — write 후 permission 설정 (레이스 윈도우)
- **Fix**: tmp 파일에 쓰고 rename (atomic write)

### P2-5: post_edit body None
- `handlers.rs:256` — `existing.body` 가 `None`이면 빈 문자열이 됨
- **Fix**: body가 None이고 file도 없으면 에러 반환

### P2-6: serde rename_all verification
- GraphQL 쿼리의 snake_case 필드명과 Rust의 camelCase serde 설정이 올바른지 확인
- **Fix**: 주석 또는 테스트로 명시적 문서화

### P2-7: variables.clone() optimization
- `client.rs:150` — `execute_graphql`에서 `variables.clone()` 불필요한 경우가 대부분
- **Fix**: 인증 실패 시에만 clone 필요하므로 지연 clone 패턴 적용

---

## CI Environment (chromaport/deepwiki-cli 참고)

### CI Workflow: 5 parallel jobs
1. **fmt-check**: `cargo fmt --all -- --check`
2. **clippy**: `cargo clippy --all-targets --all-features -- -D warnings`
3. **test**: `cargo test`
4. **typos**: `crate-ci/typos@v1.36.2`
5. **ls-lint**: file naming convention check

### Config Files
- `.cargo/config.toml`: aliases (ck, fmt-check, lint)
- `rust-toolchain.toml`: stable channel
- `clippy.toml`: `allow-dbg-in-tests = true`
- `rustfmt.toml`: `newline_style = "Unix"`
- `.typos.toml`: exclude target/Cargo.lock, allow project terms
- `.ls-lint.json`: `*.rs` → snake_case

### Dev Dependencies
- `assert_cmd` 2: CLI integration testing
- `predicates` 3: assertion helpers

### Actions Used
- `dtolnay/rust-toolchain@stable`
- `Swatinem/rust-cache@v2`
- `crate-ci/typos@v1.36.2`
- Concurrency groups with `cancel-in-progress`

---

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| CI runner | ubuntu-latest | Standard for Rust CI |
| Rust channel | stable | No nightly features needed |
| Test framework | assert_cmd + predicates | CLI integration test standard |
| Typo checker | typos (crate-ci) | Fast, Rust-native |
| File naming | ls-lint | snake_case enforcement |
| Cargo aliases | ck, fmt-check, lint | DX consistency with reference repos |

---

## Reference

- [chromaport CI](https://github.com/hamsurang/chromaport)
- [deepwiki-cli CI](https://github.com/hamsurang/deepwiki-cli)
