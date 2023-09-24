use axum::Json;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tracing::{debug, info};
use zing_game::game::GameState;

use crate::{
    client_connection::{ClientConnections, SerializedNotifications},
    game_error::GameError,
    table::{Table, TableInfo},
    user::User,
    util::random_id,
    ws_notifications::NotificationSenderHandle,
};

#[derive(Default)]
pub struct ZingState {
    users: HashMap<String, Arc<User>>,
    tables: HashMap<String, Table>,
    connections: ClientConnections,
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

    pub fn logout(&mut self, login_id: &str) -> Result<(), GameError> {
        self.users
            .remove_entry(login_id)
            .ok_or(GameError::Unauthorized("user not found (bad id cookie)"))
            .map(|(_, user)| {
                *user.logged_in.write().expect("unexpected concurrency") = false;
                // TODO: remove table if all users have left or logged out
            })
    }

    pub fn get_user(&self, login_id: &str) -> Result<Arc<User>, GameError> {
        self.users.get(login_id).map_or(
            Err(GameError::Unauthorized("user not found (bad id cookie)")),
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
            game: table.game.as_ref().map(|game| game.state().phase()),
        }
    }

    pub fn create_table(&mut self, login_id: &str) -> Result<Json<TableInfo>, GameError> {
        let table_id = random_id();

        let user = self.get_user(login_id)?;
        user.tables
            .write()
            .expect("unexpected concurrency")
            .push(table_id.clone());

        let table = Table::new(user);
        let table_info = self.table_info(&table_id, &table);

        self.tables.insert(table_id, table);

        Ok(Json(table_info))
    }

    pub fn list_tables(&self, login_id: &str) -> Result<Json<Vec<TableInfo>>, GameError> {
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

    pub fn get_table(&self, login_id: &str, table_id: &str) -> Result<Json<TableInfo>, GameError> {
        self.get_user(login_id)?;

        let table = self
            .tables
            .get(table_id)
            .ok_or(GameError::NotFound("table id not found"))?;

        let result = self.table_info(table_id, table);

        Ok(Json(result))
    }

    pub async fn join_table(
        state: &RwLock<ZingState>,
        login_id: &str,
        table_id: &str,
    ) -> Result<Json<TableInfo>, GameError> {
        let (notifications, result) = {
            let mut self_ = state.write().unwrap();

            let user = self_.get_user(login_id)?;
            let table_id = table_id.to_owned();
            if user
                .tables
                .read()
                .expect("unexpected concurrency")
                .contains(&table_id)
            {
                return Err(GameError::Conflict("trying to join table again"));
            }

            let table = self_
                .tables
                .get_mut(&table_id)
                .ok_or(GameError::NotFound("table id not found"))?;

            if table.games_have_started() {
                return Err(GameError::Conflict(
                    "cannot join a table after games have started",
                ));
            }

            user.tables
                .write()
                .expect("unexpected concurrency")
                .push(table_id.clone());
            table.players.push(user);

            let table = self_.tables.get(&table_id).unwrap();
            let result = self_.table_info(&table_id, table);

            let notifications: SerializedNotifications = self_
                .connections
                .iter()
                .filter_map(|c| {
                    table
                        .user_index(&c.user.login_id)
                        .map(|_| c.serialized_notification(serde_json::to_string(&result).unwrap()))
                })
                .collect();

            (notifications, result)
        };

        Self::send_notifications(notifications, state, None).await;

        Ok(Json(result))
    }

    pub fn leave_table(&mut self, login_id: &str, table_id: &str) -> Result<(), GameError> {
        let user = self.get_user(login_id)?;

        let table_index_in_user = user
            .tables
            .read()
            .expect("unexpected concurrency")
            .iter()
            .position(|id| *id == table_id)
            .ok_or(GameError::Unauthorized(
                "trying to leave table before joining",
            ))?;

        let table = self
            .tables
            .get_mut(table_id)
            .ok_or(GameError::NotFound("table id not found"))?;

        if table.games_have_started() {
            return Err(GameError::Conflict(
                // TODO: or should we allow this? it's less destructive than logging out.
                "cannot leave a table after games have started",
            ));
        }

        let user_index_in_table = table.user_index(login_id).expect("inconsistent state");

        // TODO: remove table if all remaining users are logged out
        table.players.remove(user_index_in_table);
        table.connections.remove_user(&user.login_id);
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
    ) -> Result<(), GameError> {
        // start a game (sync code), collect initial game status notifications
        let notifications = {
            let self_ = state.read().unwrap();
            self_.get_user(login_id)?;

            let table = self_
                .tables
                .get(table_id)
                .ok_or(GameError::NotFound("table id not found"))?;

            table.user_index(login_id).ok_or(GameError::NotFound(
                "user has not joined table at which game should start",
            ))?;

            drop(self_);
            let mut self_ = state.write().unwrap();

            let table = self_.tables.get_mut(table_id).unwrap();
            table.start_game()?;
            table.initial_game_status_messages()
        };

        // send initial card notifications
        Self::send_notifications(notifications, state, Some(table_id)).await;

        // finally, perform first dealer card actions
        let notifications = {
            let mut self_ = state.write().unwrap();
            let table = self_.tables.get_mut(table_id).unwrap();
            table.setup_game()?;
            table.action_notifications()
        };

        // send notifications about dealer actions
        Self::send_notifications(notifications, state, Some(table_id)).await;

        Ok(())
    }

    pub async fn send_notifications(
        notifications: SerializedNotifications,
        state: &RwLock<ZingState>,
        table_id: Option<&str>,
    ) {
        // send notifications (async, we don't want to hold the state locked)
        let mut broken_connections = Vec::new();
        for notification in notifications {
            debug!(
                "notifying connection {} ({})",
                &notification.connection_id,
                &notification.msg[..30]
            );
            if notification.send().await.is_err() {
                info!("removing broken client connection");
                broken_connections.push(notification.connection_id);
            };
        }

        if !broken_connections.is_empty() {
            // lock the state again to remove broken connections:
            let mut state = state.write().unwrap();
            if let Some(table_id) = table_id {
                let table = state.tables.get_mut(table_id).unwrap();
                for connection_id in broken_connections {
                    table.connections.remove(connection_id);
                }
            } else {
                for connection_id in broken_connections {
                    state.connections.remove(connection_id);
                }
            }
        }
    }

    pub fn game_status(
        &self,
        login_id: &str,
        table_id: &str,
    ) -> Result<Json<GameState>, GameError> {
        self.get_user(login_id)?;

        let table = self
            .tables
            .get(table_id)
            .ok_or(GameError::NotFound("table id not found"))?;

        table
            .user_index(login_id)
            .ok_or(GameError::NotFound("user has not joined table"))?;

        table
            .game_status(login_id)
            .map_or(Err(GameError::NotFound("no game active")), |game| {
                Ok(Json(game))
            })
    }

    pub fn finish_game(&mut self, login_id: &str, table_id: &str) -> Result<(), GameError> {
        self.get_user(login_id)?;

        let table = self
            .tables
            .get_mut(table_id)
            .ok_or(GameError::NotFound("table id not found"))?;

        table.user_index(login_id).ok_or(GameError::NotFound(
            "user has not joined table at which game should be finished",
        ))?;

        table.finish_game()
    }

    pub fn check_user_can_connect(
        &self,
        login_id: &str,
        table_id: &str,
    ) -> Result<bool, GameError> {
        self.get_user(login_id)?;

        let table = self
            .tables
            .get(table_id)
            .ok_or(GameError::NotFound("table id not found"))?;

        table
            .user_index(login_id)
            .ok_or(GameError::NotFound("connecting user has not joined table"))?;

        Ok(true)
    }

    pub async fn add_user_global_connection(
        state: &RwLock<ZingState>,
        login_id: String,
        sender: NotificationSenderHandle,
    ) {
        let mut self_ = state.write().unwrap();
        let _err = self_.get_user(&login_id).map(|user| {
            self_.connections.add(user, sender);
        });
    }

    pub async fn add_user_table_connection(
        state: &RwLock<ZingState>,
        login_id: String,
        table_id: String,
        sender: NotificationSenderHandle,
    ) {
        let mut notification = None;
        {
            let mut self_ = state.write().unwrap();

            let _err = self_.get_user(&login_id).map(|user| {
                self_.tables.get_mut(&table_id).map(|table| {
                    notification = table.connection_opened(user, sender);
                })
            });
        }

        if let Some(notification) = notification {
            // send current state to newly connected user
            Self::send_notifications(vec![notification], state, Some(&table_id)).await;
        }
    }

    pub async fn play_card(
        state: &RwLock<ZingState>,
        login_id: &str,
        table_id: &str,
        card_index: usize,
    ) -> Result<(), GameError> {
        let notifications;
        let result;

        {
            let mut self_ = state.write().unwrap();

            self_.get_user(login_id)?;

            let table = self_
                .tables
                .get_mut(table_id)
                .ok_or(GameError::NotFound("table id not found"))?;

            let player = table.user_index(login_id).ok_or(GameError::NotFound(
                "user has not joined table at which card should be played",
            ))?;

            let game = table
                .game
                .as_mut()
                .ok_or(GameError::Conflict("game not started yet"))?;

            result = game
                .play_card(player, card_index)
                .map_err(GameError::Conflict);

            notifications = table.action_notifications();
        }

        // send notifications about performed actions
        Self::send_notifications(notifications, state, Some(table_id)).await;

        result
    }
}
