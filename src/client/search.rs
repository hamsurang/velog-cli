use crate::models::{GraphQLResponse, SearchPostsData, SearchResult};

use super::{VelogClient, API_V2};

const SEARCH_POSTS_QUERY: &str = r#"
    query ($keyword: String!, $offset: Int, $limit: Int, $username: String) {
        searchPosts(keyword: $keyword, offset: $offset, limit: $limit, username: $username) {
            count
            posts {
                id title short_description thumbnail
                likes url_slug released_at updated_at tags
                user { username }
            }
        }
    }
"#;

impl VelogClient {
    /// 포스트 검색 (anonymous, v2 API, offset-based)
    pub async fn search_posts(
        &self,
        keyword: &str,
        offset: u32,
        limit: u32,
        username: Option<&str>,
    ) -> anyhow::Result<SearchResult> {
        let vars = serde_json::json!({
            "keyword": keyword,
            "offset": offset,
            "limit": limit,
            "username": username,
        });
        let resp: GraphQLResponse<SearchPostsData> = self
            .raw_graphql(API_V2, SEARCH_POSTS_QUERY, Some(&vars))
            .await?;
        Ok(resp.into_result()?.search_posts)
    }
}
