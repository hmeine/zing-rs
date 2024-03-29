use sea_orm::{prelude::*, ActiveValue, Order, QueryOrder};

use serde::{Serialize, Serializer};
use zing_game::{
    client_notification::ClientNotification,
    game::{GamePhase, GameState},
    zing_game::{ZingGame, ZingGamePoints},
};

use crate::util::random_id;
use crate::{
    client_connection::{
        ClientConnection, ClientConnections, SerializedNotification, SerializedNotifications,
    },
    entities,
    entities::prelude::*,
    game_error::GameError,
    ws_notifications::NotificationSenderHandle,
};

pub struct LoadedTable {
    table: entities::table::Model,
    // ATTENTION: user entities will not be kept up to date; only use this for
    // list of players (with id/token/name):
    players: Vec<entities::user::Model>,
    pub connections: ClientConnections,
    pub game_results: Vec<ZingGamePoints>,
    pub game: Option<ZingGame>,
}

#[derive(Serialize)]
pub struct TableInfo {
    pub id: String,
    #[serde(serialize_with = "serialize_datetime_as_iso8601")]
    pub created_at: DateTimeWithTimeZone,
    pub user_names: Vec<String>,
    pub game_results: Vec<ZingGamePoints>,
    pub game: Option<GamePhase>,
}

fn serialize_datetime_as_iso8601<S>(
    datetime: &DateTimeWithTimeZone,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = format!("{}", datetime.format("%+"));
    serializer.serialize_str(&s)
}

impl LoadedTable {
    pub async fn create_for_user(
        user: entities::user::Model,
        db_conn: &DatabaseConnection,
    ) -> Result<Self, GameError> {
        let table_token = random_id();

        // insert new table entity
        let table = entities::table::ActiveModel {
            token: ActiveValue::Set(table_token.clone()),
            ..Default::default()
        }
        .insert(db_conn)
        .await
        .map_err(|_| GameError::DBError("DB insert failed unexpectedly"))?;

        // insert new table_join entity for this user
        entities::table_join::ActiveModel {
            user_id: ActiveValue::Set(user.id),
            table_id: ActiveValue::Set(table.id),
            table_pos: ActiveValue::Set(0),
        }
        .insert(db_conn)
        .await
        .map_err(|_| GameError::DBError("DB insert failed unexpectedly"))?;

        Ok(Self {
            table,
            players: vec![user],
            connections: ClientConnections::new(),
            game_results: Vec::new(),
            game: None,
        })
    }

    pub async fn new_from_db(table: entities::table::Model, db_conn: &DatabaseConnection) -> Self {
        let players = table
            .find_related(User)
            .order_by(entities::table_join::Column::TablePos, Order::Asc)
            .all(db_conn)
            .await
            .unwrap();

        let game_results = GameResults::find()
            .filter(entities::game_results::Column::TableId.eq(table.id))
            .all(db_conn)
            .await
            .unwrap()
            .into_iter()
            .map(|game_results| ZingGamePoints {
                card_points: (
                    game_results.card_points0 as u32,
                    game_results.card_points1 as u32,
                ),
                card_count_points: (
                    game_results.card_count_points0 as u32,
                    game_results.card_count_points1 as u32,
                ),
                zing_points: (
                    game_results.zing_points0 as u32,
                    game_results.zing_points1 as u32,
                ),
            })
            .collect();

        let game = table
            .game
            .clone()
            .map(|game_json| serde_json::from_value(game_json).unwrap());

        Self {
            table,
            players,
            connections: ClientConnections::new(),
            game_results,
            game,
        }
    }

    pub fn table(&self) -> entities::table::Model {
        self.table.clone()
    }

    pub fn token(&self) -> String {
        self.table.token.clone()
    }

    pub fn game_json(&self) -> Option<Json> {
        self.game
            .as_ref()
            .map(|game| serde_json::to_value(game).expect("JSON conversion should not be fallible"))
    }

    fn player_names(&self) -> Vec<String> {
        self.players.iter().map(|user| user.name.clone()).collect()
    }

    pub fn table_info(&self) -> TableInfo {
        TableInfo {
            id: self.table.token.to_owned(),
            created_at: self.table.created_at,
            user_names: self.player_names(),
            game_results: self.game_results.clone(),
            game: self.game.as_ref().map(|game| game.state().phase()),
        }
    }

    pub fn games_have_started(&self) -> bool {
        self.game.is_some() || !self.game_results.is_empty()
    }

    pub fn user_left(&mut self, player_index: usize) {
        self.players.remove(player_index);
    }

    pub fn user_joined(&mut self, user: &entities::user::Model) {
        self.players.push(user.clone());
    }

    pub fn start_game(&mut self) -> Result<(), GameError> {
        if self.game.is_some() {
            return Err(GameError::Conflict("game already started"));
        }

        let names = self.player_names();
        let players_at_table = names.len();
        if (players_at_table != 2) && (players_at_table != 4) {
            return Err(GameError::Conflict(
                "game can only start when there are exactly two or four players present",
            ));
        }

        let dealer_index = self.game_results.len() % names.len();
        self.game = Some(ZingGame::new_with_player_names(names, dealer_index));

        // TODO: move game JSON storing code here after finding out how to
        // return it as closure / future

        Ok(())
    }

    pub fn player_index(&self, login_token: &str) -> Option<usize> {
        self.players
            .iter()
            .position(|user| user.token == login_token)
    }

    fn game_status_notification(&self, c: &ClientConnection) -> SerializedNotification {
        c.client_notification(&ClientNotification::GameStatus(
            self.game_status(c.client_login_token())
                .expect("game should be started, so must have valid state"),
            self.player_index(c.client_login_token()).unwrap(), // FIXME: include in game status result?
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
                let known_actions = *c
                    .actions_sent
                    .read()
                    .expect("RwLock poisoned through panic");
                {
                    if current_actions > known_actions {
                        let player_index = self.player_index(c.client_login_token()).unwrap();
                        *c.actions_sent
                            .write()
                            .expect("RwLock poisoned through panic") = current_actions;
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
                }
            })
            .collect()
    }

    pub fn game_status(&self, login_token: &str) -> Option<GameState> {
        let player_index = self.player_index(login_token);

        self.game
            .as_ref()
            .map(|game| game.state().new_view_for_player(player_index.unwrap()))
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
        user: entities::user::Model,
        sender: NotificationSenderHandle,
    ) -> Option<SerializedNotification> {
        self.connections.add(user, sender);
        // self.game.as_ref().map(..) would be more ideomatic, but does not work
        // with the .await yet:
        if let Some(game) = self.game.as_ref() {
            // add() cannot return this, because its self is mutable
            let new_conn = self.connections.last().unwrap();
            *new_conn.actions_sent.write().unwrap() = game.history().len();
            Some(self.game_status_notification(new_conn))
        } else {
            None
        }
    }
}
