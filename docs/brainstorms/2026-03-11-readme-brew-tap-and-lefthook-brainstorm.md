# Brainstorm: README brew tap 추가 & lefthook pre-commit 도입

**Date:** 2026-03-11
**Status:** Draft

## What We're Building

### 1. README brew tap 단계 추가

README의 Homebrew 설치 안내에 `brew tap` 단계가 빠져 있다. 현재 `brew install hamsurang/velog-cli/velog-cli` 단축 형태만 있는데, 사용자에게 친숙한 2단계 분리 형태로 수정한다.

**Before:**
```bash
brew install hamsurang/velog-cli/velog-cli
```

**After:**
```bash
brew tap hamsurang/velog-cli
brew install velog-cli
```

영문/한국어 섹션 모두 수정 필요.

### 2. lefthook을 이용한 pre-commit hook 도입

CI에서 검증하는 6개 항목 중 5개를 커밋 전에 로컬에서 미리 잡을 수 있다. lefthook을 도입하여 CI 실패를 사전에 방지한다.

**포함할 검증 항목:**

| Hook | 명령어 | 예상 시간 |
|------|--------|----------|
| rustfmt | `cargo fmt --all -- --check` | ~1s |
| clippy | `cargo clippy --all-targets --all-features -- -D warnings` | ~5-15s |
| typos | `typos` | <1s |
| test | `cargo test` | ~10-30s |

**제외 항목:**
- `audit` (rustsec): 네트워크 의존, 코드 변경과 무관, CI에서만 실행
- `ls-lint`: Node.js(npx) 의존성이 필요하여 제외. CI에서만 실행

## Why This Approach

### lefthook 선택 이유

- **zero dependency**: Go 바이너리 하나로 동작, Node.js 불필요
- **brew로 설치 가능**: `brew install lefthook` — 프로젝트 툴체인과 일관성
- **빠른 실행**: 병렬 실행 지원으로 hook 시간 최소화
- **설정 간단**: `lefthook.yml` 하나로 관리
- husky는 Node.js 의존, pre-commit은 Python 의존 — Rust 프로젝트에 불필요한 런타임 추가

## Key Decisions

1. **brew 설치 안내**: 2단계 분리 형태로 수정 (`brew tap` → `brew install`)
2. **hook 도구**: lefthook 채택 (zero dependency, brew 설치 가능)
3. **hook 범위**: fmt-check + clippy + typos + test (ls-lint, audit 제외)
4. **hook 타이밍**: pre-commit (push가 아닌 commit 시점)
5. **ls-lint**: Node.js 의존성 회피를 위해 pre-commit에서 제외, CI에서만 실행
6. **문서 위치**: README에 Contributing 섹션 추가 (별도 파일 없음)
7. **테스트 범위**: `cargo test` 전체 실행 (Rust 특성상 파일 단위 필터링 어려움)

## Resolved Questions

1. ~~**ls-lint 실행 방식**~~: pre-commit에서 제외. Node.js 의존성을 피하고 CI에서만 검증.
2. ~~**lefthook 설치 안내 위치**~~: README에 Contributing 섹션 추가.
3. ~~**cargo test 범위**~~: 전체 테스트 실행. 보통 30초 이내로 완료.
