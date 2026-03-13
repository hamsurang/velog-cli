use serde::{Deserialize, Serialize};

use super::Post;

// ---- Series Domain Models ----

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Series {
    pub id: Option<String>,
    pub name: Option<String>,
    pub url_slug: Option<String>,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub series_posts: Option<Vec<SeriesPost>>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct SeriesPost {
    pub id: Option<String>,
    pub post: Option<Post>,
    pub index: Option<i32>,
}

// ---- Response Wrappers ----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesListData {
    pub series_list: Vec<Series>,
}

#[derive(Deserialize)]
pub struct SeriesData {
    pub series: Series,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSeriesData {
    pub create_series: Series,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditSeriesData {
    pub edit_series: Series,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveSeriesData {
    pub remove_series: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendToSeriesData {
    pub append_series_post: SeriesPost,
}

// ---- Compact Output ----

#[derive(Serialize, Debug)]
pub struct CompactSeries {
    pub name: String,
    pub slug: String,
    pub description: String,
    pub post_count: usize,
}

impl From<&Series> for CompactSeries {
    fn from(s: &Series) -> Self {
        CompactSeries {
            name: s.name.clone().unwrap_or_default(),
            slug: s.url_slug.clone().unwrap_or_default(),
            description: s.description.clone().unwrap_or_default(),
            post_count: s.series_posts.as_ref().map(|p| p.len()).unwrap_or(0),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct CompactSeriesDetail {
    pub name: String,
    pub slug: String,
    pub description: String,
    pub posts: Vec<CompactSeriesPostEntry>,
}

#[derive(Serialize, Debug)]
pub struct CompactSeriesPostEntry {
    pub index: i32,
    pub title: String,
    pub slug: String,
}

impl From<&Series> for CompactSeriesDetail {
    fn from(s: &Series) -> Self {
        let posts = s
            .series_posts
            .as_ref()
            .map(|sps| {
                sps.iter()
                    .map(|sp| {
                        let (title, slug) = sp
                            .post
                            .as_ref()
                            .map(|p| (p.title.clone(), p.url_slug.clone()))
                            .unwrap_or_default();
                        CompactSeriesPostEntry {
                            index: sp.index.unwrap_or(0),
                            title,
                            slug,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        CompactSeriesDetail {
            name: s.name.clone().unwrap_or_default(),
            slug: s.url_slug.clone().unwrap_or_default(),
            description: s.description.clone().unwrap_or_default(),
            posts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_list_data_deserializes() {
        let json = r#"{
            "seriesList": [
                { "id": "s1", "name": "Rust 입문", "url_slug": "rust-intro", "description": "Rust 입문 시리즈", "thumbnail": null, "created_at": null, "updated_at": null, "series_posts": [] }
            ]
        }"#;
        let data: SeriesListData = serde_json::from_str(json).unwrap();
        assert_eq!(data.series_list.len(), 1);
        assert_eq!(data.series_list[0].name.as_deref(), Some("Rust 입문"));
    }

    #[test]
    fn series_data_deserializes_with_posts() {
        let json = r#"{
            "series": {
                "id": "s1", "name": "Rust 입문", "url_slug": "rust-intro",
                "description": "desc", "thumbnail": null,
                "created_at": "2026-01-01", "updated_at": "2026-01-02",
                "series_posts": [
                    { "id": "sp1", "index": 1, "post": { "id": "p1", "title": "Ch1", "url_slug": "ch1", "short_description": "", "thumbnail": null, "likes": 0, "is_private": false, "is_temp": false, "released_at": null, "updated_at": null, "tags": [], "user": null } }
                ]
            }
        }"#;
        let data: SeriesData = serde_json::from_str(json).unwrap();
        assert_eq!(data.series.name.as_deref(), Some("Rust 입문"));
        let posts = data.series.series_posts.as_ref().unwrap();
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].index, Some(1));
        assert_eq!(posts[0].post.as_ref().unwrap().title, "Ch1");
    }

    #[test]
    fn create_series_data_deserializes() {
        let json = r#"{ "createSeries": { "id": "s1", "name": "New", "url_slug": "new", "description": null, "thumbnail": null, "created_at": null, "updated_at": null, "series_posts": null } }"#;
        let data: CreateSeriesData = serde_json::from_str(json).unwrap();
        assert_eq!(data.create_series.name.as_deref(), Some("New"));
    }

    #[test]
    fn remove_series_data_deserializes() {
        let json = r#"{ "removeSeries": true }"#;
        let data: RemoveSeriesData = serde_json::from_str(json).unwrap();
        assert!(data.remove_series);
    }

    #[test]
    fn compact_series_from_series() {
        let s = Series {
            id: Some("s1".into()),
            name: Some("Rust 입문".into()),
            url_slug: Some("rust-intro".into()),
            description: Some("desc".into()),
            thumbnail: None,
            created_at: None,
            updated_at: None,
            series_posts: Some(vec![
                SeriesPost {
                    id: Some("sp1".into()),
                    post: None,
                    index: Some(1),
                },
                SeriesPost {
                    id: Some("sp2".into()),
                    post: None,
                    index: Some(2),
                },
            ]),
        };
        let compact = CompactSeries::from(&s);
        assert_eq!(compact.name, "Rust 입문");
        assert_eq!(compact.slug, "rust-intro");
        assert_eq!(compact.post_count, 2);
    }

    #[test]
    fn compact_series_detail_from_series() {
        let s = Series {
            id: Some("s1".into()),
            name: Some("Rust 입문".into()),
            url_slug: Some("rust-intro".into()),
            description: Some("desc".into()),
            thumbnail: None,
            created_at: None,
            updated_at: None,
            series_posts: Some(vec![SeriesPost {
                id: Some("sp1".into()),
                post: Some(Post {
                    id: "p1".into(),
                    title: "Ch1".into(),
                    url_slug: "ch1".into(),
                    short_description: None,
                    body: None,
                    thumbnail: None,
                    likes: 0,
                    is_private: false,
                    is_temp: false,
                    released_at: None,
                    updated_at: None,
                    tags: Some(vec![]),
                    meta: None,
                    user: None,
                }),
                index: Some(1),
            }]),
        };
        let detail = CompactSeriesDetail::from(&s);
        assert_eq!(detail.name, "Rust 입문");
        assert_eq!(detail.posts.len(), 1);
        assert_eq!(detail.posts[0].title, "Ch1");
        assert_eq!(detail.posts[0].index, 1);
    }

    #[test]
    fn compact_series_serializes() {
        let cs = CompactSeries {
            name: "test".to_string(),
            slug: "test-slug".to_string(),
            description: "desc".to_string(),
            post_count: 3,
        };
        let json = serde_json::to_string(&cs).unwrap();
        assert!(json.contains(r#""name":"test""#));
        assert!(json.contains(r#""post_count":3"#));
    }
}
