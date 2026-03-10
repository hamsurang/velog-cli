---
title: "feat: E2E Browser Testing with fantoccini"
type: feat
status: completed
date: 2026-03-10
origin: docs/brainstorms/2026-03-10-rust-e2e-browser-testing-brainstorm.md
---

# E2E Browser Testing with fantoccini

## Enhancement Summary

**Deepened on:** 2026-03-10
**Sections enhanced:** 6
**Research agents used:** fantoccini docs, architecture reviewer, security reviewer, spec flow analyzer, velog.io DOM researcher

### Key Improvements
1. **PostGuard 강화** — 에러 로깅 + CI orphan sweep `always()` 단계 추가 (silent failure 방지)
2. **CDN 폴링 정밀화** — 요소 존재가 아닌 제목 텍스트 매칭으로 stale cache false positive 차단
3. **CI 보안 강화** — `permissions: contents: read`, 스크린샷 `retention-days: 3`, ChromeDriver `--allowed-ips`
4. **Credential 격리** — `CredentialsGuard`로 테스트 후 자동 삭제, `TestConfig` Debug redaction
5. **tokio rt-multi-thread** — fantoccini가 multi-thread runtime 필요, feature flag로 조건부 활성화

### New Considerations Discovered
- `Drop` trait은 SIGKILL/OOM/cancel 시 호출되지 않음 → orphan sweep CI 단계 필수
- 현재 tokio가 `rt` + `macros`만 사용 → E2E에 `rt-multi-thread` 필요
- 기존 CI의 `cancel-in-progress: true`가 E2E에 위험 → 별도 workflow 파일 권장

---

## Overview

velog-cli의 전체 플로우를 검증하는 E2E 테스트를 추가한다. `assert_cmd`로 CLI를 실행하고, `fantoccini`(WebDriver)로 headless Chrome에서 velog.io 페이지에 결과가 반영되었는지 확인한다. (see brainstorm: docs/brainstorms/2026-03-10-rust-e2e-browser-testing-brainstorm.md)

## Problem Statement

현재 테스트는 CLI 파싱(assert_cmd 8개)만 커버한다. 실제 API 호출, 데이터 저장, 웹 렌더링까지의 end-to-end 검증이 없어 regression을 잡지 못한다.

## Proposed Solution

### Architecture

```
[cargo test --features e2e]
        │
        ▼
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│  assert_cmd  │────▶│ velog GraphQL │────▶│  velog.io    │
│  (CLI 실행)   │     │  API (v3)    │     │ (Next.js)   │
└─────────────┘     └──────────────┘     └──────┬──────┘
                                                 │
                                          ┌──────▼──────┐
                                          │ fantoccini   │
                                          │ (headless    │
                                          │  Chrome)     │
                                          └─────────────┘
```

### 핵심 설계 결정

1. **fantoccini 0.22 + ChromeDriver** — Rust-native, tokio 기반 (see brainstorm)
2. **Published 글만 브라우저 검증** — draft는 공개 URL 404이므로 API 레벨만 검증
3. **각 테스트 self-contained** — create → verify → delete, 순서 의존 없음
4. **credential 직접 주입** — `auth::save_credentials()` 호출로 credentials.json 생성 (interactive login 우회)
5. **`[e2e]` 접두사 + PostGuard + orphan sweep** — 다층 cleanup 전략
6. **feature flag `e2e`** — 일반 `cargo test`에서 제외, 별도 CI workflow

## Implementation Phases

### Phase 1: Infrastructure Setup

#### Cargo.toml

- [x] `[features]` 섹션에 `e2e = []` 추가
- [x] dev-dependencies에 `fantoccini = "0.21"` 추가
- [x] `[[test]]` 섹션으로 e2e 테스트 바이너리 등록

```toml
# Cargo.toml 추가 내용

[features]
e2e = []

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
fantoccini = "0.22"
serde_json = "1"

[[test]]
name = "e2e_tests"
path = "tests/e2e_tests.rs"
required-features = ["e2e"]
```

#### Research Insights: Cargo Feature Gate

**Best Practices:**
- `[[test]] required-features = ["e2e"]`로 등록하면 일반 `cargo test`에서 자동 스킵됨
- `fantoccini`는 `[dev-dependencies]`에 무조건 포함해도 됨 — test binary가 빌드되지 않으면 링크 안됨
- tokio의 `rt-multi-thread` feature가 fantoccini에 필요할 수 있음 — E2E 테스트 파일에서 `#[tokio::test(flavor = "multi_thread")]` 사용 권장

**Edge Cases:**
- 현재 `Cargo.toml`의 tokio는 `features = ["rt", "macros"]`만 사용 (single-thread). E2E 테스트의 tokio 런타임은 dev-dependencies의 별도 tokio로 해결 가능:

```toml
[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

#### tests/e2e_helpers.rs

- [x] `TestConfig` struct (환경변수에서 credentials, username, webdriver URL 로드)
- [x] `TestConfig`에 **redacting Debug impl** (토큰 마스킹)
- [x] `setup_credentials()` — CI secrets → credentials.json 직접 작성
- [x] `CredentialsGuard` — Drop에서 credentials.json 삭제
- [x] `make_browser()` — headless Chrome fantoccini 클라이언트 생성
- [x] `PostGuard` struct — Drop에서 글 삭제 **+ 에러 로깅**
- [x] `orphan_sweep()` — `[e2e]` 접두사 글 일괄 삭제
- [x] `timestamp()` — 고유 slug/title 생성용 유닉스 타임스탬프

```rust
// tests/e2e_helpers.rs

use velog_cli::auth::{self, Credentials};

pub struct TestConfig {
    pub username: String,
    pub access_token: String,
    pub refresh_token: String,
    pub webdriver_url: String,
}

// Security: 토큰 값이 로그에 노출되지 않도록 redacting Debug
impl std::fmt::Debug for TestConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestConfig")
            .field("username", &self.username)
            .field("access_token", &"[REDACTED]")
            .field("refresh_token", &"[REDACTED]")
            .field("webdriver_url", &self.webdriver_url)
            .finish()
    }
}

impl TestConfig {
    pub fn from_env() -> Self {
        Self {
            username: std::env::var("VELOG_TEST_USERNAME")
                .expect("VELOG_TEST_USERNAME required"),
            access_token: std::env::var("VELOG_TEST_ACCESS_TOKEN")
                .expect("VELOG_TEST_ACCESS_TOKEN required"),
            refresh_token: std::env::var("VELOG_TEST_REFRESH_TOKEN")
                .expect("VELOG_TEST_REFRESH_TOKEN required"),
            webdriver_url: std::env::var("WEBDRIVER_URL")
                .unwrap_or_else(|_| "http://localhost:9515".into()),
        }
    }
}

pub fn setup_credentials(config: &TestConfig) {
    let creds = Credentials {
        access_token: config.access_token.clone(),
        refresh_token: config.refresh_token.clone(),
    };
    auth::save_credentials(&creds).expect("failed to write test credentials");
}

/// 테스트 후 credentials.json 자동 삭제
pub struct CredentialsGuard;
impl Drop for CredentialsGuard {
    fn drop(&mut self) {
        let _ = auth::delete_credentials();
    }
}

pub async fn make_browser(config: &TestConfig) -> fantoccini::Client {
    let mut caps = serde_json::Map::new();
    caps.insert("goog:chromeOptions".into(), serde_json::json!({
        "args": [
            "--headless=new",           // Chrome 112+ headless mode
            "--no-sandbox",             // CI runner 필수
            "--disable-dev-shm-usage",  // Docker/CI /dev/shm 부족 방지
            "--window-size=1920,1080"
        ]
    }));
    fantoccini::ClientBuilder::native()
        .capabilities(caps)
        .connect(&config.webdriver_url)
        .await
        .expect("ChromeDriver not running")
}

/// Drop 시 글 삭제 + 에러 로깅 (silent failure 방지)
pub struct PostGuard {
    pub slug: String,
}

impl Drop for PostGuard {
    fn drop(&mut self) {
        let result = std::process::Command::new(env!("CARGO_BIN_EXE_velog"))
            .args(["post", "delete", "--slug", &self.slug, "--yes"])
            .env("HOME", std::env::var("HOME").unwrap_or_default())
            .output();
        match result {
            Ok(o) if o.status.success() => {}
            Ok(o) => eprintln!(
                "[PostGuard] cleanup FAILED for slug '{}': {}",
                self.slug,
                String::from_utf8_lossy(&o.stderr)
            ),
            Err(e) => eprintln!(
                "[PostGuard] cleanup ERROR for slug '{}': {e}",
                self.slug
            ),
        }
    }
}

pub fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
```

#### Research Insights: Cleanup Reliability

**Critical Finding (Architecture Review):**
`Drop`은 다음 상황에서 호출되지 **않는다:**
- SIGKILL (OOM killer, runner timeout)
- `cancel-in-progress`로 CI job 취소
- tokio 런타임이 mid-await에서 종료

**Best Practice: 다층 cleanup 전략**
1. **Layer 1: PostGuard Drop** — 정상 종료 및 panic 시 cleanup (best-effort)
2. **Layer 2: CI orphan sweep** — `if: always()` 단계로 `[e2e]` 접두사 글 일괄 삭제
3. **Layer 3: 테스트 시작 시 stale orphan 정리** — 이전 실행에서 남은 글 삭제

```rust
/// CI orphan sweep: [e2e] 접두사 글 일괄 삭제 (#[ignore]로 별도 실행)
#[tokio::test]
#[ignore] // cargo test --features e2e orphan_sweep -- --ignored
async fn orphan_sweep() {
    let config = TestConfig::from_env();
    setup_credentials(&config);
    // velog post list로 [e2e] 접두사 글 찾아서 삭제
    // 구현은 Phase 2에서
}
```

### Phase 2: Core E2E Tests

#### tests/e2e_tests.rs

- [x] `e2e_create_publish_and_verify()` — 글 생성(--publish) → 브라우저에서 **제목 텍스트 매칭** 확인 → 삭제
- [x] `e2e_edit_and_verify()` — 글 생성 → 수정 → 브라우저에서 변경 확인 → 삭제
- [x] `e2e_publish_draft_and_verify()` — draft 생성 → publish → 브라우저에서 공개 URL 확인 → 삭제

```rust
// tests/e2e_tests.rs

#[path = "e2e_helpers.rs"]
mod e2e_helpers;

use e2e_helpers::*;
use assert_cmd::Command;
use fantoccini::Locator;
use std::time::Duration;

fn velog_cmd() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("velog").unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn e2e_create_publish_and_verify() {
    let config = TestConfig::from_env();
    setup_credentials(&config);
    let _creds_guard = CredentialsGuard;

    let ts = timestamp();
    let slug = format!("e2e-test-{}", ts);
    let title = format!("[e2e] Test Post {}", ts);

    // 1. CLI로 글 생성
    velog_cmd()
        .args(["post", "create", "--title", &title,
               "--file", "tests/fixtures/sample.md",
               "--tags", "e2e,test", "--slug", &slug, "--publish"])
        .assert()
        .success();

    // cleanup guard 등록 (Drop 시 삭제 시도 + 에러 로깅)
    let _guard = PostGuard { slug: slug.clone() };

    // 2. 브라우저에서 확인
    let browser = make_browser(&config).await;
    let url = format!("https://velog.io/@{}/{}", config.username, slug);

    // CDN 전파 대기: 페이지 로드 + 제목 텍스트 매칭
    // NOTE: 단순 element 존재 체크가 아닌 content-specific 폴링
    //       (stale cache의 false positive 방지)
    let mut verified = false;
    for _ in 0..30 {  // 최대 15초 (500ms × 30)
        browser.goto(&url).await.unwrap();
        if let Ok(h1) = browser.find(Locator::Css("h1")).await {
            if let Ok(text) = h1.text().await {
                if text.contains(&title) {
                    verified = true;
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    assert!(verified, "Post title not found in browser within 15s");

    browser.close().await.unwrap();
    // _guard drop → 글 삭제, _creds_guard drop → credentials 삭제
}

#[tokio::test(flavor = "multi_thread")]
async fn e2e_edit_and_verify() {
    let config = TestConfig::from_env();
    setup_credentials(&config);
    let _creds_guard = CredentialsGuard;

    let ts = timestamp();
    let slug = format!("e2e-edit-{}", ts);
    let original_title = format!("[e2e] Original {}", ts);
    let updated_title = format!("[e2e] Updated {}", ts);

    // 1. 글 생성
    velog_cmd()
        .args(["post", "create", "--title", &original_title,
               "--file", "tests/fixtures/sample.md",
               "--tags", "e2e", "--slug", &slug, "--publish"])
        .assert()
        .success();
    let _guard = PostGuard { slug: slug.clone() };

    // 2. 글 수정
    velog_cmd()
        .args(["post", "edit", "--slug", &slug, "--title", &updated_title])
        .assert()
        .success();

    // 3. 브라우저에서 변경 확인
    let browser = make_browser(&config).await;
    let url = format!("https://velog.io/@{}/{}", config.username, slug);

    let mut verified = false;
    for _ in 0..30 {
        browser.goto(&url).await.unwrap();
        if let Ok(h1) = browser.find(Locator::Css("h1")).await {
            if let Ok(text) = h1.text().await {
                if text.contains(&updated_title) {
                    verified = true;
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    assert!(verified, "Updated title not found in browser within 15s");

    browser.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn e2e_publish_draft_and_verify() {
    let config = TestConfig::from_env();
    setup_credentials(&config);
    let _creds_guard = CredentialsGuard;

    let ts = timestamp();
    let slug = format!("e2e-draft-{}", ts);
    let title = format!("[e2e] Draft {}", ts);

    // 1. draft 생성 (--publish 없음)
    velog_cmd()
        .args(["post", "create", "--title", &title,
               "--file", "tests/fixtures/sample.md",
               "--tags", "e2e", "--slug", &slug])
        .assert()
        .success();
    let _guard = PostGuard { slug: slug.clone() };

    // 2. publish
    velog_cmd()
        .args(["post", "publish", "--slug", &slug])
        .assert()
        .success();

    // 3. 공개 URL 접근 확인
    let browser = make_browser(&config).await;
    let url = format!("https://velog.io/@{}/{}", config.username, slug);

    let mut verified = false;
    for _ in 0..30 {
        browser.goto(&url).await.unwrap();
        if let Ok(h1) = browser.find(Locator::Css("h1")).await {
            if let Ok(text) = h1.text().await {
                if text.contains(&title) {
                    verified = true;
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    assert!(verified, "Published draft not accessible in browser within 15s");

    browser.close().await.unwrap();
}
```

#### tests/fixtures/sample.md

- [x] E2E 테스트용 마크다운 본문 파일

```markdown
# E2E Test Content

This is a test post created by velog-cli E2E tests.
It will be automatically deleted after the test completes.

## Code Block Test

```rust
fn main() {
    println!("Hello from E2E test!");
}
```
```

#### Research Insights: Test Scenario Design

**Critical Finding (Spec Flow Analysis):**
- **URL slug 캡처:** `post_create`가 stdout에 URL을 출력하므로, `assert_cmd`의 `.output().stdout`로 파싱 가능. 그러나 slug을 직접 지정(`--slug`)하는 것이 더 안정적.
- **Slug 충돌:** 이전 실행에서 같은 slug이 남아있으면 `writePost`가 에러 또는 접미사 추가. 타임스탬프 slug으로 충돌 방지.
- **Private post:** `--private --publish`로 생성된 글은 미인증 브라우저에서 접근 불가. E2E scope에서 제외 유지.

### Phase 3: CI Integration

#### .github/workflows/e2e.yml (별도 workflow 파일 권장)

- [x] 별도 workflow 파일로 분리 (기존 ci.yml의 `cancel-in-progress: true`와 충돌 방지)
- [x] `permissions: contents: read` 명시 (최소 권한)
- [x] `browser-actions/setup-chrome@v2` (install-chromedriver: true)
- [x] ChromeDriver `--allowed-ips=127.0.0.1` 제한
- [x] secrets 환경변수 주입 + `RUST_LOG=error` (토큰 로깅 방지)
- [x] `cancel-in-progress: false` + `timeout-minutes: 15`
- [x] 실패 시 스크린샷 artifact 업로드 (`retention-days: 3`)
- [x] `if: always()` orphan sweep 단계

```yaml
# .github/workflows/e2e.yml

name: E2E Browser Tests

on:
  push:
    branches: [main]
  # PR에서도 실행하되, secrets 접근 가능한 경우만
  pull_request:
    branches: [main]

permissions:
  contents: read  # 최소 권한 — checkout만 필요

concurrency:
  group: e2e-${{ github.ref }}
  cancel-in-progress: false  # cleanup 보장을 위해 취소 방지

jobs:
  e2e:
    # SECURITY: GitHub-hosted runner만 사용.
    # ChromeDriver는 인증 없이 localhost에 바인딩됨.
    runs-on: ubuntu-latest
    timeout-minutes: 15
    # PR에서는 secrets가 비어있을 수 있음 (fork PR)
    if: github.event_name == 'push' || github.event.pull_request.head.repo.full_name == github.repository
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Setup Chrome & ChromeDriver
        uses: browser-actions/setup-chrome@v2
        with:
          chrome-version: stable
          install-chromedriver: true

      - name: Start ChromeDriver
        run: chromedriver --port=9515 --allowed-ips="127.0.0.1" &

      - name: Run E2E tests
        run: cargo test --features e2e -- --test-threads=1
        env:
          VELOG_TEST_USERNAME: ${{ secrets.VELOG_TEST_USERNAME }}
          VELOG_TEST_ACCESS_TOKEN: ${{ secrets.VELOG_TEST_ACCESS_TOKEN }}
          VELOG_TEST_REFRESH_TOKEN: ${{ secrets.VELOG_TEST_REFRESH_TOKEN }}
          WEBDRIVER_URL: http://localhost:9515
          RUST_LOG: error  # 토큰이 debug 로그에 노출되지 않도록

      - name: Orphan post sweep
        if: always()
        run: cargo test --features e2e orphan_sweep -- --ignored --test-threads=1
        env:
          VELOG_TEST_USERNAME: ${{ secrets.VELOG_TEST_USERNAME }}
          VELOG_TEST_ACCESS_TOKEN: ${{ secrets.VELOG_TEST_ACCESS_TOKEN }}
          VELOG_TEST_REFRESH_TOKEN: ${{ secrets.VELOG_TEST_REFRESH_TOKEN }}

      - name: Upload screenshots on failure
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: e2e-screenshots
          path: tests/screenshots/
          retention-days: 3  # 민감 정보 노출 최소화
```

#### Research Insights: CI Security

**HIGH (Security Review):**
- `permissions: contents: read` 필수 — 기본값은 write 권한이 포함되어 공급망 공격에 취약
- 스크린샷 artifact에 토큰이 렌더링될 수 있음 → `retention-days: 3`으로 노출 최소화
- Fork PR에서는 secrets 접근 불가 → `if` 조건으로 fork PR 스킵

**MEDIUM (Security Review):**
- ChromeDriver `--allowed-ips=127.0.0.1` — self-hosted runner에서 동일 네트워크 프로세스의 무단 접근 차단
- `RUST_LOG=error` — debug 레벨 로그에 토큰이 노출될 가능성 차단

## Technical Considerations

### CDN 전파 지연 대응

velog.io는 Next.js + CDN으로 서빙된다. 글 생성 후 브라우저에서 즉시 접근하면 404가 될 수 있다.

**전략:** 수동 retry loop로 최대 15초(500ms × 30회) 폴링. **단순 요소 존재 체크가 아닌 제목 텍스트 매칭**으로 stale cache의 false positive를 방지한다.

**Critical Finding (Architecture Review):**
Next.js ISR은 HTTP 200으로 stale 캐시를 서빙할 수 있다. `wait().for_element(Locator::Css("h1"))`만으로는 이전 테스트의 캐시된 페이지에서 false positive가 발생한다. 반드시 **고유한 타임스탬프가 포함된 제목 텍스트**를 매칭해야 한다.

### 테스트 격리

- 각 테스트는 고유 slug 사용 (`e2e-test-{unix_timestamp}`)
- **다층 cleanup:** PostGuard Drop (Layer 1) → CI orphan sweep (Layer 2) → 테스트 시작 시 stale 정리 (Layer 3)
- `CredentialsGuard`로 테스트 후 credentials.json 자동 삭제
- `--test-threads=1`로 직렬 실행 (velog API rate limit + credential 파일 경합 방지)

### Credential 주입

`auth login`은 `rpassword`로 interactive-only이므로, E2E 셋업에서 `auth::save_credentials()`를 직접 호출하여 credentials.json을 생성한다. 테스트 종료 시 `CredentialsGuard`가 `auth::delete_credentials()`를 호출하여 CI runner에 credentials가 남지 않도록 한다.

**Note:** `save_credentials`는 `~/.config/velog-cli/credentials.json`에 쓰므로, `--test-threads=1`로 직렬 실행하여 credential 파일 경합을 방지한다.

### 브라우저 셀렉터 전략

velog.io는 외부 서비스로 DOM 구조 변경 가능. 최대한 안정적인 셀렉터 사용:
- 제목: `h1` (포스트 페이지의 첫 h1) + **텍스트 매칭으로 이중 검증**
- 본문: page source 텍스트 검색 (CSS-in-JS 셀렉터는 불안정)
- 태그: `a[href*="/tags/"]` 패턴
- 404 감지: `h2` 텍스트에 "존재하지 않는" 포함 여부

**Reference:** velog 프론트엔드 소스: https://github.com/velopert/velog-client

### Token 만료 대응

**진단 전략:**
- `current_user()` 실패 → access_token 만료 (restore_token으로 자동 갱신 시도)
- `restore_token()` 실패 → refresh_token도 만료 → CI secrets 갱신 필요
- CI에서 pre-flight check로 `velog auth status` 실행 고려 (선택)

## Acceptance Criteria

- [x] `cargo test` — 기존 8개 CLI 테스트 통과 (e2e 미실행)
- [ ] `cargo test --features e2e` — 3개 E2E 시나리오 통과 (CI + secrets 필요)
  - [ ] create + publish → 브라우저 제목 텍스트 매칭 → delete
  - [ ] edit → 브라우저 변경 제목 매칭 → delete
  - [ ] publish draft → 공개 URL 접근 + 제목 매칭 → delete
- [x] PostGuard 실패 시 에러 로깅 (silent failure 없음)
- [x] CI orphan sweep이 `always()` 단계로 실행
- [x] CredentialsGuard로 테스트 후 credentials.json 자동 삭제
- [x] CI workflow에 `permissions: contents: read` 설정
- [x] 스크린샷 artifact `retention-days: 3`
- [x] 기존 CI job에 영향 없음

## Dependencies & Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| velog.io DOM 구조 변경 | 셀렉터 깨짐 | h1 + 텍스트 매칭, fallback text search |
| CDN 전파 지연 | flaky test | content-specific 15초 폴링 |
| 테스트 계정 토큰 만료 | CI 전체 실패 | CI secrets 정기 갱신, restore_token 자동 갱신 |
| velog API rate limit | 연속 실패 | --test-threads=1 직렬 실행 |
| ChromeDriver 버전 불일치 | 연결 실패 | setup-chrome@v2의 install-chromedriver |
| SIGKILL/OOM으로 Drop 미실행 | orphan post | CI orphan sweep `always()` 단계 |
| CI runner에 credential 잔류 | 보안 | CredentialsGuard + ephemeral runner |
| Fork PR에서 secrets 미접근 | E2E 스킵 | `if` 조건으로 fork PR 필터링 |

## Sources & References

- **Origin brainstorm:** [docs/brainstorms/2026-03-10-rust-e2e-browser-testing-brainstorm.md](docs/brainstorms/2026-03-10-rust-e2e-browser-testing-brainstorm.md) — Key decisions: fantoccini 선택, feature flag 분리, cleanup guard
- fantoccini docs: https://docs.rs/fantoccini/latest/fantoccini/
- fantoccini GitHub: https://github.com/jonhoo/fantoccini
- fantoccini Wait API: https://docs.rs/fantoccini/latest/fantoccini/wait/struct.Wait.html
- browser-actions/setup-chrome: https://github.com/browser-actions/setup-chrome
- velog-client (프론트엔드): https://github.com/velopert/velog-client
- 기존 CLI 테스트: `tests/cli_tests.rs`
- 기존 CI: `.github/workflows/ci.yml`
