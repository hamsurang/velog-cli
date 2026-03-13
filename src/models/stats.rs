use serde::{Deserialize, Serialize};

// ---- Stats Domain Models ----

#[derive(Deserialize, Debug)]
pub struct Stats {
    pub total: i32,
    pub count_by_day: Vec<DayCount>,
}

#[derive(Deserialize, Debug)]
pub struct DayCount {
    pub count: i32,
    pub day: String,
}

// ---- Response Wrappers ----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetStatsData {
    pub get_stats: Stats,
}

// ---- Compact Output ----

#[derive(Serialize, Debug)]
pub struct CompactStats {
    pub total: i32,
    pub today: i32,
    pub yesterday: i32,
    pub daily: Vec<CompactDayCount>,
}

#[derive(Serialize, Debug)]
pub struct CompactDayCount {
    pub date: String,
    pub views: i32,
}

impl CompactStats {
    pub fn from_stats(stats: &Stats, today: &str, yesterday: &str) -> Self {
        let today_count = stats
            .count_by_day
            .iter()
            .find(|d| d.day.starts_with(today))
            .map(|d| d.count)
            .unwrap_or(0);
        let yesterday_count = stats
            .count_by_day
            .iter()
            .find(|d| d.day.starts_with(yesterday))
            .map(|d| d.count)
            .unwrap_or(0);
        let daily = stats
            .count_by_day
            .iter()
            .map(|d| CompactDayCount {
                date: d.day.chars().take(10).collect(),
                views: d.count,
            })
            .collect();
        CompactStats {
            total: stats.total,
            today: today_count,
            yesterday: yesterday_count,
            daily,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_stats_data_deserializes() {
        let json = r#"{
            "getStats": {
                "total": 1234,
                "count_by_day": [
                    { "count": 10, "day": "2026-03-13T00:00:00.000Z" },
                    { "count": 8, "day": "2026-03-12T00:00:00.000Z" }
                ]
            }
        }"#;
        let data: GetStatsData = serde_json::from_str(json).unwrap();
        assert_eq!(data.get_stats.total, 1234);
        assert_eq!(data.get_stats.count_by_day.len(), 2);
        assert_eq!(data.get_stats.count_by_day[0].count, 10);
    }

    #[test]
    fn compact_stats_from_stats() {
        let stats = Stats {
            total: 100,
            count_by_day: vec![
                DayCount {
                    count: 10,
                    day: "2026-03-13T00:00:00.000Z".into(),
                },
                DayCount {
                    count: 8,
                    day: "2026-03-12T00:00:00.000Z".into(),
                },
                DayCount {
                    count: 5,
                    day: "2026-03-11T00:00:00.000Z".into(),
                },
            ],
        };
        let compact = CompactStats::from_stats(&stats, "2026-03-13", "2026-03-12");
        assert_eq!(compact.total, 100);
        assert_eq!(compact.today, 10);
        assert_eq!(compact.yesterday, 8);
        assert_eq!(compact.daily.len(), 3);
        assert_eq!(compact.daily[0].date, "2026-03-13");
    }

    #[test]
    fn compact_stats_missing_today() {
        let stats = Stats {
            total: 50,
            count_by_day: vec![DayCount {
                count: 3,
                day: "2026-03-11T00:00:00.000Z".into(),
            }],
        };
        let compact = CompactStats::from_stats(&stats, "2026-03-13", "2026-03-12");
        assert_eq!(compact.today, 0);
        assert_eq!(compact.yesterday, 0);
    }

    #[test]
    fn compact_stats_serializes() {
        let cs = CompactStats {
            total: 100,
            today: 10,
            yesterday: 8,
            daily: vec![CompactDayCount {
                date: "2026-03-13".into(),
                views: 10,
            }],
        };
        let json = serde_json::to_string(&cs).unwrap();
        assert!(json.contains(r#""total":100"#));
        assert!(json.contains(r#""today":10"#));
    }
}
