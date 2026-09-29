use crate::cinemas::CINEMAS;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

fn callback_button(text: impl Into<String>, data: impl Into<String>) -> InlineKeyboardButton {
    InlineKeyboardButton::callback(text, data)
}

pub fn cinema_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(
        CINEMAS
            .iter()
            .map(|cinema| {
                vec![callback_button(
                    cinema.label,
                    format!("cinema_{}", cinema.id),
                )]
            })
            .collect::<Vec<_>>(),
    )
}

pub fn main_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            callback_button("🎬 Filmes de Hoje", "filmes_hoje"),
            callback_button("📅 Filmes de Amanhã", "filmes_amanha"),
        ],
        vec![callback_button(
            "🆕 Próximos Lançamentos",
            "proximos_lancamentos",
        )],
        vec![callback_button("🔄 Trocar de Cinema", "trocar_cinema")],
    ])
}

pub fn back_button_markup(cinema_url: &str) -> InlineKeyboardMarkup {
    let mut rows = Vec::new();
    if let Ok(url) = cinema_url.parse() {
        rows.push(vec![InlineKeyboardButton::url("🎫 Comprar Ingressos", url)]);
    }
    rows.push(vec![
        callback_button("⬅️ Voltar ao menu", "voltar_menu"),
        callback_button("🔄 Trocar cinema", "trocar_cinema"),
    ]);
    InlineKeyboardMarkup::new(rows)
}

pub fn carousel_keyboard(
    kind: &str,
    index: usize,
    total: usize,
    cinema_url: &str,
) -> InlineKeyboardMarkup {
    let mut rows = Vec::new();
    if total > 1 {
        let mut navigation = Vec::new();
        if index > 0 {
            navigation.push(callback_button(
                "◀ Anterior",
                format!("carousel_{kind}_{}_{}", index - 1, total),
            ));
        }
        if index + 1 < total {
            navigation.push(callback_button(
                "Próximo ▶",
                format!("carousel_{kind}_{}_{}", index + 1, total),
            ));
        }
        if !navigation.is_empty() {
            rows.push(navigation);
        }
    }
    if let Ok(url) = cinema_url.parse() {
        rows.push(vec![InlineKeyboardButton::url("🎫 Comprar Ingressos", url)]);
    }
    rows.push(vec![
        callback_button("⬅️ Voltar ao menu", "voltar_menu"),
        callback_button("🔄 Trocar cinema", "trocar_cinema"),
    ]);
    InlineKeyboardMarkup::new(rows)
}
