use crate::types::{
    iso_to_maceio_date, maceio_date, now_rfc3339, CacheData, Error, MovieStatic, Result, Session,
    SessionDayCache, UpcomingCache, UpcomingItem,
};
use aws_sdk_s3::primitives::ByteStream;
use dashmap::DashMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Warm Lambda containers reuse the S3 payload for this long instead of paying
/// a GetObject round trip on every Telegram update.
const S3_LOAD_TTL: Duration = Duration::from_secs(60);

struct CachedBytes {
    at: Instant,
    bytes: Option<Vec<u8>>,
}

fn s3_load_cache() -> &'static DashMap<String, CachedBytes> {
    static CACHE: OnceLock<DashMap<String, CachedBytes>> = OnceLock::new();
    CACHE.get_or_init(DashMap::new)
}

async fn shared_s3_client() -> &'static aws_sdk_s3::Client {
    static CLIENT: tokio::sync::OnceCell<aws_sdk_s3::Client> = tokio::sync::OnceCell::const_new();

    CLIENT
        .get_or_init(|| async {
            let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            let mut s3_config = aws_sdk_s3::config::Builder::from(&config);
            if let Ok(endpoint) = std::env::var("AWS_ENDPOINT_URL") {
                if !endpoint.is_empty() {
                    s3_config = s3_config.endpoint_url(endpoint).force_path_style(true);
                }
            }
            aws_sdk_s3::Client::from_conf(s3_config.build())
        })
        .await
}

enum Backend {
    Local(PathBuf),
    S3 {
        client: aws_sdk_s3::Client,
        bucket: String,
        key: String,
    },
}

pub(crate) struct JsonStore {
    backend: Backend,
}

impl JsonStore {
    pub(crate) async fn from_env(key_env: &str, local_path: &str) -> Result<Self> {
        let backend = if let Some(bucket) = std::env::var("S3_BUCKET")
            .ok()
            .filter(|bucket| !bucket.is_empty())
        {
            let client = shared_s3_client().await.clone();
            Backend::S3 {
                client,
                bucket,
                key: std::env::var(key_env).unwrap_or_else(|_| {
                    key_env
                        .strip_suffix("_KEY")
                        .unwrap_or("cache")
                        .to_lowercase()
                        + ".json"
                }),
            }
        } else {
            Backend::Local(PathBuf::from(local_path))
        };
        Ok(Self { backend })
    }

    fn cache_key(&self) -> Option<String> {
        match &self.backend {
            Backend::Local(_) => None,
            Backend::S3 { bucket, key, .. } => Some(format!("{bucket}/{key}")),
        }
    }

    pub(crate) async fn load(&self) -> Result<Option<Vec<u8>>> {
        let cache_key = self.cache_key();
        if let Some(key) = &cache_key {
            if let Some(entry) = s3_load_cache().get(key) {
                if entry.at.elapsed() < S3_LOAD_TTL {
                    return Ok(entry.bytes.clone());
                }
            }
        }

        let bytes = self.load_uncached().await?;
        if let Some(key) = cache_key {
            s3_load_cache().insert(
                key,
                CachedBytes {
                    at: Instant::now(),
                    bytes: bytes.clone(),
                },
            );
        }
        Ok(bytes)
    }

    async fn load_uncached(&self) -> Result<Option<Vec<u8>>> {
        match &self.backend {
            Backend::Local(path) => match tokio::fs::read(path).await {
                Ok(data) => Ok(Some(data)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(Error::Msg(error.to_string())),
            },
            Backend::S3 {
                client,
                bucket,
                key,
            } => match client.get_object().bucket(bucket).key(key).send().await {
                Ok(response) => {
                    let data = response
                        .body
                        .collect()
                        .await
                        .map_err(|error| Error::Msg(error.to_string()))?
                        .into_bytes()
                        .to_vec();
                    Ok(Some(data))
                }
                Err(error)
                    if error
                        .as_service_error()
                        .is_some_and(|service_error| service_error.is_no_such_key()) =>
                {
                    Ok(None)
                }
                Err(error) => Err(Error::Msg(error.to_string())),
            },
        }
    }

    pub(crate) async fn save(&self, data: Vec<u8>) -> Result<()> {
        let cache_key = self.cache_key();
        self.save_uncached(data.clone()).await?;
        if let Some(key) = cache_key {
            s3_load_cache().insert(
                key,
                CachedBytes {
                    at: Instant::now(),
                    bytes: Some(data),
                },
            );
        }
        Ok(())
    }

    async fn save_uncached(&self, data: Vec<u8>) -> Result<()> {
        match &self.backend {
            Backend::Local(path) => {
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(|error| Error::Msg(error.to_string()))?;
                }
                tokio::fs::write(path, data)
                    .await
                    .map_err(|error| Error::Msg(error.to_string()))
            }
            Backend::S3 {
                client,
                bucket,
                key,
            } => {
                client
                    .put_object()
                    .bucket(bucket)
                    .key(key)
                    .body(ByteStream::from(data))
                    .send()
                    .await
                    .map_err(|error| Error::Msg(error.to_string()))?;
                Ok(())
            }
        }
    }
}

pub struct NormalizedCache {
    pub data: CacheData,
    store: JsonStore,
    dirty: bool,
}

impl NormalizedCache {
    pub async fn load() -> Result<Self> {
        let store = JsonStore::from_env("CACHE_KEY", "data/cache.json").await?;
        let data = match store.load().await? {
            Some(bytes) => match serde_json::from_slice(&bytes) {
                Ok(data) => data,
                Err(error) => {
                    tracing::warn!("Cache JSON is invalid; starting with empty cache: {error}");
                    CacheData::default()
                }
            },
            None => CacheData::default(),
        };
        Ok(Self {
            data,
            store,
            dirty: false,
        })
    }

    pub async fn save_if_dirty(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let data = serde_json::to_vec(&self.data).map_err(|error| Error::Msg(error.to_string()))?;
        self.store.save(data).await?;
        self.dirty = false;
        Ok(())
    }

    pub fn merge_movies(&mut self, movies: &HashMap<String, MovieStatic>) {
        let mut added = false;
        for (id, movie) in movies {
            if let std::collections::hash_map::Entry::Vacant(entry) =
                self.data.movies.entry(id.clone())
            {
                entry.insert(movie.clone());
                added = true;
            }
        }
        if added {
            self.data.movies_updated_at = Some(now_rfc3339());
            self.dirty = true;
        }
    }

    pub fn set_sessions(
        &mut self,
        date: &str,
        sessions: Vec<Session>,
        fetched_at: String,
        theater_id: &str,
    ) {
        self.data
            .sessions
            .entry(theater_id.to_owned())
            .or_default()
            .insert(
                date.to_owned(),
                SessionDayCache {
                    fetched_at,
                    items: sessions,
                },
            );
        self.dirty = true;
        let today = maceio_date(0);
        for dates in self.data.sessions.values_mut() {
            dates.retain(|date, _| date >= &today);
        }
    }

    pub fn get_sessions(&self, date: &str, theater_id: &str) -> Option<&SessionDayCache> {
        let cached = self.data.sessions.get(theater_id)?.get(date)?;
        let fetched_day = iso_to_maceio_date(&cached.fetched_at);
        let today = maceio_date(0);
        (date >= today.as_str() && fetched_day == today).then_some(cached)
    }

    pub fn set_upcoming(&mut self, items: Vec<UpcomingItem>, fetched_at: String, theater_id: &str) {
        self.data
            .upcoming
            .insert(theater_id.to_owned(), UpcomingCache { fetched_at, items });
        self.dirty = true;
    }

    pub fn get_upcoming(&self, theater_id: &str) -> Option<UpcomingCache> {
        let cached = self.data.upcoming.get(theater_id)?;
        (iso_to_maceio_date(&cached.fetched_at) == maceio_date(0)).then(|| cached.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::{Backend, JsonStore, NormalizedCache};
    use crate::types::{CacheData, Session, SessionDayCache};
    use std::path::PathBuf;

    #[test]
    fn keeps_future_day_sessions_valid_when_fetched_today() {
        let tomorrow = crate::types::maceio_date(1);
        let cache = NormalizedCache {
            data: CacheData {
                sessions: [(
                    "1162".to_string(),
                    [(
                        tomorrow.clone(),
                        SessionDayCache {
                            fetched_at: crate::types::now_rfc3339(),
                            items: vec![Session {
                                id: "session-1".into(),
                                movie_id: 42,
                                time: "20:00".into(),
                                ..Session::default()
                            }],
                        },
                    )]
                    .into_iter()
                    .collect(),
                )]
                .into_iter()
                .collect(),
                ..CacheData::default()
            },
            store: JsonStore {
                backend: Backend::Local(PathBuf::new()),
            },
            dirty: false,
        };

        let cached = cache.get_sessions(&tomorrow, "1162");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().items[0].movie_id, 42);
    }
}
