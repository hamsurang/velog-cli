---
title: "Fix P1 Code Review Findings"
type: fix
status: completed
date: 2026-03-11
---

# Fix P1 Code Review Findings

## Overview

Code review에서 발견된 4건의 P1 (Critical) 항목을 수정한다. 보안, CI, 안정성, 성능 전반에 걸친 개선.

## P1-1: JWT `exp` 만료 검증 추가

**파일:** `src/auth.rs:110-129` (`validate_velog_jwt`)

**문제:** `iss`와 `sub`만 확인하고 `exp` claim을 검사하지 않는다. 만료된 토큰이 로그인 시 경고 없이 저장되어, 이후 API 호출에서 실패한다.

**해결:**
- `exp` claim 파싱 (Unix timestamp, `u64`)
- `SystemTime::now()`와 비교
- 만료 시 **경고 출력** (hard fail 아님 — refresh로 복구 가능하므로)
- 만료 임박(5분 이내) 시에도 경고

```rust
// auth.rs — validate_velog_jwt 내부 추가
if let Some(exp) = claims["exp"].as_u64() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    if exp < now {
        eprintln!("Warning: {} token is expired. It will be refreshed automatically.", expected_sub);
    } else if exp < now + 300 {
        eprintln!("Warning: {} token expires in less than 5 minutes.", expected_sub);
    }
}
```

## P1-2: CI `permissions` 설정

**파일:** `.github/workflows/ci.yml`

**문제:** `permissions` 블록이 없어 기본적으로 과도한 권한(write-all)이 부여된다. 최소 권한 원칙 위반.

**해결:**
- workflow 최상위에 `permissions: contents: read` 추가

```yaml
permissions:
  contents: read
```

`on:` 블록과 `concurrency:` 블록 사이에 삽입.

## P1-3: `is_auth_error` 문자열 매칭 개선

**파일:** `src/models.rs:41-51` (`GraphQLResponse::is_auth_error`)

**문제:** `"not logged in"`, `"Unauthorized"` 등 서버 메시지 문자열에 의존한다. 서버 메시지가 변경되면 토큰 갱신이 작동하지 않는다.

**해결:**
- extension code `"UNAUTHENTICATED"` 검사를 **최우선**으로
- 문자열 매칭은 fallback으로 유지 (extension이 없는 구형 응답 대비)
- 대소문자 무시 비교 추가

```rust
pub fn is_auth_error(&self) -> bool {
    self.errors.as_ref().is_some_and(|errs| {
        errs.iter().any(|e| {
            // 1차: extension code (안정적)
            let has_code = e.extensions
                .as_ref()
                .and_then(|ext| ext.get("code"))
                .and_then(|v| v.as_str())
                .is_some_and(|c| c == "UNAUTHENTICATED");
            // 2차: 메시지 fallback (구형 응답 호환)
            let has_msg = {
                let msg = e.message.to_lowercase();
                msg.contains("not logged in") || msg.contains("unauthorized")
            };
            has_code || has_msg
        })
    })
}
```

## P1-4: `currentUser` 불필요 호출 제거 (username 캐싱)

**파일:** `src/auth.rs` (Credentials), `src/handlers.rs:30-36` (`with_auth_client`)

**문제:** 모든 인증 커맨드가 `currentUser` API를 호출하여 username을 가져온다. 불필요한 네트워크 RTT (~300ms).

**해결:**
1. `Credentials`에 `username` 필드 추가 (Optional, 기존 파일 호환)
2. `auth_login` 시 username 저장
3. `with_auth_client()`에서 캐시된 username 사용, 없으면 기존 로직 fallback

```rust
// auth.rs
#[derive(Serialize, Deserialize, Clone)]
pub struct Credentials {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}
```

```rust
// handlers.rs — with_auth_client 대체
async fn with_auth_client() -> anyhow::Result<(VelogClient, String)> {
    let creds = require_auth()?;
    let mut client = VelogClient::new(creds.clone())?;

    let username = if let Some(u) = creds.username {
        u
    } else {
        // 캐시 미스: API 호출 후 username 저장
        let (user, new_creds) = client.current_user().await?;
        let mut save_creds = new_creds.unwrap_or(creds);
        save_creds.username = Some(user.username.clone());
        auth::save_credentials(&save_creds)?;
        user.username
    };

    Ok((client, username))
}
```

- 반환 타입이 `(VelogClient, User)` → `(VelogClient, String)`으로 변경
- 호출부 (`post_list`, `post_show`, `post_edit` 등)에서 `user.username` → `username` 으로 수정
- `auth_status`는 email 표시를 위해 여전히 `currentUser` 호출 유지

## Acceptance Criteria

- [x] P1-1: `validate_velog_jwt`에서 만료된 토큰 입력 시 경고 메시지 출력
- [x] P1-2: CI workflow에 `permissions: contents: read` 존재
- [x] P1-3: `is_auth_error`가 extension code를 우선 검사
- [x] P1-3: 대소문자 무시 문자열 매칭 fallback 동작
- [x] P1-4: `Credentials`에 `username` 필드 추가, 기존 credentials.json 역호환
- [x] P1-4: 캐시된 username이 있으면 `currentUser` API 호출 스킵
- [x] P1-4: 캐시 미스 시 API 호출 후 username 자동 저장
- [x] 모든 기존 테스트 통과 (`cargo test`)
- [x] clippy 경고 없음 (`cargo clippy`)

## Implementation Order

1. **P1-2** (CI permissions) — 1줄 변경, 즉시 적용 가능
2. **P1-3** (is_auth_error) — 단일 함수 수정, 의존성 없음
3. **P1-1** (JWT exp) — 단일 함수 수정, 의존성 없음
4. **P1-4** (username 캐싱) — 가장 큰 변경, Credentials 구조체 + 핸들러 시그니처 변경

## Dependencies & Risks

- **P1-4 역호환**: `username` 필드에 `#[serde(default)]` 사용으로 기존 credentials.json 파싱 보장
- **P1-4 시그니처 변경**: `with_auth_client` 반환 타입 변경으로 모든 호출부 수정 필요
- **P1-1 시간대**: `exp`는 UTC Unix timestamp, `SystemTime::now()`도 UTC 기반이므로 문제 없음
