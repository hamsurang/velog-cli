# Brainstorm: velog CLI Output Format Options

**Date:** 2026-03-11
**Status:** Complete

## What We're Building

velog CLI에 `--format` 글로벌 플래그를 추가하여 3가지 출력 모드를 제공한다:

| 모드 | 용도 | stdout | stderr |
|---|---|---|---|
| `compact` (기본값) | AI 에이전트, 파이프라인 | minified JSONL (약어 키) | JSON |
| `pretty` | 사람이 터미널에서 직접 사용 | 테이블/색상/마크다운 (현재 동작) | 색상 텍스트 (현재 동작) |
| `silent` | 스크립트에서 mutation 결과만 필요할 때 | 조회=compact 동일, mutation=출력 없음 | 출력 없음 |

### compact 모드 출력 예시

**post list (stdout):**
```
{"title":"My Post","slug":"my-post","status":"pub","tags":["rust"],"date":"2026-03-10"}
{"title":"Draft","slug":"draft-1","status":"draft","tags":[],"date":"2026-03-09"}
```

**post show (stdout):**
```
{"title":"My Post","slug":"my-post","status":"pub","tags":["rust"],"date":"2026-03-10","body":"..."}
```

**post create (stdout):**
```
{"url":"https://velog.io/@username/my-post"}
```

**post create (stderr):**
```
{"status":"ok","msg":"Post created"}
```

**auth status (stdout):**
```
{"logged_in":true,"username":"gimminsu"}
```

**auth login/logout (stderr):**
```
{"status":"ok","msg":"Logged in"}
```

### format 제외 명령

`velog completions <shell>`은 `--format` 플래그의 영향을 받지 않는다.

### silent 모드 동작

- **조회 명령** (post list, post show): compact와 동일하게 데이터 출력
- **mutation 명령** (create, edit, delete, publish): 출력 없음, exit code로만 결과 전달 (0=성공, 1=실패, 2=인증 오류)

### pretty 모드 동작

현재 velog CLI의 출력과 완전히 동일 (comfy-table, colored, termimad).

## Why This Approach

### 문제

현재 velog CLI 출력은 human-friendly하게만 설계되어 있다:
- comfy-table 유니코드 테이블
- colored ANSI 색상/볼드
- termimad 마크다운 렌더링
- 이모지 (✓) 포함 상태 메시지

AI 에이전트가 이 출력을 파싱하려면 불필요한 토큰을 소모하고, 파싱 오류 가능성이 높다.

### 선택 근거

- **compact가 기본값인 이유**: AI 에이전트 사용이 주요 유스케이스. 사람이 직접 쓸 때는 `--format pretty`를 명시하거나 alias 설정
- **JSONL 형식**: 리스트 출력에서 한 줄에 하나의 객체로 스트리밍 파싱 가능
- **축약 값**: 키 이름(title, slug, tags, date)은 원본 유지, 값만 축약 (status→pub/draft/priv). 매핑 없이 의미 파악 가능
- **환경변수 미지원**: YAGNI. --format 플래그로 충분
- **--fields, --template 미지원**: YAGNI. 필요 시 나중에 추가

## Key Decisions

1. **플래그**: `--format compact|pretty|silent` (글로벌, clap `Cli` 구조체에 추가)
2. **기본값**: `compact` (agent-first)
3. **compact 포맷**: minified JSONL, 키 원본 유지 + 축약 값 (status=pub/draft/priv)
4. **stderr 처리**: compact에서 JSON, silent에서 제거, pretty에서 현재 동작 유지
5. **silent 조회**: compact와 동일 (데이터가 목적이므로)
6. **silent mutation**: 출력 없음, exit code만
7. **mutation stdout**: compact에서 URL도 JSON으로 감싸기 (`{"url":"..."}`)
8. **auth 명령**: compact/silent 적용. auth status는 JSON stdout, login/logout은 JSON stderr
9. **제외 명령**: `completions`는 format 영향 없음
10. **범위**: 최소 MVP - format 플래그만, 확장 옵션 없음

## Open Questions

없음 - 모든 핵심 결정이 대화를 통해 해결됨.
