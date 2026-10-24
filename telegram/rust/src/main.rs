use lambda_http::{run, service_fn, Body, Error, Request, Response};
use maceio_cine_telegram::decode_update;
use maceio_cine_telegram::handlers::handle_update;
use maceio_cine_telegram::prefs::Preferences;
use maceio_cine_telegram::store::NormalizedCache;
use teloxide::Bot;

async fn process(request: Request) -> Result<(), Error> {
    let update = match decode_update(request.body().as_ref()) {
        Ok(update) => update,
        Err(error) => {
            tracing::warn!(%error, "Ignoring malformed Telegram update");
            return Ok(());
        }
    };
    tracing::info!(update_id = update.id.0, "Received Telegram update");
    let token = std::env::var("TELEGRAM_BOT_TOKEN")?;
    let bot = Bot::new(token);
    let (mut cache, mut prefs) = tokio::try_join!(NormalizedCache::load(), Preferences::load())?;
    handle_update(&bot, update, &mut cache, &mut prefs).await?;
    cache.save_if_dirty().await?;
    Ok(())
}

async fn handler(request: Request) -> Result<Response<Body>, Error> {
    // Always acknowledge Telegram with 2xx. Any error here is logged locally;
    // returning 5xx would make Telegram redeliver the update and amplify it.
    if let Err(error) = process(request).await {
        tracing::error!(%error, "Failed to process webhook request");
    }
    Ok(Response::new(Body::Text("{\"ok\":true}".to_owned())))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .without_time()
        .init();

    run(service_fn(handler)).await
}
