# Brainstorm: velog-cli

**Date**: 2026-03-10
**Status**: Complete

---

## What We're Building

Rust로 만드는 velog.io CLI 클라이언트. 사용자의 쿠키 정보를 통해 터미널에서 velog 포스트의 CRUD와 목록 조회를 수행하는 도구.

### 핵심 유즈케이스

1. **글 작성**: 로컬 마크다운 파일을 velog에 발행
2. **글 수정**: 기존 포스트를 로컬 파일로 수정 후 반영
3. **글 삭제**: 포스트 삭제
4. **글 조회**: 특정 포스트의 내용을 터미널에서 확인
5. **글 목록**: 내 포스트 목록 조회 (발행/임시저장 구분)
6. **임시저장 관리**: 임시저장 글 목록 조회 및 발행 전환

### 예상 CLI 인터페이스

```bash
# 인증
velog auth login              # 쿠키 입력으로 로그인
velog auth status             # 현재 인증 상태 확인
velog auth logout             # 저장된 쿠키 삭제

# 포스트 CRUD
velog post list               # 내 포스트 목록 (기본: 발행된 글)
velog post list --drafts      # 임시저장 글 목록
velog post show <slug>        # 특정 포스트 조회
velog post create --file <path> --title "제목" --tags "rust,cli"
velog post edit <id> --file <path>
velog post delete <id>
velog post publish <id>       # 임시저장 → 발행 전환
```

---

## Why This Approach

### 인증: 수동 쿠키 입력 (MVP)

- 사용자가 브라우저 DevTools에서 `access_token`, `refresh_token` 쿠키를 복사하여 CLI에 입력
- `~/.config/velog-cli/credentials.json`에 저장 (파일 권한 0600)
- 토큰 만료 시 `restoreToken` GraphQL 쿼리로 자동 갱신 시도
- 갱신 실패 시 재로그인 안내

**선택 이유**: 가장 단순한 구현으로 MVP를 빠르게 출시. 이메일 매직링크는 GitHub OAuth 전용 사용자를 커버하지 못할 수 있어, 쿠키 방식이 모든 사용자를 지원.

### API 방식: 수동 GraphQL 쿼리 + serde

- GraphQL 쿼리를 Rust 문자열 상수로 관리
- `reqwest`로 `https://v3.velog.io/graphql`에 POST 요청
- `serde`/`serde_json`으로 응답 역직렬화
- 사용할 쿼리가 10개 미만이므로 코드젠(graphql_client)의 복잡도가 불필요

### 출력: Rich 터미널 스타일

- `comfy-table`로 포스트 목록 테이블 출력
- `colored`/`owo-colors`로 컬러 텍스트
- 마크다운 본문은 `termimad` 또는 `bat` 스타일 렌더링 검토

### 에디터: 파일 경로 지정

- `--file` 플래그로 로컬 .md 파일 지정
- 스크립트/자동화 파이프라인과 연동 용이
- 메타데이터(title, tags, slug)는 CLI 플래그로 전달 (frontmatter 미사용)

---

## Key Decisions

| 결정 사항 | 선택 | 대안 |
|-----------|------|------|
| 인증 방식 | 수동 쿠키 입력 | 이메일 매직링크, 둘 다 지원 |
| 기능 범위 | 포스트 CRUD + 목록 + 임시저장 | CRUD만, 소셜 기능 포함 |
| CLI 프레임워크 | clap (derive) | argh |
| GraphQL 방식 | 수동 쿼리 + serde | graphql_client 코드젠 |
| 출력 스타일 | 테이블 + 색상 (Rich 스타일) | JSON 우선, 둘 다 |
| 본문 입력 | --file 플래그 | $EDITOR, 둘 다 |
| 메타데이터 입력 | CLI 플래그 (--title, --tags 등) | YAML frontmatter |
| 설정 저장 | XDG 규약 (~/.config, ~/.cache) | ~/.velog/, 환경변수만 |
| 이미지 업로드 | MVP 제외 (외부 URL 사용) | CLI에서 업로드 지원 |
| 시리즈 관리 | MVP 제외 | 기본 지원 |

---

## Tech Stack

### 핵심 의존성

| 크레이트 | 용도 |
|----------|------|
| `clap` (derive) | CLI 파싱, 서브커맨드, 자동완성 |
| `reqwest` | HTTP 클라이언트 (async, cookies) |
| `tokio` | 비동기 런타임 |
| `serde` / `serde_json` | JSON 직렬화/역직렬화 |
| `comfy-table` | 터미널 테이블 출력 |
| `colored` | 컬러 텍스트 출력 |
| `dirs` | XDG 디렉토리 경로 |
| `anyhow` / `thiserror` | 에러 핸들링 |

### 프로젝트 구조 (예상)

```
src/
  main.rs           # 엔트리포인트
  cli.rs            # clap 커맨드 정의
  client.rs         # GraphQL API 클라이언트
  auth.rs           # 인증 (쿠키 저장/로드/갱신)
  models.rs         # Post, User 등 데이터 모델
  formatter.rs      # 터미널 출력 포맷팅
  config.rs         # 설정 관리 (XDG)
  error.rs          # 커스텀 에러 타입
```

---

## velog API Reference

### 인증 헤더

```
Cookie: access_token=<jwt>; refresh_token=<jwt>
```

또는:

```
Authorization: Bearer <access_token>
```

### 주요 GraphQL 엔드포인트

**URL**: `POST https://v3.velog.io/graphql`

| 연산 | 타입 | 인증 필요 | 용도 |
|------|------|-----------|------|
| `currentUser` | Query | Yes | 현재 로그인 사용자 확인 |
| `restoreToken` | Query | Yes | 토큰 갱신 |
| `posts` | Query | No | 포스트 목록 (cursor, username, temp_only, tag) |
| `post` | Query | No | 단일 포스트 조회 (username + url_slug) |
| `writePost` | Mutation | Yes | 포스트 작성 |
| `editPost` | Mutation | Yes | 포스트 수정 |
| `removePost` | Mutation | Yes | 포스트 삭제 |

### writePost 입력 필드

```
title: String!
body: String!
tags: [String]!
is_markdown: Boolean!  (항상 true)
is_temp: Boolean!
is_private: Boolean!
url_slug: String!
thumbnail: String
meta: JSON!            (기본: {})
series_id: ID
```

### 토큰 구조

- **access_token**: JWT, 1시간 만료, `{ user_id, iss: "velog.io", sub: "access_token" }`
- **refresh_token**: JWT, 30일 만료, `{ user_id, token_id, iss: "velog.io", sub: "refresh_token" }`

---

## Resolved Questions

1. **마크다운 파일의 frontmatter 포맷**: CLI 플래그로만 메타데이터 전달. frontmatter 미사용. 파일은 순수 마크다운만 포함.
2. **이미지 업로드**: MVP에서 제외. 이미지는 외부 URL을 본문에 직접 삽입하는 방식으로 우회.
3. **시리즈(Series) 관리**: MVP에서 제외. 포스트 CRUD에 집중하고 시리즈는 추후 추가.

## Open Questions

(없음)

---

## Reference

- [velog-server](https://github.com/velopert/velog-server) — Koa + Apollo GraphQL + PostgreSQL
- [velog-client](https://github.com/velopert/velog-client) — React + Apollo Client
- [twitter-cli](https://github.com/jackwener/twitter-cli) — Python CLI 아키텍처 참고 (쿠키 인증 패턴)
- GraphQL endpoint: `https://v3.velog.io/graphql`
- REST auth: `https://api.velog.io/api/v2/auth/`
