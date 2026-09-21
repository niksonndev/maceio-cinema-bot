use maceio_cine_whatsapp::cache::NormalizedCache;
use maceio_cine_whatsapp::cinemas::{cinema_list_text, parse_cinema_choice};
use maceio_cine_whatsapp::data::{get_movies_for_date, get_upcoming_movies};
use maceio_cine_whatsapp::format::{format_single_movie_card, format_single_upcoming_card};
use maceio_cine_whatsapp::ingresso::fetch_normalized;
use maceio_cine_whatsapp::prefs::Prefs;
use maceio_cine_whatsapp::types::Cinema;
use std::sync::Arc;
use whatsapp_rust::prelude::*;

const MAX_CARDS: usize = 15;

fn normalize_command(text: &str) -> String {
    text.trim()
        .trim_start_matches('/')
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase()
}

fn is_group_chat(chat: &impl std::fmt::Display) -> bool {
    chat.to_string().contains("@g.us")
}

fn jid_key(chat: &impl std::fmt::Display) -> String {
    chat.to_string()
}

async fn reply(ctx: &MessageContext, text: &str) {
    if let Err(err) = ctx.reply(text).await {
        tracing::error!("Failed to reply: {err}");
    }
}

fn help_text() -> &'static str {
    "🍿 *Maceió Cine Bot*\n\n\
     Comandos:\n\
     *start* — apresentar o bot e listar cinemas\n\
     *hoje* — filmes em cartaz hoje\n\
     *proximos* — pré-vendas e lançamentos\n\
     *cinemas* — ver ou trocar o cinema\n\
     *atualizar* — forçar atualização de hoje\n\n\
     Envie *1*, *2* ou *3* para escolher o cinema."
}

async fn require_cinema<'a>(
    ctx: &MessageContext,
    prefs: &Prefs,
    jid: &str,
) -> Option<&'static Cinema> {
    match prefs.get_user_cinema(jid) {
        Some(c) => Some(c),
        None => {
            reply(
                ctx,
                &format!(
                    "⚠️ Você ainda não escolheu um cinema.\n\n{}",
                    cinema_list_text()
                ),
            )
            .await;
            None
        }
    }
}

async fn send_today(ctx: &MessageContext, cache: &NormalizedCache, cinema: &Cinema) {
    match get_movies_for_date(cache, None, cinema.id).await {
        Ok((movies, date, _)) => {
            if movies.is_empty() {
                reply(ctx, "📭 Nenhum filme em cartaz para esta data.").await;
                return;
            }
            for filme in movies.iter().take(MAX_CARDS) {
                let card = format_single_movie_card(filme, cinema.label, Some(&date)).await;
                reply(ctx, &card).await;
            }
        }
        Err(err) => {
            reply(ctx, &format!("❌ Erro ao buscar filmes: {err}")).await;
        }
    }
}

async fn send_upcoming(ctx: &MessageContext, cache: &NormalizedCache, cinema: &Cinema) {
    match get_upcoming_movies(cache, cinema.id).await {
        Ok((items, _)) => {
            if items.is_empty() {
                reply(ctx, "📭 Nenhum lançamento próximo encontrado.").await;
                return;
            }
            for item in items.iter().take(MAX_CARDS) {
                let card = format_single_upcoming_card(item, cinema.label).await;
                reply(ctx, &card).await;
            }
        }
        Err(err) => {
            reply(ctx, &format!("❌ Erro ao buscar lançamentos: {err}")).await;
        }
    }
}

pub async fn handle_message(ctx: MessageContext, cache: Arc<NormalizedCache>, prefs: Arc<Prefs>) {
    if ctx.info.source.is_from_me {
        return;
    }
    if is_group_chat(&ctx.info.source.chat) {
        return;
    }
    let Some(raw) = ctx.message.text_content() else {
        return;
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return;
    }

    let jid = jid_key(&ctx.info.source.chat);
    let cmd = normalize_command(raw);

    match cmd.as_str() {
        "start" | "oi" | "olá" | "ola" | "help" | "ajuda" => {
            reply(
                &ctx,
                &format!(
                    "Olá! Eu sou o seu guia de cinema em Maceió. 🍿\n\n{}\n\n{}",
                    cinema_list_text(),
                    help_text()
                ),
            )
            .await;
        }
        "hoje" => {
            let Some(cinema) = require_cinema(&ctx, &prefs, &jid).await else {
                return;
            };
            reply(&ctx, "⏳ Buscando filmes de hoje...").await;
            send_today(&ctx, &cache, cinema).await;
        }
        "proximos" | "próximos" => {
            let Some(cinema) = require_cinema(&ctx, &prefs, &jid).await else {
                return;
            };
            reply(&ctx, "⏳ Buscando próximos lançamentos...").await;
            send_upcoming(&ctx, &cache, cinema).await;
        }
        "cinemas" | "cinema" => {
            let current = prefs.get_user_cinema(&jid);
            let header = match current {
                Some(c) => format!("🎬 Cinema atual: *{}*\n\n", c.label),
                None => String::new(),
            };
            reply(&ctx, &format!("{header}{}", cinema_list_text())).await;
        }
        "atualizar" => {
            let Some(cinema) = require_cinema(&ctx, &prefs, &jid).await else {
                return;
            };
            reply(&ctx, "🔄 Atualizando programação de hoje...").await;
            match fetch_normalized(None, cinema.id).await {
                Ok(normalized) => {
                    cache.merge_movies(&normalized.movies);
                    let date = normalized
                        .date
                        .clone()
                        .unwrap_or_else(|| maceio_cine_whatsapp::types::maceio_date(0));
                    cache.set_sessions(
                        &date,
                        normalized.sessions,
                        normalized.fetched_at,
                        cinema.id,
                    );
                    send_today(&ctx, &cache, cinema).await;
                }
                Err(err) => {
                    reply(&ctx, &format!("❌ Erro ao atualizar: {err}")).await;
                }
            }
        }
        _ => {
            if let Some(cinema) = parse_cinema_choice(raw) {
                prefs.set_user_cinema(&jid, cinema.id);
                reply(
                    &ctx,
                    &format!(
                        "✅ Cinema selecionado: *{}*\n\nEnvie *hoje* ou *proximos*.",
                        cinema.label
                    ),
                )
                .await;
                return;
            }
            if prefs.get_user_cinema(&jid).is_none() {
                reply(
                    &ctx,
                    &format!("⚠️ Escolha um cinema primeiro.\n\n{}", cinema_list_text()),
                )
                .await;
            }
        }
    }
}
