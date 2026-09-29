use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("telegram: {0}")]
    Telegram(#[from] teloxide::RequestError),
    #[error("{0}")]
    Msg(String),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MovieStatic {
    pub id: i64,
    pub title: String,
    pub original_title: Option<String>,
    pub url_key: String,
    pub duration: Option<i64>,
    pub content_rating: Option<String>,
    pub rating_color: Option<String>,
    pub genres: Vec<String>,
    pub distributor: Option<String>,
    pub poster: Option<String>,
    pub backdrop: Option<String>,
    pub trailer: Option<String>,
    pub tags: Vec<String>,
    pub is_reexhibition: bool,
    pub in_pre_sale: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub movie_id: i64,
    pub time: String,
    pub price: Option<f64>,
    pub room: Option<String>,
    pub format: String,
    pub audio: Option<String>,
    pub checkout_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DenormalizedSession {
    pub time: String,
    pub session_id: String,
    pub price_inteira: Option<f64>,
    pub price_meia: Option<f64>,
    pub gratuito: bool,
    pub room: Option<String>,
    pub format: String,
    pub audio: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DenormalizedMovie {
    #[serde(flatten)]
    pub movie: MovieStatic,
    pub name: String,
    pub sessions: Vec<DenormalizedSession>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UpcomingItem {
    pub id: i64,
    pub title: String,
    pub original_title: Option<String>,
    pub content_rating: Option<String>,
    pub genres: Vec<String>,
    pub poster: Option<String>,
    pub in_pre_sale: bool,
    pub formats: Vec<String>,
    pub price_from: Option<f64>,
    pub first_date: String,
    pub first_date_formatted: String,
    pub first_date_day_of_week: String,
    #[serde(rename = "siteURL")]
    pub site_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDayCache {
    pub fetched_at: String,
    pub items: Vec<Session>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpcomingCache {
    pub fetched_at: String,
    pub items: Vec<UpcomingItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheData {
    #[serde(default)]
    pub movies: HashMap<String, MovieStatic>,
    #[serde(default)]
    pub sessions: HashMap<String, HashMap<String, SessionDayCache>>,
    #[serde(default)]
    pub upcoming: HashMap<String, UpcomingCache>,
    #[serde(default)]
    pub movies_updated_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NormalizedSessions {
    pub movies: HashMap<String, MovieStatic>,
    pub sessions: Vec<Session>,
    pub date: Option<String>,
    pub fetched_at: String,
}

#[derive(Debug, Clone)]
pub struct Cinema {
    pub id: &'static str,
    pub name: &'static str,
    pub label: &'static str,
    pub url: &'static str,
}

#[derive(Debug, Clone, Default)]
pub struct RatingsResult {
    pub imdb: Option<String>,
    pub rotten_tomatoes: Option<String>,
    pub tmdb: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngressoImage {
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum IngressoTag {
    Name(String),
    Obj { name: String },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngressoSessionType {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub alias: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngressoRawSession {
    pub id: String,
    pub time: String,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub room: Option<String>,
    #[serde(default)]
    pub types: Option<Vec<IngressoSessionType>>,
    #[serde(default, rename = "siteURL")]
    pub site_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngressoSessionGroup {
    #[serde(default)]
    pub sessions: Option<Vec<IngressoRawSession>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngressoRawMovie {
    pub id: i64,
    pub title: String,
    #[serde(default)]
    pub original_title: Option<String>,
    #[serde(default)]
    pub url_key: Option<String>,
    #[serde(default)]
    pub duration: Option<serde_json::Value>,
    #[serde(default)]
    pub content_rating: Option<String>,
    #[serde(default)]
    pub rating_details: Option<RatingDetails>,
    #[serde(default)]
    pub genres: Option<Vec<String>>,
    #[serde(default)]
    pub distributor: Option<String>,
    #[serde(default)]
    pub images: Option<Vec<IngressoImage>>,
    #[serde(default)]
    pub trailers: Option<Vec<IngressoTrailer>>,
    #[serde(default)]
    pub complete_tags: Option<Vec<IngressoTag>>,
    #[serde(default)]
    pub tags: Option<Vec<IngressoTag>>,
    #[serde(default)]
    pub is_reexhibition: Option<bool>,
    #[serde(default)]
    pub in_pre_sale: Option<bool>,
    #[serde(default)]
    pub session_types: Option<Vec<IngressoSessionGroup>>,
    #[serde(default)]
    pub rooms: Option<Vec<IngressoSessionGroup>>,
    #[serde(default, rename = "siteURLByTheater")]
    pub site_url_by_theater: Option<String>,
    #[serde(default, rename = "siteURL")]
    pub site_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RatingDetails {
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IngressoTrailer {
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngressoDateEntry {
    pub date: String,
    #[serde(default)]
    pub date_formatted: Option<String>,
    #[serde(default)]
    pub day_of_week: Option<String>,
    #[serde(default)]
    pub movies: Option<Vec<IngressoRawMovie>>,
}

pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn maceio_date(days_offset: i64) -> String {
    use chrono::{Duration, Utc};
    use chrono_tz::America::Maceio;
    (Utc::now().with_timezone(&Maceio).date_naive() + Duration::days(days_offset))
        .format("%Y-%m-%d")
        .to_string()
}

pub fn iso_to_maceio_date(iso: &str) -> String {
    use chrono::{DateTime, Utc};
    use chrono_tz::America::Maceio;
    DateTime::parse_from_rfc3339(iso)
        .map(|dt| dt.with_timezone(&Maceio).format("%Y-%m-%d").to_string())
        .or_else(|_| {
            iso.parse::<DateTime<Utc>>()
                .map(|dt| dt.with_timezone(&Maceio).format("%Y-%m-%d").to_string())
        })
        .unwrap_or_else(|_| iso.chars().take(10).collect())
}

#[cfg(test)]
mod tests {
    use super::{CacheData, UpcomingItem};

    #[test]
    fn cache_round_trip_preserves_legacy_site_url_key() {
        let cache: CacheData = serde_json::from_str(
            r#"{"upcoming":{"1162":{"fetchedAt":"2026-09-29T00:00:00Z","items":[{"id":1,"title":"Test","originalTitle":null,"contentRating":null,"genres":[],"poster":null,"inPreSale":true,"formats":[],"priceFrom":null,"firstDate":"2026-10-01","firstDateFormatted":"1 de outubro de 2026","firstDateDayOfWeek":"quinta-feira","siteURL":"https://example.com"}]}}}"#,
        )
        .unwrap();
        let serialized = serde_json::to_value(cache).unwrap();

        assert_eq!(
            serialized["upcoming"]["1162"]["items"][0]["siteURL"],
            "https://example.com"
        );
        assert!(serialized["upcoming"]["1162"]["items"][0]
            .get("siteUrl")
            .is_none());
    }

    #[test]
    fn cache_defaults_missing_top_level_maps() {
        let cache: CacheData = serde_json::from_str(r#"{"movies":{}}"#).unwrap();
        assert!(cache.sessions.is_empty());
        assert!(cache.upcoming.is_empty());
    }

    #[test]
    fn upcoming_site_url_uses_legacy_field_name() {
        let item: UpcomingItem = serde_json::from_str(
            r#"{"id":1,"title":"Test","firstDate":"2026-10-01","siteURL":"https://example.com"}"#,
        )
        .unwrap();
        assert_eq!(item.site_url.as_deref(), Some("https://example.com"));
    }
}
