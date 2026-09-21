use crate::cinemas::find_cinema_by_id;
use crate::types::Cinema;
use dashmap::DashMap;

#[derive(Default)]
pub struct Prefs {
    map: DashMap<String, String>,
}

impl Prefs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_user_cinema(&self, jid: &str, theater_id: &str) {
        self.map.insert(jid.to_string(), theater_id.to_string());
    }

    pub fn get_user_cinema(&self, jid: &str) -> Option<&'static Cinema> {
        let theater_id = self.map.get(jid)?;
        find_cinema_by_id(theater_id.as_str())
    }
}
