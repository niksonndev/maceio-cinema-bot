use crate::cache::NormalizedCache;
use crate::cinemas::DEFAULT_THEATER_ID;
use crate::ingresso::{fetch_normalized, fetch_upcoming};
use crate::normalize::denormalize;
use crate::types::{maceio_date, DenormalizedMovie, Result, UpcomingItem};

pub fn get_date_string(days_offset: i64) -> String {
    maceio_date(days_offset)
}

pub async fn get_movies_for_date(
    cache: &NormalizedCache,
    date: Option<&str>,
    theater_id: &str,
) -> Result<(Vec<DenormalizedMovie>, String, bool)> {
    let theater = if theater_id.is_empty() {
        DEFAULT_THEATER_ID
    } else {
        theater_id
    };
    let target_date = date
        .map(str::to_string)
        .unwrap_or_else(|| get_date_string(0));

    if let Some(cached) = cache.get_sessions(&target_date, theater) {
        let movies = denormalize(&cache.get_all_movies(), &cached.items);
        return Ok((movies, target_date, true));
    }

    let normalized = fetch_normalized(date, theater).await?;
    cache.merge_movies(&normalized.movies);
    let date_key = normalized.date.clone().unwrap_or(target_date.clone());
    cache.set_sessions(
        &date_key,
        normalized.sessions.clone(),
        normalized.fetched_at,
        theater,
    );
    let movies = denormalize(&normalized.movies, &normalized.sessions);
    Ok((movies, date_key, false))
}

pub async fn get_upcoming_movies(
    cache: &NormalizedCache,
    theater_id: &str,
) -> Result<(Vec<UpcomingItem>, bool)> {
    let theater = if theater_id.is_empty() {
        DEFAULT_THEATER_ID
    } else {
        theater_id
    };
    if let Some(cached) = cache.get_upcoming(theater) {
        return Ok((cached.items, true));
    }
    let (items, fetched_at) = fetch_upcoming(theater).await?;
    cache.set_upcoming(items.clone(), fetched_at, theater);
    Ok((items, false))
}
