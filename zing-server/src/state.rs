use axum::{http, Json};
use chrono::prelude::*;
use rand::distributions::{Alphanumeric, DistString};
use serde::{Serialize, Serializer};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use zing_game::{
    game::GameState,
    zing_game::{ZingGame, ZingGamePoints},
};

use crate::ws_notifications::NotificationSenderHandle;

pub type ErrorResponse = (http::StatusCode, &'static str);

type TableNotification = (String, String, NotificationSenderHandle);
type TableNotifications = Vec<TableNotification>;

fn random_id() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 16)
}

pub struct User {
    login_id: String,
    name: String,
    logged_in: RwLock<bool>,
    tables: RwLock<Vec<String>>,
}

struct ClientConnection {
    connection_id: String,
    player: Arc<User>,
    sender: NotificationSenderHandle,
}

impl ClientConnection {
    pub fn notification(&self, msg: String) -> TableNotification {
        (self.connection_id.clone(), msg, self.sender.clone())
    }
}

pub struct Table {
    created_at: DateTime<Utc>,
    players: Vec<Arc<User>>,
    connections: Vec<ClientConnection>,
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
        self.players
            .iter()
            .position(|player| player.login_id == login_id)
    }

    pub fn start_game(&mut self) -> Result<(), ErrorResponse> {
        if self.game.is_some() {
            return Err((http::StatusCode::CONFLICT, "game already started"));
        }

        let names: Vec<String> = self.players.iter().map(|user| user.name.clone()).collect();
        let dealer_index = self.game_results.len() % names.len();
        self.game = Some(ZingGame::new_with_player_names(names, dealer_index));
        Ok(())
    }

    pub fn initial_game_status_messages(&self) -> TableNotifications {
        self.connections
            .iter()
            .map(|c| {
                c.notification(
                    serde_json::to_string(&self.game_status(&c.player.login_id)).unwrap(),
                )
            })
            .collect()
    }

    pub fn setup_game(&mut self) -> Result<(), ErrorResponse> {
        match &mut self.game {
            None => Err((http::StatusCode::CONFLICT, "game not started yet")),
            Some(game) => {
                game.setup_game();
                Ok(())
            }
        }
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

    pub fn connection_opened(&mut self, user: Arc<User>, connection: NotificationSenderHandle) {
        self.connections.push(ClientConnection {
            connection_id: random_id(),
            player: user,
            sender: connection,
        });
    }

    pub fn connection_closed(&mut self, connection_id: String) {
        for (i, c) in self.connections.iter().enumerate() {
            if c.connection_id == connection_id {
                self.connections.remove(i);
                break;
            }
        }
    }
}

#[derive(Default)]
pub struct ZingState {
    users: HashMap<String, Arc<User>>,
    tables: HashMap<String, Table>,
}

impl ZingState {
    pub fn login(&mut self, user_name: &str) -> String {
        let login_id = random_id();
        self.users.insert(
            login_id.clone(),
            Arc::new(User {
                login_id: login_id.clone(),
                name: user_name.to_owned(),
                logged_in: RwLock::new(true),
                tables: RwLock::new(Vec::new()),
            }),
        );
        login_id
    }

    pub fn logout(&mut self, login_id: &str) -> Result<(), ErrorResponse> {
        self.users
            .remove_entry(login_id)
            .ok_or((
                http::StatusCode::UNAUTHORIZED,
                "user not found (bad id cookie)",
            ))
            .map(|(_, user)| {
                *user.logged_in.write().expect("unexpected concurrency") = false;
                // TODO: remove table if all users have left or logged out
            })
    }

    pub fn get_user(&self, login_id: &str) -> Result<Arc<User>, ErrorResponse> {
        self.users.get(login_id).map_or(
            Err((
                http::StatusCode::UNAUTHORIZED,
                "user not found (bad id cookie)",
            )),
            |user| Ok(user.clone()),
        )
    }

    pub fn whoami(&self, login_id: &str) -> Option<String> {
        self.users.get(login_id).map(|user| user.name.clone())
    }

    pub fn table_info(&self, id: &str, table: &Table) -> TableInfo {
        TableInfo {
            id: id.to_owned(),
            created_at: table.created_at,
            user_names: table.players.iter().map(|user| user.name.clone()).collect(),
            game_results: table.game_results.clone(),
        }
    }

    pub fn create_table(&mut self, login_id: &str) -> Result<Json<TableInfo>, ErrorResponse> {
        let table_id = random_id();

        let user = self.get_user(login_id)?;
        user.tables
            .write()
            .expect("unexpected concurrency")
            .push(table_id.clone());

        let table = Table {
            created_at: Utc::now(),
            players: vec![user],
            connections: Vec::new(),
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
            .read()
            .expect("unexpected concurrency")
            .iter()
            .map(|table_id| self.table_info(table_id, self.tables.get(table_id).unwrap()))
            .collect::<Vec<_>>();

        Ok(Json(table_infos))
    }

    pub fn get_table(
        &self,
        login_id: &str,
        table_id: &str,
    ) -> Result<Json<TableInfo>, ErrorResponse> {
        self.get_user(login_id)?;

        let table = self
            .tables
            .get(table_id)
            .ok_or((http::StatusCode::NOT_FOUND, "table id not found"))?;

        let result = self.table_info(table_id, &table);

        Ok(Json(result))
    }

    pub fn join_table(
        &mut self,
        login_id: &str,
        table_id: &str,
    ) -> Result<Json<TableInfo>, ErrorResponse> {
        let user = self.get_user(login_id)?;
        let table_id = table_id.to_owned();
        if user
            .tables
            .read()
            .expect("unexpected concurrency")
            .contains(&table_id)
        {
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

        user.tables
            .write()
            .expect("unexpected concurrency")
            .push(table_id.clone());
        table.players.push(user.clone());

        let table = self.tables.get(&table_id).unwrap();
        let result = self.table_info(&table_id, &table);

        Ok(Json(result))
    }

    pub fn leave_table(&mut self, login_id: &str, table_id: &str) -> Result<(), ErrorResponse> {
        let user = self.get_user(login_id)?;

        let table_index_in_user = user
            .tables
            .read()
            .expect("unexpected concurrency")
            .iter()
            .position(|id| *id == table_id)
            .ok_or((
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

        let user_index_in_table = table.user_index(login_id).expect("inconsistent state");

        // TODO: remove table if all remaining users are logged out
        table.players.remove(user_index_in_table);
        table.connections.remove(user_index_in_table);
        if table.players.is_empty() {
            self.tables.remove(table_id);
        }
        user.tables
            .write()
            .expect("unexpected concurrency")
            .remove(table_index_in_user);

        Ok(())
    }

    pub async fn start_game(
        state: &RwLock<ZingState>,
        login_id: &str,
        table_id: &str,
    ) -> Result<(), ErrorResponse> {
        // start a game (sync code), collect initial game status notifications
        let notifications;
        {
            let self_ = state.read().unwrap();
            self_.get_user(login_id)?;

            let table = self_
                .tables
                .get(table_id)
                .ok_or((http::StatusCode::NOT_FOUND, "table id not found"))?;

            table.user_index(login_id).ok_or((
                http::StatusCode::NOT_FOUND,
                "user has not joined table at which game should start",
            ))?;

            drop(self_);
            let mut self_ = state.write().unwrap();

            let table = self_.tables.get_mut(table_id).unwrap();
            table.start_game()?;
            notifications = table.initial_game_status_messages();
        }

        // send initial card notifications
        Self::send_notifications(notifications, state, table_id).await;

        // finally, perform first dealer card actions (TODO: notifications)
        let mut state = state.write().unwrap();
        let table = state.tables.get_mut(table_id).unwrap();
        table.setup_game()
    }

    pub async fn send_notifications(
        notifications: TableNotifications,
        state: &RwLock<ZingState>,
        table_id: &str,
    ) {
        // send notifications (async, we don't want to hold the state locked)
        let mut broken_connections = Vec::new();
        for (connection_id, msg, connection) in notifications {
            if connection.send(msg).await.is_err() {
                broken_connections.push(connection_id);
            };
        }

        // lock the state again to remove broken connections:
        let mut state = state.write().unwrap();
        let table = state.tables.get_mut(table_id).unwrap();
        for connection_id in broken_connections {
            table.connection_closed(connection_id);
        }
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
        self.get_user(&login_id).map_or(false, |user| {
            self.tables.get_mut(&table_id).map_or(false, |table| {
                table.connection_opened(user, sender);
                true
            })
        });
    }

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
