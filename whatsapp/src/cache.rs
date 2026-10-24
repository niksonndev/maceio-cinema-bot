use crate::types::{
    iso_to_maceio_date, maceio_date, now_rfc3339, CacheData, DenormalizedMovie, MovieStatic,
    Session, SessionDayCache, UpcomingCache, UpcomingItem,
};
use std::collections::HashMap;
use std::sync::Mutex;

pub struct NormalizedCache {
    data: Mutex<CacheData>,
}

impl Default for NormalizedCache {
    fn default() -> Self {
        Self::new()
    }
}

impl NormalizedCache {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(CacheData::default()),
        }
    }

    pub fn merge_movies(&self, movies: &HashMap<String, MovieStatic>) -> usize {
        let mut data = self.data.lock().expect("cache lock");
        let mut added = 0;
        for (id, movie) in movies {
            if !data.movies.contains_key(id) {
                data.movies.insert(id.clone(), movie.clone());
                added += 1;
            }
        }
        if added > 0 {
            data.movies_updated_at = Some(now_rfc3339());
            tracing::info!("{added} new movie(s) added to static cache");
        }
        added
    }

    pub fn set_sessions(
        &self,
        date: &str,
        sessions: Vec<Session>,
        fetched_at: String,
        theater_id: &str,
    ) {
        let mut data = self.data.lock().expect("cache lock");
        let theater = data.sessions.entry(theater_id.to_string()).or_default();
        let n = sessions.len();
        theater.insert(
            date.to_string(),
            SessionDayCache {
                fetched_at,
                items: sessions,
            },
        );
        purge_old_sessions(&mut data);
        tracing::info!("{n} session(s) saved for {date} (theater {theater_id})");
    }

    pub fn get_sessions(&self, date: &str, theater_id: &str) -> Option<SessionDayCache> {
        if !self.has_fresh_sessions(date, theater_id) {
            return None;
        }
        let data = self.data.lock().expect("cache lock");
        let cached = data.sessions.get(theater_id)?.get(date)?;
        tracing::info!("Cache hit: sessions for {date} (theater {theater_id})");
        Some(cached.clone())
    }

    pub fn has_fresh_sessions(&self, date: &str, theater_id: &str) -> bool {
        let mut data = self.data.lock().expect("cache lock");
        let Some(theater) = data.sessions.get_mut(theater_id) else {
            return false;
        };
        let Some(cached) = theater.get(date) else {
            return false;
        };
        if cached.fetched_at.is_empty() {
            return false;
        }
        let cached_day = iso_to_maceio_date(&cached.fetched_at);
        let today = maceio_date(0);
        if cached_day != today {
            tracing::info!("Session cache for {date} expired ({cached_day} → {today})");
            theater.remove(date);
            return false;
        }
        true
    }

    pub fn get_all_movies(&self) -> HashMap<String, MovieStatic> {
        self.data.lock().expect("cache lock").movies.clone()
    }

    /// Denormalizes sessions while holding the cache lock once, avoiding a
    /// clone of the entire movie map.
    pub fn denormalize(&self, sessions: &[Session]) -> Vec<DenormalizedMovie> {
        let data = self.data.lock().expect("cache lock");
        crate::normalize::denormalize(&data.movies, sessions)
    }

    pub fn set_upcoming(&self, items: Vec<UpcomingItem>, fetched_at: String, theater_id: &str) {
        let mut data = self.data.lock().expect("cache lock");
        let n = items.len();
        data.upcoming
            .insert(theater_id.to_string(), UpcomingCache { fetched_at, items });
        tracing::info!("{n} upcoming release(s) saved (theater {theater_id})");
    }

    pub fn get_upcoming(&self, theater_id: &str) -> Option<UpcomingCache> {
        if !self.has_fresh_upcoming(theater_id) {
            return None;
        }
        let data = self.data.lock().expect("cache lock");
        let cached = data.upcoming.get(theater_id)?;
        tracing::info!("Cache hit: upcoming releases (theater {theater_id})");
        Some(cached.clone())
    }

    pub fn has_fresh_upcoming(&self, theater_id: &str) -> bool {
        let mut data = self.data.lock().expect("cache lock");
        let Some(cached) = data.upcoming.get(theater_id) else {
            return false;
        };
        if cached.fetched_at.is_empty() {
            return false;
        }
        let cached_day = iso_to_maceio_date(&cached.fetched_at);
        let today = maceio_date(0);
        if cached_day != today {
            tracing::info!(
                "Upcoming cache expired for theater {theater_id} ({cached_day} → {today})"
            );
            data.upcoming.remove(theater_id);
            return false;
        }
        true
    }
}

fn purge_old_sessions(data: &mut CacheData) {
    let today = maceio_date(0);
    for theater in data.sessions.values_mut() {
        theater.retain(|date, _| date >= &today);
    }
}
