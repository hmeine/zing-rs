use chrono::prelude::*;
use std::sync::Arc;

use serde::{Serialize, Serializer};
use zing_game::{
    client_notification::ClientNotification,
    game::{GamePhase, GameState},
    zing_game::{ZingGame, ZingGamePoints},
};

use crate::{
    client_connection::{
        ClientConnection, ClientConnections, SerializedNotification, SerializedNotifications,
    },
    game_error::GameError,
    user::User,
    ws_notifications::NotificationSenderHandle,
};

pub struct Table {
    created_at: DateTime<Utc>,
    players: Vec<Arc<User>>,
    pub connections: ClientConnections,
    pub game_results: Vec<ZingGamePoints>,
    pub game: Option<ZingGame>,
}

#[derive(Serialize)]
pub struct TableInfo {
    pub id: String,
    #[serde(serialize_with = "serialize_datetime_as_iso8601")]
    pub created_at: DateTime<Utc>,
    pub user_names: Vec<String>,
    pub game_results: Vec<ZingGamePoints>,
    pub game: Option<GamePhase>,
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
    pub fn new(user: Arc<User>) -> Self {
        Self {
            created_at: Utc::now(),
            players: vec![user],
            connections: ClientConnections::new(),
            game_results: Vec::new(),
            game: None,
        }
    }

    pub fn table_info(&self, id: &str) -> TableInfo {
        TableInfo {
            id: id.to_owned(),
            created_at: self.created_at,
            user_names: self.players.iter().map(|user| user.name.clone()).collect(),
            game_results: self.game_results.clone(),
            game: self.game.as_ref().map(|game| game.state().phase()),
        }
    }

    pub fn games_have_started(&self) -> bool {
        self.game.is_some() || !self.game_results.is_empty()
    }

    pub fn has_logged_in_users(&self) -> bool {
        for user in &self.players {
            if *user.logged_in.read().expect("RwLock poisoned through panic") {
                return true;
            }
        }
        false
    }

    pub fn user_index(&self, login_token: &str) -> Option<usize> {
        self.players
            .iter()
            .position(|player| player.login_token == login_token)
    }

    pub fn user_joined(&mut self, user: Arc<User>) {
        self.players.push(user);
    }

    pub fn user_left(&mut self, login_token: &str) {
        let user_index_in_table = self
            .user_index(login_token)
            .expect("user_left() requires user to be present");
        self.players.remove(user_index_in_table);
        self.connections.remove_user_with_token(login_token);
    }

    pub fn start_game(&mut self) -> Result<(), GameError> {
        if self.game.is_some() {
            return Err(GameError::Conflict("game already started"));
        }

        let players_at_table = self.players.len();
        if (players_at_table != 2) && (players_at_table != 4) {
            return Err(GameError::Conflict(
                "game can only start when there are exactly two or four players present",
            ));
        }

        let names: Vec<String> = self.players.iter().map(|user| user.name.clone()).collect();
        let dealer_index = self.game_results.len() % names.len();
        self.game = Some(ZingGame::new_with_player_names(names, dealer_index));
        Ok(())
    }

    fn game_status_notification(&self, c: &ClientConnection) -> SerializedNotification {
        c.client_notification(&ClientNotification::GameStatus(
            self.game_status(c.client_login_token())
                .expect("game should be started, so must have valid state"),
            self.user_index(c.client_login_token()).unwrap(),
        ))
    }

    pub fn initial_game_status_messages(&self) -> SerializedNotifications {
        self.connections
            .iter()
            .map(|c| self.game_status_notification(c))
            .collect()
    }

    pub fn setup_game(&mut self) -> Result<(), GameError> {
        match &mut self.game {
            None => Err(GameError::Conflict("game not started yet")),
            Some(game) => {
                game.setup_game();
                Ok(())
            }
        }
    }

    pub fn action_notifications(&self) -> SerializedNotifications {
        let history = self
            .game
            .as_ref()
            .expect("action_notifications() called without active game")
            .history();
        let current_actions = history.len();

        self.connections
            .iter()
            .filter_map(|c| {
                let known_actions = *c.actions_sent.read().expect("RwLock poisoned through panic");
                if current_actions > known_actions {
                    let player_index = self.user_index(c.client_login_token()).unwrap();
                    *c.actions_sent.write().expect("RwLock poisoned through panic") = current_actions;
                    Some(
                        c.client_notification(&ClientNotification::CardActions(
                            history[known_actions..current_actions]
                                .iter()
                                .map(|action| action.new_view_for_player(player_index))
                                .collect(),
                        )),
                    )
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn game_status(&self, login_token: &str) -> Option<GameState> {
        let player_index = self.user_index(login_token).unwrap();

        self.game
            .as_ref()
            .map(|game| game.state().new_view_for_player(player_index))
    }

    pub fn finish_game(&mut self) -> Result<(), GameError> {
        let game = self
            .game
            .as_ref()
            .ok_or(GameError::Conflict("no active game"))?;
        if !game.finished() {
            return Err(GameError::Conflict("game still running"));
        }
        self.game = None;
        Ok(())
    }

    pub fn connection_opened(
        &mut self,
        user: Arc<User>,
        sender: NotificationSenderHandle,
    ) -> Option<SerializedNotification> {
        self.connections.add(user, sender);
        self.game.as_ref().map(|_game| {
            // add() cannot return this, because its self is mutable
            let new_conn = self.connections.last().unwrap();
            *new_conn.actions_sent.write().unwrap() = _game.history().len();
            self.game_status_notification(new_conn)
        })
    }
}
