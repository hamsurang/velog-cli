use crate::auth::Credentials;
use crate::models::{
    Comment, CommentsData, EditCommentData, GraphQLResponse, RemoveCommentData, WriteCommentData,
};

use super::{VelogClient, API_V2};

const GET_COMMENTS_QUERY: &str = r#"
    query ($post_id: ID!) {
        comments(post_id: $post_id) {
            id text level reply_to deleted
            created_at updated_at
            user { username }
            replies {
                id text level reply_to deleted
                created_at updated_at
                user { username }
                replies {
                    id text level reply_to deleted
                    created_at updated_at
                    user { username }
                }
            }
        }
    }
"#;

const WRITE_COMMENT_MUTATION: &str = r#"
    mutation ($post_id: ID!, $text: String!, $comment_id: ID) {
        writeComment(post_id: $post_id, text: $text, comment_id: $comment_id) {
            id text created_at
            user { username }
        }
    }
"#;

const EDIT_COMMENT_MUTATION: &str = r#"
    mutation ($id: ID!, $text: String!) {
        editComment(id: $id, text: $text) {
            id text updated_at
            user { username }
        }
    }
"#;

const REMOVE_COMMENT_MUTATION: &str = r#"
    mutation ($id: ID!) {
        removeComment(id: $id)
    }
"#;

impl VelogClient {
    /// 댓글 목록 조회 (anonymous, v2 API)
    pub async fn get_comments(&self, post_id: &str) -> anyhow::Result<Vec<Comment>> {
        let vars = serde_json::json!({ "post_id": post_id });
        let resp: GraphQLResponse<CommentsData> = self
            .raw_graphql(API_V2, GET_COMMENTS_QUERY, Some(&vars))
            .await?;
        Ok(resp.into_result()?.comments)
    }

    /// 댓글 작성 (authenticated, v2 API)
    pub async fn write_comment(
        &mut self,
        post_id: &str,
        text: &str,
        reply_to: Option<&str>,
    ) -> anyhow::Result<(Comment, Option<Credentials>)> {
        let vars = serde_json::json!({
            "post_id": post_id,
            "text": text,
            "comment_id": reply_to,
        });
        let (data, creds): (WriteCommentData, _) = self
            .execute_graphql(API_V2, WRITE_COMMENT_MUTATION, Some(vars))
            .await?;
        Ok((data.write_comment, creds))
    }

    /// 댓글 수정 (authenticated, v2 API)
    pub async fn edit_comment(
        &mut self,
        id: &str,
        text: &str,
    ) -> anyhow::Result<(Comment, Option<Credentials>)> {
        let vars = serde_json::json!({
            "id": id,
            "text": text,
        });
        let (data, creds): (EditCommentData, _) = self
            .execute_graphql(API_V2, EDIT_COMMENT_MUTATION, Some(vars))
            .await?;
        Ok((data.edit_comment, creds))
    }

    /// 댓글 삭제 (authenticated, v2 API)
    pub async fn remove_comment(
        &mut self,
        id: &str,
    ) -> anyhow::Result<(bool, Option<Credentials>)> {
        let vars = serde_json::json!({ "id": id });
        let (data, creds): (RemoveCommentData, _) = self
            .execute_graphql(API_V2, REMOVE_COMMENT_MUTATION, Some(vars))
            .await?;
        Ok((data.remove_comment, creds))
    }
}
