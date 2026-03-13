use crate::auth::Credentials;
use crate::models::{
    FollowData, GraphQLResponse, LikePostData, Post, ReadingListData, UnfollowData, UnlikePostData,
    User,
};

use super::{VelogClient, API_V2, API_V3};

const LIKE_POST_MUTATION: &str = r#"
    mutation ($id: ID!) {
        likePost(id: $id) {
            id title url_slug likes
        }
    }
"#;

const UNLIKE_POST_MUTATION: &str = r#"
    mutation ($id: ID!) {
        unlikePost(id: $id) {
            id title url_slug likes
        }
    }
"#;

const GET_USER_QUERY: &str = r#"
    query ($username: String!) {
        user(username: $username) {
            id username email
        }
    }
"#;

const FOLLOW_MUTATION: &str = r#"
    mutation ($user_id: ID!) {
        followUser(user_id: $user_id)
    }
"#;

const UNFOLLOW_MUTATION: &str = r#"
    mutation ($user_id: ID!) {
        unfollowUser(user_id: $user_id)
    }
"#;

const GET_READING_LIST_QUERY: &str = r#"
    query ($type: ReadingListType!, $limit: Int, $cursor: ID) {
        readingList(type: $type, limit: $limit, cursor: $cursor) {
            id title short_description thumbnail
            likes is_private is_temp url_slug
            released_at updated_at tags
            user { username }
        }
    }
"#;

// ---- User query response wrapper ----
#[derive(serde::Deserialize)]
struct UserData {
    user: Option<User>,
}

impl VelogClient {
    /// 좋아요 (authenticated, v2 API)
    pub async fn like_post(&mut self, id: &str) -> anyhow::Result<(Post, Option<Credentials>)> {
        let vars = serde_json::json!({ "id": id });
        let (data, creds): (LikePostData, _) = self
            .execute_graphql(API_V2, LIKE_POST_MUTATION, Some(vars))
            .await?;
        Ok((data.like_post, creds))
    }

    /// 좋아요 취소 (authenticated, v2 API)
    pub async fn unlike_post(&mut self, id: &str) -> anyhow::Result<(Post, Option<Credentials>)> {
        let vars = serde_json::json!({ "id": id });
        let (data, creds): (UnlikePostData, _) = self
            .execute_graphql(API_V2, UNLIKE_POST_MUTATION, Some(vars))
            .await?;
        Ok((data.unlike_post, creds))
    }

    /// 유저 정보 조회 (anonymous, v3 API)
    pub async fn get_user(&self, username: &str) -> anyhow::Result<User> {
        let vars = serde_json::json!({ "username": username });
        let resp: GraphQLResponse<UserData> = self
            .raw_graphql(API_V3, GET_USER_QUERY, Some(&vars))
            .await?;
        resp.into_result()?
            .user
            .ok_or_else(|| anyhow::anyhow!("User not found: {}", username))
    }

    /// 팔로우 (authenticated, v2 API)
    pub async fn follow_user(
        &mut self,
        user_id: &str,
    ) -> anyhow::Result<(bool, Option<Credentials>)> {
        let vars = serde_json::json!({ "user_id": user_id });
        let (data, creds): (FollowData, _) = self
            .execute_graphql(API_V2, FOLLOW_MUTATION, Some(vars))
            .await?;
        Ok((data.follow_user, creds))
    }

    /// 언팔로우 (authenticated, v2 API)
    pub async fn unfollow_user(
        &mut self,
        user_id: &str,
    ) -> anyhow::Result<(bool, Option<Credentials>)> {
        let vars = serde_json::json!({ "user_id": user_id });
        let (data, creds): (UnfollowData, _) = self
            .execute_graphql(API_V2, UNFOLLOW_MUTATION, Some(vars))
            .await?;
        Ok((data.unfollow_user, creds))
    }

    /// 리딩리스트 조회 (authenticated, v2 API)
    pub async fn get_reading_list(
        &mut self,
        list_type: &str,
        limit: u32,
        cursor: Option<&str>,
    ) -> anyhow::Result<(Vec<Post>, Option<Credentials>)> {
        let vars = serde_json::json!({
            "type": list_type,
            "limit": limit,
            "cursor": cursor,
        });
        let (data, creds): (ReadingListData, _) = self
            .execute_graphql(API_V2, GET_READING_LIST_QUERY, Some(vars))
            .await?;
        Ok((data.reading_list, creds))
    }
}
