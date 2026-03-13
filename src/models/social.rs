use serde::Deserialize;

use super::Post;

// ---- Like Response Wrappers ----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LikePostData {
    pub like_post: Post,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlikePostData {
    pub unlike_post: Post,
}

// ---- Follow Response Wrappers ----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowData {
    pub follow_user: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnfollowData {
    pub unfollow_user: bool,
}

// ---- Reading List Response Wrapper ----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingListData {
    pub reading_list: Vec<Post>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_post_data_deserializes() {
        let json = r#"{
            "likePost": {
                "id": "p1", "title": "Test", "url_slug": "test",
                "short_description": "", "thumbnail": null,
                "likes": 5, "is_private": false, "is_temp": false,
                "released_at": null, "updated_at": null, "tags": [], "user": null
            }
        }"#;
        let data: LikePostData = serde_json::from_str(json).unwrap();
        assert_eq!(data.like_post.likes, 5);
    }

    #[test]
    fn unlike_post_data_deserializes() {
        let json = r#"{
            "unlikePost": {
                "id": "p1", "title": "Test", "url_slug": "test",
                "short_description": "", "thumbnail": null,
                "likes": 4, "is_private": false, "is_temp": false,
                "released_at": null, "updated_at": null, "tags": [], "user": null
            }
        }"#;
        let data: UnlikePostData = serde_json::from_str(json).unwrap();
        assert_eq!(data.unlike_post.likes, 4);
    }

    #[test]
    fn follow_data_deserializes() {
        let json = r#"{ "followUser": true }"#;
        let data: FollowData = serde_json::from_str(json).unwrap();
        assert!(data.follow_user);
    }

    #[test]
    fn unfollow_data_deserializes() {
        let json = r#"{ "unfollowUser": true }"#;
        let data: UnfollowData = serde_json::from_str(json).unwrap();
        assert!(data.unfollow_user);
    }

    #[test]
    fn reading_list_data_deserializes() {
        let json = r#"{
            "readingList": [
                {
                    "id": "p1", "title": "Test", "url_slug": "test",
                    "short_description": "", "thumbnail": null,
                    "likes": 10, "is_private": false, "is_temp": false,
                    "released_at": null, "updated_at": null, "tags": [], "user": null
                }
            ]
        }"#;
        let data: ReadingListData = serde_json::from_str(json).unwrap();
        assert_eq!(data.reading_list.len(), 1);
    }

    #[test]
    fn reading_list_empty() {
        let json = r#"{ "readingList": [] }"#;
        let data: ReadingListData = serde_json::from_str(json).unwrap();
        assert!(data.reading_list.is_empty());
    }
}
