use crate::types::{
    now_rfc3339, DenormalizedMovie, DenormalizedSession, IngressoDateEntry, IngressoRawMovie,
    IngressoSessionGroup, IngressoTag, MovieStatic, NormalizedSessions, Session, UpcomingItem,
};
use std::collections::{HashMap, HashSet};

fn tag_name(t: &IngressoTag) -> String {
    match t {
        IngressoTag::Name(s) => s.clone(),
        IngressoTag::Obj { name } => name.clone(),
    }
}

fn duration_num(v: &Option<serde_json::Value>) -> Option<i64> {
    match v {
        Some(serde_json::Value::Number(n)) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Some(serde_json::Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

pub fn extract_movie_static(raw: &IngressoRawMovie) -> MovieStatic {
    let poster = raw.images.as_ref().and_then(|imgs| {
        imgs.iter()
            .find(|i| i.r#type.as_deref() == Some("PosterPortrait"))
            .and_then(|i| i.url.clone())
    });
    let backdrop = raw.images.as_ref().and_then(|imgs| {
        imgs.iter()
            .find(|i| i.r#type.as_deref() == Some("PosterHorizontal"))
            .and_then(|i| i.url.clone())
    });
    let trailer = raw
        .trailers
        .as_ref()
        .and_then(|t| t.first())
        .and_then(|t| t.url.clone());
    let tags = raw
        .complete_tags
        .as_ref()
        .or(raw.tags.as_ref())
        .map(|tags| tags.iter().map(tag_name).collect())
        .unwrap_or_default();

    MovieStatic {
        id: raw.id,
        title: raw.title.clone(),
        original_title: raw
            .original_title
            .as_ref()
            .filter(|s| !s.is_empty())
            .cloned(),
        url_key: raw.url_key.clone().unwrap_or_default(),
        duration: duration_num(&raw.duration).filter(|&n| n != 0),
        content_rating: raw
            .content_rating
            .as_ref()
            .filter(|s| !s.is_empty())
            .cloned(),
        rating_color: raw.rating_details.as_ref().and_then(|r| r.color.clone()),
        genres: raw.genres.clone().unwrap_or_default(),
        distributor: raw.distributor.clone().filter(|s| !s.is_empty()),
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
    let mut sessions = Vec::new();
    let Some(groups) = session_types else {
        return sessions;
    };
    for group in groups {
        for s in group.sessions.as_deref().unwrap_or(&[]) {
            let format = s
                .types
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .find(|t| {
                    t.name.as_deref() != Some("Dublado") && t.name.as_deref() != Some("Legendado")
                })
                .and_then(|t| t.alias.clone())
                .unwrap_or_else(|| "2D".into());
            let audio = s
                .types
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .find(|t| {
                    t.name.as_deref() == Some("Dublado") || t.name.as_deref() == Some("Legendado")
                })
                .and_then(|t| t.alias.clone());
            sessions.push(Session {
                id: s.id.clone(),
                movie_id,
                time: s.time.clone(),
                price: s.price,
                room: s.room.clone().filter(|r| !r.is_empty()),
                format,
                audio,
                checkout_url: s.site_url.clone().filter(|u| !u.is_empty()),
            });
        }
    }
    sessions
}

pub fn normalize_sessions_response(api_response: Option<&IngressoDateEntry>) -> NormalizedSessions {
    let Some(data) = api_response else {
        return NormalizedSessions {
            movies: HashMap::new(),
            sessions: Vec::new(),
            date: None,
            fetched_at: now_rfc3339(),
        };
    };
    let Some(movies_raw) = data.movies.as_ref() else {
        return NormalizedSessions {
            movies: HashMap::new(),
            sessions: Vec::new(),
            date: None,
            fetched_at: now_rfc3339(),
        };
    };

    let mut movies = HashMap::new();
    let mut sessions = Vec::new();
    for raw in movies_raw {
        let key = raw.id.to_string();
        movies
            .entry(key)
            .or_insert_with(|| extract_movie_static(raw));
        sessions.extend(extract_sessions(raw.id, raw.session_types.as_deref()));
    }

    NormalizedSessions {
        movies,
        sessions,
        date: if data.date.is_empty() {
            None
        } else {
            Some(data.date.clone())
        },
        fetched_at: now_rfc3339(),
    }
}

pub fn normalize_upcoming_from_sessions(
    future_dates: &[IngressoDateEntry],
    today_movie_ids: &HashSet<i64>,
) -> Vec<UpcomingItem> {
    let mut seen: HashMap<i64, UpcomingItem> = HashMap::new();

    for date_entry in future_dates {
        for raw in date_entry.movies.as_deref().unwrap_or(&[]) {
            if today_movie_ids.contains(&raw.id) || seen.contains_key(&raw.id) {
                continue;
            }
            let poster = raw.images.as_ref().and_then(|imgs| {
                imgs.iter()
                    .find(|i| i.r#type.as_deref() == Some("PosterPortrait"))
                    .and_then(|i| i.url.clone())
            });
            let mut formats = HashSet::new();
            let mut min_price: Option<f64> = None;
            let groups = raw
                .session_types
                .as_deref()
                .or(raw.rooms.as_deref())
                .unwrap_or(&[]);
            for group in groups {
                for s in group.sessions.as_deref().unwrap_or(&[]) {
                    for t in s.types.as_deref().unwrap_or(&[]) {
                        if t.name.as_deref() != Some("Dublado")
                            && t.name.as_deref() != Some("Legendado")
                        {
                            if let Some(alias) = &t.alias {
                                formats.insert(alias.clone());
                            }
                        }
                    }
                    if let Some(price) = s.price {
                        min_price = Some(match min_price {
                            Some(m) if m < price => m,
                            _ => price,
                        });
                    }
                }
            }
            seen.insert(
                raw.id,
                UpcomingItem {
                    id: raw.id,
                    title: raw.title.clone(),
                    original_title: raw.original_title.clone().filter(|s| !s.is_empty()),
                    content_rating: raw.content_rating.clone().filter(|s| !s.is_empty()),
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
                        .filter(|s| !s.is_empty()),
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
        if !grouped.contains_key(&session.movie_id) {
            let Some(movie) = movies.get(&session.movie_id.to_string()) else {
                continue;
            };
            grouped.insert(
                session.movie_id,
                DenormalizedMovie {
                    name: movie.title.clone(),
                    movie: movie.clone(),
                    sessions: Vec::new(),
                },
            );
        }
        if let Some(entry) = grouped.get_mut(&session.movie_id) {
            entry.sessions.push(DenormalizedSession {
                time: session.time.clone(),
                session_id: session.id.clone(),
                price_inteira: session.price,
                price_meia: session.price.map(|p| (p / 2.0 * 100.0).round() / 100.0),
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
    use super::*;
    use crate::types::IngressoRawSession;

    #[test]
    fn extract_format_and_audio() {
        let groups = vec![IngressoSessionGroup {
            sessions: Some(vec![IngressoRawSession {
                id: "s1".into(),
                time: "20:00".into(),
                price: Some(40.0),
                room: Some("1".into()),
                types: Some(vec![
                    crate::types::IngressoSessionType {
                        name: Some("Normal".into()),
                        alias: Some("2D".into()),
                    },
                    crate::types::IngressoSessionType {
                        name: Some("Dublado".into()),
                        alias: Some("DUB".into()),
                    },
                ]),
                site_url: None,
            }]),
        }];
        let sessions = extract_sessions(1, Some(&groups));
        assert_eq!(sessions[0].format, "2D");
        assert_eq!(sessions[0].audio.as_deref(), Some("DUB"));
    }
}
