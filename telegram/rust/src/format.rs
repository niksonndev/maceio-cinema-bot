use crate::ratings::{format_ratings_line, get_movie_ratings};
use crate::types::{maceio_date, DenormalizedMovie, DenormalizedSession, UpcomingItem};
use chrono::{Datelike, NaiveDate};

const MONTHS: [&str; 12] = [
    "janeiro",
    "fevereiro",
    "março",
    "abril",
    "maio",
    "junho",
    "julho",
    "agosto",
    "setembro",
    "outubro",
    "novembro",
    "dezembro",
];

fn format_date_pt(date: Option<&str>) -> String {
    let Some(date) = date.and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()) else {
        return "data não disponível".into();
    };
    format!(
        "{} de {} de {}",
        date.day(),
        MONTHS[date.month0() as usize],
        date.year()
    )
}

fn format_content_rating(raw: Option<&str>) -> Option<String> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    let normalized = raw.to_lowercase();
    Some(if normalized == "l" || normalized == "livre" {
        "🟢 Livre".into()
    } else if normalized.starts_with("10") {
        "🟡 10 anos".into()
    } else if normalized.starts_with("12") {
        "🟡 12 anos".into()
    } else if normalized.starts_with("14") {
        "🟠 14 anos".into()
    } else if normalized.starts_with("16") {
        "🔴 16 anos".into()
    } else if normalized.starts_with("18") {
        "🔴 18 anos".into()
    } else {
        format!("🔹 {raw}")
    })
}

fn format_price_tag(session: Option<&DenormalizedSession>) -> String {
    match session {
        Some(session) if session.gratuito => " — Gratuito ✨".into(),
        Some(session) if session.price_inteira.is_some() => {
            format!(" — R$ {:.2}", session.price_inteira.unwrap_or_default()).replace('.', ",")
        }
        _ => String::new(),
    }
}

fn format_sessions_block(movie: &DenormalizedMovie) -> String {
    if movie.sessions.is_empty() {
        return String::new();
    }
    let mut groups: Vec<(String, Vec<&DenormalizedSession>)> = Vec::new();
    for session in &movie.sessions {
        let format = if session.format.is_empty() {
            "2D"
        } else {
            &session.format
        };
        if let Some((_, entries)) = groups.iter_mut().find(|(key, _)| key == format) {
            entries.push(session);
        } else {
            groups.push((format.to_owned(), vec![session]));
        }
    }
    let mut block = String::from("   🕒 Sessões (preços para ingresso inteira):\n");
    for (format, sessions) in groups {
        let icon = match format.as_str() {
            "2D" => "🎞",
            "Cinépic" => "🖥",
            "VIP" => "⭐",
            _ => "🎬",
        };
        let times = sessions
            .iter()
            .map(|session| session.time.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let price = sessions
            .iter()
            .find(|session| session.price_inteira.is_some());
        block.push_str(&format!(
            "   {icon} *{format}:* {times}{}\n",
            format_price_tag(price.copied())
        ));
    }
    block
}

fn format_when(days: i64, item: &UpcomingItem) -> String {
    if days == 1 {
        format!("amanhã ({})", item.first_date_formatted)
    } else if days <= 7 {
        format!(
            "nesta *{}* ({})",
            item.first_date_day_of_week, item.first_date_formatted
        )
    } else {
        format!(
            "em {} ({})",
            item.first_date_formatted, item.first_date_day_of_week
        )
    }
}

pub async fn format_movie_card(
    movie: &DenormalizedMovie,
    cinema_label: &str,
    date: Option<&str>,
) -> String {
    let mut text = format!(
        "*🎬 PROGRAMAÇÃO*\n📍 {cinema_label}\n📅 {}\n\n*{}*\n",
        format_date_pt(date),
        movie.name
    );
    let genres = (!movie.movie.genres.is_empty()).then(|| movie.movie.genres.join(", "));
    let rating = format_content_rating(movie.movie.content_rating.as_deref());
    let has_rating = rating.is_some();
    if genres.is_some() || rating.is_some() {
        text.push_str("   ");
        if let Some(rating) = rating {
            text.push_str(&format!("🎟 Classificação etária: {rating}"));
        }
        if let Some(genres) = genres {
            if has_rating {
                text.push_str(&format!(" — _{genres}_"));
            } else {
                text.push_str(&format!("_{genres}_"));
            }
        }
        text.push('\n');
    }
    let title = movie
        .movie
        .original_title
        .as_deref()
        .filter(|title| !title.is_empty())
        .unwrap_or(&movie.name);
    text.push_str(&format_ratings_line(
        get_movie_ratings(title, None).await.as_ref(),
    ));
    text.push_str(&format_sessions_block(movie));
    text
}

pub async fn format_upcoming_card(item: &UpcomingItem, cinema_label: &str) -> String {
    let days = NaiveDate::parse_from_str(&item.first_date, "%Y-%m-%d")
        .ok()
        .zip(NaiveDate::parse_from_str(&maceio_date(0), "%Y-%m-%d").ok())
        .map(|(date, today)| (date - today).num_days())
        .unwrap_or_default();
    let presale = if item.in_pre_sale {
        " 🔥 PRÉ-VENDA"
    } else {
        ""
    };
    let mut text = format!(
        "*🆕 PRÓXIMOS LANÇAMENTOS*\n📍 {cinema_label}\n\n🎬 *{}{presale}*\n",
        item.title
    );
    let title = item
        .original_title
        .as_deref()
        .filter(|title| !title.is_empty())
        .unwrap_or(&item.title);
    let ratings = format_ratings_line(get_movie_ratings(title, None).await.as_ref());
    if !ratings.is_empty() {
        text.push_str(&format!("   {}\n", ratings.trim()));
    }
    text.push_str(&format!("   📅 Estreia {}\n", format_when(days, item)));
    if !item.genres.is_empty() {
        text.push_str(&format!("   _{}_\n", item.genres.join(", ")));
    }
    if !item.formats.is_empty() {
        text.push_str(&format!("   {}\n", item.formats.join(", ")));
    }
    if let Some(price) = item.price_from {
        text.push_str(&format!("   A partir de R$ {:.2}\n", price).replace('.', ","));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::{format_content_rating, format_date_pt};

    #[test]
    fn formats_dates_and_content_ratings() {
        assert_eq!(
            format_date_pt(Some("2026-02-24")),
            "24 de fevereiro de 2026"
        );
        assert_eq!(format_date_pt(None), "data não disponível");
        assert_eq!(
            format_content_rating(Some("14")).as_deref(),
            Some("🟠 14 anos")
        );
    }
}
