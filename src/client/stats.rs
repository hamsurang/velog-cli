use crate::auth::Credentials;
use crate::models::{GetStatsData, Stats};

use super::{VelogClient, API_V2};

const GET_STATS_QUERY: &str = r#"
    query ($post_id: ID!) {
        getStats(post_id: $post_id) {
            total
            count_by_day { count day }
        }
    }
"#;

impl VelogClient {
    /// 포스트 통계 조회 (authenticated, v2 API)
    pub async fn get_stats(
        &mut self,
        post_id: &str,
    ) -> anyhow::Result<(Stats, Option<Credentials>)> {
        let vars = serde_json::json!({ "post_id": post_id });
        let (data, creds): (GetStatsData, _) = self
            .execute_graphql(API_V2, GET_STATS_QUERY, Some(vars))
            .await?;
        Ok((data.get_stats, creds))
    }
}
