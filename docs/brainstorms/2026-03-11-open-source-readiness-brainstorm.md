---
title: "velog-cli 오픈소스 공개 준비"
date: 2026-03-11
topic: open-source-readiness
---

# velog-cli 오픈소스 공개 준비

## What We're Building

velog-cli를 `hamsurang/velog-cli`에 오픈소스로 공개하기 위한 준비 작업. 라이선스, 문서, CI/CD, 테스트, 리포지토리 위생 등 공개 전 필수 사항을 정리한다.

## Key Decisions

### 1. 라이선스: MIT

- Rust 생태계에서 가장 일반적
- chromaport와 동일한 라이선스
- `Cargo.toml`에 `license = "MIT"` 추가 + `LICENSE` 파일 생성

### 2. 배포 채널: crates.io + Homebrew + GitHub Release

- chromaport의 `cargo-dist` 파이프라인 재활용
- `hamsurang/homebrew-velog-cli` (또는 기존 homebrew tap에 추가) 통해 brew 지원
- `cargo install velog-cli` 지원을 위한 crates.io 배포
- targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`

### 3. GitHub 리포지토리: `hamsurang/velog-cli`

- chromaport와 같은 조직 하위에 배치
- homebrew tap도 hamsurang 조직에 생성

### 4. 내부 문서 처리

- `docs/` (brainstorms, plans, solutions) — 공개 유지 (개발 과정 투명성)
- `.omc/` — `.gitignore`에 추가 (OMC 에이전트 상태 파일)
- `todos/` — 공개 유지 (개발 과정 투명성)

### 5. README: 이중 언어 (EN + KR)

- 영어 메인 + 한국어 섹션 병기
- velog 자체가 한국어 플랫폼이므로 한국어 사용자 배려

### 6. 테스트: 핵심 로직 단위테스트 추가

- `validate_velog_jwt` — JWT 파싱, exp 검증 로직
- `is_auth_error` — extension code 우선, 문자열 fallback
- `parse_tags` — 쉼표 분리, 중복 제거, 공백 트림
- slug 검증 로직 — 유효/무효 패턴

## Why This Approach

chromaport에서 이미 검증된 cargo-dist + homebrew 파이프라인이 있으므로 동일한 패턴을 재활용하면 최소 비용으로 프로덕션급 배포를 달성할 수 있다.

## Scope

### 포함

- LICENSE 파일 + Cargo.toml 메타데이터
- README.md (EN + KR)
- .gitignore 업데이트
- 핵심 로직 단위테스트
- cargo-dist 설정 (release workflow)
- homebrew tap 리포 생성 준비
- CONTRIBUTING.md (간단한 가이드)

### 제외 (추후)

- SECURITY.md (공개 후 필요 시 추가)
- CODE_OF_CONDUCT.md (Contributor Covenant 추가 시)
- zeroize 적용 (P2 이슈, 추후 결정)
- API 에러 메시지 sanitize (P2 이슈)

## Open Questions

(없음 — 모든 핵심 결정 완료)
