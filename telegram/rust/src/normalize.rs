use crate::types::{
    now_rfc3339, DenormalizedMovie, DenormalizedSession, IngressoDateEntry, IngressoRawMovie,
    IngressoSessionGroup, IngressoTag, MovieStatic, NormalizedSessions, Session, UpcomingItem,
};
use std::collections::{HashMap, HashSet};

fn tag_name(tag: &IngressoTag) -> String {
    match tag {
        IngressoTag::Name(value) => value.clone(),
        IngressoTag::Obj { name } => name.clone(),
    }
}

fn duration_num(value: &Option<serde_json::Value>) -> Option<i64> {
    match value {
        Some(serde_json::Value::Number(number)) => number
            .as_i64()
            .or_else(|| number.as_f64().map(|value| value as i64)),
        Some(serde_json::Value::String(value)) => value.parse().ok(),
        _ => None,
    }
}

pub fn extract_movie_static(raw: &IngressoRawMovie) -> MovieStatic {
    let poster = raw.images.as_ref().and_then(|images| {
        images
            .iter()
            .find(|image| image.r#type.as_deref() == Some("PosterPortrait"))
            .and_then(|image| image.url.clone())
    });
    let backdrop = raw.images.as_ref().and_then(|images| {
        images
            .iter()
            .find(|image| image.r#type.as_deref() == Some("PosterHorizontal"))
            .and_then(|image| image.url.clone())
    });
    let trailer = raw
        .trailers
        .as_ref()
        .and_then(|trailers| trailers.first())
        .and_then(|trailer| trailer.url.clone());
    let tags = raw
        .complete_tags
        .as_ref()
        .or(raw.tags.as_ref())
        .map(|tags| tags.iter().map(tag_name).collect())
        .unwrap_or_default();

    MovieStatic {
        id: raw.id,
        title: raw.title.clone(),
        original_title: raw.original_title.clone().filter(|value| !value.is_empty()),
        url_key: raw.url_key.clone().unwrap_or_default(),
        duration: duration_num(&raw.duration).filter(|duration| *duration != 0),
        content_rating: raw.content_rating.clone().filter(|value| !value.is_empty()),
        rating_color: raw
            .rating_details
            .as_ref()
            .and_then(|rating| rating.color.clone()),
        genres: raw.genres.clone().unwrap_or_default(),
        distributor: raw.distributor.clone().filter(|value| !value.is_empty()),
        poster,
        backdrop,
        trailer,
        tags,
        is_reexhibition: raw.is_reexhibition.unwrap_or(false),
        in_pre_sale: raw.in_pre_sale.unwrap_or(false),
    }
}

pub fn extract_sessions(
    movie_id: i64,
    session_types: Option<&[IngressoSessionGroup]>,
) -> Vec<Session> {
    let Some(groups) = session_types else {
        return Vec::new();
    };
    groups
        .iter()
        .flat_map(|group| group.sessions.as_deref().unwrap_or_default())
        .map(|session| {
            let types = session.types.as_deref().unwrap_or_default();
            let format = types
                .iter()
                .find(|kind| {
                    kind.name.as_deref() != Some("Dublado")
                        && kind.name.as_deref() != Some("Legendado")
                })
                .and_then(|kind| kind.alias.clone())
                .unwrap_or_else(|| "2D".into());
            let audio = types
                .iter()
                .find(|kind| {
                    kind.name.as_deref() == Some("Dublado")
                        || kind.name.as_deref() == Some("Legendado")
                })
                .and_then(|kind| kind.alias.clone());
            Session {
                id: session.id.clone(),
                movie_id,
                time: session.time.clone(),
                price: session.price,
                room: session.room.clone().filter(|value| !value.is_empty()),
                format,
                audio,
                checkout_url: session.site_url.clone().filter(|value| !value.is_empty()),
            }
        })
        .collect()
}

fn canonical_movie_key(raw: &IngressoRawMovie) -> String {
    if let Some(url_key) = raw
        .url_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return format!("url_key:{url_key}");
    }

    let title = raw.title.trim();
    let original_title = raw
        .original_title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_default();
    let duration = raw
        .duration
        .as_ref()
        .and_then(|value| match value {
            serde_json::Value::Number(number) => number
                .as_i64()
                .or_else(|| number.as_f64().map(|value| value as i64)),
            serde_json::Value::String(value) => value.parse().ok(),
            _ => None,
        })
        .map(|value| value.to_string())
        .unwrap_or_default();
    let genres = raw
        .genres
        .as_deref()
        .map(|genres| genres.join("|"))
        .unwrap_or_default();
    let edition = [
        raw.content_rating.as_deref().unwrap_or_default(),
        raw.distributor.as_deref().unwrap_or_default(),
        if raw.is_reexhibition.unwrap_or(false) {
            "reexhibition"
        } else {
            ""
        },
        if raw.in_pre_sale.unwrap_or(false) {
            "pre_sale"
        } else {
            ""
        },
        &duration,
        &genres,
    ]
    .join("|");

    format!("title:{title}|original:{original_title}|edition:{edition}")
}

pub fn normalize_sessions_response(entry: Option<&IngressoDateEntry>) -> NormalizedSessions {
    let Some(entry) = entry else {
        return empty_normalized();
    };
    let Some(raw_movies) = entry.movies.as_ref() else {
        return empty_normalized();
    };

    let mut movies = HashMap::new();
    let mut sessions = Vec::new();
    let mut primary_movie_ids = HashMap::new();

    for raw in raw_movies {
        let key = canonical_movie_key(raw);
        let movie_id = *primary_movie_ids.entry(key).or_insert(raw.id);

        movies
            .entry(movie_id.to_string())
            .or_insert_with(|| extract_movie_static(raw));

        sessions.extend(extract_sessions(movie_id, raw.session_types.as_deref()));
    }

    NormalizedSessions {
        movies,
        sessions,
        date: (!entry.date.is_empty()).then(|| entry.date.clone()),
        fetched_at: now_rfc3339(),
    }
}

fn empty_normalized() -> NormalizedSessions {
    NormalizedSessions {
        movies: HashMap::new(),
        sessions: Vec::new(),
        date: None,
        fetched_at: now_rfc3339(),
    }
}

pub fn normalize_upcoming_from_sessions(
    future_dates: &[IngressoDateEntry],
    today_movie_ids: &HashSet<i64>,
) -> Vec<UpcomingItem> {
    let mut seen: HashMap<String, UpcomingItem> = HashMap::new();

    for date_entry in future_dates {
        for raw in date_entry.movies.as_deref().unwrap_or_default() {
            if today_movie_ids.contains(&raw.id) {
                continue;
            }

            let key = canonical_movie_key(raw);
            if let Some(existing) = seen.get_mut(&key) {
                if let Some(price) = raw
                    .session_types
                    .as_deref()
                    .or(raw.rooms.as_deref())
                    .unwrap_or_default()
                    .iter()
                    .flat_map(|group| group.sessions.as_deref().unwrap_or_default())
                    .filter_map(|session| session.price)
                    .min_by(|left, right| left.total_cmp(right))
                {
                    existing.price_from = Some(
                        existing
                            .price_from
                            .map_or(price, |current| current.min(price)),
                    );
                }
                continue;
            }

            let poster = raw.images.as_ref().and_then(|images| {
                images
                    .iter()
                    .find(|image| image.r#type.as_deref() == Some("PosterPortrait"))
                    .and_then(|image| image.url.clone())
            });
            let mut formats = HashSet::new();
            let mut min_price: Option<f64> = None;
            let groups = raw
                .session_types
                .as_deref()
                .or(raw.rooms.as_deref())
                .unwrap_or_default();
            for session in groups
                .iter()
                .flat_map(|group| group.sessions.as_deref().unwrap_or_default())
            {
                for kind in session.types.as_deref().unwrap_or_default() {
                    if kind.name.as_deref() != Some("Dublado")
                        && kind.name.as_deref() != Some("Legendado")
                    {
                        if let Some(alias) = &kind.alias {
                            formats.insert(alias.clone());
                        }
                    }
                }
                if let Some(price) = session.price {
                    min_price = Some(min_price.map_or(price, |current| current.min(price)));
                }
            }
            seen.insert(
                key,
                UpcomingItem {
                    id: raw.id,
                    title: raw.title.clone(),
                    original_title: raw.original_title.clone().filter(|value| !value.is_empty()),
                    content_rating: raw.content_rating.clone().filter(|value| !value.is_empty()),
                    genres: raw.genres.clone().unwrap_or_default(),
                    poster,
                    in_pre_sale: raw.in_pre_sale.unwrap_or(false),
                    formats: formats.into_iter().collect(),
                    price_from: min_price,
                    first_date: date_entry.date.clone(),
                    first_date_formatted: date_entry
                        .date_formatted
                        .clone()
                        .unwrap_or_else(|| date_entry.date.clone()),
                    first_date_day_of_week: date_entry.day_of_week.clone().unwrap_or_default(),
                    site_url: raw
                        .site_url_by_theater
                        .clone()
                        .or(raw.site_url.clone())
                        .filter(|value| !value.is_empty()),
                },
            );
        }
    }

    seen.into_values().collect()
}

pub fn denormalize(
    movies: &HashMap<String, MovieStatic>,
    sessions: &[Session],
) -> Vec<DenormalizedMovie> {
    let mut grouped: HashMap<i64, DenormalizedMovie> = HashMap::new();
    for session in sessions {
        if let std::collections::hash_map::Entry::Vacant(entry) = grouped.entry(session.movie_id) {
            let Some(movie) = movies.get(&session.movie_id.to_string()) else {
                continue;
            };
            entry.insert(DenormalizedMovie {
                name: movie.title.clone(),
                movie: movie.clone(),
                sessions: Vec::new(),
            });
        }
        if let Some(movie) = grouped.get_mut(&session.movie_id) {
            movie.sessions.push(DenormalizedSession {
                time: session.time.clone(),
                session_id: session.id.clone(),
                price_inteira: session.price,
                price_meia: session
                    .price
                    .map(|price| (price / 2.0 * 100.0).round() / 100.0),
                gratuito: session.price.is_none() || session.price == Some(0.0),
                room: session.room.clone(),
                format: session.format.clone(),
                audio: session.audio.clone(),
            });
        }
    }
    grouped.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::{denormalize, extract_sessions};
    use crate::types::{
        IngressoDateEntry, IngressoRawSession, IngressoSessionGroup, IngressoSessionType,
        MovieStatic,
    };
    use std::collections::{HashMap, HashSet};

    #[test]
    fn normalizes_api_movies_with_string_ids() {
        let entry: IngressoDateEntry = serde_json::from_value(serde_json::json!({
            "date": "2026-09-29",
            "movies": [{
                "id": "28844",
                "title": "Precisamos Falar",
                "sessionTypes": [{
                    "sessions": [{
                        "id": "87112724",
                        "time": "20:00",
                        "price": 27.36
                    }]
                }]
            }]
        }))
        .unwrap();

        let normalized = super::normalize_sessions_response(Some(&entry));

        assert!(normalized.movies.contains_key("28844"));
        assert_eq!(normalized.sessions.len(), 1);
        assert_eq!(normalized.sessions[0].movie_id, 28844);
    }

    #[test]
    fn extracts_format_audio_and_checkout_url() {
        let groups = [IngressoSessionGroup {
            sessions: Some(vec![IngressoRawSession {
                id: "session-1".into(),
                time: "20:00".into(),
                price: Some(40.0),
                room: Some("Sala 1".into()),
                types: Some(vec![
                    IngressoSessionType {
                        name: Some("Normal".into()),
                        alias: Some("2D".into()),
                    },
                    IngressoSessionType {
                        name: Some("Dublado".into()),
                        alias: Some("DUB".into()),
                    },
                ]),
                site_url: Some("https://example.com".into()),
            }]),
        }];
        let sessions = extract_sessions(1, Some(&groups));

        assert_eq!(sessions[0].format, "2D");
        assert_eq!(sessions[0].audio.as_deref(), Some("DUB"));
        assert_eq!(
            sessions[0].checkout_url.as_deref(),
            Some("https://example.com")
        );
    }

    #[test]
    fn deduplicates_same_movie_across_multiple_provider_ids() {
        let entry: IngressoDateEntry = serde_json::from_value(serde_json::json!({
            "date": "2026-09-29",
            "movies": [
                {
                    "id": "101",
                    "title": "Vingadores: Ultimato",
                    "urlKey": "vingadores-ultimato",
                    "sessionTypes": [{
                        "sessions": [{
                            "id": "session-a",
                            "time": "18:30",
                            "price": 29.9
                        }]
                    }]
                },
                {
                    "id": "202",
                    "title": "Vingadores: Ultimato",
                    "urlKey": "vingadores-ultimato",
                    "sessionTypes": [{
                        "sessions": [{
                            "id": "session-b",
                            "time": "21:00",
                            "price": 39.9
                        }]
                    }]
                }
            ]
        }))
        .unwrap();

        let normalized = super::normalize_sessions_response(Some(&entry));

        assert_eq!(normalized.movies.len(), 1);
        assert_eq!(normalized.sessions.len(), 2);
        assert!(normalized
            .sessions
            .iter()
            .all(|session| session.movie_id == 101 || session.movie_id == 202));
    }

    #[test]
    fn deduplicates_upcoming_movies_by_canonical_identity() {
        let future = vec![
            IngressoDateEntry {
                date: "2026-10-01".into(),
                date_formatted: None,
                day_of_week: None,
                movies: Some(vec![serde_json::from_value(serde_json::json!({
                    "id": "101",
                    "title": "Vingadores: Ultimato",
                    "urlKey": "vingadores-ultimato",
                    "sessionTypes": [{
                        "sessions": [{
                            "id": "session-a",
                            "time": "18:30",
                            "price": 29.9
                        }]
                    }]
                }))
                .unwrap()]),
            },
            IngressoDateEntry {
                date: "2026-10-02".into(),
                date_formatted: None,
                day_of_week: None,
                movies: Some(vec![serde_json::from_value(serde_json::json!({
                    "id": "202",
                    "title": "Vingadores: Ultimato",
                    "urlKey": "vingadores-ultimato",
                    "sessionTypes": [{
                        "sessions": [{
                            "id": "session-b",
                            "time": "21:00",
                            "price": 39.9
                        }]
                    }]
                }))
                .unwrap()]),
            },
        ];

        let upcoming = super::normalize_upcoming_from_sessions(&future, &HashSet::new());

        assert_eq!(upcoming.len(), 1);
        assert_eq!(upcoming[0].title, "Vingadores: Ultimato");
    }

    #[test]
    fn denormalizes_ticket_prices() {
        let movies = HashMap::from([(
            "1".into(),
            MovieStatic {
                id: 1,
                title: "Test".into(),
                ..MovieStatic::default()
            },
        )]);
        let sessions = [crate::types::Session {
            id: "s1".into(),
            movie_id: 1,
            price: Some(55.86),
            ..crate::types::Session::default()
        }];
        let movies = denormalize(&movies, &sessions);

        assert_eq!(movies[0].sessions[0].price_meia, Some(27.93));
    }
}
