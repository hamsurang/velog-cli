# Brainstorm: Feed / Trending / Recent / Curated 기능

**Date:** 2026-03-12
**Status:** Approved

## What We're Building

기존 `velog post list` 커맨드를 확장하여 4가지 새로운 콘텐츠 조회 기능을 추가한다:

1. **Trending** (`--trending`): 인기 포스트 조회 (일/주/월/연 기간 필터)
2. **Recent** (`--recent`): 최신 포스트 시간순 조회
3. **Feed** (`--feed`): 팔로우 중인 유저들의 포스트 피드
4. **User posts** (`-u <username>`): 특정 유저의 전체 포스트 리스트
5. **Curated**: velog 공식 큐레이션 포스트 (API 지원 여부에 따라)

### CLI 인터페이스

```bash
# Trending
velog post list --trending                     # 기본: 주간 인기
velog post list --trending --period day         # 일간
velog post list --trending --period week        # 주간
velog post list --trending --period month       # 월간
velog post list --trending --period year        # 연간

# Recent
velog post list --recent                       # 최신 포스트
velog post list --recent --limit 30            # 30개 조회

# Feed (로그인 필요)
velog post list --feed

# User posts (비로그인 가능)
velog post list -u <username>
velog post list -u <username> --limit 50

# 공통 옵션
--limit <n>      # 결과 개수 (기본 20)
--page <n>       # 페이지 번호 (기본 1)
--format          # pretty/compact/silent (기존 글로벌 옵션)
```

## Why This Approach

**`post list` 확장 방식을 선택한 이유:**

- 기존 2단계 CLI 구조(`velog <group> <action>`)와 일관성 유지
- `--format`, `--limit`, `--page` 같은 공통 옵션을 자연스럽게 공유
- 사용자 학습 비용 최소화 — "포스트 목록은 `post list`에서"라는 단일 멘탈 모델
- 기존 출력 로직(pretty table, compact JSON) 재활용 극대화

**상호 배타 플래그 규칙:**
`--trending`, `--recent`, `--feed`, `--drafts`는 상호 배타적. 동시 사용 시 에러 반환.
`-u <username>`은 `--trending`/`--recent`과는 별개(자체 독립 모드), `--feed`/`--drafts`와는 상호 배타.

## Key Decisions

| 결정 사항 | 선택 | 근거 |
|-----------|------|------|
| CLI 구조 | `post list` 확장 | 기존 패턴 일관성, 사용자 선택 |
| 페이지네이션 | `--limit` + `--page` | 직관적, offset 계산으로 API 매핑 |
| 기간 필터 | day/week/month/year | velog.io와 동일, 기본값 week |
| 인증 정책 | trending/recent/user: 비로그인, feed: 로그인 필요 | 공개 데이터는 anonymous 접근 |
| Curated | velog 공식 큐레이션 | API 지원 여부 먼저 확인 필요 |
| 기본 limit | 20 | 터미널에서 적당한 양 |

## Implementation Notes

### 선행 작업: API 스키마 확인

velog GraphQL API(v2)의 실제 스키마를 introspection 또는 네트워크 분석으로 확인해야 할 항목:
- `trendingPosts` 쿼리: 파라미터(limit, offset, timeframe 등)
- `recentPosts` 쿼리: 파라미터(limit, cursor/offset 등)
- 팔로우 피드 관련 쿼리
- 큐레이션 관련 쿼리
- 페이지네이션 방식 (cursor vs offset)

### 변경 파일 (예상)

| 파일 | 변경 내용 |
|------|-----------|
| `src/cli.rs` | `PostCommands::List` 플래그 추가, `Period` enum |
| `src/client.rs` | 새 GraphQL 쿼리 상수 + 메서드 추가 |
| `src/models.rs` | 응답 타입 추가 (필요 시) |
| `src/handlers.rs` | `post_list` 핸들러 분기 확장, anonymous 클라이언트 활용 |
| `src/main.rs` | dispatch 인자 전달 확장 |
| `tests/cli_tests.rs` | 통합 테스트 추가 |
| `README.md` | 새 명령어 문서화 |

### 기존 패턴 활용

- `VelogClient::anonymous()` — trending/recent/user posts에 활용
- `output::emit_data()` — compact/silent JSON 출력
- `print_posts_table()` — pretty 모드 테이블 출력 (컬럼 조정 가능)

## Open Questions

_(모두 해결됨 — 아래 Resolved Questions 참조)_

## Resolved Questions

1. **CLI 구조** → `post list` 확장 방식
2. **페이지네이션** → `--limit` + `--page`
3. **기간 필터** → day/week/month/year (기본 week)
4. **인증 정책** → 공개 데이터는 비로그인 허용
5. **Curated 정의** → velog 공식 큐레이션 (API 확인 후 구현)

## Success Criteria

- [ ] 모든 명령어가 pretty/compact/silent 3가지 포맷으로 정상 출력
- [ ] 단위 테스트 + 통합 테스트 추가
- [ ] README 업데이트
- [ ] velog GraphQL API 실제 스키마 확인 후 구현
