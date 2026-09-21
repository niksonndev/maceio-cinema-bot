mod handlers;
mod http;
mod wa;

use maceio_cine_whatsapp::cache::NormalizedCache;
use maceio_cine_whatsapp::prefs::Prefs;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cache = Arc::new(NormalizedCache::new());
    let prefs = Arc::new(Prefs::new());

    tokio::spawn(http::serve());
    wa::run(cache, prefs).await
}
