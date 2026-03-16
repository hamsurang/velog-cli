---
title: "Open Source Trust & Safety Hardening"
type: feat
status: completed
date: 2026-03-16
origin: docs/plans/2026-03-15-feat-open-source-community-readiness-plan.md
---

# Open Source Trust & Safety Hardening

## Overview

velog-cli의 Community Health는 100%를 달성했고, 기본 branch protection도 설정된 상태. 이 계획은 `gh` CLI 기반 레포 감사에서 발견된 **미충족 신뢰도 지표**를 개선한다:

- Branch protection 세부 규칙 강화
- 커뮤니티 성장 인프라 (Discussions, Funding, Labels)

**현재 상태 (2026-03-16 감사 결과):**

| 카테고리 | 현재 | 목표 |
|----------|------|------|
| Community Health | 100% | 유지 |
| Required status checks | 6개, strict | 유지 |
| Dismiss stale reviews | false | **true** |
| Require last push approval | false | false (유지) |
| Enforce admins | false | false (유지) |
| Required conversation resolution | false | **true** |
| Discussions | disabled | **enabled** |
| FUNDING.yml | 없음 | **추가** |
| Custom labels | 기본 9개만 | **확장** |

### Scope 제외 결정 (SpecFlow 분석 근거)

- **`enforce_admins`**: 1-2인 메인테이너 프로젝트에서 활성화 시 self-merge 불가. 외부 기여자 PR에는 이미 protection이 적용되므로 실질적 보안 이득 없음.
- **`require_last_push_approval`**: enforce_admins와 결합 시 머지 데드락 발생. Dependabot PR도 차단될 수 있음 (봇이 push 후 자가 승인 불가). 팀 규모가 3인 이상이 될 때 재평가.
- **Commit signing (`required_signatures`)**: 기여자 진입 장벽 상승. GPG/SSH 키 설정 강제는 초기 오픈소스에 부적합.

---

## Phase 1: Branch Protection 강화 (gh API)

> 코드 변경 없이 `gh` API로 즉시 적용

### 1-1. Dismiss stale reviews 활성화

승인 후 새 커밋이 push되면 기존 승인을 자동 해제하여 재리뷰를 강제한다.

```bash
gh api repos/hamsurang/velog-cli/branches/main/protection \
  -X PUT \
  -H "Accept: application/vnd.github+json" \
  --input - <<'EOF'
{
  "required_status_checks": {
    "strict": true,
    "contexts": ["fmt-check", "clippy", "test", "typos", "ls-lint", "audit"]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": true,
    "require_code_owner_reviews": true,
    "require_last_push_approval": false,
    "required_approving_review_count": 1
  },
  "restrictions": null
}
EOF
```

**주의:** Branch protection API는 전체 설정을 덮어쓰므로, 기존 설정을 모두 포함해야 한다.

### 1-2. Required conversation resolution 활성화

리뷰 코멘트가 모두 resolved 상태여야 머지 가능.

```bash
gh api repos/hamsurang/velog-cli/branches/main/protection/required_conversation_resolution \
  -X PATCH \
  --input - <<'EOF'
{
  "required_conversation_resolution": true
}
EOF
```

> **Note:** GitHub API에서 conversation resolution은 별도 엔드포인트가 아닌 branch protection PUT에 포함. Phase 1-1의 PUT 요청에 함께 적용하거나, GitHub Web UI Settings > Branches에서 토글.

### Acceptance Criteria

- [x] `gh api repos/hamsurang/velog-cli/branches/main/protection` 응답에서:
  - `required_pull_request_reviews.dismiss_stale_reviews` == `true`
  - `required_conversation_resolution.enabled` == `true`
- [ ] 테스트: PR에 승인 후 새 커밋 push → 승인 상태가 자동 해제되는지 확인
- [ ] 테스트: 미해결 리뷰 코멘트가 있는 PR → 머지 버튼 비활성화 확인

---

## Phase 2: Discussions 활성화

### 2-1. Discussions 활성화

```bash
gh repo edit hamsurang/velog-cli --enable-discussions
```

### 2-2. Discussion 카테고리 설정

GitHub Web UI (Settings > Discussions)에서 카테고리 정리:

| 카테고리 | Format | 용도 |
|----------|--------|------|
| Announcements | Announcement | 릴리스 공지, 중요 변경사항 (메인테이너만 생성) |
| Q&A | Question | 사용법 질문, 트러블슈팅 |
| Ideas | Open-ended | 기능 아이디어, RFC 전 단계 논의 |
| Show and Tell | Open-ended | velog-cli 활용 사례 공유 |

> 기본 제공되는 General, Polls 카테고리는 삭제하여 채널을 단순하게 유지.

### 2-3. CONTRIBUTING.md 업데이트

기존 CONTRIBUTING.md에 Discussions 안내 추가:

```markdown
## Questions & Discussions

- **Bug reports**: [Open an issue](https://github.com/hamsurang/velog-cli/issues/new?template=bug-report.yml)
- **Feature ideas**: [Start a discussion](https://github.com/hamsurang/velog-cli/discussions/categories/ideas) first, then create an issue
- **Usage questions**: [Q&A discussions](https://github.com/hamsurang/velog-cli/discussions/categories/q-a)
```

### Acceptance Criteria

- [x] `gh repo view --json hasDiscussionsEnabled` == `true`
- [ ] Announcements, Q&A, Ideas, Show and Tell 카테고리 존재 (Web UI에서 수동 정리 필요)
- [x] CONTRIBUTING.md에 Discussions 링크 추가

---

## Phase 3: FUNDING.yml 추가

GitHub Sponsors 버튼을 활성화하여 후원 경로를 제공한다.

### 3-1. `.github/FUNDING.yml` 생성

```yaml
github: [minsoo-web]
```

> 초기에는 GitHub Sponsors만 설정. 추후 필요 시 `ko_fi`, `open_collective` 등 추가.

### Acceptance Criteria

- [x] 레포 페이지에 "Sponsor" 버튼 표시 (FUNDING.yml 커밋 후 활성화)
- [x] `.github/FUNDING.yml` 생성 완료

---

## Phase 4: Custom Labels 추가

기본 라벨 외에 트리아지 워크플로우를 위한 라벨을 추가한다.

### 4-1. Priority 라벨

```bash
gh label create "priority: critical" --color "B60205" --description "Needs immediate attention" --repo hamsurang/velog-cli
gh label create "priority: high" --color "D93F0B" --description "Should be addressed soon" --repo hamsurang/velog-cli
gh label create "priority: medium" --color "FBCA04" --description "Normal priority" --repo hamsurang/velog-cli
gh label create "priority: low" --color "0E8A16" --description "Nice to have" --repo hamsurang/velog-cli
```

### 4-2. Area 라벨

```bash
gh label create "area: auth" --color "1D76DB" --description "Authentication & tokens" --repo hamsurang/velog-cli
gh label create "area: cli" --color "1D76DB" --description "CLI interface & arguments" --repo hamsurang/velog-cli
gh label create "area: api" --color "1D76DB" --description "velog.io API interaction" --repo hamsurang/velog-cli
gh label create "area: ci" --color "1D76DB" --description "CI/CD & automation" --repo hamsurang/velog-cli
```

### 4-3. Status 라벨

```bash
gh label create "status: needs triage" --color "EDEDED" --description "Awaiting maintainer review" --repo hamsurang/velog-cli
gh label create "status: blocked" --color "B60205" --description "Blocked by dependency or decision" --repo hamsurang/velog-cli
gh label create "status: in progress" --color "0075CA" --description "Currently being worked on" --repo hamsurang/velog-cli
```

### Acceptance Criteria

- [x] `gh label list --repo hamsurang/velog-cli` 에서 priority/area/status 라벨 모두 표시
- [x] 라벨 색상이 카테고리별로 일관성 유지 (priority: 빨-주-노-초, area: 파랑, status: 회색계열)

---

## 작업 순서 요약

| 순서 | Phase | 방법 | 비고 |
|------|-------|------|------|
| 1 | Branch Protection 강화 | `gh api` PUT | dismiss_stale_reviews + conversation_resolution |
| 2 | Discussions 활성화 | `gh repo edit` + Web UI | 카테고리 정리 필요 |
| 3 | FUNDING.yml | 파일 생성 + commit | GitHub Sponsors |
| 4 | Custom Labels | `gh label create` | 11개 라벨 추가 |

**단계적 적용 권장:** Phase 1 적용 후 1-2주 모니터링 → Phase 2-4 순차 적용. Branch protection 변경이 기존 워크플로우에 미치는 영향을 먼저 확인.

## Dependencies & Risks

| 리스크 | 영향 | 완화 방안 |
|--------|------|----------|
| dismiss_stale_reviews로 인한 리뷰 반복 부담 | 중 | 1인 프로젝트에서는 self-merge 시 영향 없음 (enforce_admins=false) |
| conversation_resolution이 Dependabot PR 차단 | 중 | Dependabot PR에는 리뷰 코멘트가 거의 없으므로 실질적 영향 낮음 |
| Discussions 모더레이션 부담 | 중 | 카테고리를 4개로 제한, 활성 사용자 증가 시 재평가 |
| 라벨 수 증가로 인한 복잡성 | 낮 | 수동 적용, 자동화는 이슈 볼륨 증가 시 도입 |

## Rollback 절차

Branch protection 변경이 문제를 일으킬 경우:

```bash
# dismiss_stale_reviews 롤백
gh api repos/hamsurang/velog-cli/branches/main/protection \
  -X PUT \
  -H "Accept: application/vnd.github+json" \
  --input - <<'EOF'
{
  "required_status_checks": {
    "strict": true,
    "contexts": ["fmt-check", "clippy", "test", "typos", "ls-lint", "audit"]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": false,
    "require_code_owner_reviews": true,
    "require_last_push_approval": false,
    "required_approving_review_count": 1
  },
  "restrictions": null
}
EOF
```

## 재평가 기준 (팀 규모 변경 시)

팀이 3인 이상으로 성장하면 다음 항목을 재평가:

- `enforce_admins: true` — 모든 사용자에게 동일한 규칙 적용
- `require_last_push_approval: true` — 마지막 push에 대한 별도 승인
- `required_signatures: true` — commit 서명 필수
- Label automation (GitHub Actions 기반 auto-labeling)

## Sources

- **선행 계획:** [docs/plans/2026-03-15-feat-open-source-community-readiness-plan.md](docs/plans/2026-03-15-feat-open-source-community-readiness-plan.md) — Community Health 100% 달성 완료
- **선행 brainstorm:** [docs/brainstorms/2026-03-11-open-source-readiness-brainstorm.md](docs/brainstorms/2026-03-11-open-source-readiness-brainstorm.md)
- **gh 감사 결과:** 2026-03-16 `gh api repos/hamsurang/velog-cli/branches/main/protection` 기반 분석
- **SpecFlow 분석:** enforce_admins + require_last_push_approval 조합이 1인 메인테이너 프로젝트에서 머지 데드락을 유발하는 리스크 확인
- **GitHub Branch Protection API:** https://docs.github.com/en/rest/branches/branch-protection
