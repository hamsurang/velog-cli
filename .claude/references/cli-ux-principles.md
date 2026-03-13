# CLI UX Design Principles

CLI Advisor가 참조하는 전문 지식 레퍼런스.
clig.dev, POSIX, GNU, 12 Factor CLI Apps 등에서 추출한 핵심 원칙.

---

## 1. Help & Documentation

### 첫 줄의 중요성
Help 출력의 첫 줄은 명령어가 무엇을 하는지 한 문장으로 설명해야 한다.

```
Good: "Manage your velog.io blog posts from the terminal"
Bad:  "velog-cli v0.3.0"
```

### Usage 패턴
모든 명령어의 help에 Usage 라인을 포함한다.

```
Usage: velog post create [OPTIONS] --title <TITLE>
```

### 예제 포함
복잡한 명령어에는 Examples 섹션을 추가한다.

```
Examples:
  velog post create -t "My Post" -f content.md --publish
  cat draft.md | velog post create -t "Piped Post"
```

### Progressive Disclosure
`--help`는 핵심 정보를 먼저, 드물게 사용하는 옵션은 뒤에 배치한다.

### 상호 배타 플래그 문서화
help 텍스트에서 상호 배타적 플래그 그룹을 명시한다.
사용자가 시행착오 없이 올바른 조합을 선택할 수 있어야 한다.

```
Listing modes (mutually exclusive):
  --trending    Show trending posts
  --recent      Show recent posts
  -u <USER>     Show user's posts
  --drafts      Show draft posts (requires auth)
```

---

## 2. Error Messages

### 구조: What + Why + Fix
1. **What**: 무엇이 잘못됐는가
2. **Why**: 왜 잘못됐는가 (가능한 경우)
3. **Fix**: 어떻게 고칠 수 있는가

```
Good: error: --period requires --trending flag
      Try: velog post list --trending --period week

Bad:  error: invalid option combination
```

### 유사 명령어 제안
clap의 `suggest` 기능 활용:
```
error: unrecognized subcommand 'lisst'
  Did you mean 'list'?
```

### stderr로 출력
에러는 반드시 stderr로. stdout은 정상 데이터 전용.

### 적절한 컨텍스트
사용자 입력값을 에러 메시지에 포함:
```
Good: error: invalid period 'monthly'. Valid values: day, week, month, year
Bad:  error: invalid period value
```

### 에러 메시지 포맷 일관성
clap이 생성하는 에러와 앱이 생성하는 에러의 포맷이 다르면 혼란을 준다.
가능한 한 일관된 포맷을 유지한다.

---

## 3. Output Design

### stdout vs stderr 역할 분리
- **stdout**: 데이터 (파이프 가능)
- **stderr**: 메시지, 에러, 경고

### Machine-Readable Output
JSON 출력은 유효하고 일관된 스키마를 따라야 한다:
```json
{"error": "message", "exit_code": 1}
{"posts": [...], "next_cursor": "..."}
```

### 색상 사용
- `NO_COLOR` 환경변수 존중
- stdout이 TTY가 아니면 자동으로 색상 비활성화
- 색상은 보조 수단, 유일한 구분자가 되면 안 됨

### 테이블 출력
- 터미널 너비에 맞게 조정
- 헤더 포함
- 데이터 없을 때 "No posts found" 같은 메시지

---

## 4. Exit Codes

### 표준 규약
- `0`: 성공
- `1`: 일반 에러 (런타임, 네트워크 등)
- `2`: 잘못된 사용법 (CLI 인자 에러)

### 일관성
동일 유형 에러는 항상 동일한 exit code. 스크립트에서 분기 처리에 사용되므로 예측 가능해야 한다.

---

## 5. Interactivity & Confirmation

### 파괴적 작업
삭제 전 확인 프롬프트. `-y`/`--yes`로 스크립트에서 건너뛰기.
기본값은 안전한 쪽(No).

### 비대화형 환경 감지
stdin이 TTY가 아닐 때 대화형 프롬프트 금지.

---

## 6. Naming & Consistency

### 명령어 네이밍
- 동사-목적어: `post create`, `auth login`
- 완전한 단어: `delete` > `del`
- 업계 관례: `show`(단일), `list`(목록), CRUD 패턴

### 플래그 네이밍
- Long: `--format`, Short: `-f`
- Boolean: `--publish`, `--private`
- 부정: `--no-color`
- 프로젝트 내 일관성: 파일은 항상 `-f`, 사용자는 `-u`, 제한은 `--limit`

---

## 7. Composability

### 파이프라인 친화적
```bash
velog post list --format compact | jq '.[].title'
echo "# Hello" | velog post create -t "Test"
```

### stdin 지원
`-`로 stdin 읽기 (Unix 관례).

### 무음 모드
`--silent` 모드에서는 데이터만 출력, 진행 메시지 억제.

---

## 8. Flag Interaction Design

이 섹션은 플래그 간 관계 설계에 대한 원칙이다. CLI가 복잡해질수록 이 영역의 중요성이 커진다.

### 의존 플래그 (requires)
플래그 A가 플래그 B 없이는 의미가 없을 때, A에 `requires = "B"`를 설정한다.
이렇게 하면 clap이 자동으로 명확한 에러를 생성한다.

```
error: the following required arguments were not provided:
  --trending

Usage: velog post list --trending --period <PERIOD>
```

### 상호 배타 플래그 (conflicts_with)
함께 사용할 수 없는 플래그에 `conflicts_with`를 설정한다.
**모든 방향에서 충돌을 설정해야 한다** — A→B만 설정하고 B→A를 빠뜨리면 한 방향에서는 감지가 안 된다.

### 조용한 실패 방지
가장 위험한 UX 버그는 "에러 없이 잘못 동작하는 것"이다.
플래그가 무시되는 상황은 반드시 방지해야 한다:
- `--period`가 `--trending` 없이 전달될 때 에러를 내야 한다
- `--cursor`가 지원하지 않는 모드에서 조용히 무시되면 안 된다
- 사용자가 의도한 동작이 실행되지 않는데 에러가 없으면, 디버깅이 극도로 어렵다

### 검증 순서
여러 종류의 에러가 동시에 발생할 수 있을 때, 사용자에게 가장 유용한 에러를 먼저 보여준다:
1. 인자/플래그 에러 (사용자가 즉시 수정 가능)
2. 인증 에러 (별도 action 필요)
3. 비즈니스 로직 에러 (서버 응답 필요)

인증 에러가 인자 에러보다 먼저 체크되면, 사용자는 로그인한 후에야 인자가 잘못됐다는 것을 알게 된다.

---

## 9. Performance

### 체감 속도
- 로컬 작업 (help, validation): < 100ms
- 네트워크 요청: 진행 표시 제공
- 불필요한 네트워크 요청 제거

### 시작 시간
Rust CLI는 즉각 시작 — JVM/Python 대비 큰 장점.

---

## 10. Well-Known CLI References

- **gh** (GitHub CLI): 명령어 구조, help, 에러 메시지의 모범
- **docker**: 하위 명령어 계층
- **kubectl**: 출력 포맷 옵션
- **cargo**: Rust 생태계의 CLI UX 표준
- **ripgrep**: 성능과 UX의 균형
