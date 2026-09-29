use crate::normalize::{normalize_sessions_response, normalize_upcoming_from_sessions};
use crate::types::{
    maceio_date, now_rfc3339, Error, IngressoDateEntry, NormalizedSessions, Result, UpcomingItem,
};
use std::collections::HashSet;
use std::sync::OnceLock;

const BASE_URL: &str = "https://api-content.ingresso.com";
const CITY_ID: u32 = 53;
const DEFAULT_THEATER_ID: &str = "1162";

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json, text/plain, */*"),
        );
        headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            reqwest::header::HeaderValue::from_static("pt-BR,pt;q=0.9,en;q=0.8"),
        );
        reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("reqwest client")
    })
}

fn parse_date_entries(value: serde_json::Value) -> Vec<IngressoDateEntry> {
    match value {
        serde_json::Value::Array(entries) => entries
            .into_iter()
            .filter_map(|entry| serde_json::from_value(entry).ok())
            .collect(),
        other => serde_json::from_value(other).ok().into_iter().collect(),
    }
}

pub async fn fetch_normalized(date: Option<&str>, theater_id: &str) -> Result<NormalizedSessions> {
    let target_date = date.map(str::to_owned).unwrap_or_else(|| maceio_date(0));
    let theater = if theater_id.is_empty() {
        DEFAULT_THEATER_ID
    } else {
        theater_id
    };
    let url = format!(
        "{BASE_URL}/v0/sessions/city/{CITY_ID}/theater/{theater}/partnership/home/groupBy/sessionType"
    );
    let value: serde_json::Value = client()
        .get(url)
        .query(&[("date", target_date.as_str())])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let entries = parse_date_entries(value);
    let mut normalized = normalize_sessions_response(entries.first());
    normalized.date = Some(target_date);
    Ok(normalized)
}

pub async fn fetch_upcoming(theater_id: &str) -> Result<(Vec<UpcomingItem>, String)> {
    let theater = if theater_id.is_empty() {
        DEFAULT_THEATER_ID
    } else {
        theater_id
    };
    let url = format!("{BASE_URL}/v0/sessions/city/{CITY_ID}/theater/{theater}");
    let value: serde_json::Value = client()
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let all_dates = parse_date_entries(value);
    let today = maceio_date(0);
    let mut today_movie_ids = HashSet::new();
    for entry in all_dates.iter().filter(|entry| entry.date <= today) {
        for movie in entry.movies.as_deref().unwrap_or_default() {
            today_movie_ids.insert(movie.id);
        }
    }
    let future: Vec<_> = all_dates
        .into_iter()
        .filter(|entry| entry.date > today)
        .collect();
    let upcoming = normalize_upcoming_from_sessions(&future, &today_movie_ids)
        .into_iter()
        .filter(|item| item.in_pre_sale)
        .collect();
    Ok((upcoming, now_rfc3339()))
}

pub fn api_error(error: impl std::fmt::Display) -> Error {
    Error::Msg(error.to_string())
}
