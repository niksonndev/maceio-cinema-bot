pub mod api;
pub mod cinemas;
pub mod data;
pub mod format;
pub mod handlers;
pub mod keyboards;
pub mod normalize;
pub mod poster;
pub mod prefs;
pub mod ratings;
pub mod store;
pub mod text;
pub mod types;

use crate::types::{Error, Result};
use std::sync::OnceLock;
use teloxide::types::Update;
use teloxide::Bot;

pub fn decode_update(body: &[u8]) -> std::result::Result<Update, serde_json::Error> {
    serde_json::from_slice(body)
}

/// Process-wide Telegram client so warm Lambda invocations reuse the HTTP
/// connection pool instead of performing a new TLS handshake per update.
pub fn bot() -> Result<&'static Bot> {
    static BOT: OnceLock<Bot> = OnceLock::new();
    if let Some(bot) = BOT.get() {
        return Ok(bot);
    }
    let token = std::env::var("TELEGRAM_BOT_TOKEN")
        .map_err(|error| Error::Msg(format!("TELEGRAM_BOT_TOKEN: {error}")))?;
    Ok(BOT.get_or_init(|| Bot::new(token)))
}

#[cfg(test)]
mod tests {
    use super::decode_update;

    #[test]
    fn decodes_api_gateway_webhook_fixture() {
        let event: serde_json::Value =
            serde_json::from_str(include_str!("../../events/webhook-event.json")).unwrap();
        let body = event["body"].as_str().unwrap();
        let update = decode_update(body.as_bytes()).unwrap();

        assert_eq!(update.id.0, 1);
    }
}
