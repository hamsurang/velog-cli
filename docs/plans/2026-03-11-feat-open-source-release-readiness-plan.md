---
title: "Open Source Release Readiness"
type: feat
status: completed
date: 2026-03-11
origin: docs/brainstorms/2026-03-11-open-source-readiness-brainstorm.md
---

# Open Source Release Readiness

## Overview

velog-cli를 `hamsurang/velog-cli`에 오픈소스로 공개하기 위한 준비 작업. 라이선스, 문서, CI/CD 파이프라인, 단위테스트, 리포지토리 위생을 정비한다.

(see brainstorm: docs/brainstorms/2026-03-11-open-source-readiness-brainstorm.md)

## Phase 1: 리포지토리 위생 + 라이선스

- [x] `LICENSE` 파일 생성 (MIT)
- [x] `Cargo.toml` 메타데이터 추가

```toml
# Cargo.toml
[package]
license = "MIT"
description = "Command-line interface for velog.io"
repository = "https://github.com/hamsurang/velog-cli"
homepage = "https://github.com/hamsurang/velog-cli"
keywords = ["velog", "blog", "cli", "markdown"]
categories = ["command-line-utilities"]
readme = "README.md"
```

- [x] `.gitignore` 업데이트 (`.omc/` 추가)
- [x] `.cargo/config.toml` 확인 — `fmt-check`, `lint` 별칭이 CI에서 사용됨
- [x] `cargo-audit` CI 단계 추가 (`ci.yml`에 `rustsec/audit-check` action)

## Phase 2: README.md (EN + KR)

- [x] README.md 작성 — 이중 언어

**구조:**
1. 프로젝트 소개 + 배지 (CI, crates.io, license)
2. Installation (cargo install, brew, GitHub Release)
3. Quick Start (auth login → post list → post create)
4. Commands Reference (표)
5. 한국어 섹션 (설치, 사용법 요약)
6. Contributing + License

## Phase 3: 핵심 로직 단위테스트

- [x] `parse_tags` 테스트 (`src/handlers.rs`)
  - 기본 분리, 공백 트림, 중복 제거, 빈 문자열
- [x] `validate_velog_jwt` 테스트 (`src/auth.rs`)
  - 유효 JWT, 잘못된 포맷, 잘못된 iss, 잘못된 sub, 만료된 exp
- [x] `is_auth_error` 테스트 (`src/models.rs`)
  - extension code UNAUTHENTICATED, 문자열 fallback, 대소문자 무시, 비인증 에러
- [x] slug 검증 로직 테스트 (`src/handlers.rs`)
  - 유효 slug, 대문자 포함, 연속 하이픈, 시작/끝 하이픈

**참고:** `parse_tags`, slug 검증은 private 함수이므로 `#[cfg(test)] mod tests`를 각 모듈 내부에 추가. `validate_velog_jwt`와 `is_auth_error`는 pub이므로 `tests/` 디렉토리에서도 가능하지만 모듈 내부 테스트가 관례적.

## Phase 4: cargo-dist Release Workflow

- [x] `Cargo.toml`에 `[workspace.metadata.dist]`와 `[profile.dist]` 직접 추가
  - chromaport의 설정 참조: `cargo-dist-version = "0.31.0"`
  - targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`
  - installers: `["homebrew"]`
  - tap: `"hamsurang/homebrew-velog-cli"`
  - publish-jobs: `["homebrew"]`

```toml
# Cargo.toml에 추가
[profile.dist]
inherits = "release"
lto = "thin"

[workspace.metadata.dist]
cargo-dist-version = "0.31.0"
ci = "github"
installers = ["homebrew"]
tap = "hamsurang/homebrew-velog-cli"
publish-jobs = ["homebrew"]
targets = ["aarch64-apple-darwin", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu"]
```

- [x] `cargo dist generate` 또는 chromaport `release.yml` 기반으로 `.github/workflows/release.yml` 생성
- [ ] GitHub에서 `hamsurang/homebrew-velog-cli` 리포 생성 (수동)
- [ ] `HOMEBREW_TAP_TOKEN` secret 설정 (수동)

## Phase 5: CONTRIBUTING.md

- [x] 간단한 기여 가이드 작성
  - PR 프로세스
  - `cargo fmt`, `cargo clippy`, `cargo test` 통과 필수
  - 커밋 메시지 컨벤션 (conventional commits)

## Acceptance Criteria

- [ ] `LICENSE` 파일 존재 (MIT)
- [ ] `Cargo.toml`에 `license`, `description`, `repository`, `homepage`, `keywords`, `categories`, `readme` 필드
- [ ] `.gitignore`에 `.omc/` 포함
- [ ] `README.md` 존재 (EN + KR, 설치/사용법/명령어 레퍼런스)
- [ ] 핵심 로직 단위테스트 4종 추가 (`parse_tags`, `validate_velog_jwt`, `is_auth_error`, slug 검증)
- [ ] `cargo test` 전체 통과
- [ ] `.github/workflows/release.yml` 존재 (cargo-dist 기반)
- [ ] `Cargo.toml`에 `[workspace.metadata.dist]` 설정
- [ ] `CONTRIBUTING.md` 존재

## Dependencies & Risks

- **HOMEBREW_TAP_TOKEN**: GitHub org secret 설정 필요 — 리포 생성은 수동
- **cargo-dist 버전**: chromaport와 동일한 v0.31.0 사용으로 호환성 보장
- **기존 release profile**: 현재 `[profile.release]`에 `strip=true, opt-level="z", lto=true, codegen-units=1` 설정 → `[profile.dist]`는 `inherits = "release"`로 상속하되, cargo-dist가 `lto = "thin"` 오버라이드 (빌드 속도 vs 바이너리 크기 트레이드오프)

## Sources

- **Origin brainstorm:** [docs/brainstorms/2026-03-11-open-source-readiness-brainstorm.md](docs/brainstorms/2026-03-11-open-source-readiness-brainstorm.md)
- **chromaport reference:** `hamsurang/chromaport` — Cargo.toml 메타데이터, release.yml, homebrew tap 패턴
- **chromaport homebrew tap:** `hamsurang/homebrew-chromaport`
