use crate::auth::Credentials;
use crate::models::{
    AppendToSeriesData, CreateSeriesData, EditSeriesData, GraphQLResponse,
    RemoveSeriesData, Series, SeriesData, SeriesListData,
};

use super::{VelogClient, API_V2};

const GET_SERIES_LIST_QUERY: &str = r#"
    query ($username: String!) {
        seriesList(username: $username) {
            id name url_slug description thumbnail
            created_at updated_at
            series_posts { id index }
        }
    }
"#;

const GET_SERIES_QUERY: &str = r#"
    query ($username: String!, $url_slug: String!) {
        series(username: $username, url_slug: $url_slug) {
            id name url_slug description thumbnail
            created_at updated_at
            series_posts {
                id index
                post {
                    id title url_slug short_description
                    released_at updated_at
                }
            }
        }
    }
"#;

const CREATE_SERIES_MUTATION: &str = r#"
    mutation ($name: String!, $url_slug: String!) {
        createSeries(name: $name, url_slug: $url_slug) {
            id name url_slug description
        }
    }
"#;

const EDIT_SERIES_MUTATION: &str = r#"
    mutation ($id: ID!, $name: String!, $series_order: [ID]) {
        editSeries(id: $id, name: $name, series_order: $series_order) {
            id name url_slug description
            series_posts { id index }
        }
    }
"#;

const APPEND_TO_SERIES_MUTATION: &str = r#"
    mutation ($series_id: ID!, $post_id: ID!) {
        appendSeriesPost(series_id: $series_id, post_id: $post_id) {
            id index
        }
    }
"#;

const REMOVE_SERIES_MUTATION: &str = r#"
    mutation ($id: ID!) {
        removeSeries(id: $id)
    }
"#;

impl VelogClient {
    /// 시리즈 목록 (anonymous, v2 API)
    pub async fn get_series_list(
        &self,
        username: &str,
    ) -> anyhow::Result<Vec<Series>> {
        let vars = serde_json::json!({ "username": username });
        let resp: GraphQLResponse<SeriesListData> = self
            .raw_graphql(API_V2, GET_SERIES_LIST_QUERY, Some(&vars))
            .await?;
        Ok(resp.into_result()?.series_list)
    }

    /// 시리즈 상세 (anonymous, v2 API)
    pub async fn get_series(
        &self,
        username: &str,
        url_slug: &str,
    ) -> anyhow::Result<Series> {
        let vars = serde_json::json!({
            "username": username,
            "url_slug": url_slug,
        });
        let resp: GraphQLResponse<SeriesData> = self
            .raw_graphql(API_V2, GET_SERIES_QUERY, Some(&vars))
            .await?;
        Ok(resp.into_result()?.series)
    }

    /// 시리즈 생성 (authenticated, v2 API)
    pub async fn create_series(
        &mut self,
        name: &str,
        url_slug: &str,
    ) -> anyhow::Result<(Series, Option<Credentials>)> {
        let vars = serde_json::json!({
            "name": name,
            "url_slug": url_slug,
        });
        let (data, creds): (CreateSeriesData, _) = self
            .execute_graphql(API_V2, CREATE_SERIES_MUTATION, Some(vars))
            .await?;
        Ok((data.create_series, creds))
    }

    /// 시리즈 수정 (authenticated, v2 API)
    pub async fn edit_series(
        &mut self,
        id: &str,
        name: &str,
        series_order: Option<Vec<String>>,
    ) -> anyhow::Result<(Series, Option<Credentials>)> {
        let vars = serde_json::json!({
            "id": id,
            "name": name,
            "series_order": series_order,
        });
        let (data, creds): (EditSeriesData, _) = self
            .execute_graphql(API_V2, EDIT_SERIES_MUTATION, Some(vars))
            .await?;
        Ok((data.edit_series, creds))
    }

    /// 시리즈에 포스트 추가 (authenticated, v2 API)
    pub async fn append_to_series(
        &mut self,
        series_id: &str,
        post_id: &str,
    ) -> anyhow::Result<(crate::models::SeriesPost, Option<Credentials>)> {
        let vars = serde_json::json!({
            "series_id": series_id,
            "post_id": post_id,
        });
        let (data, creds): (AppendToSeriesData, _) = self
            .execute_graphql(API_V2, APPEND_TO_SERIES_MUTATION, Some(vars))
            .await?;
        Ok((data.append_series_post, creds))
    }

    /// 시리즈 삭제 (authenticated, v2 API)
    pub async fn remove_series(
        &mut self,
        id: &str,
    ) -> anyhow::Result<(bool, Option<Credentials>)> {
        let vars = serde_json::json!({ "id": id });
        let (data, creds): (RemoveSeriesData, _) = self
            .execute_graphql(API_V2, REMOVE_SERIES_MUTATION, Some(vars))
            .await?;
        Ok((data.remove_series, creds))
    }
}
