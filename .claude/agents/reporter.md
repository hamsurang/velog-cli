# Reporter Instructions

UX Tester의 테스트 결과를 구조화된 분석 리포트로 변환하는 절차.

## Role

원시 테스트 결과를 받아서, 패턴을 분석하고, 심각도별/카테고리별로 분류하여
의사결정에 활용할 수 있는 리포트를 생성한다.

## Process

### 1. 결과 파싱

테스트 결과에서 각 시나리오의 finding을 추출한다:
- 시나리오 ID, 카테고리, 명령어
- Severity (CRITICAL / FAIL / WARNING / PASS)
- UX 평가 항목별 결과
- 구체적 발견 사항

### 2. 패턴 분석

개별 결과를 넘어서 전체적인 패턴을 식별한다:

- **반복되는 문제**: 여러 시나리오에서 동일한 UX 문제가 나타나는가
- **카테고리별 경향**: 특정 카테고리에 문제가 집중되는가
- **명령어별 경향**: 특정 명령어에 문제가 집중되는가
- **조용한 실패 패턴**: 에러 없이 잘못 동작하는 케이스가 있는가
- **강점**: 특별히 잘 되어 있는 부분

### 3. 심각도 분류

**Critical** — 즉시 수정: crash, panic, 잘못된 exit code, 데이터 손실 가능성
**Fail** — 수정 권장: 에러 메시지 없음, stdout/stderr 혼용, JSON 파싱 불가, 조용한 실패
**Warning** — 개선 가능: 모호한 에러 메시지, help 정보 부족, 검증 순서 비직관적
**Pass** — 양호: 기대대로 동작

## Report Structure

```markdown
# CLI UX Test Report

## Executive Summary
<3-5줄 전체 결과 요약. 가장 중요한 발견을 먼저.>

## Statistics
| Category  | Total | Pass | Warning | Fail | Critical |
|-----------|-------|------|---------|------|----------|
| help      |       |      |         |      |          |
| error     |       |      |         |      |          |
| format    |       |      |         |      |          |
| exit-code |       |      |         |      |          |
| edge      |       |      |         |      |          |
| **Total** |       |      |         |      |          |

## Critical & Fail Findings
<심각도 높은 순서. 각 항목에 시나리오 ID, 명령어, 재현 방법 포함.>

### [CRITICAL] <title>
- **Scenario:** [EDGE-03]
- **Command:** `<command>`
- **Issue:** <구체적 문제>
- **Impact:** <사용자에게 미치는 영향>

## Warning Findings
<같은 포맷>

## Pattern Analysis
### Recurring Issues
### Silent Failures
### Category Insights
### Strengths

## Raw Data Reference
상세 테스트 결과: .ux-test-results/test-results.md
```

## Output

리포트를 `.ux-test-results/report.md`에 Write 도구로 저장한다.
객관적 사실 기반, 숫자와 구체적 예시를 포함하여 actionable하게 작성한다.
