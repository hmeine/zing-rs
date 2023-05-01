use axum::{http, Json};
use chrono::prelude::*;
use rand::distributions::{Alphanumeric, DistString};
use serde::{Serialize, Serializer};
use tracing::{debug, info};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use zing_game::{
    client_notification::ClientNotification,
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
    actions_sent: RwLock<usize>,
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

        let players_at_table = self.players.len();
        if (players_at_table != 2) && (players_at_table != 4) {
            return Err((
                http::StatusCode::CONFLICT,
                "game can only start when there are exactly two or four players present",
            ));
        }

        let names: Vec<String> = self.players.iter().map(|user| user.name.clone()).collect();
        let dealer_index = self.game_results.len() % names.len();
        self.game = Some(ZingGame::new_with_player_names(names, dealer_index));
        Ok(())
    }

    fn game_status_notification(&self, c: &ClientConnection) -> TableNotification {
        c.notification(
            serde_json::to_string(&ClientNotification::GameStatus(
                self.game_status(&c.player.login_id)
                    .expect("game should be started, so must have valid state"),
                self.user_index(&c.player.login_id).unwrap(),
            ))
            .unwrap(),
        )
    }

    pub fn initial_game_status_messages(&self) -> TableNotifications {
        self.connections
            .iter()
            .map(|c| self.game_status_notification(c))
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

    pub fn action_notifications(&self) -> TableNotifications {
        let history = self
            .game
            .as_ref()
            .expect("action_notifications() called without active game")
            .history();
        let current_actions = history.len();

        self.connections
            .iter()
            .filter_map(|c| {
                let known_actions = *c.actions_sent.read().expect("unexpected concurrency");
                if current_actions > known_actions {
                    let player_index = self.user_index(&c.player.login_id).unwrap();
                    *c.actions_sent.write().expect("unexpected concurrency") = current_actions;
                    Some(
                        c.notification(
                            serde_json::to_string(&ClientNotification::CardActions(
                                history[known_actions..current_actions]
                                    .iter()
                                    .map(|action| action.new_view_for_player(player_index))
                                    .collect(),
                            ))
                            .unwrap(),
                        ),
                    )
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn game_status(&self, login_id: &str) -> Option<GameState> {
        let player_index = self.user_index(login_id).unwrap();

        self.game
            .as_ref()
            .map(|game| game.state().new_view_for_player(player_index))
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

    pub fn connection_opened(
        &mut self,
        user: Arc<User>,
        connection: NotificationSenderHandle,
    ) -> Option<TableNotification> {
        self.connections.push(ClientConnection {
            connection_id: random_id(),
            player: user,
            sender: connection,
            actions_sent: RwLock::new(0),
        });
        self.game.as_ref().map(|_game| {
            let new_conn = self.connections.last().unwrap();
            *new_conn.actions_sent.write().unwrap() = _game.history().len();
            self.game_status_notification(new_conn)
        })
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

        let result = self.table_info(table_id, table);

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
        table.players.push(user);

        let table = self.tables.get(&table_id).unwrap();
        let result = self.table_info(&table_id, table);

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
        let notifications = {
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
            table.initial_game_status_messages()
        };

        // send initial card notifications
        Self::send_notifications(notifications, state, table_id).await;

        // finally, perform first dealer card actions
        let notifications;
        {
            let mut state = state.write().unwrap();
            let table = state.tables.get_mut(table_id).unwrap();
            table.setup_game()?;
            notifications = table.action_notifications();
        }

        // send notifications about dealer actions
        Self::send_notifications(notifications, state, table_id).await;

        Ok(())
    }

    pub async fn send_notifications(
        notifications: TableNotifications,
        state: &RwLock<ZingState>,
        table_id: &str,
    ) {
        // send notifications (async, we don't want to hold the state locked)
        let mut broken_connections = Vec::new();
        for (connection_id, msg, connection) in notifications {
            debug!(
                "sending notification to {} ({})",
                &connection_id,
                &msg[..30]
            );
            if connection.send(msg).await.is_err() {
                info!("removing broken client connection");
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
    ) -> Result<Json<GameState>, ErrorResponse> {
        self.get_user(login_id)?;

        let table = self
            .tables
            .get(table_id)
            .ok_or((http::StatusCode::NOT_FOUND, "table id not found"))?;

        table
            .user_index(login_id)
            .ok_or((http::StatusCode::NOT_FOUND, "user has not joined table"))?;

        table.game_status(login_id).map_or(
            Err((http::StatusCode::NOT_FOUND, "no game active")),
            |game| Ok(Json(game)),
        )
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

    pub async fn add_user_connection(
        state: &RwLock<ZingState>,
        login_id: String,
        table_id: String,
        sender: NotificationSenderHandle,
    ) {
        let mut notification = None;
        {
            let mut self_ = state.write().unwrap();

            // it would be nice if we could socket.close() if the following expression is false:
            self_.get_user(&login_id).map_or(false, |user| {
                self_.tables.get_mut(&table_id).map_or(false, |table| {
                    notification = table.connection_opened(user, sender);
                    true
                })
            });
        }

        if let Some(notification) = notification {
            Self::send_notifications(vec![notification], state, &table_id).await;
        }
    }

    pub async fn play_card(
        state: &RwLock<ZingState>,
        login_id: &str,
        table_id: &str,
        card_index: usize,
    ) -> Result<(), ErrorResponse> {
        let notifications;
        let result;

        {
            let mut self_ = state.write().unwrap();

            self_.get_user(login_id)?;

            let table = self_
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

            result = game
                .play_card(player, card_index)
                .map_err(|msg| (http::StatusCode::CONFLICT, msg));

            notifications = table.action_notifications();
        }

        // send notifications about performed actions
        Self::send_notifications(notifications, state, table_id).await;

        result
    }
}
