use serde::{Deserialize, Serialize};

// ---- Tag Domain Models ----

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Tag {
    pub id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
    pub posts_count: Option<i32>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct UserTag {
    pub tag: Tag,
    pub posts_count: i32,
}

// ---- Response Wrappers ----

#[derive(Deserialize)]
pub struct TagsData {
    pub tags: Vec<Tag>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTagsData {
    pub user_tags: Vec<UserTag>,
}

// ---- Compact Output ----

#[derive(Serialize, Debug)]
pub struct CompactTag {
    pub name: String,
    pub posts_count: i32,
}

impl From<&Tag> for CompactTag {
    fn from(tag: &Tag) -> Self {
        CompactTag {
            name: tag.name.clone().unwrap_or_default(),
            posts_count: tag.posts_count.unwrap_or(0),
        }
    }
}

impl From<&UserTag> for CompactTag {
    fn from(ut: &UserTag) -> Self {
        CompactTag {
            name: ut.tag.name.clone().unwrap_or_default(),
            posts_count: ut.posts_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_data_deserializes() {
        let json = r#"{
            "tags": [
                { "id": "t1", "name": "rust", "posts_count": 42, "description": null, "thumbnail": null },
                { "id": "t2", "name": "cli", "posts_count": 10, "description": null, "thumbnail": null }
            ]
        }"#;
        let data: TagsData = serde_json::from_str(json).unwrap();
        assert_eq!(data.tags.len(), 2);
        assert_eq!(data.tags[0].name.as_deref(), Some("rust"));
        assert_eq!(data.tags[0].posts_count, Some(42));
    }

    #[test]
    fn user_tags_data_deserializes() {
        let json = r#"{
            "userTags": [
                { "tag": { "id": "t1", "name": "rust" }, "posts_count": 5 }
            ]
        }"#;
        let data: UserTagsData = serde_json::from_str(json).unwrap();
        assert_eq!(data.user_tags.len(), 1);
        assert_eq!(data.user_tags[0].posts_count, 5);
    }

    #[test]
    fn compact_tag_from_tag() {
        let tag = Tag {
            id: Some("t1".into()),
            name: Some("rust".into()),
            description: None,
            thumbnail: None,
            posts_count: Some(42),
        };
        let compact = CompactTag::from(&tag);
        assert_eq!(compact.name, "rust");
        assert_eq!(compact.posts_count, 42);
    }

    #[test]
    fn compact_tag_from_user_tag() {
        let ut = UserTag {
            tag: Tag {
                id: None,
                name: Some("cli".into()),
                description: None,
                thumbnail: None,
                posts_count: None,
            },
            posts_count: 7,
        };
        let compact = CompactTag::from(&ut);
        assert_eq!(compact.name, "cli");
        assert_eq!(compact.posts_count, 7);
    }

    #[test]
    fn compact_tag_serializes() {
        let tag = CompactTag {
            name: "rust".to_string(),
            posts_count: 42,
        };
        let json = serde_json::to_string(&tag).unwrap();
        assert!(json.contains(r#""name":"rust""#));
        assert!(json.contains(r#""posts_count":42"#));
    }
}
