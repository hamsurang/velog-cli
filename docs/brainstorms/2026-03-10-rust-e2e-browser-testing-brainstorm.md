---
title: "Rust E2E Testing with Browser Verification (fantoccini)"
type: exploration
status: complete
date: 2026-03-10
---

# Rust E2E Testing: CLI → API → Browser Verification

## What We're Building

velog-cli의 전체 플로우를 검증하는 E2E 테스트 파이프라인.
`assert_cmd`로 CLI를 실행하고, `fantoccini`(WebDriver)로 headless Chrome에서
실제 velog.io 페이지에 글이 생성/수정/삭제되었는지까지 확인한다.

### 테스트 시나리오 예시

```
1. velog post create --title "E2E Test Post" --file test.md --tags "test" --publish
2. fantoccini로 https://velog.io/@testuser/e2e-test-post 접속
3. 제목, 본문, 태그가 올바르게 렌더링되었는지 assert
4. velog post delete --slug e2e-test-post --yes
5. fantoccini로 같은 URL 접속 → 404 또는 not found 확인
```

## Why This Approach

### fantoccini (Rust-native WebDriver) 선택 이유

- **단일 언어:** `cargo test` 하나로 CLI + 브라우저 검증 모두 수행
- **WebDriver 표준:** ChromeDriver/GeckoDriver와 호환, headless 모드 지원
- **async:** tokio 기반으로 현재 프로젝트와 자연스럽게 통합
- **CI 친화적:** GitHub Actions에서 headless Chrome + chromedriver 설정 간단

### 대안 검토

| 대안 | 기각 이유 |
|------|-----------|
| Playwright (JS) | 두 언어/런타임 필요, 테스트 간 데이터 전달 번거로움 |
| thirtyfour (Selenium) | cookie jar 의존성 포함, fantoccini 대비 바이너리 크기 증가 |
| API 응답만 검증 | "브라우저에서 실제로 보이는가"를 검증하지 못함 |

## Key Decisions

1. **fantoccini + ChromeDriver** 조합 사용
2. **테스트 전용 velog 계정** 활용 (토큰은 CI secrets로 관리)
3. **테스트 후 cleanup:** 생성한 글은 반드시 삭제 (테스트 계정 오염 방지)
4. **CI에서 headless Chrome** 실행 (별도 job 또는 기존 test job 확장)
5. **E2E 테스트는 별도 feature flag** (`cargo test --features e2e`)로 분리
   - 일반 `cargo test`에서는 실행되지 않음
   - CI에서 별도 job으로 실행 (secrets 접근 가능한 환경에서만)

## Risks & Mitigations

### 1. 페이지 로딩 타이밍 (Flaky Test 위험)

velog.io는 Next.js 기반으로 글 생성 직후 브라우저 접속 시 CDN 캐시/SSR 빌드 지연으로 404가 발생할 수 있다.

**대응:** retry with backoff 패턴 적용. 글 생성 후 최대 N초간 폴링하며 페이지가 로드될 때까지 대기. `fantoccini`의 `wait` API 또는 직접 retry loop 구현.

### 2. 테스트 실패 시 Cleanup 보장

테스트 중간에 panic/assertion 실패 시 생성된 글이 삭제되지 않아 테스트 계정이 오염된다.

**대응:** Rust의 `Drop` trait 활용한 cleanup guard 패턴. 테스트 시작 시 생성한 slug을 guard에 등록하고, guard가 drop될 때 삭제 API를 호출한다. 또는 테스트 시작 전 `[e2e-test]` 접두사가 붙은 기존 글을 일괄 삭제하는 setup 단계를 둔다.

## Scope & Constraints

### In Scope
- post create → 브라우저 확인 → delete 사이클
- post edit → 브라우저에서 변경 확인
- post publish (draft → published) → 공개 URL 접근 가능 확인

### Out of Scope (YAGNI)
- auth login 브라우저 테스트 (토큰 기반이라 브라우저 불필요)
- 다른 사용자 글 조회 테스트 (현재 기능상 중요도 낮음)
- 성능 테스트, 부하 테스트

## Technical Notes

### 필요한 dev-dependencies

```toml
[dev-dependencies]
fantoccini = "0.21"    # WebDriver client
tokio = { version = "1", features = ["full"] }  # async test runtime
serde_json = "1"       # test fixture data
```

### ChromeDriver 설정 (CI)

```yaml
- name: Setup Chrome & ChromeDriver
  uses: browser-actions/setup-chrome@v1
  with:
    chrome-version: stable
- name: Start ChromeDriver
  run: chromedriver --port=4444 &
```

### fantoccini 기본 패턴

```rust
#[cfg(feature = "e2e")]
#[tokio::test]
async fn e2e_post_create_and_verify_in_browser() {
    let client = fantoccini::ClientBuilder::native()
        .connect("http://localhost:4444")
        .await
        .expect("WebDriver not running");

    // 1. CLI로 글 생성
    // 2. client.goto(url).await
    // 3. client.find(Locator::Css("h1")).await → 제목 확인
    // 4. CLI로 글 삭제
    // 5. cleanup

    client.close().await.unwrap();
}
```

### 환경변수 설계

```
VELOG_TEST_ACCESS_TOKEN   # 테스트 계정 access_token
VELOG_TEST_REFRESH_TOKEN  # 테스트 계정 refresh_token
VELOG_TEST_USERNAME       # 테스트 계정 username
WEBDRIVER_URL             # ChromeDriver URL (default: http://localhost:4444)
```

## Resolved Questions

- **Q: fantoccini vs thirtyfour?** → fantoccini (더 가볍고 tokio 네이티브)
- **Q: 실제 API vs mock?** → 실제 API (목적이 end-to-end 검증)
- **Q: CI에서 어떻게 실행?** → 별도 e2e job, secrets 필요, feature flag로 분리
- **Q: 테스트 계정?** → 있음, CI secrets로 토큰 관리
