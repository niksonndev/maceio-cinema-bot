use crate::cinemas::find_cinema_by_id;
use crate::store::JsonStore;
use crate::types::{Error, Result};
use std::collections::HashMap;

pub struct Preferences {
    data: HashMap<String, String>,
    store: JsonStore,
}

impl Preferences {
    pub async fn load() -> Result<Self> {
        let store = JsonStore::from_env("PREFS_KEY", "data/prefs.json").await?;
        let data = match store.load().await? {
            Some(bytes) => match serde_json::from_slice(&bytes) {
                Ok(data) => data,
                Err(error) => {
                    tracing::warn!("Preferences JSON is invalid; starting empty: {error}");
                    HashMap::new()
                }
            },
            None => HashMap::new(),
        };
        Ok(Self { data, store })
    }

    pub fn get_user_cinema(&self, chat_id: i64) -> Option<&'static crate::types::Cinema> {
        find_cinema_by_id(self.data.get(&chat_id.to_string())?)
    }

    pub async fn set_user_cinema(&mut self, chat_id: i64, theater_id: &str) -> Result<()> {
        self.data.insert(chat_id.to_string(), theater_id.to_owned());
        let data =
            serde_json::to_vec_pretty(&self.data).map_err(|error| Error::Msg(error.to_string()))?;
        self.store.save(data).await
    }
}
