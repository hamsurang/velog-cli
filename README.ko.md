# velog-cli

> [velog.io](https://velog.io) 블로그를 터미널에서 관리하는 비공식 CLI 도구

[![CI](https://github.com/hamsurang/velog-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/hamsurang/velog-cli/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/velog-cli.svg)](https://crates.io/crates/velog-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

[English](./README.md)

---

## 설치

### Homebrew (macOS)

```bash
brew tap hamsurang/velog-cli
brew install velog-cli
```

### Cargo

```bash
cargo install velog-cli
```

### GitHub Releases

[Releases](https://github.com/hamsurang/velog-cli/releases) 페이지에서 빌드된 바이너리를 다운로드할 수 있습니다.

---

## 빠른 시작

### 1단계: 인증

브라우저 DevTools → Application → Cookies → `velog.io`에서 토큰을 복사한 후:

```bash
velog auth login
# access_token 붙여넣기 (입력 숨김)
# refresh_token 붙여넣기 (입력 숨김)
```

### 2단계: 글 목록 확인

```bash
velog post list                          # 내 글 목록
velog post list --drafts                 # 임시 글 목록
velog post list --trending               # 트렌딩
velog post list --trending --period week # 이번 주 트렌딩
velog post list --recent                 # 최신 글
velog post list -u <username>            # 특정 유저의 글
```

### 3단계: 새 글 작성

```bash
velog post create --title "제목" --file post.md --tags "태그1,태그2" --publish
```

### 4단계: 글 수정

```bash
velog post edit my-post-slug --file updated.md --title "수정된 제목"
```

---

## 출력 형식

`--format` 옵션으로 출력 스타일을 선택할 수 있습니다:

| 형식 | 설명 | 용도 |
|------|------|------|
| `pretty` | 테이블, 색상, 마크다운 렌더링 (기본값) | 터미널에서 직접 사용 |
| `compact` | 압축 JSON | AI 에이전트, 스크립트, 파이프라인 |
| `silent` | 쿼리는 JSON, 뮤테이션은 출력 없음 | CI/CD, 종료 코드만 필요한 경우 |

```bash
velog post list                        # 사람이 읽기 쉬운 형태 (기본값)
velog --format compact post list       # 기계 판독용 JSON
velog --format silent post create ...  # 종료 코드만 반환
```

---

## 명령어

### 인증

| 명령어 | 설명 |
|--------|------|
| `velog auth login` | velog.io 토큰으로 인증 |
| `velog auth status` | 현재 로그인 상태 확인 |
| `velog auth logout` | 저장된 인증 정보 삭제 |

### 포스트

| 명령어 | 설명 |
|--------|------|
| `velog post list` | 글 목록 (내 글, 트렌딩, 최신, 유저별) |
| `velog post show <slug>` | 글 보기 (`-u <user>`로 다른 유저의 글 조회) |
| `velog post create` | 마크다운으로 새 글 작성 |
| `velog post edit <slug>` | 글 수정 |
| `velog post delete <slug>` | 글 삭제 (`-y`로 확인 생략) |
| `velog post publish <slug>` | 임시 글 발행 |
| `velog post like <slug>` | 좋아요 |
| `velog post unlike <slug>` | 좋아요 취소 |

### 검색 & 태그

| 명령어 | 설명 |
|--------|------|
| `velog search <keyword>` | 글 검색 (전체 또는 유저별) |
| `velog tags list` | 태그 목록 (트렌딩 또는 가나다순) |
| `velog tags posts <tag>` | 특정 태그의 글 목록 |

### 시리즈

| 명령어 | 설명 |
|--------|------|
| `velog series list` | 시리즈 목록 |
| `velog series show <name>` | 시리즈 내 글 목록 |
| `velog series create <name>` | 새 시리즈 생성 |
| `velog series delete <name>` | 시리즈 삭제 |
| `velog series edit <name>` | 시리즈 이름/설명 수정 |
| `velog series add <name> <slug>` | 시리즈에 글 추가 |
| `velog series remove <name> <slug>` | 시리즈에서 글 제거 |
| `velog series reorder <name>` | 시리즈 내 글 순서 변경 |

### 댓글

| 명령어 | 설명 |
|--------|------|
| `velog comment list <slug>` | 글의 댓글 목록 |
| `velog comment write <slug>` | 댓글 작성 |
| `velog comment reply <comment-id>` | 답글 작성 |
| `velog comment edit <comment-id>` | 댓글 수정 |
| `velog comment delete <comment-id>` | 댓글 삭제 |

### 소셜 & 통계

| 명령어 | 설명 |
|--------|------|
| `velog stats <slug>` | 글 통계 조회 |
| `velog follow <username>` | 팔로우 |
| `velog unfollow <username>` | 언팔로우 |
| `velog reading-list` | 좋아요 및 읽은 글 목록 |

### 유틸리티

| 명령어 | 설명 |
|--------|------|
| `velog completions <shell>` | 셸 자동완성 생성 (bash/zsh/fish) |

---

## 셸 자동완성

```bash
# Bash
velog completions bash > ~/.bash_completion.d/velog

# Zsh
velog completions zsh > ~/.zfunc/_velog

# Fish
velog completions fish > ~/.config/fish/completions/velog.fish
```

---

## 작동 방식

velog-cli는 velog.io의 GraphQL API와 통신합니다. 토큰은 `~/.config/velog-cli/credentials.json`에 `0600` 권한으로 안전하게 저장되며, 만료된 토큰은 자동으로 갱신됩니다.

---

## 기여하기

```bash
brew install lefthook typos-cli
lefthook install
```

커밋 시 `cargo fmt-check`, `clippy`, `typos`, `cargo test`가 자동으로 실행됩니다.

> `audit`과 `ls-lint`은 CI에서만 실행됩니다. ls-lint를 로컬에서 실행하려면: `npx @ls-lint/ls-lint`

자세한 내용은 [CONTRIBUTING.md](CONTRIBUTING.md)를 참고하세요.

---

## 면책 조항

이 도구는 **비공식** 프로젝트이며, [velog.io](https://velog.io) 또는 Chaf Inc.와 제휴, 보증, 연관되어 있지 않습니다. "velog"는 Chaf Inc.의 상표입니다. 모든 제품명, 로고, 브랜드는 각 소유자의 자산입니다.

## 라이선스

MIT — [LICENSE](./LICENSE) 참고
