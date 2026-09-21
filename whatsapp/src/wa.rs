use maceio_cine_whatsapp::cache::NormalizedCache;
use maceio_cine_whatsapp::prefs::Prefs;
use std::path::Path;
use std::sync::Arc;
use whatsapp_rust::prelude::*;

use crate::handlers;

pub async fn run(cache: Arc<NormalizedCache>, prefs: Arc<Prefs>) -> anyhow::Result<()> {
    let session_path = std::env::var("SESSION_PATH").unwrap_or_else(|_| "data/whatsapp.db".into());
    if let Some(parent) = Path::new(&session_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    tracing::info!("Opening WhatsApp session store at {session_path}");
    let store = SqliteStore::new(&session_path).await?;

    let cache_h = cache.clone();
    let prefs_h = prefs.clone();

    let bot = Bot::builder()
        .with_backend(store)
        .on_qr_code(|code, timeout| async move {
            tracing::info!(
                "Scan this QR with a dedicated WhatsApp number (valid {}s):\n{code}",
                timeout.as_secs()
            );
            println!("\n{code}\n");
        })
        .on_connected(|_client| async {
            tracing::info!("WhatsApp connected");
        })
        .on_logged_out(|_info| async {
            tracing::error!("WhatsApp logged out — scan a new QR on next start");
        })
        .on_message(move |ctx| {
            let cache = cache_h.clone();
            let prefs = prefs_h.clone();
            async move {
                handlers::handle_message(ctx, cache, prefs).await;
            }
        })
        .build()
        .await?;

    tracing::info!("WhatsApp bot starting (dedicated number only; unofficial client, ban risk)");
    let handle = bot.spawn();
    whatsapp_rust::shutdown_signal().await;
    tracing::info!("Shutdown signal received");
    handle.shutdown().await;
    Ok(())
}
