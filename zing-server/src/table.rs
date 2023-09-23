use chrono::prelude::*;
use std::sync::Arc;

use serde::{Serialize, Serializer};
use zing_game::{
    client_notification::ClientNotification,
    game::{GamePhase, GameState},
    zing_game::{ZingGame, ZingGamePoints},
};

use crate::{
    client_connection::{ClientConnection, SerializedNotification, SerializedNotifications},
    game_error::GameError,
    user::User,
    ws_notifications::NotificationSenderHandle,
};

pub struct Table {
    pub created_at: DateTime<Utc>,
    pub players: Vec<Arc<User>>,
    pub connections: Vec<ClientConnection>,
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
            connections: Vec::new(),
            game_results: Vec::new(),
            game: None,
        }
    }

    pub fn games_have_started(&self) -> bool {
        self.game.is_some() || !self.game_results.is_empty()
    }

    pub fn user_index(&self, login_id: &str) -> Option<usize> {
        self.players
            .iter()
            .position(|player| player.login_id == login_id)
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
            self.game_status(&c.user.login_id)
                .expect("game should be started, so must have valid state"),
            self.user_index(&c.user.login_id).unwrap(),
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
                let known_actions = *c.actions_sent.read().expect("unexpected concurrency");
                if current_actions > known_actions {
                    let player_index = self.user_index(&c.user.login_id).unwrap();
                    *c.actions_sent.write().expect("unexpected concurrency") = current_actions;
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

    pub fn game_status(&self, login_id: &str) -> Option<GameState> {
        let player_index = self.user_index(login_id).unwrap();

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
        self.game_results.push(game.points());
        self.game = None;
        Ok(())
    }

    pub fn connection_opened(
        &mut self,
        user: Arc<User>,
        sender: NotificationSenderHandle,
    ) -> Option<SerializedNotification> {
        self.connections.push(ClientConnection::new(user, sender));
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
