use crate::types::RatingsResult;
use dashmap::DashMap;
use serde::Deserialize;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const OMDB_BASE: &str = "https://www.omdbapi.com/";
const TMDB_BASE: &str = "https://api.themoviedb.org/3";
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

struct CacheEntry {
    at: Instant,
    data: Option<RatingsResult>,
}

fn cache() -> &'static DashMap<String, CacheEntry> {
    static CACHE: OnceLock<DashMap<String, CacheEntry>> = OnceLock::new();
    CACHE.get_or_init(DashMap::new)
}

fn cache_key(title: &str, year: Option<&str>) -> String {
    format!(
        "{}|{}",
        title.trim().to_lowercase(),
        year.unwrap_or_default()
    )
}

#[derive(Deserialize)]
struct OmdbResponse {
    #[serde(rename = "Response")]
    response: Option<String>,
    #[serde(rename = "imdbRating")]
    imdb_rating: Option<String>,
    #[serde(rename = "Ratings")]
    ratings: Option<Vec<OmdbRating>>,
}

#[derive(Deserialize)]
struct OmdbRating {
    #[serde(rename = "Source")]
    source: Option<String>,
    #[serde(rename = "Value")]
    value: Option<String>,
}

fn extract_rotten_tomatoes(ratings: &[OmdbRating]) -> Option<String> {
    ratings.iter().find_map(|rating| {
        let source = rating.source.as_deref()?.to_lowercase();
        if !source.contains("rotten tomatoes") {
            return None;
        }
        let value = rating.value.as_deref()?.trim();
        (!value.is_empty() && value != "N/A").then(|| value.to_owned())
    })
}

async fn fetch_from_omdb(title: &str, year: Option<&str>) -> Option<RatingsResult> {
    let api_key = std::env::var("OMDb_API_KEY")
        .or_else(|_| std::env::var("OMDB_API_KEY"))
        .ok()
        .filter(|value| !value.is_empty())?;
    let mut request = reqwest::Client::new()
        .get(OMDB_BASE)
        .query(&[
            ("apikey", api_key.as_str()),
            ("t", title),
            ("type", "movie"),
            ("r", "json"),
        ])
        .timeout(Duration::from_secs(5));
    if let Some(year) = year {
        request = request.query(&[("y", year)]);
    }
    let response: OmdbResponse = request.send().await.ok()?.json().await.ok()?;
    if response.response.as_deref() == Some("False") {
        return None;
    }
    let imdb = response
        .imdb_rating
        .filter(|value| value != "N/A" && !value.trim().is_empty());
    let rotten_tomatoes = extract_rotten_tomatoes(response.ratings.as_deref().unwrap_or_default());
    (imdb.is_some() || rotten_tomatoes.is_some()).then_some(RatingsResult {
        imdb,
        rotten_tomatoes,
        tmdb: None,
    })
}

#[derive(Deserialize)]
struct TmdbSearch {
    results: Option<Vec<TmdbMovie>>,
}

#[derive(Deserialize)]
struct TmdbMovie {
    vote_average: Option<f64>,
}

async fn fetch_from_tmdb(title: &str, year: Option<&str>) -> Option<RatingsResult> {
    let api_key = std::env::var("TMDB_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())?;
    let mut request = reqwest::Client::new()
        .get(format!("{TMDB_BASE}/search/movie"))
        .query(&[
            ("api_key", api_key.as_str()),
            ("query", title),
            ("include_adult", "false"),
            ("language", "en-US"),
        ])
        .timeout(Duration::from_secs(5));
    if let Some(year) = year {
        request = request.query(&[("year", year)]);
    }
    let response: TmdbSearch = request.send().await.ok()?.json().await.ok()?;
    let vote = response
        .results?
        .into_iter()
        .next()?
        .vote_average
        .filter(|vote| *vote > 0.0)?;
    Some(RatingsResult {
        imdb: None,
        rotten_tomatoes: None,
        tmdb: Some(format!("{vote:.1}")),
    })
}

pub async fn get_movie_ratings(title: &str, year: Option<&str>) -> Option<RatingsResult> {
    let key = cache_key(title, year);
    if let Some(cached) = cache().get(&key) {
        if cached.at.elapsed() < CACHE_TTL {
            return cached.data.clone();
        }
    }
    let result = match fetch_from_omdb(title, year).await {
        result @ Some(_) => result,
        None => fetch_from_tmdb(title, year).await,
    };
    cache().insert(
        key,
        CacheEntry {
            at: Instant::now(),
            data: result.clone(),
        },
    );
    result
}

pub fn format_ratings_line(ratings: Option<&RatingsResult>) -> String {
    let Some(ratings) = ratings else {
        return String::new();
    };
    let mut parts = Vec::new();
    if let Some(imdb) = &ratings.imdb {
        parts.push(format!("⭐ IMDb: {imdb}/10"));
    }
    if let Some(rotten_tomatoes) = &ratings.rotten_tomatoes {
        parts.push(format!("🍅 RT: {rotten_tomatoes}"));
    }
    if let Some(tmdb) = &ratings.tmdb {
        parts.push(format!("⭐ TMDb: {tmdb}/10"));
    }
    if parts.is_empty() {
        return String::new();
    }
    format!("   📊 Avaliações: {}\n\n", parts.join(" | "))
}
