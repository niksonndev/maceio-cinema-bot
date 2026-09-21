use axum::{routing::get, Json, Router};
use serde_json::{json, Value};
use std::time::Duration;

async fn health() -> Json<Value> {
    Json(json!({
        "status": "Bot is online!",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

async fn keepalive(url: String) {
    let client = reqwest::Client::new();
    let mut interval = tokio::time::interval(Duration::from_secs(10 * 60));
    loop {
        interval.tick().await;
        match client.get(&url).send().await {
            Ok(res) => tracing::info!("Auto-ping {url} → {}", res.status()),
            Err(err) => tracing::warn!("Auto-ping failed: {err}"),
        }
    }
}

pub async fn serve() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(10000);

    if let Ok(url) = std::env::var("RENDER_EXTERNAL_URL") {
        if !url.is_empty() {
            tracing::info!("Auto-ping every 10 min → {url}");
            tokio::spawn(keepalive(url));
        }
    }

    let app = Router::new().route("/", get(health));
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("bind health port");
    tracing::info!("Health check on 0.0.0.0:{port}");
    axum::serve(listener, app).await.expect("health server");
}
