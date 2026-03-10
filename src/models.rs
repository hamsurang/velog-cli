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
