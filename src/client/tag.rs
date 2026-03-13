use crate::models::{GraphQLResponse, PostsData, TagsData, UserTagsData};

use super::{VelogClient, API_V2};

const GET_TAGS_QUERY: &str = r#"
    query ($sort: String!, $limit: Int, $cursor: ID) {
        tags(sort: $sort, limit: $limit, cursor: $cursor) {
            id name description thumbnail posts_count
        }
    }
"#;

const GET_USER_TAGS_QUERY: &str = r#"
    query ($username: String!) {
        userTags(username: $username) {
            tag { id name description thumbnail }
            posts_count
        }
    }
"#;

const GET_POSTS_BY_TAG_QUERY: &str = r#"
    query ($tag: String!, $username: String, $limit: Int, $cursor: ID) {
        posts(tag: $tag, username: $username, limit: $limit, cursor: $cursor) {
            id title short_description thumbnail
            likes is_private is_temp url_slug
            released_at updated_at tags
            user { username }
        }
    }
"#;

impl VelogClient {
    /// 태그 목록 (anonymous, v2 API)
    pub async fn get_tags(
        &self,
        sort: &str,
        limit: u32,
        cursor: Option<&str>,
    ) -> anyhow::Result<Vec<crate::models::Tag>> {
        let vars = serde_json::json!({
            "sort": sort,
            "limit": limit,
            "cursor": cursor,
        });
        let resp: GraphQLResponse<TagsData> = self
            .raw_graphql(API_V2, GET_TAGS_QUERY, Some(&vars))
            .await?;
        Ok(resp.into_result()?.tags)
    }

    /// 유저별 태그 목록 (anonymous, v2 API)
    pub async fn get_user_tags(
        &self,
        username: &str,
    ) -> anyhow::Result<Vec<crate::models::UserTag>> {
        let vars = serde_json::json!({ "username": username });
        let resp: GraphQLResponse<UserTagsData> = self
            .raw_graphql(API_V2, GET_USER_TAGS_QUERY, Some(&vars))
            .await?;
        Ok(resp.into_result()?.user_tags)
    }

    /// 태그별 포스트 목록 (anonymous, v2 API, cursor-based)
    pub async fn get_posts_by_tag(
        &self,
        tag: &str,
        username: Option<&str>,
        limit: u32,
        cursor: Option<&str>,
    ) -> anyhow::Result<Vec<crate::models::Post>> {
        let vars = serde_json::json!({
            "tag": tag,
            "username": username,
            "limit": limit,
            "cursor": cursor,
        });
        let resp: GraphQLResponse<PostsData> = self
            .raw_graphql(API_V2, GET_POSTS_BY_TAG_QUERY, Some(&vars))
            .await?;
        Ok(resp.into_result()?.posts)
    }
}
