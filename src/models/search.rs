use serde::{Deserialize, Serialize};

use super::post::Post;

// ---- Search Response Wrapper ----

#[derive(Deserialize)]
pub struct SearchPostsData {
    #[serde(rename = "searchPosts")]
    pub search_posts: Vec<Post>,
}

// ---- Compact Search Result ----

#[derive(Serialize, Debug)]
pub struct CompactSearchResult {
    pub title: String,
    pub slug: String,
    pub user: Option<String>,
    pub tags: Vec<String>,
    pub short_description: Option<String>,
    pub date: String,
}

impl From<&Post> for CompactSearchResult {
    fn from(post: &Post) -> Self {
        CompactSearchResult {
            title: post.title.clone(),
            slug: post.url_slug.clone(),
            user: post.user.as_ref().map(|u| u.username.clone()),
            tags: post.tags.clone().unwrap_or_default(),
            short_description: post.short_description.clone(),
            date: post.date_short(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PostUser;

    #[test]
    fn search_posts_data_deserializes() {
        let json = r#"{
            "searchPosts": [
                {
                    "id": "s1", "title": "Rust Guide", "likes": 10,
                    "url_slug": "rust-guide",
                    "short_description": "A guide to Rust",
                    "released_at": "2026-03-10T00:00:00.000Z",
                    "updated_at": null, "tags": ["rust", "tutorial"],
                    "user": { "username": "author1" },
                    "thumbnail": null
                }
            ]
        }"#;
        let data: SearchPostsData = serde_json::from_str(json).unwrap();
        assert_eq!(data.search_posts.len(), 1);
        assert_eq!(data.search_posts[0].title, "Rust Guide");
        assert_eq!(data.search_posts[0].tags.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn search_posts_data_empty() {
        let json = r#"{ "searchPosts": [] }"#;
        let data: SearchPostsData = serde_json::from_str(json).unwrap();
        assert!(data.search_posts.is_empty());
    }

    #[test]
    fn compact_search_result_from_post() {
        let post = Post {
            id: "s1".to_string(),
            title: "Test".to_string(),
            short_description: Some("desc".to_string()),
            body: None,
            thumbnail: None,
            likes: 5,
            is_private: false,
            is_temp: false,
            url_slug: "test-post".to_string(),
            released_at: Some("2026-03-10T00:00:00.000Z".to_string()),
            updated_at: None,
            tags: Some(vec!["rust".to_string()]),
            meta: None,
            user: Some(PostUser {
                username: "author".to_string(),
            }),
        };
        let compact = CompactSearchResult::from(&post);
        assert_eq!(compact.title, "Test");
        assert_eq!(compact.slug, "test-post");
        assert_eq!(compact.user.as_deref(), Some("author"));
        assert_eq!(compact.date, "2026-03-10");
    }

    #[test]
    fn compact_search_result_serializes() {
        let result = CompactSearchResult {
            title: "Test".to_string(),
            slug: "test".to_string(),
            user: Some("author".to_string()),
            tags: vec!["rust".to_string()],
            short_description: Some("desc".to_string()),
            date: "2026-03-10".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains(r#""title":"Test""#));
        assert!(json.contains(r#""user":"author""#));
    }
}
