use std::sync::RwLock;

pub struct User {
    pub login_token: String,
    pub name: String,
    pub logged_in: RwLock<bool>,
    pub tables: RwLock<Vec<String>>,
}
