use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use maceio_cine_telegram::api::{fetch_normalized, fetch_upcoming};
use maceio_cine_telegram::cinemas::CINEMAS;
use maceio_cine_telegram::data::get_date_string;
use maceio_cine_telegram::store::NormalizedCache;
use teloxide::prelude::Requester;
use teloxide::types::BotCommand;

async fn handler(_: LambdaEvent<serde_json::Value>) -> Result<serde_json::Value, Error> {
    let bot = maceio_cine_telegram::bot()?;
    let mut cache = NormalizedCache::load().await?;
    let dates = [get_date_string(0), get_date_string(1)];
    let mut updated_movies = 0;

    for cinema in &CINEMAS {
        for date in &dates {
            match fetch_normalized(Some(date), cinema.id).await {
                Ok(normalized) => {
                    let before = cache.data.movies.len();
                    cache.merge_movies(&normalized.movies);
                    updated_movies += cache.data.movies.len() - before;
                    let date_key = normalized.date.unwrap_or_else(|| date.clone());
                    cache.set_sessions(
                        &date_key,
                        normalized.sessions,
                        normalized.fetched_at,
                        cinema.id,
                    );
                }
                Err(error) => {
                    tracing::error!(theater = cinema.id, date = %date, %error, "Failed updating sessions")
                }
            }
        }

        match fetch_upcoming(cinema.id).await {
            Ok((items, fetched_at)) => cache.set_upcoming(items, fetched_at, cinema.id),
            Err(error) => {
                tracing::error!(theater = cinema.id, %error, "Failed updating upcoming releases")
            }
        }
    }
    cache.save_if_dirty().await?;

    if let Ok(webhook_url) = std::env::var("WEBHOOK_URL") {
        if !webhook_url.is_empty() {
            match webhook_url.parse() {
                Ok(url) => {
                    if let Err(error) = bot.set_webhook(url).await {
                        tracing::warn!(%error, "Failed to set Telegram webhook");
                    }
                }
                Err(error) => tracing::warn!(%error, "Invalid WEBHOOK_URL"),
            }
        }
    }

    let commands = [
        BotCommand::new("start", "Iniciar o bot e escolher cinema"),
        BotCommand::new("hoje", "Filmes em cartaz no cinema selecionado"),
        BotCommand::new("proximos", "Lançamentos futuros e pré-vendas"),
        BotCommand::new("cinemas", "Trocar de cinema selecionado"),
    ];
    if let Err(error) = bot.set_my_commands(commands).await {
        tracing::warn!(%error, "Failed to configure Telegram commands");
    }

    tracing::info!(updated_movies, "Daily cache warming complete");
    Ok(serde_json::json!({ "ok": true, "totalMovies": updated_movies }))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .without_time()
        .init();
    run(service_fn(handler)).await
}
