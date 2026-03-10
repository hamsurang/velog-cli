---
title: "feat: README brew tap 추가 & lefthook pre-commit 도입"
type: feat
status: completed
date: 2026-03-11
origin: docs/brainstorms/2026-03-11-readme-brew-tap-and-lefthook-brainstorm.md
---

# feat: README brew tap 추가 & lefthook pre-commit 도입

## Overview

두 가지 개선:
1. README Homebrew 설치 안내에 `brew tap` 단계 추가 (영문/한국어 모두)
2. lefthook을 이용한 pre-commit hook 도입으로 CI 실패 사전 방지

## Acceptance Criteria

### Part 1: README brew tap

- [x] 영문 섹션: `brew tap hamsurang/velog-cli` + `brew install velog-cli` 2단계로 변경
- [x] 한국어 섹션: 동일하게 2단계로 변경 (기존 `# Homebrew (macOS)` 코멘트 유지)

### Part 2: lefthook pre-commit

- [x] `lefthook.yml` 생성 — pre-commit hook 4개: fmt-check, clippy, typos, test
- [x] README Contributing 섹션에 개발 환경 설정 안내 추가 (lefthook + typos-cli 설치, `lefthook install`)
- [x] CONTRIBUTING.md에 lefthook 자동 검증 안내 추가 (기존 수동 커맨드 체인은 참고용으로 유지)

## Implementation

### Step 1: `lefthook.yml` 생성

```yaml
# lefthook.yml
pre-commit:
  parallel: true
  commands:
    fmt-check:
      run: cargo fmt-check
      fail_text: "Formatting 오류. `cargo fmt --all`로 수정하세요."
    clippy:
      run: cargo lint
      fail_text: "Clippy 경고. `cargo lint`으로 확인하세요."
    typos:
      run: typos
      fail_text: "오타 발견. `typos -w`로 자동 수정하세요."
    test:
      run: cargo test
      fail_text: "테스트 실패. `cargo test`로 확인하세요."
```

> `parallel: true`로 fmt-check, clippy, typos는 병렬 실행. test는 clippy와 컴파일 캐시를 공유하므로 병렬이어도 효율적.
> Cargo alias 활용: `fmt-check`, `lint` (`.cargo/config.toml`에 이미 정의됨).

### Step 2: README.md 수정

#### 2a. 영문 brew 설치 (line 12-14)

```bash
brew tap hamsurang/velog-cli
brew install velog-cli
```

#### 2b. 한국어 brew 설치 (line 116-118)

기존 combined bash 블록 구조 유지하면서 수정:
```bash
# Homebrew (macOS)
brew tap hamsurang/velog-cli
brew install velog-cli
```

#### 2c. 영문 Contributing 섹션 확장 (line 100-102)

기존 stub:
```markdown
## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
```

확장:
```markdown
## Contributing

### Development Setup

```bash
# Install lefthook (git hooks manager)
brew install lefthook

# Install typos (spell checker)
brew install typos-cli

# Activate pre-commit hooks
lefthook install
```

Pre-commit hooks automatically run `cargo fmt-check`, `clippy`, `typos`, and `cargo test` on every commit.

> **Note:** `audit` and `ls-lint` are CI-only checks. If you need to run ls-lint locally: `npx @ls-lint/ls-lint`

See [CONTRIBUTING.md](CONTRIBUTING.md) for full guidelines.
```

#### 2d. 한국어 섹션에 Contributing 추가

한국어 섹션 하단에 동일한 개발 환경 설정 안내 추가.

### Step 3: CONTRIBUTING.md 업데이트

기존 "Before Submitting a PR" 섹션에 lefthook 안내 추가:

```markdown
> **Tip:** `lefthook install`을 실행하면 위 검증이 커밋 시 자동으로 실행됩니다.
> WIP 커밋 시 hook을 건너뛰려면: `git commit --no-verify`
```

기존 수동 커맨드 체인(`cargo fmt-check && cargo lint && cargo test`)은 참고용으로 유지.

## Edge Cases & Notes

- **lefthook 미설치**: `lefthook.yml`만 있으면 아무 효과 없음. `lefthook install` 실행 필수 — README에 명시
- **typos 미설치**: hook 실행 시 "command not found" 에러 — README에 설치 안내 명시
- **느린 테스트**: `cargo test`가 30초+ 소요 가능. WIP 커밋 시 `git commit --no-verify`로 건너뛰기 가능 — CONTRIBUTING.md에 안내
- **CI와 로컬 차이**: `audit`(네트워크), `ls-lint`(Node.js)는 CI에서만 실행 — README에 명시
- **typos 설정**: `.typos.toml`, `_typos.toml` 두 파일 모두 자동 인식됨, 별도 경로 지정 불필요
- **Windows/Linux**: lefthook과 typos 모두 크로스 플랫폼 지원. brew 외에도 `cargo install lefthook`, `cargo install typos-cli`로 설치 가능

## Files to Change

| File | Action |
|------|--------|
| `lefthook.yml` | **생성** — pre-commit hook 설정 |
| `README.md` | **수정** — brew tap 2단계, Contributing 섹션 확장 |
| `CONTRIBUTING.md` | **수정** — lefthook 안내 추가 |

## Sources

- **Origin brainstorm:** [docs/brainstorms/2026-03-11-readme-brew-tap-and-lefthook-brainstorm.md](../brainstorms/2026-03-11-readme-brew-tap-and-lefthook-brainstorm.md) — Key decisions: lefthook 채택, hook 범위(fmt+clippy+typos+test), ls-lint/audit 제외, README Contributing 섹션
- Cargo aliases: `.cargo/config.toml`
- CI workflow: `.github/workflows/ci.yml`
- Homebrew tap: `hamsurang/homebrew-velog-cli` (Cargo.toml line 72)
