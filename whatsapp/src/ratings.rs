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

fn memory() -> &'static DashMap<String, CacheEntry> {
    static CACHE: OnceLock<DashMap<String, CacheEntry>> = OnceLock::new();
    CACHE.get_or_init(DashMap::new)
}

fn cache_key(title: &str, year: Option<&str>) -> String {
    format!("{}|{}", title.trim().to_lowercase(), year.unwrap_or(""))
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

fn extract_rt(ratings: &[OmdbRating]) -> Option<String> {
    ratings.iter().find_map(|r| {
        let src = r.source.as_deref()?.to_lowercase();
        if src.contains("rotten tomatoes") {
            let v = r.value.as_deref()?.trim();
            if v.is_empty() || v == "N/A" {
                None
            } else {
                Some(v.to_string())
            }
        } else {
            None
        }
    })
}

async fn fetch_from_omdb(title: &str, year: Option<&str>) -> Option<RatingsResult> {
    let api_key = std::env::var("OMDb_API_KEY")
        .or_else(|_| std::env::var("OMDB_API_KEY"))
        .ok()
        .filter(|s| !s.is_empty())?;
    let mut req = reqwest::Client::new()
        .get(OMDB_BASE)
        .query(&[
            ("apikey", api_key.as_str()),
            ("t", title),
            ("type", "movie"),
            ("r", "json"),
        ])
        .timeout(Duration::from_secs(5));
    if let Some(y) = year {
        req = req.query(&[("y", y)]);
    }
    let data: OmdbResponse = req.send().await.ok()?.json().await.ok()?;
    if data.response.as_deref() == Some("False") {
        return None;
    }
    let imdb = data
        .imdb_rating
        .filter(|s| s != "N/A" && !s.trim().is_empty());
    let rt = extract_rt(data.ratings.as_deref().unwrap_or(&[]));
    if imdb.is_none() && rt.is_none() {
        return None;
    }
    Some(RatingsResult {
        imdb,
        rotten_tomatoes: rt,
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
        .filter(|s| !s.is_empty())?;
    let mut req = reqwest::Client::new()
        .get(format!("{TMDB_BASE}/search/movie"))
        .query(&[
            ("api_key", api_key.as_str()),
            ("query", title),
            ("include_adult", "false"),
            ("language", "en-US"),
        ])
        .timeout(Duration::from_secs(5));
    if let Some(y) = year {
        req = req.query(&[("year", y)]);
    }
    let data: TmdbSearch = req.send().await.ok()?.json().await.ok()?;
    let vote = data
        .results?
        .into_iter()
        .next()?
        .vote_average
        .filter(|v| *v > 0.0)?;
    Some(RatingsResult {
        imdb: None,
        rotten_tomatoes: None,
        tmdb: Some(format!("{vote:.1}")),
    })
}

pub async fn get_movie_ratings(title: &str, year: Option<&str>) -> Option<RatingsResult> {
    let key = cache_key(title, year);
    if let Some(cached) = memory().get(&key) {
        if cached.at.elapsed() < CACHE_TTL {
            return cached.data.clone();
        }
    }

    let mut result = match fetch_from_omdb(title, year).await {
        some @ Some(_) => some,
        None => fetch_from_tmdb(title, year).await,
    };
    if result
        .as_ref()
        .is_some_and(|r| r.imdb.is_none() && r.rotten_tomatoes.is_none() && r.tmdb.is_none())
    {
        result = None;
    }
    memory().insert(
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
    if let Some(rt) = &ratings.rotten_tomatoes {
        parts.push(format!("🍅 RT: {rt}"));
    }
    if let Some(tmdb) = &ratings.tmdb {
        parts.push(format!("⭐ TMDb: {tmdb}/10"));
    }
    if parts.is_empty() {
        return String::new();
    }
    format!("   📊 Avaliações: {}\n\n", parts.join(" | "))
}
