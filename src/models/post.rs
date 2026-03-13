use serde::{Deserialize, Serialize};

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
    #[serde(default)]
    pub is_private: bool,
    #[serde(default)]
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

#[derive(Deserialize)]
pub struct TrendingPostsData {
    #[serde(rename = "trendingPosts")]
    pub trending_posts: Vec<Post>,
}

#[derive(Deserialize)]
pub struct RecentPostsData {
    #[serde(rename = "recentPosts")]
    pub recent_posts: Vec<Post>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// Compact auth status for `auth status`
#[derive(Serialize, Debug)]
pub struct CompactAuthStatus {
    pub logged_in: bool,
    pub username: String,
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
            user: post.user.as_ref().map(|u| u.username.clone()),
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
            user: post.user.as_ref().map(|u| u.username.clone()),
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

    // ---- Response wrapper deserialization tests ----

    #[test]
    fn trending_posts_data_deserializes() {
        let json = r#"{
            "trendingPosts": [
                {
                    "id": "t1", "title": "Trending", "likes": 42,
                    "url_slug": "trending-post",
                    "released_at": "2026-03-10T00:00:00.000Z",
                    "updated_at": null, "tags": ["rust"],
                    "user": { "username": "author1" }
                }
            ]
        }"#;
        let data: TrendingPostsData = serde_json::from_str(json).unwrap();
        assert_eq!(data.trending_posts.len(), 1);
        assert_eq!(data.trending_posts[0].title, "Trending");
        assert_eq!(data.trending_posts[0].likes, 42);
    }

    #[test]
    fn recent_posts_data_deserializes() {
        let json = r#"{
            "recentPosts": [
                {
                    "id": "r1", "title": "Recent", "likes": 5,
                    "url_slug": "recent-post",
                    "released_at": null, "updated_at": "2026-03-11T00:00:00.000Z",
                    "tags": [], "user": { "username": "author2" }
                }
            ]
        }"#;
        let data: RecentPostsData = serde_json::from_str(json).unwrap();
        assert_eq!(data.recent_posts.len(), 1);
        assert_eq!(data.recent_posts[0].title, "Recent");
    }

    #[test]
    fn post_serde_default_missing_is_temp_and_is_private() {
        let json = r#"{
            "id": "p1", "title": "No Flags", "likes": 0,
            "url_slug": "no-flags",
            "released_at": null, "updated_at": null, "tags": null
        }"#;
        let post: Post = serde_json::from_str(json).unwrap();
        assert!(!post.is_temp);
        assert!(!post.is_private);
    }

    #[test]
    fn compact_post_includes_user_when_present() {
        let mut post = make_test_post(false, false);
        post.user = Some(PostUser {
            username: "teo".to_string(),
        });
        let compact = CompactPost::from(&post);
        assert_eq!(compact.user.as_deref(), Some("teo"));
        let json = serde_json::to_string(&compact).unwrap();
        assert!(json.contains(r#""user":"teo""#));
    }

    #[test]
    fn compact_post_omits_user_when_none() {
        let post = make_test_post(false, false);
        let compact = CompactPost::from(&post);
        assert!(compact.user.is_none());
        let json = serde_json::to_string(&compact).unwrap();
        assert!(!json.contains("user"));
    }
}
