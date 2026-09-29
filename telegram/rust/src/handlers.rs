use crate::cinemas::find_cinema_by_id;
use crate::data::{get_date_string, get_movies_for_date, get_upcoming_movies};
use crate::format::{format_movie_card, format_upcoming_card};
use crate::keyboards::{back_button_markup, carousel_keyboard, cinema_keyboard, main_keyboard};
use crate::prefs::Preferences;
use crate::store::NormalizedCache;
use crate::types::{Cinema, Error, Result};
use teloxide::prelude::*;
use teloxide::types::{InputFile, InputMedia, InputMediaPhoto, ParseMode, UpdateKind};

enum CarouselKind {
    Today,
    Tomorrow,
    Upcoming,
}

impl CarouselKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "hoje" => Some(Self::Today),
            "amanha" => Some(Self::Tomorrow),
            "proximos" => Some(Self::Upcoming),
            _ => None,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Today => "hoje",
            Self::Tomorrow => "amanha",
            Self::Upcoming => "proximos",
        }
    }

    fn date(&self) -> Option<String> {
        match self {
            Self::Today => Some(get_date_string(0)),
            Self::Tomorrow => Some(get_date_string(1)),
            Self::Upcoming => None,
        }
    }
}

pub async fn handle_update(
    bot: &Bot,
    update: Update,
    cache: &mut NormalizedCache,
    prefs: &mut Preferences,
) -> Result<()> {
    let failure_target = match &update.kind {
        UpdateKind::Message(message) => message
            .text()
            .and_then(parse_command)
            .map(|command| (message.chat.id, error_prefix(command).to_owned())),
        UpdateKind::CallbackQuery(query) => query.regular_message().map(|message| {
            (
                message.chat.id,
                error_prefix(query.data.as_deref().unwrap_or_default()).to_owned(),
            )
        }),
        _ => None,
    };

    let result = match update.kind {
        UpdateKind::Message(message) => {
            let Some(text) = message.text() else {
                return Ok(());
            };
            let Some(command) = parse_command(text) else {
                return Ok(());
            };
            handle_command(bot, cache, prefs, message.chat.id, command).await
        }
        UpdateKind::CallbackQuery(query) => handle_callback(bot, cache, prefs, query).await,
        _ => Ok(()),
    };

    if let Err(error) = result {
        tracing::error!(%error, "Failed to process Telegram update");
        if !matches!(&error, Error::Telegram(_)) {
            if let Some((chat_id, prefix)) = failure_target {
                bot.send_message(chat_id, format!("{prefix}: {error}"))
                    .await?;
                return Ok(());
            }
        }
        return Err(error);
    }
    Ok(())
}

fn error_prefix(command: &str) -> &'static str {
    match command {
        "hoje" | "filmes_hoje" | "filmes_amanha" => "❌ Erro ao buscar filmes",
        "proximos" | "proximos_lancamentos" => "❌ Erro ao buscar lançamentos",
        "atualizar" => "❌ Erro ao atualizar",
        _ => "❌ Erro ao processar",
    }
}

#[allow(deprecated)]
fn markdown_mode() -> ParseMode {
    ParseMode::Markdown
}

fn poster_input_file(poster: &str) -> Result<InputFile> {
    let url = reqwest::Url::parse(poster).map_err(|error| Error::Msg(error.to_string()))?;
    Ok(InputFile::url(url))
}

fn parse_command(text: &str) -> Option<&str> {
    let command = text.split_whitespace().next()?;
    if !command.starts_with('/') {
        return None;
    }
    command[1..].split('@').next()
}

async fn handle_command(
    bot: &Bot,
    cache: &mut NormalizedCache,
    prefs: &mut Preferences,
    chat_id: ChatId,
    command: &str,
) -> Result<()> {
    match command {
        "start" => {
            bot.send_message(
                chat_id,
                "Olá! Eu sou o seu guia de cinema em Maceió. 🍿\nEscolha abaixo qual cinema você deseja consultar:",
            )
            .reply_markup(cinema_keyboard())
            .await?;
        }
        "hoje" | "proximos" => {
            let Some(cinema) = prefs.get_user_cinema(chat_id.0) else {
                ask_cinema_first(bot, chat_id).await?;
                return Ok(());
            };
            let kind = if command == "hoje" {
                CarouselKind::Today
            } else {
                CarouselKind::Upcoming
            };
            let loading = if matches!(kind, CarouselKind::Today) {
                "⏳ Buscando filmes de hoje..."
            } else {
                "⏳ Buscando próximos lançamentos..."
            };
            with_loading(bot, chat_id, loading, async {
                send_carousel_page(bot, cache, chat_id, &kind, 0, cinema).await
            })
            .await?;
        }
        "cinemas" => send_cinema_picker(bot, prefs, chat_id).await?,
        "atualizar" => {
            let Some(cinema) = prefs.get_user_cinema(chat_id.0) else {
                ask_cinema_first(bot, chat_id).await?;
                return Ok(());
            };
            with_loading(
                bot,
                chat_id,
                "🔄 Atualizando programação de hoje...",
                async {
                    let normalized = crate::api::fetch_normalized(None, cinema.id).await?;
                    cache.merge_movies(&normalized.movies);
                    let date = normalized
                        .date
                        .clone()
                        .unwrap_or_else(|| get_date_string(0));
                    cache.set_sessions(
                        &date,
                        normalized.sessions,
                        normalized.fetched_at,
                        cinema.id,
                    );
                    cache.save_if_dirty().await?;
                    send_carousel_page(bot, cache, chat_id, &CarouselKind::Today, 0, cinema).await
                },
            )
            .await?;
        }
        _ => {}
    }
    Ok(())
}

async fn handle_callback(
    bot: &Bot,
    cache: &mut NormalizedCache,
    prefs: &mut Preferences,
    query: teloxide::types::CallbackQuery,
) -> Result<()> {
    bot.answer_callback_query(query.id.clone()).await?;
    let Some(message) = query.regular_message() else {
        return Ok(());
    };
    let chat_id = message.chat.id;
    let message_id = message.id;
    let has_photo = message.photo().is_some();
    let data = query.data.as_deref().unwrap_or_default();

    if let Some(theater_id) = data.strip_prefix("cinema_") {
        let Some(cinema) = find_cinema_by_id(theater_id) else {
            bot.send_message(chat_id, "❌ Cinema não encontrado.")
                .await?;
            return Ok(());
        };
        prefs.set_user_cinema(chat_id.0, theater_id).await?;
        bot.send_message(
            chat_id,
            format!(
                "✅ Cinema selecionado: *{}*\n\nEscolha uma opção:",
                cinema.label
            ),
        )
        .parse_mode(markdown_mode())
        .reply_markup(main_keyboard())
        .await?;
        return Ok(());
    }

    if data == "trocar_cinema" {
        send_cinema_picker(bot, prefs, chat_id).await?;
        return Ok(());
    }

    let Some(cinema) = prefs.get_user_cinema(chat_id.0) else {
        ask_cinema_first(bot, chat_id).await?;
        return Ok(());
    };

    if let Some((kind, index)) = parse_carousel_callback(data) {
        edit_carousel_page(
            bot, cache, chat_id, message_id, &kind, index, cinema, has_photo,
        )
        .await?;
        return Ok(());
    }

    match data {
        "filmes_hoje" => {
            let kind = CarouselKind::Today;
            let date = kind.date().unwrap_or_default();
            if cache.get_sessions(&date, cinema.id).is_none() {
                with_loading(
                    bot,
                    chat_id,
                    "⏳ Buscando filmes de hoje... Aguarde um momento!",
                    async { send_carousel_page(bot, cache, chat_id, &kind, 0, cinema).await },
                )
                .await?;
            } else {
                send_carousel_page(bot, cache, chat_id, &kind, 0, cinema).await?;
            }
        }
        "filmes_amanha" => {
            let kind = CarouselKind::Tomorrow;
            let date = kind.date().unwrap_or_default();
            if cache.get_sessions(&date, cinema.id).is_none() {
                with_loading(
                    bot,
                    chat_id,
                    "⏳ Buscando filmes de amanhã... Aguarde um momento!",
                    async { send_carousel_page(bot, cache, chat_id, &kind, 0, cinema).await },
                )
                .await?;
            } else {
                send_carousel_page(bot, cache, chat_id, &kind, 0, cinema).await?;
            }
        }
        "proximos_lancamentos" => {
            let kind = CarouselKind::Upcoming;
            if cache.get_upcoming(cinema.id).is_none() {
                with_loading(bot, chat_id, "⏳ Buscando próximos lançamentos...", async {
                    send_carousel_page(bot, cache, chat_id, &kind, 0, cinema).await
                })
                .await?;
            } else {
                send_carousel_page(bot, cache, chat_id, &kind, 0, cinema).await?;
            }
        }
        "voltar_menu" => {
            bot.send_message(
                chat_id,
                format!("*🎬 {}*\n\nEscolha uma opção:", cinema.label),
            )
            .parse_mode(markdown_mode())
            .reply_markup(main_keyboard())
            .await?;
        }
        "como_funciona" => {
            let text = "❓ *Como Funciona*\n\nEste bot fornece informações sobre filmes em cartaz nos cinemas de Maceió.\n\n💡 *Funcionalidades:*\n🎬 Filmes de Hoje — Veja os filmes em exibição hoje\n📅 Filmes de Amanhã — Veja os filmes em exibição amanhã\n🆕 Próximos Lançamentos — Veja o que está chegando\n🔄 Trocar Cinema — Mude o cinema selecionado\n💰 Preços — Extraídos automaticamente da API\n\n";
            send_with_back_button(bot, chat_id, text, cinema.url).await?;
        }
        _ => send_with_back_button(bot, chat_id, "❓ Opção não reconhecida.", cinema.url).await?,
    }
    Ok(())
}

async fn ask_cinema_first(bot: &Bot, chat_id: ChatId) -> Result<()> {
    bot.send_message(
        chat_id,
        "⚠️ Você ainda não escolheu um cinema. Escolha abaixo qual cinema deseja consultar:",
    )
    .reply_markup(cinema_keyboard())
    .await?;
    Ok(())
}

async fn send_cinema_picker(bot: &Bot, prefs: &Preferences, chat_id: ChatId) -> Result<()> {
    let text = match prefs.get_user_cinema(chat_id.0) {
        Some(cinema) => format!("🎬 Cinema atual: *{}*\nEscolha outro cinema:", cinema.label),
        None => "🎬 Escolha o cinema que deseja consultar:".into(),
    };
    bot.send_message(chat_id, text)
        .parse_mode(markdown_mode())
        .reply_markup(cinema_keyboard())
        .await?;
    Ok(())
}

async fn send_with_back_button(
    bot: &Bot,
    chat_id: ChatId,
    text: &str,
    cinema_url: &str,
) -> Result<()> {
    bot.send_message(chat_id, text)
        .parse_mode(markdown_mode())
        .reply_markup(back_button_markup(cinema_url))
        .await?;
    Ok(())
}

async fn with_loading<F>(bot: &Bot, chat_id: ChatId, text: &str, work: F) -> Result<()>
where
    F: std::future::Future<Output = Result<()>>,
{
    let loading = bot.send_message(chat_id, text).await?;
    let result = work.await;
    let _ = bot.delete_message(chat_id, loading.id).await;
    result
}

async fn send_carousel_page(
    bot: &Bot,
    cache: &mut NormalizedCache,
    chat_id: ChatId,
    kind: &CarouselKind,
    index: usize,
    cinema: &Cinema,
) -> Result<()> {
    if matches!(kind, CarouselKind::Upcoming) {
        let items = get_upcoming_movies(cache, cinema.id).await?;
        if items.is_empty() {
            send_with_back_button(
                bot,
                chat_id,
                "📭 *Nenhum lançamento próximo encontrado.*",
                cinema.url,
            )
            .await?;
            return Ok(());
        }
        let safe_index = index.min(items.len() - 1);
        let item = &items[safe_index];
        let text = format_upcoming_card(item, cinema.label).await;
        let markup = carousel_keyboard(kind.as_str(), safe_index, items.len(), cinema.url);
        send_card(bot, chat_id, &text, item.poster.as_deref(), markup).await?;
        return Ok(());
    }

    let date = kind.date().unwrap_or_default();
    let query_date = matches!(kind, CarouselKind::Tomorrow).then_some(date.as_str());
    let (movies, resolved_date) = get_movies_for_date(cache, query_date, cinema.id).await?;
    if movies.is_empty() {
        send_with_back_button(
            bot,
            chat_id,
            "📭 *Nenhum filme em cartaz para esta data.*",
            cinema.url,
        )
        .await?;
        return Ok(());
    }
    let safe_index = index.min(movies.len() - 1);
    let movie = &movies[safe_index];
    let text = format_movie_card(movie, cinema.label, Some(&resolved_date)).await;
    let markup = carousel_keyboard(kind.as_str(), safe_index, movies.len(), cinema.url);
    send_card(bot, chat_id, &text, movie.movie.poster.as_deref(), markup).await?;
    Ok(())
}

async fn send_card(
    bot: &Bot,
    chat_id: ChatId,
    text: &str,
    poster: Option<&str>,
    markup: teloxide::types::InlineKeyboardMarkup,
) -> Result<()> {
    if let Some(poster) = poster {
        bot.send_photo(chat_id, poster_input_file(poster)?)
            .caption(text)
            .parse_mode(markdown_mode())
            .reply_markup(markup)
            .await?;
    } else {
        bot.send_message(chat_id, text)
            .parse_mode(markdown_mode())
            .reply_markup(markup)
            .await?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn edit_carousel_page(
    bot: &Bot,
    cache: &mut NormalizedCache,
    chat_id: ChatId,
    message_id: teloxide::types::MessageId,
    kind: &CarouselKind,
    index: usize,
    cinema: &Cinema,
    has_photo: bool,
) -> Result<()> {
    if matches!(kind, CarouselKind::Upcoming) {
        let items = get_upcoming_movies(cache, cinema.id).await?;
        if items.is_empty() {
            send_with_back_button(
                bot,
                chat_id,
                "📭 *Nenhum lançamento próximo encontrado.*",
                cinema.url,
            )
            .await?;
            return Ok(());
        }
        let safe_index = index.min(items.len() - 1);
        let item = &items[safe_index];
        let text = format_upcoming_card(item, cinema.label).await;
        let markup = carousel_keyboard(kind.as_str(), safe_index, items.len(), cinema.url);
        edit_card(
            bot,
            chat_id,
            message_id,
            &text,
            item.poster.as_deref(),
            markup,
            has_photo,
        )
        .await?;
        return Ok(());
    }

    let date = kind.date().unwrap_or_default();
    let query_date = matches!(kind, CarouselKind::Tomorrow).then_some(date.as_str());
    let (movies, resolved_date) = get_movies_for_date(cache, query_date, cinema.id).await?;
    if movies.is_empty() {
        send_with_back_button(
            bot,
            chat_id,
            "📭 *Nenhum filme em cartaz para esta data.*",
            cinema.url,
        )
        .await?;
        return Ok(());
    }
    let safe_index = index.min(movies.len() - 1);
    let movie = &movies[safe_index];
    let text = format_movie_card(movie, cinema.label, Some(&resolved_date)).await;
    let markup = carousel_keyboard(kind.as_str(), safe_index, movies.len(), cinema.url);
    edit_card(
        bot,
        chat_id,
        message_id,
        &text,
        movie.movie.poster.as_deref(),
        markup,
        has_photo,
    )
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn edit_card(
    bot: &Bot,
    chat_id: ChatId,
    message_id: teloxide::types::MessageId,
    text: &str,
    poster: Option<&str>,
    markup: teloxide::types::InlineKeyboardMarkup,
    has_photo: bool,
) -> Result<()> {
    if has_photo {
        if let Some(poster) = poster {
            let media = InputMedia::Photo(
                InputMediaPhoto::new(poster_input_file(poster)?)
                    .caption(text)
                    .parse_mode(markdown_mode()),
            );
            bot.edit_message_media(chat_id, message_id, media)
                .reply_markup(markup)
                .await?;
            return Ok(());
        }
    }
    bot.edit_message_text(chat_id, message_id, text)
        .parse_mode(markdown_mode())
        .await?;
    bot.edit_message_reply_markup(chat_id, message_id)
        .reply_markup(markup)
        .await?;
    Ok(())
}

fn parse_carousel_callback(data: &str) -> Option<(CarouselKind, usize)> {
    let mut parts = data.split('_');
    if parts.next()? != "carousel" {
        return None;
    }
    let kind = CarouselKind::parse(parts.next()?)?;
    let index = parts.next()?.parse().ok()?;
    let _total: usize = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some((kind, index))
}

#[cfg(test)]
mod tests {
    use super::{error_prefix, parse_carousel_callback, parse_command, CarouselKind};

    #[test]
    fn parses_bot_commands_with_username_suffix() {
        assert_eq!(parse_command("/hoje@MaceioBot"), Some("hoje"));
        assert_eq!(parse_command("/start"), Some("start"));
        assert_eq!(parse_command("hoje"), None);
    }

    #[test]
    fn parses_carousel_callback_payload() {
        let (kind, index) = parse_carousel_callback("carousel_proximos_2_5").unwrap();
        assert!(matches!(kind, CarouselKind::Upcoming));
        assert_eq!(index, 2);
        assert!(parse_carousel_callback("carousel_unknown_2_5").is_none());
    }

    #[test]
    fn maps_failures_to_existing_user_messages() {
        assert_eq!(error_prefix("hoje"), "❌ Erro ao buscar filmes");
        assert_eq!(error_prefix("proximos"), "❌ Erro ao buscar lançamentos");
        assert_eq!(error_prefix("atualizar"), "❌ Erro ao atualizar");
    }
}
