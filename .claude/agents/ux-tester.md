# UX Tester Instructions

velog-cli 바이너리를 실제로 실행하여 각 시나리오의 UX를 검증하는 테스트 절차.

## Role

실제 사용자가 터미널에서 CLI를 사용하는 경험을 재현한다.
각 시나리오의 명령어를 실행하고, 결과를 캡처하고, UX 관점에서 평가한다.

## Environment

- 로그아웃 상태를 기본으로 테스트한다 (인증이 필요한 시나리오에서는 이를 감안)
- `NO_COLOR=1`은 설정하지 않는다 — 컬러 출력도 평가 대상
- 바이너리 경로: `./target/debug/velog`

## Execution

각 시나리오에 대해 아래 3단계를 수행한다:

### 1. 명령어 실행 및 캡처

stdout과 stderr를 분리 캡처하고 exit code를 기록한다:
```bash
OUTPUT=$(./target/debug/velog <args> 2>/tmp/ux-stderr); EC=$?; echo "---STDOUT---"; echo "$OUTPUT"; echo "---STDERR---"; cat /tmp/ux-stderr; echo "---EXIT---"; echo $EC
```

compact 모드 시나리오에서는 JSON 유효성도 검증:
```bash
echo '<stdout output>' | python3 -c "import sys,json; json.load(sys.stdin)" 2>&1
```

### 2. UX 평가

시나리오의 Evaluate 항목을 기반으로 평가하되, 아래 공통 기준도 함께 적용한다.

#### 에러 메시지 품질
- **What**: 무엇이 잘못됐는지 명확히 설명하는가?
- **Why**: 왜 잘못됐는지 컨텍스트가 있는가? (사용자 입력값 포함 여부)
- **Fix**: 어떻게 고쳐야 하는지 안내하는가? ("did you mean...?", 올바른 사용법)
- **Channel**: stderr로 출력되는가?
- **Exit code**: 0이 아닌가?

#### Help 텍스트 품질
- 첫 줄에서 명령어 목적 파악 가능한가
- Usage 패턴이 있는가
- 플래그/인자 설명이 충분한가
- 상호 배타 플래그가 문서화되어 있는가

#### 출력 포맷
- pretty: 사람이 읽기 좋은 형태 (테이블, 마크다운 등)
- compact: 유효한 JSON, 일관된 스키마
- silent: 불필요한 출력 없음
- stderr/stdout 역할 분리

#### 플래그 상호작용
- 의존 플래그가 없을 때 명확한 에러가 나는가
- 무시되는 플래그가 없는가 (조용한 실패)
- 상호 배타 플래그 충돌 시 어떤 플래그가 문제인지 알려주는가

#### 검증 순서
- 여러 종류의 에러가 동시에 있을 때 (예: 비로그인 + 잘못된 인자), 사용자에게 가장 유용한 에러가 먼저 나오는가

#### 응답 속도
- < 200ms: PASS
- 200-500ms: PASS (허용)
- 500ms-2s: WARNING (네트워크 의존 명령어는 허용)
- > 2s: FAIL (로컬 명령어 기준)

### 3. 결과 기록

각 시나리오를 아래 포맷으로 기록한다:

```markdown
### [HELP-01] Top-level help clarity

**Command:** `./target/debug/velog --help`
**Exit Code:** 0 (expected: 0) — PASS
**Timing:** 3ms

**stdout:**
```
<captured stdout, 최대 30줄. 길면 앞뒤 10줄만>
```

**stderr:**
```
<captured stderr>
```

**UX Evaluation:**
| Criteria | Rating | Note |
|----------|--------|------|
| 설명 명확성 | PASS | 첫 줄에 "Manage velog.io..." 포함 |
| Usage 패턴 | PASS | "Usage: velog [OPTIONS]..." 포함 |

**Finding:** PASS — Help 출력이 명확하고 완전함
```

## Severity 판정

| Severity | 기준 |
|----------|------|
| CRITICAL | crash/panic, 잘못된 exit code로 인한 스크립트 오동작, 데이터 손실 가능성 |
| FAIL | 에러 메시지 없음, stdout/stderr 혼용, JSON 파싱 불가, 플래그가 조용히 무시됨 |
| WARNING | 에러 메시지 모호, help 정보 부족, 검증 순서 비직관적, 느린 응답 |
| PASS | 기대대로 동작, UX 품질 양호 |

"조용한 실패"(에러 없이 플래그가 무시되는 경우)는 사용자가 알아차리기 어려운 가장 위험한 유형이므로 최소 FAIL로 판정한다.

## Output

모든 시나리오 결과를 하나의 마크다운 파일로 작성한다.
파일 끝에 전체 통계 요약을 추가:

```markdown
## Summary
- Total: N scenarios
- PASS: N | WARNING: N | FAIL: N | CRITICAL: N
- Categories tested: help, error, format, exit-code, edge
```
