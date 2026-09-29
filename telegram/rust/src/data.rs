use crate::api::{fetch_normalized, fetch_upcoming};
use crate::normalize::denormalize;
use crate::store::NormalizedCache;
use crate::types::{maceio_date, DenormalizedMovie, Result, UpcomingItem};

pub fn get_date_string(days_offset: i64) -> String {
    maceio_date(days_offset)
}

pub async fn get_movies_for_date(
    cache: &mut NormalizedCache,
    date: Option<&str>,
    theater_id: &str,
) -> Result<(Vec<DenormalizedMovie>, String)> {
    let target_date = date
        .map(str::to_owned)
        .unwrap_or_else(|| get_date_string(0));
    if let Some(cached) = cache.get_sessions(&target_date, theater_id) {
        return Ok((denormalize(&cache.data.movies, &cached.items), target_date));
    }

    let normalized = fetch_normalized(date, theater_id).await?;
    cache.merge_movies(&normalized.movies);
    let date_key = normalized.date.clone().unwrap_or(target_date);
    cache.set_sessions(
        &date_key,
        normalized.sessions.clone(),
        normalized.fetched_at,
        theater_id,
    );
    cache.save_if_dirty().await?;
    let movies = denormalize(&cache.data.movies, &normalized.sessions);
    Ok((movies, date_key))
}

pub async fn get_upcoming_movies(
    cache: &mut NormalizedCache,
    theater_id: &str,
) -> Result<Vec<UpcomingItem>> {
    if let Some(cached) = cache.get_upcoming(theater_id) {
        return Ok(cached.items);
    }
    let (items, fetched_at) = fetch_upcoming(theater_id).await?;
    cache.set_upcoming(items.clone(), fetched_at, theater_id);
    cache.save_if_dirty().await?;
    Ok(items)
}
