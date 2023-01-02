use axum::{http, Json};
use chrono::prelude::*;
use rand::distributions::{Alphanumeric, DistString};
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use zing_game::{
    game::GameState,
    zing_game::{ZingGame, ZingGamePoints},
};

use crate::ws_notifications::NotificationSenderHandle;

pub type ErrorResponse = (http::StatusCode, &'static str);

fn random_id() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 16)
}

#[derive(Default)]
pub struct User {
    name: String,
    tables: Vec<String>,
}

pub struct Table {
    created_at: DateTime<Utc>,
    login_ids: Vec<String>,
    connections: Vec<Option<NotificationSenderHandle>>,
    game_results: Vec<ZingGamePoints>,
    game: Option<ZingGame>,
}

#[derive(Serialize)]
pub struct TableInfo {
    id: String,
    #[serde(serialize_with = "serialize_datetime_as_iso8601")]
    created_at: DateTime<Utc>,
    user_names: Vec<String>,
    game_results: Vec<ZingGamePoints>,
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

#[derive(Serialize)]
pub struct GameStatus {
    active: bool,
    ended: bool,
    state: Option<GameState>,
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
        for player_index in 0..self.login_ids.len() {
            if self.connections[player_index].is_some() {
                let msg = serde_json::to_string(&self.game_status(&self.login_ids[player_index]))
                    .unwrap();
                if self.connections[player_index]
                    .as_mut()
                    .unwrap()
                    .send(msg)
                    .is_err()
                {
                    self.connections[player_index] = None;
                };
            }
        }
        self.game.as_mut().unwrap().setup_game();
        Ok(())
    }

    pub fn game_status(&self, login_id: &str) -> GameStatus {
        let player_index = self.user_index(login_id).unwrap();

        GameStatus {
            active: self.game.is_some(),
            ended: self.game.as_ref().map_or(false, |game| game.finished()),
            state: self
                .game
                .as_ref()
                .map(|game| game.state().new_view_for_player(player_index)),
        }
    }

    pub fn finish_game(&mut self) -> Result<(), ErrorResponse> {
        let game = self
            .game
            .as_ref()
            .ok_or((http::StatusCode::CONFLICT, "no active game"))?;
        if !game.finished() {
            return Err((http::StatusCode::CONFLICT, "game still running"));
        }
        self.game_results.push(game.points());
        self.game = None;
        Ok(())
    }

    pub fn connection_opened(&mut self, user_index: usize, sender: NotificationSenderHandle) {
        self.connections[user_index] = Some(sender);
    }
}

#[derive(Default)]
pub struct ZingState {
    users: HashMap<String, User>,
    tables: HashMap<String, Table>,
}

impl ZingState {
    pub fn login(&mut self, user_name: &str) -> String {
        let login_id = random_id();
        self.users.insert(
            login_id.clone(),
            User {
                name: user_name.to_owned(),
                ..Default::default()
            },
        );
        login_id
    }

    pub fn logout(&mut self, login_id: &str) -> Result<(), ErrorResponse> {
        self.users
            .remove(login_id)
            .ok_or((
                http::StatusCode::UNAUTHORIZED,
                "user not found (bad id cookie)",
            ))
            .map(|_| ())
    }

    pub fn get_user(&self, login_id: &str) -> Result<&User, ErrorResponse> {
        self.users.get(login_id).ok_or((
            http::StatusCode::UNAUTHORIZED,
            "user not found (bad id cookie)",
        ))
    }

    pub fn get_user_mut(&mut self, login_id: &str) -> Result<&mut User, ErrorResponse> {
        self.users.get_mut(login_id).ok_or((
            http::StatusCode::UNAUTHORIZED,
            "user not found (bad id cookie)",
        ))
    }

    pub fn whoami(&self, login_id: &str) -> Option<String> {
        self.users.get(login_id).map(|user| user.name.clone())
    }

    pub fn table_info(&self, id: &str, table: &Table) -> TableInfo {
        TableInfo {
            id: id.to_owned(),
            created_at: table.created_at,
            user_names: table
                .login_ids
                .iter()
                .map(|id| self.get_user(id).unwrap().name.clone())
                .collect(),
            game_results: table.game_results.clone(),
        }
    }

    pub fn create_table(&mut self, login_id: &str) -> Result<Json<TableInfo>, ErrorResponse> {
        let table_id = random_id();

        let user = self.get_user_mut(login_id)?;
        user.tables.push(table_id.clone());

        let table = Table {
            created_at: Utc::now(),
            login_ids: vec![login_id.to_owned()],
            connections: vec![None],
            game_results: Vec::new(),
            game: None,
        };
        let result = self.table_info(&table_id, &table);

        self.tables.insert(table_id, table);

        Ok(Json(result))
    }

    pub fn list_tables(&self, login_id: &str) -> Result<Json<Vec<TableInfo>>, ErrorResponse> {
        let user = self.get_user(login_id)?;

        let table_infos = user
            .tables
            .iter()
            .map(|table_id| self.table_info(table_id, self.tables.get(table_id).unwrap()))
            .collect::<Vec<_>>();

        Ok(Json(table_infos))
    }

    pub fn join_table(&mut self, login_id: &str, table_id: &str) -> Result<Json<TableInfo>, ErrorResponse> {
        let user = self.get_user(login_id)?;
        let table_id = table_id.to_owned();
        if user.tables.contains(&table_id) {
            return Err((http::StatusCode::CONFLICT, "trying to join table again"));
        }

        let table = self
            .tables
            .get_mut(&table_id)
            .ok_or((http::StatusCode::NOT_FOUND, "table id not found"))?;

        if table.games_have_started() {
            return Err((
                http::StatusCode::CONFLICT,
                "cannot join a table after games have started",
            ));
        }

        table.login_ids.push(login_id.to_owned());
        table.connections.push(None);
        self.get_user_mut(login_id)?.tables.push(table_id.clone());
        
        let table = self.tables.get(&table_id).unwrap();
        let result = self.table_info(&table_id, &table);

        Ok(Json(result))
    }

    pub fn leave_table(&mut self, login_id: &str, table_id: &str) -> Result<(), ErrorResponse> {
        let user = self.get_user(login_id)?;

        let table_index_in_user = user.tables.iter().position(|id| *id == table_id).ok_or((
            http::StatusCode::UNAUTHORIZED,
            "trying to leave table before joining",
        ))?;

        let table = self
            .tables
            .get_mut(table_id)
            .ok_or((http::StatusCode::NOT_FOUND, "table id not found"))?;

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

        table.login_ids.remove(user_index_in_table);
        table.connections.remove(user_index_in_table);
        if table.login_ids.is_empty() {
            self.tables.remove(table_id);
        }
        self.get_user_mut(login_id)?
            .tables
            .remove(table_index_in_user);

        Ok(())
    }

    pub fn start_game(&mut self, login_id: &str, table_id: &str) -> Result<(), ErrorResponse> {
        self.get_user(login_id)?;

        let table = self
            .tables
            .get_mut(table_id)
            .ok_or((http::StatusCode::NOT_FOUND, "table id not found"))?;

        table.user_index(login_id).ok_or((
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

    pub fn game_status(
        &self,
        login_id: &str,
        table_id: &str,
    ) -> Result<Json<GameStatus>, ErrorResponse> {
        self.get_user(login_id)?;

        let table = self
            .tables
            .get(table_id)
            .ok_or((http::StatusCode::NOT_FOUND, "table id not found"))?;

        table
            .user_index(login_id)
            .ok_or((http::StatusCode::NOT_FOUND, "user has not joined table"))?;

        Ok(Json(table.game_status(login_id)))
    }

    pub fn finish_game(&mut self, login_id: &str, table_id: &str) -> Result<(), ErrorResponse> {
        self.get_user(login_id)?;

        let table = self
            .tables
            .get_mut(table_id)
            .ok_or((http::StatusCode::NOT_FOUND, "table id not found"))?;

        table.user_index(login_id).ok_or((
            http::StatusCode::NOT_FOUND,
            "user has not joined table at which game should start",
        ))?;

        table.finish_game()
    }

    pub fn check_user_can_connect(
        &self,
        login_id: &str,
        table_id: &str,
    ) -> Result<bool, ErrorResponse> {
        self.get_user(login_id)?;

        let table = self
            .tables
            .get(table_id)
            .ok_or((http::StatusCode::NOT_FOUND, "table id not found"))?;

        table.user_index(login_id).ok_or((
            http::StatusCode::NOT_FOUND,
            "connecting user has not joined table",
        ))?;

        Ok(true)
    }

    pub fn add_user_connection(
        &mut self,
        login_id: String,
        table_id: String,
        sender: NotificationSenderHandle,
    ) {
        // it would be nice if we could socket.close() if the following expression is false:
        self.tables.get_mut(&table_id).map_or(false, |table| {
            table.user_index(&login_id).map_or(false, |user_index| {
                table.connection_opened(user_index, sender);
                true
            })
        });
    }

    //    loop {
    //        if socket
    //            .send(Message::Text(String::from("Hi!")))
    //            .await
    //            .is_err()
    //        {
    //            println!("client disconnected");
    //            return;
    //        }
    //        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    //    }

    pub fn play_card(
        &mut self,
        login_id: &str,
        table_id: &str,
        card_index: usize,
    ) -> Result<(), ErrorResponse> {
        self.get_user(login_id)?;

        let table = self
            .tables
            .get_mut(table_id)
            .ok_or((http::StatusCode::NOT_FOUND, "table id not found"))?;

        let player = table.user_index(login_id).ok_or((
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
