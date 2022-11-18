use axum::{
    extract::ws::{Message, WebSocket},
    http::{
        self,
        header::{self, HeaderName},
    },
    response::IntoResponse,
    Json,
};
use chrono::prelude::*;
use futures::stream::SplitSink;
use rand::distributions::{Alphanumeric, DistString};
use serde::{Serialize, Serializer};
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

#[derive(Serialize)]
pub struct Table {
    #[serde(serialize_with = "serialize_datetime_as_iso8601")]
    created_at: DateTime<Utc>,
    login_ids: Vec<String>,
    //#[serde(skip)]
    //connections: Vec<Option<SplitSink<WebSocket, Message>>>,
    game_results: Vec<ZingGamePoints>,
    #[serde(skip)]
    game: Option<ZingGame>,
}

fn serialize_datetime_as_iso8601<S>(
    datetime: &DateTime<Utc>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = format!("{}", datetime.format("%+"));
    serializer.serialize_str(&s)
}

impl Table {
    pub fn games_have_started(&self) -> bool {
        self.game.is_some() || !self.game_results.is_empty()
    }

    pub fn user_index(&self, login_id: &str) -> Option<usize> {
        self.login_ids.iter().position(|id| *id == login_id)
    }

    pub fn start_game(&mut self, names: Vec<String>) -> Result<(), ErrorResponse> {
        if self.game.is_some() {
            return Err((http::StatusCode::CONFLICT, "game already started"));
        }

        let dealer = self.game_results.len() % names.len();
        self.game = Some(ZingGame::new_with_player_names(names, dealer));
        self.game.as_mut().unwrap().setup_game();
        Ok(())
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

    pub fn create_table(&mut self, login_id: String) -> Result<String, ErrorResponse> {
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
                login_ids: vec![login_id],
                game_results: Vec::new(),
                game: None,
            },
        );

        Ok(table_id)
    }

    pub fn list_tables(&self, login_id: String) -> Result<impl IntoResponse, ErrorResponse> {
        let user = self.users.get(&login_id).ok_or((
            http::StatusCode::UNAUTHORIZED,
            "user not found (bad id cookie)",
        ))?;

        Ok((
            [(header::CONTENT_TYPE, "application/json")],
            serde_json::to_string(
                &user
                    .tables
                    .iter()
                    .map(|table_id| (table_id, self.tables.get(table_id).unwrap()))
                    .collect::<HashMap<_, _>>(),
            )
            .unwrap(),
        ))
    }

    pub fn join_table(&mut self, login_id: String, table_id: String) -> Result<(), ErrorResponse> {
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
        table.login_ids.push(login_id);

        Ok(())
    }

    pub fn leave_table(&mut self, login_id: String, table_id: String) -> Result<(), ErrorResponse> {
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
            .login_ids
            .iter()
            .position(|id| *id == login_id)
            .expect("inconsistent state");

        user.tables.remove(table_index_in_user);
        table.login_ids.remove(user_index_in_table);
        if table.login_ids.is_empty() {
            self.tables.remove(&table_id);
        }

        Ok(())
    }

    pub fn start_game(&mut self, login_id: String, table_id: String) -> Result<(), ErrorResponse> {
        let user = self.users.get(&login_id).ok_or((
            http::StatusCode::UNAUTHORIZED,
            "user not found (bad id cookie)",
        ))?;

        let table = self
            .tables
            .get_mut(&table_id)
            .ok_or((http::StatusCode::NOT_FOUND, "table id not found"))?;

        table.user_index(&login_id).ok_or((
            http::StatusCode::NOT_FOUND,
            "user has not joined table at which game should start",
        ))?;

        table.start_game(
            table
                .login_ids
                .iter()
                .map(|login_id| self.users.get(login_id).unwrap().name.clone())
                .collect(),
        )
    }

    pub fn play_card(
        &mut self,
        login_id: String,
        table_id: String,
        card_index: usize,
    ) -> Result<(), ErrorResponse> {
        self.users.get(&login_id).ok_or((
            http::StatusCode::UNAUTHORIZED,
            "user not found (bad id cookie)",
        ))?;

        let table = self
            .tables
            .get_mut(&table_id)
            .ok_or((http::StatusCode::NOT_FOUND, "table id not found"))?;

        let player = table.user_index(&login_id).ok_or((
            http::StatusCode::NOT_FOUND,
            "user has not joined table at which game should start",
        ))?;

        let game = table
            .game
            .as_mut()
            .ok_or((http::StatusCode::CONFLICT, "game not started yet"))?;

        game.play_card(player, card_index)
            .map_err(|msg| (http::StatusCode::CONFLICT, msg))
    }
}
