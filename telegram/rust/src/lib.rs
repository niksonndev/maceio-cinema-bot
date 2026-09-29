pub mod api;
pub mod cinemas;
pub mod data;
pub mod format;
pub mod handlers;
pub mod keyboards;
pub mod normalize;
pub mod prefs;
pub mod ratings;
pub mod store;
pub mod types;

use teloxide::types::Update;

pub fn decode_update(body: &[u8]) -> Result<Update, serde_json::Error> {
    serde_json::from_slice(body)
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
