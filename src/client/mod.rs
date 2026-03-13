use std::time::Duration;

use anyhow::Context as _;
use reqwest::header::{HeaderValue, COOKIE};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::auth::Credentials;
use crate::models::{CurrentUserData, GraphQLRequest, GraphQLResponse, RestoreTokenData};

mod comment;
mod post;
mod search;
mod series;
mod social;
mod stats;
mod tag;

pub(crate) const API_V3: &str = "https://v3.velog.io/graphql";
pub(crate) const API_V2: &str = "https://v2.velog.io/graphql";

const CURRENT_USER_QUERY: &str = r#"{ currentUser { id username email } }"#;

const RESTORE_TOKEN_QUERY: &str = r#"{ restoreToken { accessToken refreshToken } }"#;

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
    pub(crate) async fn raw_graphql<V: Serialize, T: DeserializeOwned>(
        &self,
        url: &str,
        query: &'static str,
        variables: Option<&V>,
    ) -> anyhow::Result<GraphQLResponse<T>> {
        let mut req = self
            .http
            .post(url)
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

        let resp = req.send().await?;
        let status = resp.status();
        let body = resp.bytes().await?;
        if body.is_empty() || !status.is_success() {
            let preview: String = String::from_utf8_lossy(&body).chars().take(200).collect();
            anyhow::bail!("API error: status={}, body={}", status, preview);
        }
        let parsed: GraphQLResponse<T> = serde_json::from_slice(&body).with_context(|| {
            let preview: String = String::from_utf8_lossy(&body).chars().take(200).collect();
            format!("Failed to parse response: {}", preview)
        })?;
        Ok(parsed)
    }

    /// execute_graphql: raw_graphql + 인증 에러 시 1회 갱신 재시도
    pub async fn execute_graphql<V, T>(
        &mut self,
        url: &str,
        query: &'static str,
        variables: Option<V>,
    ) -> anyhow::Result<(T, Option<Credentials>)>
    where
        V: Serialize,
        T: DeserializeOwned,
    {
        let resp: GraphQLResponse<T> = self.raw_graphql(url, query, variables.as_ref()).await?;

        // data가 없고 인증 에러인 경우에만 재시도
        if resp.data.is_none() && resp.is_auth_error() && self.credentials.is_some() {
            let mut new_creds = self.restore_token().await.map_err(|e| {
                anyhow::Error::new(crate::auth::AuthError).context(format!(
                    "Token refresh failed: {e:#}. Run `velog auth login` again."
                ))
            })?;
            // 기존 credentials의 cached username 보존
            new_creds.username = self.credentials.as_ref().and_then(|c| c.username.clone());
            self.credentials = Some(new_creds.clone());
            let retry_resp: GraphQLResponse<T> =
                self.raw_graphql(url, query, variables.as_ref()).await?;
            let data = retry_resp.into_result()?;
            return Ok((data, Some(new_creds)));
        }

        let data = resp.into_result()?;
        Ok((data, None))
    }

    /// 토큰 갱신 (execute_graphql 미경유 — 무한 루프 방지)
    async fn restore_token(&self) -> anyhow::Result<Credentials> {
        let resp: GraphQLResponse<RestoreTokenData> = self
            .raw_graphql(API_V3, RESTORE_TOKEN_QUERY, None::<&()>)
            .await?;
        let data = resp.into_result()?;
        Ok(data.restore_token.into())
    }

    // ---- Auth API methods ----

    pub async fn current_user(
        &mut self,
    ) -> anyhow::Result<(crate::models::User, Option<Credentials>)> {
        let (data, creds): (CurrentUserData, _) = self
            .execute_graphql(API_V3, CURRENT_USER_QUERY, None::<()>)
            .await?;
        Ok((data.current_user, creds))
    }
}
