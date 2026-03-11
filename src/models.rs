use serde::{Deserialize, Serialize};

// ---- GraphQL Envelope ----
#[derive(Serialize)]
pub struct GraphQLRequest<V: Serialize> {
    pub query: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<V>,
}

#[derive(Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize, Debug)]
pub struct GraphQLError {
    pub message: String,
    pub extensions: Option<serde_json::Value>,
}

impl<T> GraphQLResponse<T> {
    /// data가 있으면 반환, errors만 있으면 에러, 둘 다 없으면 에러
    pub fn into_result(self) -> anyhow::Result<T> {
        match (self.data, self.errors) {
            (Some(data), _) => Ok(data), // partial errors 시에도 data 우선
            (None, Some(errs)) => {
                let msg = errs
                    .first()
                    .map(|e| e.message.clone())
                    .unwrap_or_else(|| "Unknown GraphQL error".into());
                anyhow::bail!("GraphQL error: {}", msg)
            }
            (None, None) => anyhow::bail!("Empty GraphQL response"),
        }
    }

    /// 에러 배열에서 인증 관련 에러인지 확인
    pub fn is_auth_error(&self) -> bool {
        self.errors.as_ref().is_some_and(|errs| {
            errs.iter().any(|e| {
                // 1차: extension code (안정적, 서버 메시지 변경에 무관)
                let has_code = e
                    .extensions
                    .as_ref()
                    .and_then(|ext| ext.get("code"))
                    .and_then(|v| v.as_str())
                    .is_some_and(|c| c == "UNAUTHENTICATED");
                // 2차: 메시지 fallback (구형 응답 호환, 대소문자 무시)
                let has_msg = {
                    let msg = e.message.to_lowercase();
                    msg.contains("not logged in") || msg.contains("unauthorized")
                };
                has_code || has_msg
            })
        })
    }
}

// ---- Domain Models ----
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Post {
    pub id: String,
    pub title: String,
    pub short_description: Option<String>,
    pub body: Option<String>,
    pub thumbnail: Option<String>,
    pub likes: i32,
    pub is_private: bool,
    pub is_temp: bool,
    pub url_slug: String,
    pub released_at: Option<String>,
    pub updated_at: Option<String>,
    pub tags: Option<Vec<String>>,
    pub meta: Option<serde_json::Value>,
    pub user: Option<PostUser>,
}

impl Post {
    /// 날짜 표시용: released_at 우선, updated_at fallback, YYYY-MM-DD만
    pub fn date_short(&self) -> String {
        self.released_at
            .as_deref()
            .or(self.updated_at.as_deref())
            .unwrap_or("")
            .chars()
            .take(10)
            .collect()
    }

    /// 기존 Post를 EditPostInput으로 변환 (모든 필드 보존)
    pub fn into_edit_input(self) -> EditPostInput {
        EditPostInput {
            id: self.id,
            title: self.title,
            body: self.body.unwrap_or_default(),
            tags: self.tags.unwrap_or_default(),
            is_markdown: true,
            is_temp: self.is_temp,
            is_private: self.is_private,
            url_slug: self.url_slug,
            thumbnail: self.thumbnail,
            meta: self.meta.unwrap_or_else(|| serde_json::json!({})),
            series_id: None,
        }
    }
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct PostUser {
    pub username: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserToken {
    pub access_token: String,
    pub refresh_token: String,
}

impl std::fmt::Debug for UserToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserToken")
            .field("access_token", &"[REDACTED]")
            .field("refresh_token", &"[REDACTED]")
            .finish()
    }
}

/// restoreToken 응답(UserToken) → 디스크 저장용(Credentials) 변환
impl From<UserToken> for crate::auth::Credentials {
    fn from(t: UserToken) -> Self {
        Self {
            access_token: t.access_token,
            refresh_token: t.refresh_token,
            username: None,
        }
    }
}

// ---- GraphQL Response Wrappers ----
// GraphQLResponse<T>의 T는 JSON "data" 필드의 구조와 일치해야 한다.

#[derive(Deserialize)]
pub struct CurrentUserData {
    #[serde(rename = "currentUser")]
    pub current_user: User,
}

#[derive(Deserialize)]
pub struct RestoreTokenData {
    #[serde(rename = "restoreToken")]
    pub restore_token: UserToken,
}

#[derive(Deserialize)]
pub struct PostsData {
    pub posts: Vec<Post>,
}

#[derive(Deserialize)]
pub struct PostData {
    pub post: Option<Post>,
}

/// Mutation 응답용 최소 구조체 (v2 API는 mutation에서 일부 필드를 null 반환)
#[derive(Deserialize, Debug)]
pub struct MutationPostResult {
    pub id: String,
    pub url_slug: String,
}

#[derive(Deserialize)]
pub struct WritePostData {
    #[serde(rename = "writePost")]
    pub write_post: MutationPostResult,
}

#[derive(Deserialize)]
pub struct EditPostData {
    #[serde(rename = "editPost")]
    pub edit_post: MutationPostResult,
}

#[derive(Deserialize)]
pub struct RemovePostData {
    #[serde(rename = "removePost")]
    pub remove_post: bool,
}

// ---- Compact Output Types ----
// Used by --format compact/silent for token-optimized JSON output.

/// Compact status: synthesized from Post.is_temp + Post.is_private
#[derive(Serialize, Debug, PartialEq)]
pub enum CompactStatus {
    #[serde(rename = "pub")]
    Published,
    #[serde(rename = "draft")]
    Draft,
    #[serde(rename = "priv")]
    Private,
}

/// Compact post representation (list: body omitted, detail: body included)
#[derive(Serialize, Debug)]
pub struct CompactPost {
    pub title: String,
    pub slug: String,
    pub status: CompactStatus,
    pub tags: Vec<String>,
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// Compact auth status for `auth status`
#[derive(Serialize, Debug)]
pub struct CompactAuthStatus {
    pub logged_in: bool,
    pub username: String,
}

/// Compact mutation result for create/edit/publish
#[derive(Serialize, Debug)]
pub struct CompactMutationResult {
    pub url: String,
}

/// Compact status message for stderr
#[derive(Serialize, Debug)]
pub struct CompactMessage {
    pub status: String,
    pub msg: String,
}

/// Compact error for stderr
#[derive(Serialize, Debug)]
pub struct CompactError {
    pub error: String,
    pub exit_code: i32,
}

impl CompactStatus {
    pub fn from_post(post: &Post) -> Self {
        if post.is_temp {
            CompactStatus::Draft
        } else if post.is_private {
            CompactStatus::Private
        } else {
            CompactStatus::Published
        }
    }
}

impl From<&Post> for CompactPost {
    /// Summary view (post list) — body is omitted
    fn from(post: &Post) -> Self {
        CompactPost {
            title: post.title.clone(),
            slug: post.url_slug.clone(),
            status: CompactStatus::from_post(post),
            tags: post.tags.clone().unwrap_or_default(),
            date: post.date_short(),
            body: None,
        }
    }
}

impl CompactPost {
    /// Detail view (post show) — includes body
    pub fn detail(post: &Post) -> Self {
        CompactPost {
            title: post.title.clone(),
            slug: post.url_slug.clone(),
            status: CompactStatus::from_post(post),
            tags: post.tags.clone().unwrap_or_default(),
            date: post.date_short(),
            body: Some(post.body.clone().unwrap_or_default()),
        }
    }
}

// ---- Mutation Input Types ----
#[derive(Serialize)]
pub struct WritePostInput {
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub is_markdown: bool,
    pub is_temp: bool,
    pub is_private: bool,
    pub url_slug: String,
    pub thumbnail: Option<String>,
    pub meta: serde_json::Value,
    pub series_id: Option<String>,
}

#[derive(Serialize)]
pub struct EditPostInput {
    pub id: String,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub is_markdown: bool,
    pub is_temp: bool,
    pub is_private: bool,
    pub url_slug: String,
    pub thumbnail: Option<String>,
    pub meta: serde_json::Value,
    pub series_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_response_with_error(message: &str, code: Option<&str>) -> GraphQLResponse<()> {
        let extensions = code.map(|c| serde_json::json!({ "code": c }));
        GraphQLResponse {
            data: None,
            errors: Some(vec![GraphQLError {
                message: message.to_string(),
                extensions,
            }]),
        }
    }

    #[test]
    fn is_auth_error_extension_code() {
        let resp = make_response_with_error("any message", Some("UNAUTHENTICATED"));
        assert!(resp.is_auth_error());
    }

    #[test]
    fn is_auth_error_not_logged_in_message() {
        let resp = make_response_with_error("not logged in", None);
        assert!(resp.is_auth_error());
    }

    #[test]
    fn is_auth_error_unauthorized_message() {
        let resp = make_response_with_error("Unauthorized", None);
        assert!(resp.is_auth_error());
    }

    #[test]
    fn is_auth_error_case_insensitive() {
        let resp = make_response_with_error("NOT LOGGED IN", None);
        assert!(resp.is_auth_error());
    }

    #[test]
    fn is_auth_error_not_auth() {
        let resp = make_response_with_error("Post not found", None);
        assert!(!resp.is_auth_error());
    }

    #[test]
    fn is_auth_error_no_errors() {
        let resp: GraphQLResponse<()> = GraphQLResponse {
            data: Some(()),
            errors: None,
        };
        assert!(!resp.is_auth_error());
    }

    #[test]
    fn graphql_response_into_result_with_data() {
        let resp = GraphQLResponse {
            data: Some(42),
            errors: None,
        };
        assert_eq!(resp.into_result().unwrap(), 42);
    }

    #[test]
    fn graphql_response_into_result_error_only() {
        let resp = make_response_with_error("something failed", None);
        let err = resp.into_result().unwrap_err();
        assert!(err.to_string().contains("something failed"));
    }

    #[test]
    fn graphql_response_into_result_empty() {
        let resp: GraphQLResponse<()> = GraphQLResponse {
            data: None,
            errors: None,
        };
        let err = resp.into_result().unwrap_err();
        assert!(err.to_string().contains("Empty"));
    }

    // ---- Compact output type tests ----

    fn make_test_post(is_temp: bool, is_private: bool) -> Post {
        Post {
            id: "test-id".to_string(),
            title: "Test Post".to_string(),
            short_description: None,
            body: Some("# Hello\nworld".to_string()),
            thumbnail: None,
            likes: 0,
            is_private,
            is_temp,
            url_slug: "test-post".to_string(),
            released_at: Some("2026-03-10T12:00:00.000Z".to_string()),
            updated_at: Some("2026-03-11T12:00:00.000Z".to_string()),
            tags: Some(vec!["rust".to_string(), "cli".to_string()]),
            meta: None,
            user: None,
        }
    }

    #[test]
    fn compact_status_published() {
        assert_eq!(
            CompactStatus::from_post(&make_test_post(false, false)),
            CompactStatus::Published
        );
    }

    #[test]
    fn compact_status_draft() {
        assert_eq!(
            CompactStatus::from_post(&make_test_post(true, false)),
            CompactStatus::Draft
        );
    }

    #[test]
    fn compact_status_private() {
        assert_eq!(
            CompactStatus::from_post(&make_test_post(false, true)),
            CompactStatus::Private
        );
    }

    #[test]
    fn compact_status_draft_takes_precedence_over_private() {
        // is_temp=true takes precedence: a draft that is also private shows as "draft"
        assert_eq!(
            CompactStatus::from_post(&make_test_post(true, true)),
            CompactStatus::Draft
        );
    }

    #[test]
    fn compact_status_serializes_abbreviated() {
        assert_eq!(
            serde_json::to_string(&CompactStatus::Published).unwrap(),
            r#""pub""#
        );
        assert_eq!(
            serde_json::to_string(&CompactStatus::Draft).unwrap(),
            r#""draft""#
        );
        assert_eq!(
            serde_json::to_string(&CompactStatus::Private).unwrap(),
            r#""priv""#
        );
    }

    #[test]
    fn compact_post_from_published() {
        let post = make_test_post(false, false);
        let compact = CompactPost::from(&post);
        assert_eq!(compact.title, "Test Post");
        assert_eq!(compact.slug, "test-post");
        assert_eq!(compact.status, CompactStatus::Published);
        assert_eq!(compact.tags, vec!["rust", "cli"]);
        assert_eq!(compact.date, "2026-03-10");
    }

    #[test]
    fn compact_post_serializes_to_json() {
        let post = make_test_post(false, false);
        let compact = CompactPost::from(&post);
        let json = serde_json::to_string(&compact).unwrap();
        assert!(json.contains(r#""status":"pub""#));
        assert!(json.contains(r#""slug":"test-post""#));
        assert!(json.contains(r#""date":"2026-03-10""#));
    }

    #[test]
    fn compact_post_detail_includes_body() {
        let post = make_test_post(true, false);
        let detail = CompactPost::detail(&post);
        assert_eq!(detail.body.as_deref(), Some("# Hello\nworld"));
        assert_eq!(detail.status, CompactStatus::Draft);
    }

    #[test]
    fn compact_post_summary_omits_body() {
        let post = make_test_post(false, false);
        let compact = CompactPost::from(&post);
        assert!(compact.body.is_none());
        // body should not appear in JSON at all
        let json = serde_json::to_string(&compact).unwrap();
        assert!(!json.contains("body"));
    }

    #[test]
    fn compact_post_empty_tags() {
        let mut post = make_test_post(false, false);
        post.tags = None;
        let compact = CompactPost::from(&post);
        assert!(compact.tags.is_empty());
    }

    #[test]
    fn compact_post_no_date_fallback_to_updated() {
        let mut post = make_test_post(false, false);
        post.released_at = None;
        let compact = CompactPost::from(&post);
        assert_eq!(compact.date, "2026-03-11");
    }

    #[test]
    fn compact_post_no_dates() {
        let mut post = make_test_post(false, false);
        post.released_at = None;
        post.updated_at = None;
        let compact = CompactPost::from(&post);
        assert_eq!(compact.date, "");
    }

    #[test]
    fn compact_post_list_serializes_as_array() {
        let posts = vec![
            CompactPost::from(&make_test_post(false, false)),
            CompactPost::from(&make_test_post(true, false)),
        ];
        let json = serde_json::to_string(&posts).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
    }

    #[test]
    fn compact_post_empty_list_serializes_as_empty_array() {
        let posts: Vec<CompactPost> = vec![];
        let json = serde_json::to_string(&posts).unwrap();
        assert_eq!(json, "[]");
    }

    #[test]
    fn compact_error_serializes() {
        let err = CompactError {
            error: "Not authenticated".to_string(),
            exit_code: 2,
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains(r#""error":"Not authenticated""#));
        assert!(json.contains(r#""exit_code":2"#));
    }

    #[test]
    fn compact_message_serializes() {
        let msg = CompactMessage {
            status: "ok".to_string(),
            msg: "Post created".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""status":"ok""#));
        assert!(json.contains(r#""msg":"Post created""#));
    }

    #[test]
    fn compact_mutation_result_serializes() {
        let result = CompactMutationResult {
            url: "https://velog.io/@user/test".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains(r#""url":"https://velog.io/@user/test""#));
    }
}
