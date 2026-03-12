use crate::auth::Credentials;
use crate::models::{
    EditPostData, EditPostInput, GraphQLResponse, PostData, PostsData, RecentPostsData,
    RemovePostData, TrendingPostsData, WritePostData, WritePostInput,
};

use super::{VelogClient, API_V2, API_V3};

// NOTE: velog GraphQL API는 snake_case 필드명을 사용합니다.
// Rust 모델도 snake_case를 그대로 사용하므로 별도 serde rename이 불필요합니다.

const GET_POSTS_QUERY: &str = r#"
    query Posts($username: String!, $temp_only: Boolean) {
        posts(username: $username, temp_only: $temp_only) {
            id title short_description thumbnail
            likes is_private is_temp url_slug
            released_at updated_at tags
            user { username }
        }
    }
"#;

const GET_POST_QUERY: &str = r#"
    query Post($username: String!, $url_slug: String!) {
        post(username: $username, url_slug: $url_slug) {
            id title short_description body thumbnail
            likes is_private is_temp url_slug
            released_at updated_at tags meta
            user { username }
        }
    }
"#;

const WRITE_POST_MUTATION: &str = r#"
    mutation WritePost(
        $title: String!, $body: String!, $tags: [String]!,
        $is_markdown: Boolean!, $is_temp: Boolean!, $is_private: Boolean!,
        $url_slug: String!, $thumbnail: String, $meta: JSON!, $series_id: ID
    ) {
        writePost(
            title: $title, body: $body, tags: $tags,
            is_markdown: $is_markdown, is_temp: $is_temp, is_private: $is_private,
            url_slug: $url_slug, thumbnail: $thumbnail, meta: $meta, series_id: $series_id
        ) {
            id url_slug
        }
    }
"#;

const EDIT_POST_MUTATION: &str = r#"
    mutation EditPost(
        $id: ID!, $title: String, $body: String, $tags: [String],
        $is_markdown: Boolean, $is_temp: Boolean, $is_private: Boolean,
        $url_slug: String, $thumbnail: String, $meta: JSON, $series_id: ID
    ) {
        editPost(
            id: $id, title: $title, body: $body, tags: $tags,
            is_markdown: $is_markdown, is_temp: $is_temp, is_private: $is_private,
            url_slug: $url_slug, thumbnail: $thumbnail, meta: $meta, series_id: $series_id
        ) {
            id url_slug
        }
    }
"#;

// v3 API: trending posts (anonymous, offset-based pagination)
const GET_TRENDING_POSTS_QUERY: &str = r#"
    query ($limit: Int, $offset: Int, $timeframe: String) {
        trendingPosts(input: { limit: $limit, offset: $offset, timeframe: $timeframe }) {
            id title short_description thumbnail
            likes url_slug released_at updated_at tags
            user { username }
        }
    }
"#;

// v3 API: recent posts (anonymous, cursor-based pagination)
const GET_RECENT_POSTS_QUERY: &str = r#"
    query ($limit: Int, $cursor: ID) {
        recentPosts(input: { limit: $limit, cursor: $cursor }) {
            id title short_description thumbnail
            likes url_slug released_at updated_at tags
            user { username }
        }
    }
"#;

// v2 API: user posts with limit/cursor (anonymous)
const GET_USER_POSTS_QUERY: &str = r#"
    query Posts($username: String!, $limit: Int, $cursor: ID) {
        posts(username: $username, limit: $limit, cursor: $cursor) {
            id title short_description thumbnail
            likes is_private is_temp url_slug
            released_at updated_at tags
            user { username }
        }
    }
"#;

const REMOVE_POST_MUTATION: &str = r#"
    mutation RemovePost($id: ID!) {
        removePost(id: $id)
    }
"#;

impl VelogClient {
    pub async fn get_posts(
        &mut self,
        username: &str,
        temp_only: bool,
    ) -> anyhow::Result<(Vec<crate::models::Post>, Option<Credentials>)> {
        let vars = serde_json::json!({
            "username": username,
            "temp_only": temp_only,
        });
        let (data, creds): (PostsData, _) = self
            .execute_graphql(API_V2, GET_POSTS_QUERY, Some(vars))
            .await?;
        Ok((data.posts, creds))
    }

    pub async fn get_post(
        &mut self,
        username: &str,
        url_slug: &str,
    ) -> anyhow::Result<(crate::models::Post, Option<Credentials>)> {
        let vars = serde_json::json!({
            "username": username,
            "url_slug": url_slug,
        });
        let (data, creds): (PostData, _) = self
            .execute_graphql(API_V2, GET_POST_QUERY, Some(vars))
            .await?;
        let post = data
            .post
            .ok_or_else(|| anyhow::anyhow!("Post not found: {}", url_slug))?;
        Ok((post, creds))
    }

    pub async fn write_post(
        &mut self,
        input: WritePostInput,
    ) -> anyhow::Result<(crate::models::MutationPostResult, Option<Credentials>)> {
        let vars = serde_json::to_value(&input)?;
        let (data, creds): (WritePostData, _) = self
            .execute_graphql(API_V2, WRITE_POST_MUTATION, Some(vars))
            .await?;
        Ok((data.write_post, creds))
    }

    pub async fn edit_post(
        &mut self,
        input: EditPostInput,
    ) -> anyhow::Result<(crate::models::MutationPostResult, Option<Credentials>)> {
        let vars = serde_json::to_value(&input)?;
        let (data, creds): (EditPostData, _) = self
            .execute_graphql(API_V2, EDIT_POST_MUTATION, Some(vars))
            .await?;
        Ok((data.edit_post, creds))
    }

    /// 트렌딩 포스트 (anonymous, v3 API, offset-based)
    pub async fn get_trending_posts(
        &self,
        limit: u32,
        offset: u32,
        timeframe: &str,
    ) -> anyhow::Result<Vec<crate::models::Post>> {
        let vars = serde_json::json!({
            "limit": limit,
            "offset": offset,
            "timeframe": timeframe,
        });
        let resp: GraphQLResponse<TrendingPostsData> = self
            .raw_graphql(API_V3, GET_TRENDING_POSTS_QUERY, Some(&vars))
            .await?;
        Ok(resp.into_result()?.trending_posts)
    }

    /// 최신 포스트 (anonymous, v3 API, cursor-based)
    pub async fn get_recent_posts(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> anyhow::Result<Vec<crate::models::Post>> {
        let vars = serde_json::json!({
            "limit": limit,
            "cursor": cursor,
        });
        let resp: GraphQLResponse<RecentPostsData> = self
            .raw_graphql(API_V3, GET_RECENT_POSTS_QUERY, Some(&vars))
            .await?;
        Ok(resp.into_result()?.recent_posts)
    }

    /// 특정 유저의 포스트 (anonymous 가능, v2 API, cursor-based)
    pub async fn get_user_posts(
        &self,
        username: &str,
        limit: u32,
        cursor: Option<&str>,
    ) -> anyhow::Result<Vec<crate::models::Post>> {
        let vars = serde_json::json!({
            "username": username,
            "limit": limit,
            "cursor": cursor,
        });
        let resp: GraphQLResponse<PostsData> = self
            .raw_graphql(API_V2, GET_USER_POSTS_QUERY, Some(&vars))
            .await?;
        Ok(resp.into_result()?.posts)
    }

    pub async fn remove_post(&mut self, id: &str) -> anyhow::Result<(bool, Option<Credentials>)> {
        let vars = serde_json::json!({ "id": id });
        let (data, creds): (RemovePostData, _) = self
            .execute_graphql(API_V2, REMOVE_POST_MUTATION, Some(vars))
            .await?;
        Ok((data.remove_post, creds))
    }
}
