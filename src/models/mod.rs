use serde::{Deserialize, Serialize};

mod post;
mod search;
mod series;
mod stats;
mod tag;
pub use post::*;
pub use search::*;
pub use series::*;
pub use stats::*;
pub use tag::*;

// ---- GraphQL Envelope ----
#[derive(Serialize)]
pub struct GraphQLRequest<V: Serialize> {
    pub query: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<V>,
}

#[derive(Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize, Debug)]
pub struct GraphQLError {
    pub message: String,
    pub extensions: Option<serde_json::Value>,
}

impl<T> GraphQLResponse<T> {
    /// data가 있으면 반환, errors만 있으면 에러, 둘 다 없으면 에러
    pub fn into_result(self) -> anyhow::Result<T> {
        match (self.data, self.errors) {
            (Some(data), _) => Ok(data), // partial errors 시에도 data 우선
            (None, Some(errs)) => {
                let msg = errs
                    .first()
                    .map(|e| e.message.clone())
                    .unwrap_or_else(|| "Unknown GraphQL error".into());
                anyhow::bail!("GraphQL error: {}", msg)
            }
            (None, None) => anyhow::bail!("Empty GraphQL response"),
        }
    }

    /// 에러 배열에서 인증 관련 에러인지 확인
    pub fn is_auth_error(&self) -> bool {
        self.errors.as_ref().is_some_and(|errs| {
            errs.iter().any(|e| {
                // 1차: extension code (안정적, 서버 메시지 변경에 무관)
                let has_code = e
                    .extensions
                    .as_ref()
                    .and_then(|ext| ext.get("code"))
                    .and_then(|v| v.as_str())
                    .is_some_and(|c| c == "UNAUTHENTICATED");
                // 2차: 메시지 fallback (구형 응답 호환, 대소문자 무시)
                let has_msg = {
                    let msg = e.message.to_lowercase();
                    msg.contains("not logged in") || msg.contains("unauthorized")
                };
                has_code || has_msg
            })
        })
    }
}

// ---- Compact Output Types (generic, no Post dependency) ----

/// Compact mutation result for create/edit/publish
#[derive(Serialize, Debug)]
pub struct CompactMutationResult {
    pub url: String,
}

/// Compact status message for stderr
#[derive(Serialize, Debug)]
pub struct CompactMessage {
    pub status: String,
    pub msg: String,
}

/// Compact error for stderr
#[derive(Serialize, Debug)]
pub struct CompactError {
    pub error: String,
    pub exit_code: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_response_with_error(message: &str, code: Option<&str>) -> GraphQLResponse<()> {
        let extensions = code.map(|c| serde_json::json!({ "code": c }));
        GraphQLResponse {
            data: None,
            errors: Some(vec![GraphQLError {
                message: message.to_string(),
                extensions,
            }]),
        }
    }

    #[test]
    fn is_auth_error_extension_code() {
        let resp = make_response_with_error("any message", Some("UNAUTHENTICATED"));
        assert!(resp.is_auth_error());
    }

    #[test]
    fn is_auth_error_not_logged_in_message() {
        let resp = make_response_with_error("not logged in", None);
        assert!(resp.is_auth_error());
    }

    #[test]
    fn is_auth_error_unauthorized_message() {
        let resp = make_response_with_error("Unauthorized", None);
        assert!(resp.is_auth_error());
    }

    #[test]
    fn is_auth_error_case_insensitive() {
        let resp = make_response_with_error("NOT LOGGED IN", None);
        assert!(resp.is_auth_error());
    }

    #[test]
    fn is_auth_error_not_auth() {
        let resp = make_response_with_error("Post not found", None);
        assert!(!resp.is_auth_error());
    }

    #[test]
    fn is_auth_error_no_errors() {
        let resp: GraphQLResponse<()> = GraphQLResponse {
            data: Some(()),
            errors: None,
        };
        assert!(!resp.is_auth_error());
    }

    #[test]
    fn graphql_response_into_result_with_data() {
        let resp = GraphQLResponse {
            data: Some(42),
            errors: None,
        };
        assert_eq!(resp.into_result().unwrap(), 42);
    }

    #[test]
    fn graphql_response_into_result_error_only() {
        let resp = make_response_with_error("something failed", None);
        let err = resp.into_result().unwrap_err();
        assert!(err.to_string().contains("something failed"));
    }

    #[test]
    fn graphql_response_into_result_empty() {
        let resp: GraphQLResponse<()> = GraphQLResponse {
            data: None,
            errors: None,
        };
        let err = resp.into_result().unwrap_err();
        assert!(err.to_string().contains("Empty"));
    }

    #[test]
    fn compact_error_serializes() {
        let err = CompactError {
            error: "Not authenticated".to_string(),
            exit_code: 2,
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains(r#""error":"Not authenticated""#));
        assert!(json.contains(r#""exit_code":2"#));
    }

    #[test]
    fn compact_message_serializes() {
        let msg = CompactMessage {
            status: "ok".to_string(),
            msg: "Post created".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""status":"ok""#));
        assert!(json.contains(r#""msg":"Post created""#));
    }

    #[test]
    fn compact_mutation_result_serializes() {
        let result = CompactMutationResult {
            url: "https://velog.io/@user/test".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains(r#""url":"https://velog.io/@user/test""#));
    }
}
