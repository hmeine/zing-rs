use axum::http;
use chrono::prelude::*;
use rand::distributions::{Alphanumeric, DistString};
use std::collections::HashMap;
use zing_game::zing_game::{ZingGame, ZingGamePoints};

pub type ErrorResponse = (http::StatusCode, &'static str);

fn random_id() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 16)
}

#[derive(Default)]
struct User {
    name: String,
    tables: Vec<String>,
}

struct Table {
    created_at: DateTime<Utc>,
    users: Vec<String>,
    game_results: Vec<ZingGamePoints>,
    game: Option<ZingGame>,
}

impl Table {
    pub fn games_have_started(&self) -> bool {
        self.game.is_some() || !self.game_results.is_empty()
    }
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

    pub fn whoami(&self, login_id: String) -> Option<String> {
        self.users.get(&login_id).map(|user| user.name.clone())
    }

    pub fn create_table(
        &mut self,
        login_id: String,
    ) -> Result<String, (http::StatusCode, &'static str)> {
        let user = self.users.get_mut(&login_id).ok_or((
            http::StatusCode::UNAUTHORIZED,
            "user not found (bad id cookie)",
        ))?;

        let table_id = random_id();

        user.tables.push(table_id.clone());
        self.tables.insert(
            table_id.clone(),
            Table {
                created_at: Utc::now(),
                users: vec![login_id],
                game_results: Vec::new(),
                game: None,
            },
        );

        Ok(table_id)
    }

    pub fn join_table(
        &mut self,
        login_id: String,
        table_id: String,
    ) -> Result<(), (http::StatusCode, &'static str)> {
        let user = self.users.get_mut(&login_id).ok_or((
            http::StatusCode::UNAUTHORIZED,
            "user not found (bad id cookie)",
        ))?;

        let table = self
            .tables
            .get_mut(&table_id)
            .ok_or((http::StatusCode::NOT_FOUND, "table id not found"))?;

        if user.tables.contains(&table_id) {
            return Err((http::StatusCode::CONFLICT, "trying to join table again"));
        }

        if table.games_have_started() {
            return Err((
                http::StatusCode::CONFLICT,
                "cannot join a table after games have started",
            ));
        }

        user.tables.push(table_id.clone());
        table.users.push(login_id);

        Ok(())
    }

    pub fn leave_table(
        &mut self,
        login_id: String,
        table_id: String,
    ) -> Result<(), (http::StatusCode, &'static str)> {
        let user = self.users.get_mut(&login_id).ok_or((
            http::StatusCode::UNAUTHORIZED,
            "user not found (bad id cookie)",
        ))?;

        let table = self
            .tables
            .get_mut(&table_id)
            .ok_or((http::StatusCode::NOT_FOUND, "table id not found"))?;

        let table_index_in_user = user.tables.iter().position(|id| *id == table_id).ok_or((
            http::StatusCode::UNAUTHORIZED,
            "trying to leave table before joining",
        ))?;

        if table.games_have_started() {
            return Err((
                http::StatusCode::CONFLICT,
                "cannot leave a table after games have started",
            ));
        }

        let user_index_in_table = table
            .users
            .iter()
            .position(|id| *id == login_id)
            .expect("inconsistent state");

        user.tables.remove(table_index_in_user);
        table.users.remove(user_index_in_table);
        if table.users.is_empty() {
            self.tables.remove(&table_id);
        }

        Ok(())
    }
}
