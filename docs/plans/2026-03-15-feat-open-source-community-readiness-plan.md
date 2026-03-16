---
title: "Open Source Community Readiness"
type: feat
status: active
date: 2026-03-15
origin: docs/plans/2026-03-11-feat-open-source-release-readiness-plan.md
---

# Open Source Community Readiness

## Overview

velog-cli의 기본 오픈소스 공개(라이선스, README, CI, 릴리즈)는 완료된 상태. 이 계획은 GitHub Community Profile Health를 50% → 100%로 올리고, 커뮤니티 운영에 필요한 설정/문서/자동화를 추가한다.

현재 상태 분석:
- GitHub Community Profile Health: **50%** (LICENSE, README, CONTRIBUTING만 존재)
- 누락: CODE_OF_CONDUCT, SECURITY, Issue templates, PR template
- 레포 설정: description/topics 미설정, branch protection 없음, dependabot 없음

## Phase 1: GitHub 레포지토리 설정 (gh CLI)

> 코드 변경 없이 `gh` CLI로 즉시 적용 가능한 항목

- [ ] **Repository description** 설정
  ```bash
  gh repo edit --description "CLI for velog.io — manage blog posts from the terminal"
  ```

- [ ] **Repository topics** 추가
  ```bash
  gh repo edit --add-topic velog,cli,rust,blog,markdown,terminal,velog-cli
  ```

- [ ] **Homepage URL** 설정
  ```bash
  gh repo edit --homepage "https://crates.io/crates/velog-cli"
  ```

- [ ] **Delete branch on merge** 활성화
  ```bash
  gh api repos/{owner}/{repo} -X PATCH -f delete_branch_on_merge=true
  ```

### Acceptance Criteria
- [ ] `gh repo view --json description,repositoryTopics,homepageUrl` 에서 모든 값 확인
- [ ] PR 머지 시 소스 브랜치 자동 삭제 확인

---

## Phase 2: 커뮤니티 필수 문서

### 2-1. CODE_OF_CONDUCT.md

Contributor Covenant v2.1 채택.

- [ ] `CODE_OF_CONDUCT.md` 생성 (프로젝트 루트)
  - Contributor Covenant v2.1 전문
  - 연락처: GitHub Issues 또는 maintainer 이메일
  - 한국어 프로젝트이므로 영문 원본 사용 (국제 표준)

### 2-2. SECURITY.md

인증 토큰을 다루는 CLI 특성상 보안 정책이 특히 중요.

- [ ] `SECURITY.md` 생성 (프로젝트 루트)
  - **Supported Versions**: 현재 최신 릴리즈만 지원 명시
  - **Reporting**: 보안 취약점 신고 방법 (GitHub Private Vulnerability Reporting 또는 이메일)
  - **Response Timeline**: 48시간 내 확인, 7일 내 초기 대응
  - **Scope**: 토큰 저장, 네트워크 통신, 인증 흐름 관련 이슈
  - **Out of Scope**: velog.io 자체의 취약점

```markdown
# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| latest  | :white_check_mark: |
| < latest | :x:               |

## Reporting a Vulnerability

**Please do NOT open a public issue for security vulnerabilities.**

Use [GitHub Private Vulnerability Reporting](https://github.com/hamsurang/velog-cli/security/advisories/new)
to report security issues.

### What to include
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response Timeline
- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 7 days
- **Fix release**: As soon as practical

## Scope

- Token storage and handling (`~/.config/velog-cli/credentials.json`)
- Network communication with velog.io API
- Authentication flow (JWT validation, token refresh)
- Command injection via user inputs

## Out of Scope

- Vulnerabilities in velog.io itself (report to velog.io directly)
- Issues requiring physical access to the user's machine
```

### Acceptance Criteria
- [ ] GitHub Community Profile에서 CODE_OF_CONDUCT 인식
- [ ] GitHub Community Profile에서 SECURITY 인식
- [ ] `gh api repos/{owner}/{repo}/community/profile` health_percentage 상승

---

## Phase 3: Issue & PR 템플릿

### 3-1. Bug Report 템플릿

- [ ] `.github/ISSUE_TEMPLATE/bug_report.yml` 생성 (YAML form 형식)
  - OS / 아키텍처
  - velog-cli 버전 (`velog --version`)
  - 설치 방법 (Homebrew / Cargo / Binary)
  - 재현 절차
  - 기대 동작 vs 실제 동작
  - 에러 메시지 / 로그

```yaml
name: Bug Report
description: Report a bug in velog-cli
title: "[Bug]: "
labels: ["bug"]
body:
  - type: markdown
    attributes:
      value: |
        Thanks for reporting a bug! Please fill out the sections below.
  - type: input
    id: version
    attributes:
      label: velog-cli version
      description: "Output of `velog --version`"
      placeholder: "velog-cli 0.3.2"
    validations:
      required: true
  - type: dropdown
    id: install-method
    attributes:
      label: Installation method
      options:
        - Homebrew
        - Cargo install
        - GitHub Release binary
        - Built from source
    validations:
      required: true
  - type: dropdown
    id: os
    attributes:
      label: Operating System
      options:
        - macOS (Apple Silicon)
        - macOS (Intel)
        - Linux (x86_64)
        - Other
    validations:
      required: true
  - type: textarea
    id: description
    attributes:
      label: Bug description
      description: A clear description of what the bug is
    validations:
      required: true
  - type: textarea
    id: steps
    attributes:
      label: Steps to reproduce
      description: Step-by-step instructions to reproduce the issue
      placeholder: |
        1. Run `velog auth login`
        2. Enter tokens
        3. See error...
    validations:
      required: true
  - type: textarea
    id: expected
    attributes:
      label: Expected behavior
      description: What you expected to happen
    validations:
      required: true
  - type: textarea
    id: logs
    attributes:
      label: Error output / logs
      description: Paste any error messages or logs
      render: shell
```

### 3-2. Feature Request 템플릿

- [ ] `.github/ISSUE_TEMPLATE/feature_request.yml` 생성
  - 문제/동기
  - 제안하는 해결책
  - 대안
  - 추가 컨텍스트

```yaml
name: Feature Request
description: Suggest a new feature for velog-cli
title: "[Feature]: "
labels: ["enhancement"]
body:
  - type: textarea
    id: problem
    attributes:
      label: Problem or motivation
      description: What problem does this solve? Why is this needed?
    validations:
      required: true
  - type: textarea
    id: solution
    attributes:
      label: Proposed solution
      description: How should this work? Include CLI examples if possible.
      placeholder: |
        ```bash
        velog post export --format hugo my-post-slug
        ```
    validations:
      required: true
  - type: textarea
    id: alternatives
    attributes:
      label: Alternatives considered
      description: Any alternative approaches you've thought about
  - type: textarea
    id: context
    attributes:
      label: Additional context
      description: Screenshots, links, or other context
```

### 3-3. Issue Template Config

- [ ] `.github/ISSUE_TEMPLATE/config.yml` 생성

```yaml
blank_issues_enabled: true
contact_links:
  - name: velog.io Issues
    url: https://github.com/velopert/velog-client/issues
    about: For issues with velog.io itself (not velog-cli)
```

### 3-4. PR Template

- [ ] `.github/pull_request_template.md` 생성

```markdown
## Summary

<!-- Brief description of what this PR does -->

## Changes

-

## Checklist

- [ ] `cargo fmt-check` passes
- [ ] `cargo lint` passes (clippy)
- [ ] `cargo test` passes
- [ ] `typos` check passes
- [ ] Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)

## Related Issues

<!-- Closes #123, Fixes #456 -->
```

### Acceptance Criteria
- [ ] GitHub에서 새 이슈 생성 시 Bug Report / Feature Request 폼 표시
- [ ] GitHub에서 새 PR 생성 시 템플릿 자동 로드
- [ ] `gh api repos/{owner}/{repo}/community/profile` 에서 issue_template, pull_request_template 인식

---

## Phase 4: 자동화

### 4-1. Dependabot 설정

- [ ] `.github/dependabot.yml` 생성
  - Cargo 의존성: 주간 업데이트
  - GitHub Actions: 주간 업데이트

```yaml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
      day: "monday"
    labels:
      - "dependencies"
    commit-message:
      prefix: "chore(deps)"
    open-pull-requests-limit: 5

  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "weekly"
      day: "monday"
    labels:
      - "dependencies"
    commit-message:
      prefix: "chore(ci)"
    open-pull-requests-limit: 5
```

### 4-2. CHANGELOG.md

- [ ] `CHANGELOG.md` 생성 (프로젝트 루트)
  - [Keep a Changelog](https://keepachangelog.com/) 형식 채택
  - v0.1.0 ~ v0.3.2 기존 릴리즈 내역 역추적 (git log 기반)
  - 향후 릴리즈 시 `## [Unreleased]` 섹션 업데이트 관례 수립

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.3.2] - 2026-03-13

### Fixed
- Prevent silent flag failures and separate auth exit code

## [0.3.1] - 2026-03-13

### Added
- Search, tags, series, stats, comments, and social features

<!-- ... 이전 버전은 git log에서 역추적 -->
```

### Acceptance Criteria
- [ ] Dependabot이 활성화되어 의존성 업데이트 PR 자동 생성
- [ ] CHANGELOG.md가 모든 릴리즈 버전 포함

---

## Phase 5: Branch Protection (Should-have)

- [ ] `main` 브랜치 보호 규칙 설정
  ```bash
  gh api repos/{owner}/{repo}/branches/main/protection -X PUT \
    -H "Accept: application/vnd.github+json" \
    --input - <<'EOF'
  {
    "required_status_checks": {
      "strict": true,
      "contexts": ["fmt-check", "clippy", "test", "typos"]
    },
    "enforce_admins": false,
    "required_pull_request_reviews": {
      "required_approving_review_count": 1
    },
    "restrictions": null
  }
  EOF
  ```

> **Note**: 1인 프로젝트일 경우 `enforce_admins: false`로 설정하여 maintainer는 우회 가능하게 함. `required_approving_review_count`는 외부 기여자 PR에만 실질적으로 적용.

### Acceptance Criteria
- [ ] main 브랜치에 직접 push 시 CI 통과 필수
- [ ] PR에서 CI status check 표시

---

## 작업 순서 요약

| 순서 | Phase | 예상 작업량 | 비고 |
|------|-------|------------|------|
| 1 | Phase 1: 레포 설정 | 5분 | `gh` CLI로 즉시 적용 |
| 2 | Phase 2: CODE_OF_CONDUCT + SECURITY | 15분 | 표준 템플릿 기반 |
| 3 | Phase 3: Issue/PR 템플릿 | 20분 | 4개 파일 생성 |
| 4 | Phase 4: Dependabot + CHANGELOG | 30분 | CHANGELOG 역추적 필요 |
| 5 | Phase 5: Branch protection | 5분 | `gh` API 호출 |

**총 예상**: 약 1시간 내외

## Dependencies & Risks

- **GitHub Private Vulnerability Reporting**: 레포 Settings → Security에서 활성화 필요 (수동)
- **Branch protection**: 1인 프로젝트에서 review 필수 설정 시 self-merge 필요 — `enforce_admins: false`로 우회
- **CHANGELOG 역추적**: git log로 v0.1.0~v0.3.2 변경사항 정리 필요, 정확도 한계 있음
- **Dependabot PR 폭주**: `open-pull-requests-limit: 5`로 제한

## Sources

- **선행 계획**: [docs/plans/2026-03-11-feat-open-source-release-readiness-plan.md](docs/plans/2026-03-11-feat-open-source-release-readiness-plan.md) — Phase 1~5 대부분 완료
- **GitHub Community Profile API**: `gh api repos/{owner}/{repo}/community/profile`
- **Contributor Covenant**: https://www.contributor-covenant.org/version/2/1/code_of_conduct/
- **Keep a Changelog**: https://keepachangelog.com/
- **GitHub YAML Issue Forms**: https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests
