use crate::ratings::{format_ratings_line, get_movie_ratings};
use crate::types::{maceio_date, DenormalizedMovie, DenormalizedSession, UpcomingItem};
use std::collections::HashMap;

const MESES: [&str; 12] = [
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

fn format_label(format: &str) -> &str {
    match format {
        "2D" => "2D",
        "Cinépic" => "Cinépic",
        "VIP" => "VIP",
        "3D" => "3D",
        other => other,
    }
}

fn format_icon(format: &str) -> &'static str {
    match format {
        "2D" => "🎞",
        "Cinépic" => "🖥",
        "VIP" => "⭐",
        _ => "🎬",
    }
}

pub fn format_date_pt(date_str: Option<&str>) -> String {
    let Some(date_str) = date_str else {
        return "data não disponível".into();
    };
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return "data não disponível".into();
    }
    let Ok(month) = parts[1].parse::<usize>() else {
        return "data não disponível".into();
    };
    let Ok(day) = parts[2].parse::<u32>() else {
        return "data não disponível".into();
    };
    if !(1..=12).contains(&month) {
        return "data não disponível".into();
    }
    format!("{day} de {} de {}", MESES[month - 1], parts[0])
}

pub fn format_content_rating(raw: Option<&str>) -> Option<String> {
    let raw = raw?;
    let normalized = raw.trim().to_lowercase();
    if normalized.is_empty() {
        return None;
    }
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

fn format_price_tag(r#ref: Option<&DenormalizedSession>) -> String {
    match r#ref {
        Some(s) if s.gratuito => " — Gratuito ✨".into(),
        Some(s) if s.price_inteira.is_some() => {
            let p = s.price_inteira.unwrap();
            format!(" — R$ {}", format!("{p:.2}").replace('.', ","))
        }
        _ => String::new(),
    }
}

fn format_sessions_block(filme: &DenormalizedMovie) -> String {
    if filme.sessions.is_empty() {
        return String::new();
    }
    let mut by_format: HashMap<String, Vec<&DenormalizedSession>> = HashMap::new();
    for s in &filme.sessions {
        let key = if s.format.is_empty() {
            "2D".into()
        } else {
            s.format.clone()
        };
        by_format.entry(key).or_default().push(s);
    }
    let mut block = String::from("   🕒 Sessões (preços para ingresso inteira):\n");
    for (format, sessions) in by_format {
        let icon = format_icon(&format);
        let times = sessions
            .iter()
            .map(|s| s.time.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let r#ref = sessions.iter().find(|s| s.price_inteira.is_some()).copied();
        block.push_str(&format!(
            "   {icon} *{format}:* {times}{}\n",
            format_price_tag(r#ref)
        ));
    }
    block
}

fn format_when(diff_days: i64, item: &UpcomingItem) -> String {
    if diff_days == 1 {
        format!("amanhã ({})", item.first_date_formatted)
    } else if diff_days <= 7 {
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

pub async fn format_single_movie_card(
    filme: &DenormalizedMovie,
    cinema_label: &str,
    date_str: Option<&str>,
) -> String {
    let mut text = format!(
        "*🎬 PROGRAMAÇÃO*\n📍 {cinema_label}\n📅 {}\n\n",
        format_date_pt(date_str)
    );
    text.push_str(&format!("*{name}*\n", name = filme.name));

    let genres = if filme.movie.genres.is_empty() {
        None
    } else {
        Some(filme.movie.genres.join(", "))
    };
    let content_rating = format_content_rating(filme.movie.content_rating.as_deref());
    if genres.is_some() || content_rating.is_some() {
        let mut info = String::from("   ");
        if let Some(cr) = &content_rating {
            info.push_str(&format!("🎟 Classificação etária: {cr}"));
        }
        if let Some(g) = &genres {
            if content_rating.is_some() {
                info.push_str(&format!(" — _{g}_"));
            } else {
                info.push_str(&format!("_{g}_"));
            }
        }
        text.push_str(&info);
        text.push('\n');
    }

    let title = filme
        .movie
        .original_title
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&filme.name);
    let ratings = get_movie_ratings(title, None).await;
    text.push_str(&format_ratings_line(ratings.as_ref()));
    text.push_str(&format_sessions_block(filme));
    text
}

pub async fn format_single_upcoming_card(item: &UpcomingItem, cinema_label: &str) -> String {
    let today = maceio_date(0);
    let diff_days = chrono::NaiveDate::parse_from_str(&item.first_date, "%Y-%m-%d")
        .ok()
        .and_then(|d| {
            chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d")
                .ok()
                .map(|t| (d - t).num_days())
        })
        .unwrap_or(0);
    let quando = format_when(diff_days, item);
    let presale = if item.in_pre_sale {
        " 🔥 PRÉ-VENDA"
    } else {
        ""
    };
    let mut text = format!("*🆕 PRÓXIMOS LANÇAMENTOS*\n📍 {cinema_label}\n\n");
    text.push_str(&format!("🎬 *{}*{presale}\n", item.title));

    let title = item
        .original_title
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&item.title);
    let ratings = get_movie_ratings(title, None).await;
    let ratings_line = format_ratings_line(ratings.as_ref());
    if !ratings_line.is_empty() {
        text.push_str(&format!("   {}\n", ratings_line.trim()));
    }
    text.push_str(&format!("   📅 Estreia {quando}\n"));
    if !item.genres.is_empty() {
        text.push_str(&format!("   _{}_\n", item.genres.join(", ")));
    }
    if !item.formats.is_empty() {
        let labels: Vec<&str> = item.formats.iter().map(|f| format_label(f)).collect();
        text.push_str(&format!("   {}\n", labels.join(", ")));
    }
    if let Some(price) = item.price_from {
        text.push_str(&format!(
            "   A partir de R$ {}\n",
            format!("{price:.2}").replace('.', ",")
        ));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_pt() {
        assert_eq!(
            format_date_pt(Some("2026-02-24")),
            "24 de fevereiro de 2026"
        );
        assert_eq!(format_date_pt(None), "data não disponível");
    }

    #[test]
    fn content_rating() {
        assert_eq!(
            format_content_rating(Some("L")).as_deref(),
            Some("🟢 Livre")
        );
        assert_eq!(
            format_content_rating(Some("14")).as_deref(),
            Some("🟠 14 anos")
        );
    }
}
