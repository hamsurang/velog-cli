use serde::{Deserialize, Serialize};

// ---- Comment Domain Models ----

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct Comment {
    pub id: Option<String>,
    pub text: Option<String>,
    pub user: Option<CommentUser>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub level: Option<i32>,
    pub reply_to: Option<String>,
    #[serde(default)]
    pub deleted: bool,
    pub replies: Option<Vec<Comment>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CommentUser {
    pub username: String,
}

// ---- Response Wrappers ----

#[derive(Deserialize)]
pub struct CommentsData {
    pub comments: Vec<Comment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteCommentData {
    pub write_comment: Comment,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditCommentData {
    pub edit_comment: Comment,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveCommentData {
    pub remove_comment: bool,
}

// ---- Compact Output ----

#[derive(Serialize, Debug)]
pub struct CompactComment {
    pub number: String,
    pub id: String,
    pub text: String,
    pub author: String,
    pub date: String,
    pub depth: usize,
    pub deleted: bool,
}

// ---- Numbered Comment (for display) ----

#[derive(Debug)]
pub struct NumberedComment {
    pub number: String,
    pub id: String,
    pub text: String,
    pub author: String,
    pub date: String,
    pub depth: usize,
    pub deleted: bool,
}

impl NumberedComment {
    pub fn id_short(&self) -> &str {
        if self.id.len() >= 8 {
            &self.id[..8]
        } else {
            &self.id
        }
    }

    pub fn to_compact(&self) -> CompactComment {
        CompactComment {
            number: self.number.clone(),
            id: self.id.clone(),
            text: self.text.clone(),
            author: self.author.clone(),
            date: self.date.clone(),
            depth: self.depth,
            deleted: self.deleted,
        }
    }
}

/// DFS 순회로 댓글 번호 할당
/// 최상위: #1, #2, ...
/// 답글: #1.1, #1.2, ...
pub fn assign_comment_numbers(comments: &[Comment]) -> Vec<NumberedComment> {
    let mut result = Vec::new();
    for (i, comment) in comments.iter().enumerate() {
        let number = (i + 1).to_string();
        flatten_comment(comment, &number, 0, &mut result);
    }
    result
}

fn flatten_comment(
    comment: &Comment,
    number: &str,
    depth: usize,
    result: &mut Vec<NumberedComment>,
) {
    let id = comment.id.as_deref().unwrap_or("").to_string();
    let text = if comment.deleted {
        "(deleted)".to_string()
    } else {
        comment.text.as_deref().unwrap_or("").to_string()
    };
    let author = comment
        .user
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_else(|| "(unknown)".to_string());
    let date = comment
        .created_at
        .as_deref()
        .map(|d| d.chars().take(10).collect())
        .unwrap_or_default();

    result.push(NumberedComment {
        number: number.to_string(),
        id,
        text,
        author,
        date,
        depth,
        deleted: comment.deleted,
    });

    if let Some(replies) = &comment.replies {
        for (j, reply) in replies.iter().enumerate() {
            let child_number = format!("{}.{}", number, j + 1);
            flatten_comment(reply, &child_number, depth + 1, result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_comment(id: &str, text: &str, user: &str, replies: Option<Vec<Comment>>) -> Comment {
        Comment {
            id: Some(id.to_string()),
            text: Some(text.to_string()),
            user: Some(CommentUser {
                username: user.to_string(),
            }),
            created_at: Some("2026-03-13T00:00:00.000Z".to_string()),
            updated_at: None,
            level: Some(0),
            reply_to: None,
            deleted: false,
            replies,
        }
    }

    #[test]
    fn assign_numbers_empty() {
        let result = assign_comment_numbers(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn assign_numbers_single() {
        let comments = vec![make_comment("c1", "hello", "user1", None)];
        let result = assign_comment_numbers(&comments);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].number, "1");
        assert_eq!(result[0].text, "hello");
        assert_eq!(result[0].author, "user1");
        assert_eq!(result[0].depth, 0);
    }

    #[test]
    fn assign_numbers_multiple_top_level() {
        let comments = vec![
            make_comment("c1", "first", "user1", None),
            make_comment("c2", "second", "user2", None),
            make_comment("c3", "third", "user3", None),
        ];
        let result = assign_comment_numbers(&comments);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].number, "1");
        assert_eq!(result[1].number, "2");
        assert_eq!(result[2].number, "3");
    }

    #[test]
    fn assign_numbers_nested_replies() {
        let comments = vec![make_comment(
            "c1",
            "parent",
            "user1",
            Some(vec![
                make_comment("c2", "reply1", "user2", None),
                make_comment(
                    "c3",
                    "reply2",
                    "user3",
                    Some(vec![make_comment("c4", "nested", "user4", None)]),
                ),
            ]),
        )];
        let result = assign_comment_numbers(&comments);
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].number, "1");
        assert_eq!(result[0].depth, 0);
        assert_eq!(result[1].number, "1.1");
        assert_eq!(result[1].depth, 1);
        assert_eq!(result[2].number, "1.2");
        assert_eq!(result[2].depth, 1);
        assert_eq!(result[3].number, "1.2.1");
        assert_eq!(result[3].depth, 2);
    }

    #[test]
    fn assign_numbers_deleted_comment() {
        let mut comment = make_comment("c1", "deleted text", "user1", None);
        comment.deleted = true;
        let result = assign_comment_numbers(&[comment]);
        assert_eq!(result[0].text, "(deleted)");
        assert!(result[0].deleted);
    }

    #[test]
    fn id_short_truncates() {
        let nc = NumberedComment {
            number: "1".into(),
            id: "abcdef1234567890".into(),
            text: "test".into(),
            author: "user".into(),
            date: "2026-03-13".into(),
            depth: 0,
            deleted: false,
        };
        assert_eq!(nc.id_short(), "abcdef12");
    }

    #[test]
    fn id_short_short_id() {
        let nc = NumberedComment {
            number: "1".into(),
            id: "abc".into(),
            text: "test".into(),
            author: "user".into(),
            date: "2026-03-13".into(),
            depth: 0,
            deleted: false,
        };
        assert_eq!(nc.id_short(), "abc");
    }

    #[test]
    fn comments_data_deserializes() {
        let json = r#"{
            "comments": [
                {
                    "id": "c1", "text": "hello", "level": 0,
                    "created_at": "2026-03-13", "updated_at": null,
                    "reply_to": null, "deleted": false,
                    "user": { "username": "tester" },
                    "replies": []
                }
            ]
        }"#;
        let data: CommentsData = serde_json::from_str(json).unwrap();
        assert_eq!(data.comments.len(), 1);
        assert_eq!(data.comments[0].text.as_deref(), Some("hello"));
    }

    #[test]
    fn write_comment_data_deserializes() {
        let json = r#"{ "writeComment": { "id": "c1", "text": "new comment", "user": { "username": "me" }, "created_at": null, "updated_at": null, "level": 0, "reply_to": null, "deleted": false, "replies": null } }"#;
        let data: WriteCommentData = serde_json::from_str(json).unwrap();
        assert_eq!(data.write_comment.text.as_deref(), Some("new comment"));
    }

    #[test]
    fn remove_comment_data_deserializes() {
        let json = r#"{ "removeComment": true }"#;
        let data: RemoveCommentData = serde_json::from_str(json).unwrap();
        assert!(data.remove_comment);
    }

    #[test]
    fn compact_comment_serializes() {
        let cc = CompactComment {
            number: "1.2".to_string(),
            id: "abc123".to_string(),
            text: "test".to_string(),
            author: "user".to_string(),
            date: "2026-03-13".to_string(),
            depth: 1,
            deleted: false,
        };
        let json = serde_json::to_string(&cc).unwrap();
        assert!(json.contains(r#""number":"1.2""#));
        assert!(json.contains(r#""depth":1"#));
    }
}
