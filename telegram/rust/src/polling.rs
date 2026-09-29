use axum::{routing::get, Json, Router};
use maceio_cine_telegram::handlers::handle_update;
use maceio_cine_telegram::prefs::Preferences;
use maceio_cine_telegram::store::NormalizedCache;
use serde_json::{json, Value};
use std::error::Error;
use teloxide::prelude::*;
use teloxide::types::BotCommand;

async fn health() -> Json<Value> {
    Json(json!({
        "status": "Bot is online",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let token = std::env::var("TELEGRAM_BOT_TOKEN")?;
    let bot = Bot::new(token);
    let webhook = bot.get_webhook_info().await?;
    if let Some(webhook_url) = webhook.url {
        return Err(std::io::Error::other(format!(
            "Polling refused: this token has an active webhook at {webhook_url}. Use a staging token or remove the webhook first."
        ))
        .into());
    }

    let commands = [
        BotCommand::new("start", "Iniciar o bot e escolher cinema"),
        BotCommand::new("hoje", "Filmes em cartaz no cinema selecionado"),
        BotCommand::new("proximos", "Lançamentos futuros e pré-vendas"),
        BotCommand::new("cinemas", "Trocar de cinema selecionado"),
    ];
    bot.set_my_commands(commands).await?;

    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(10000);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    let app = Router::new().route("/", get(health));
    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            tracing::error!(%error, "Health server failed");
        }
    });

    let mut cache = NormalizedCache::load().await?;
    let mut prefs = Preferences::load().await?;
    let mut offset = 0_i32;
    let mut polling_failures = 0_u8;
    tracing::info!(port, "Telegram polling started");

    loop {
        match bot.get_updates().offset(offset).timeout(25).send().await {
            Ok(updates) => {
                polling_failures = 0;
                for update in updates {
                    offset = update.id.0 as i32 + 1;
                    if let Err(error) = handle_update(&bot, update, &mut cache, &mut prefs).await {
                        tracing::error!(%error, "Failed to process Telegram update");
                    }
                    if let Err(error) = cache.save_if_dirty().await {
                        tracing::error!(%error, "Failed to save cache");
                    }
                }
            }
            Err(error) => {
                polling_failures += 1;
                tracing::error!(polling_failures, %error, "Telegram polling failed");
                if polling_failures > 5 {
                    let error: Box<dyn Error + Send + Sync> = Box::new(error);
                    return Err(error);
                }
                tokio::time::sleep(std::time::Duration::from_secs(
                    u64::from(polling_failures) * 5,
                ))
                .await;
            }
        }
    }
}
