# CLI UX E2E Test

velog-cli의 사용자 경험을 E2E로 자동 테스트하는 순차적 워크플로우.
CLI 구조를 분석하여 시나리오를 자동 생성하고, 실제 바이너리를 실행하여 UX를 검증한 후, 전문 지식 기반의 개선점을 제안한다.

## Arguments

$ARGUMENTS

인자 없으면 전체 카테고리 테스트. 특정 카테고리 지정 가능: `help`, `error`, `format`, `exit-code`, `edge`

## Pipeline

```
Build → Discover → Scenarios → Test → Report → Advisory → Summary
```

모든 작업을 직접 수행한다. 별도의 에이전트를 spawn하지 않는다.
각 Phase에서 참조할 에이전트 지침 파일을 읽고, 그 지침을 따라 직접 수행한다.

---

## Phase 1: Build & Discover

1. 바이너리 빌드:
```bash
cargo build 2>&1
```

2. 모든 명령어의 `--help` 출력을 수집하여 CLI 구조를 파악한다. 각 명령어를 Bash로 실행:
```bash
VELOG=./target/debug/velog
$VELOG --help
$VELOG auth --help
$VELOG auth login --help
$VELOG auth status --help
$VELOG auth logout --help
$VELOG post --help
$VELOG post list --help
$VELOG post show --help
$VELOG post create --help
$VELOG post edit --help
$VELOG post delete --help
$VELOG post publish --help
$VELOG completions --help
```

3. 수집된 출력에서 분석:
- 모든 명령어/하위 명령어 트리
- 각 명령어의 필수/선택 인자와 플래그
- 상호 배타적 플래그 그룹 (clap의 conflicts_with)
- 의존적 플래그 그룹 (requires 관계)
- 글로벌 플래그 (`--format pretty|compact|silent`)

4. 결과 디렉토리 생성:
```bash
mkdir -p .ux-test-results
```

---

## Phase 2: Scenario Generation

수집된 CLI 구조를 기반으로 테스트 시나리오를 자동 생성한다. **카테고리당 최소 5개, 총 25개 이상.**

### 카테고리별 생성 가이드

#### `help` — Help 텍스트 품질
- 모든 명령어의 help 출력 평가: 설명 명확성, Usage 패턴, 예제 유무
- 하위 명령어 discoverability (상위 help에서 하위 명령어 발견 가능한가)
- 플래그 설명 품질, 기본값 표시
- 상호 배타 플래그의 문서화 여부

#### `error` — 에러 메시지 품질
- 필수 인자 누락 (각 명령어별)
- 잘못된 플래그 값 (`--format invalid`, `--period xyz`, `--limit abc`)
- 상호 배타 플래그 동시 사용 (`--trending --recent`, `--trending --drafts`, `--username X --recent` 등 모든 조합)
- 존재하지 않는 하위 명령어 (오타 제안 여부)
- 비로그인 상태에서 인증 필요 명령어
- 에러 메시지의 What/Why/Fix 구조 평가

#### `format` — 출력 포맷 일관성
- 동일 명령어를 pretty/compact/silent로 실행하여 비교
- compact 모드의 JSON 유효성 (파이프로 `echo '<output>' | python3 -c "import sys,json; json.load(sys.stdin)"` 검증)
- silent 모드의 출력 억제 확인
- 에러 출력의 포맷별 일관성 (compact에서 에러도 JSON인가)
- clap이 생성하는 에러 vs 앱 레벨 에러의 포맷 차이

#### `exit-code` — Exit Code 정확성
- 성공 명령어: exit code 0
- 일반 에러: exit code 1
- 인증 에러: exit code 2
- clap 인자 에러의 exit code
- 동일 유형 에러의 exit code 일관성

#### `edge` — 엣지 케이스
- 빈 문자열 인자 (`--slug ""`, `--username ""`)
- `--limit` 경계값 (0, 1, 100, 101, -1)
- 존재하지 않는 slug/username으로 요청
- 특수문자 slug (`--slug "hello world"`, `--slug "../etc"`)
- 매우 긴 인자값 (1000자)
- **플래그 상호작용**: `--period`를 `--trending` 없이 사용, `--cursor`와 `--offset` 혼용 등 의존 플래그의 잘못된 조합
- **검증 순서**: 인증 에러와 입력 에러가 동시에 있을 때 어느 것이 먼저 보이는가
- **조용한 실패**: 에러 없이 플래그가 무시되는 경우 탐색 (예: `--recent --period day` — period가 무시될 수 있음)

### 시나리오 ID 포맷

반드시 아래 포맷을 사용한다:
- help: `[HELP-01]`, `[HELP-02]`, ...
- error: `[ERR-01]`, `[ERR-02]`, ...
- format: `[FMT-01]`, `[FMT-02]`, ...
- exit-code: `[EXIT-01]`, `[EXIT-02]`, ...
- edge: `[EDGE-01]`, `[EDGE-02]`, ...

### 시나리오 작성

각 시나리오를 아래 구조로 작성한다:

```
### [HELP-01] Top-level help clarity
**Command:** `./target/debug/velog --help`
**Expected:** exit_code=0, output_on=stdout
**Evaluate:**
  - 첫 줄에서 CLI의 목적이 파악되는가
  - Usage 패턴이 있는가
  - 모든 하위 명령어가 나열되는가
```

`$ARGUMENTS`로 특정 카테고리가 지정된 경우 해당 카테고리만 생성한다.

시나리오를 생성한 후 Write 도구로 `.ux-test-results/scenarios.md`에 저장한다.

---

## Phase 3: UX Test

`.claude/agents/ux-tester.md`를 Read 도구로 읽고, 그 지침에 따라 직접 테스트를 수행한다.

**핵심 절차:**
1. ux-tester.md를 읽는다
2. 각 시나리오의 명령어를 Bash 도구로 실행한다
3. stdout, stderr, exit code를 분리 캡처한다
4. ux-tester.md의 평가 기준에 따라 각 시나리오를 평가한다
5. Write 도구로 `.ux-test-results/test-results.md`에 저장한다
6. 저장 후 `ls -la .ux-test-results/test-results.md`로 파일 존재를 확인한다

---

## Phase 4: Report

`.claude/agents/reporter.md`를 Read 도구로 읽고, 그 지침에 따라 직접 리포트를 작성한다.

**핵심 절차:**
1. reporter.md를 읽는다
2. Phase 3의 테스트 결과를 분석한다
3. reporter.md의 리포트 구조에 따라 작성한다
4. Write 도구로 `.ux-test-results/report.md`에 저장한다
5. 저장 후 파일 존재를 확인한다

---

## Phase 5: Advisory

`.claude/agents/cli-advisor.md`와 `.claude/references/cli-ux-principles.md`를 Read 도구로 읽고, 그 지침에 따라 직접 개선 제안을 작성한다.

**핵심 절차:**
1. cli-advisor.md와 cli-ux-principles.md를 둘 다 읽는다
2. Phase 4의 리포트와 Phase 1의 CLI 구조를 분석한다
3. cli-advisor.md의 권고안 구조에 따라 작성한다
4. Write 도구로 `.ux-test-results/recommendations.md`에 저장한다
5. 저장 후 파일 존재를 확인한다

---

## Phase 6: Summary & Verification

최종 요약을 작성하고 모든 파일의 존재를 검증한다.

1. Write 도구로 `.ux-test-results/summary.md`를 작성한다:

```markdown
# CLI UX Test Summary

## Overview
- 테스트 일시: <timestamp>
- 총 시나리오: N개 (카테고리별 breakdown)

## Results
| Category | Pass | Warning | Fail | Critical |
|----------|------|---------|------|----------|
| ...      | ...  | ...     | ...  | ...      |

## Top Recommendations
1. [Critical/Fail] ...
2. ...
3. ...
4. ...
5. ...

## Detail Files
- scenarios: .ux-test-results/scenarios.md
- test results: .ux-test-results/test-results.md
- report: .ux-test-results/report.md
- recommendations: .ux-test-results/recommendations.md
```

2. 모든 파일 존재 확인:
```bash
ls -la .ux-test-results/
```

5개 파일이 모두 있어야 한다: scenarios.md, test-results.md, report.md, recommendations.md, summary.md

3. 사용자에게 summary 내용을 직접 보여주고, 상세 결과는 파일 경로를 안내한다.
