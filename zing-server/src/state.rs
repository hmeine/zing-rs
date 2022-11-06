use axum::http;
use rand::distributions::{Alphanumeric, DistString};
use std::collections::HashMap;

fn random_id() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 16)
}

#[derive(Default)]
pub struct User {
    name: String,
    tables: Vec<String>,
}

#[derive(Default)]
pub struct Table {
    users: Vec<String>,
}

#[derive(Default)]
pub struct State {
    users: HashMap<String, User>,
    tables: HashMap<String, Table>,
}

impl State {
    pub fn login(&mut self, user_name: String) -> String {
        let login_id = random_id();
        self.users.insert(
            login_id.clone(),
            User {
                name: user_name,
                ..Default::default()
            },
        );
        login_id
    }

    pub fn create_table(
        &mut self,
        login_id: String,
    ) -> Result<String, (http::StatusCode, &'static str)> {
        let user = self.users.get_mut(&login_id).ok_or((
            http::StatusCode::UNAUTHORIZED,
            "login first (bad id cookie)",
        ))?;

        let table_id = random_id();
        
        user.tables.push(table_id.clone());
        self.tables.insert(
            table_id.clone(),
            Table {
                users: vec![login_id],
            },
        );

        Ok(table_id)
    }
}
