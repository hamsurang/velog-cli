use std::time::Duration;

use anyhow::Context as _;
use reqwest::header::{HeaderValue, COOKIE};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::auth::Credentials;
use crate::models::{
    CurrentUserData, EditPostData, GraphQLRequest, GraphQLResponse, PostData, PostsData,
    RemovePostData, RestoreTokenData, WritePostData,
};
use crate::models::{EditPostInput, WritePostInput};

const CURRENT_USER_QUERY: &str = r#"{ currentUser { id username email } }"#;

const RESTORE_TOKEN_QUERY: &str = r#"{ restoreToken { accessToken refreshToken } }"#;

// NOTE: velog GraphQL API는 snake_case 필드명을 사용합니다.
// Rust 모델의 #[serde(rename_all = "camelCase")]와 매핑이 맞는지
// 실제 API 응답으로 검증 완료된 쿼리입니다.

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
            released_at updated_at tags meta series_id
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
            id title url_slug is_temp is_private
            released_at updated_at tags
            user { username }
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
            id title url_slug is_temp is_private
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

pub struct VelogClient {
    http: reqwest::Client,
    credentials: Option<Credentials>,
}

impl VelogClient {
    /// 인증된 클라이언트 (CRUD + 토큰 갱신)
    pub fn new(credentials: Credentials) -> anyhow::Result<Self> {
        Ok(Self {
            http: Self::build_http()?,
            credentials: Some(credentials),
        })
    }

    /// 미인증 클라이언트 (public 쿼리 전용)
    pub fn anonymous() -> anyhow::Result<Self> {
        Ok(Self {
            http: Self::build_http()?,
            credentials: None,
        })
    }

    fn build_http() -> anyhow::Result<reqwest::Client> {
        reqwest::Client::builder()
            .https_only(true)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("velog-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(Into::into)
    }

    /// raw_graphql: 단순 HTTP 요청 → GraphQLResponse 그대로 반환 (into_result 미호출)
    async fn raw_graphql<V: Serialize, T: DeserializeOwned>(
        &self,
        query: &'static str,
        variables: Option<&V>,
    ) -> anyhow::Result<GraphQLResponse<T>> {
        let mut req = self
            .http
            .post("https://v3.velog.io/graphql")
            .json(&GraphQLRequest { query, variables });

        if let Some(creds) = &self.credentials {
            let mut cookie = HeaderValue::from_str(&format!(
                "access_token={}; refresh_token={}",
                creds.access_token, creds.refresh_token
            ))
            .context("Token contains invalid characters for HTTP header")?;
            cookie.set_sensitive(true);
            req = req.header(COOKIE, cookie);
        }

        let resp = req.send().await?.json::<GraphQLResponse<T>>().await?;
        Ok(resp)
    }

    /// execute_graphql: raw_graphql + 인증 에러 시 1회 갱신 재시도
    pub async fn execute_graphql<V, T>(
        &mut self,
        query: &'static str,
        variables: Option<V>,
    ) -> anyhow::Result<(T, Option<Credentials>)>
    where
        V: Serialize,
        T: DeserializeOwned,
    {
        let resp: GraphQLResponse<T> = self.raw_graphql(query, variables.as_ref()).await?;

        // data가 없고 인증 에러인 경우에만 재시도
        if resp.data.is_none() && resp.is_auth_error() && self.credentials.is_some() {
            let new_creds = self.restore_token().await.map_err(|_| {
                anyhow::Error::new(crate::auth::AuthError)
                    .context("Token refresh failed. Run `velog auth login` again.")
            })?;
            self.credentials = Some(new_creds.clone());
            let retry_resp: GraphQLResponse<T> =
                self.raw_graphql(query, variables.as_ref()).await?;
            let data = retry_resp.into_result()?;
            return Ok((data, Some(new_creds)));
        }

        let data = resp.into_result()?;
        Ok((data, None))
    }

    /// 토큰 갱신 (execute_graphql 미경유 — 무한 루프 방지)
    async fn restore_token(&self) -> anyhow::Result<Credentials> {
        let resp: GraphQLResponse<RestoreTokenData> =
            self.raw_graphql(RESTORE_TOKEN_QUERY, None::<&()>).await?;
        let data = resp.into_result()?;
        Ok(data.restore_token.into())
    }

    // ---- Public API methods ----

    pub async fn current_user(
        &mut self,
    ) -> anyhow::Result<(crate::models::User, Option<Credentials>)> {
        let (data, creds): (CurrentUserData, _) =
            self.execute_graphql(CURRENT_USER_QUERY, None::<()>).await?;
        Ok((data.current_user, creds))
    }

    pub async fn get_posts(
        &mut self,
        username: &str,
        temp_only: bool,
    ) -> anyhow::Result<(Vec<crate::models::Post>, Option<Credentials>)> {
        let vars = serde_json::json!({
            "username": username,
            "temp_only": temp_only,
        });
        let (data, creds): (PostsData, _) =
            self.execute_graphql(GET_POSTS_QUERY, Some(vars)).await?;
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
        let (data, creds): (PostData, _) = self.execute_graphql(GET_POST_QUERY, Some(vars)).await?;
        Ok((data.post, creds))
    }

    pub async fn write_post(
        &mut self,
        input: WritePostInput,
    ) -> anyhow::Result<(crate::models::Post, Option<Credentials>)> {
        let vars = serde_json::to_value(&input)?;
        let (data, creds): (WritePostData, _) = self
            .execute_graphql(WRITE_POST_MUTATION, Some(vars))
            .await?;
        Ok((data.write_post, creds))
    }

    pub async fn edit_post(
        &mut self,
        input: EditPostInput,
    ) -> anyhow::Result<(crate::models::Post, Option<Credentials>)> {
        let vars = serde_json::to_value(&input)?;
        let (data, creds): (EditPostData, _) =
            self.execute_graphql(EDIT_POST_MUTATION, Some(vars)).await?;
        Ok((data.edit_post, creds))
    }

    pub async fn remove_post(&mut self, id: &str) -> anyhow::Result<(bool, Option<Credentials>)> {
        let vars = serde_json::json!({ "id": id });
        let (data, creds): (RemovePostData, _) = self
            .execute_graphql(REMOVE_POST_MUTATION, Some(vars))
            .await?;
        Ok((data.remove_post, creds))
    }
}
