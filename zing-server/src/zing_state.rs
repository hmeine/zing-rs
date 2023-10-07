use axum::Json;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tracing::debug;
use zing_game::game::{GamePhase, GameState};

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
    users: RwLock<HashMap<String, Arc<User>>>,
    tables: RwLock<HashMap<String, Table>>,
    connections: RwLock<ClientConnections>,
}

impl ZingState {
    pub fn login(&self, user_name: &str) -> String {
        let login_id = random_id();
        self.users.write().unwrap().insert(
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

    pub fn logout(&self, user: Arc<User>) -> Result<(), GameError> {
        self.users
            .write()
            .unwrap()
            .remove_entry(&user.login_id)
            .ok_or(GameError::Unauthorized("user not found (bad id cookie)"))
            .map(|(_, user)| {
                // mark user as logged out
                *user
                    .logged_in
                    .write()
                    .expect("RwLock poisoned through panic") = false;
            })?;

        // close websocket connections
        self.connections.write().unwrap().remove_user(&user.login_id);

        let mut tables = self.tables.write().unwrap();
        for table in tables.values_mut() {
            table.connections.remove_user(&user.login_id);
        }

        // remove table if all users have logged out
        tables.retain(|_table_id, table| table.has_logged_in_users());

        Ok(())
    }

    pub fn get_user(&self, login_id: &str) -> Result<Arc<User>, GameError> {
        self.users.read().unwrap().get(login_id).map_or(
            Err(GameError::Unauthorized("user not found (bad id cookie)")),
            |user| Ok(user.clone()),
        )
    }

    pub fn create_table(&self, login_id: &str) -> Result<Json<TableInfo>, GameError> {
        let table_id = random_id();

        let user = self.get_user(login_id)?;
        user.tables
            .write()
            .expect("RwLock poisoned through panic")
            .push(table_id.clone());

        let table = Table::new(user);
        let table_info = table.table_info(&table_id);

        self.tables.write().unwrap().insert(table_id, table);

        Ok(Json(table_info))
    }

    pub fn list_tables(&self, login_id: &str) -> Result<Json<Vec<TableInfo>>, GameError> {
        let user = self.get_user(login_id)?;

        let tables = self.tables.read().unwrap();

        let table_infos = user
            .tables
            .read()
            .expect("RwLock poisoned through panic")
            .iter()
            .map(|table_id| tables.get(table_id).unwrap().table_info(table_id))
            .collect::<Vec<_>>();

        Ok(Json(table_infos))
    }

    pub fn get_table_info(
        &self,
        login_id: &str,
        table_id: &str,
    ) -> Result<Json<TableInfo>, GameError> {
        self.get_user(login_id)?;

        let tables = self.tables.read().unwrap();

        let table = tables
            .get(table_id)
            .ok_or(GameError::NotFound("table id not found"))?;

        let result = table.table_info(table_id);

        Ok(Json(result))
    }

    async fn send_table_notifications(&self, table_id: &str) {
        let notifications = {
            let tables = self.tables.read().unwrap();
            let table = tables.get(table_id).unwrap();
            let result = table.table_info(table_id);

            self.connections
                .read()
                .unwrap()
                .iter()
                .filter_map(|c| {
                    table
                        .user_index(c.client_login_id())
                        .map(|_| c.serialized_notification(serde_json::to_string(&result).unwrap()))
                })
                .collect()
        };

        self.send_notifications(notifications, None).await;
    }

    pub async fn join_table(
        &self,
        login_id: &str,
        table_id: &str,
    ) -> Result<Json<TableInfo>, GameError> {
        let result = {
            let user = self.get_user(login_id)?;
            let table_id = table_id.to_owned();
            if user
                .tables
                .read()
                .expect("RwLock poisoned through panic")
                .contains(&table_id)
            {
                return Err(GameError::Conflict("trying to join table again"));
            }

            let mut tables = self.tables.write().unwrap();
            let table = tables
                .get_mut(&table_id)
                .ok_or(GameError::NotFound("table id not found"))?;

            if table.games_have_started() {
                return Err(GameError::Conflict(
                    "cannot join a table after games have started",
                ));
            }

            user.tables
                .write()
                .expect("RwLock poisoned through panic")
                .push(table_id.clone());
            table.user_joined(user);

            table.table_info(&table_id)
        };

        self.send_table_notifications(table_id).await;

        Ok(Json(result))
    }

    pub fn leave_table(&self, login_id: &str, table_id: &str) -> Result<(), GameError> {
        let user = self.get_user(login_id)?;

        let table_index_in_user = user
            .tables
            .read()
            .expect("RwLock poisoned through panic")
            .iter()
            .position(|id| *id == table_id)
            .ok_or(GameError::Unauthorized(
                "trying to leave table before joining",
            ))?;

        {
            // scope for locked self.tables
            let mut tables = self.tables.write().unwrap();
            let table = tables
                .get_mut(table_id)
                .ok_or(GameError::NotFound("table id not found"))?;

            if table.games_have_started() {
                return Err(GameError::Conflict(
                    // TODO: or should we allow this? it's less destructive than logging out.
                    "cannot leave a table after games have started",
                ));
            }

            table.user_left(login_id);
            if !table.has_logged_in_users() {
                tables.remove(table_id);
            }
        }

        user.tables
            .write()
            .expect("RwLock poisoned through panic")
            .remove(table_index_in_user);

        Ok(())
    }

    pub async fn start_game(&self, login_id: &str, table_id: &str) -> Result<(), GameError> {
        // start a game (sync code), collect initial game status notifications
        let notifications = {
            self.get_user(login_id)?;

            {
                // scope for locked self.tables
                let tables = self.tables.read().unwrap();
                let table = tables
                    .get(table_id)
                    .ok_or(GameError::NotFound("table id not found"))?;

                table.user_index(login_id).ok_or(GameError::NotFound(
                    "user has not joined table at which game should start",
                ))?;
            }

            {
                // scope for locked self.tables
                let mut tables = self.tables.write().unwrap();
                let table = tables.get_mut(table_id).unwrap();
                table.start_game()?;
                table.initial_game_status_messages()
            }
        };

        // send initial card notifications
        self.send_notifications(notifications, Some(table_id)).await;

        // finally, perform first dealer card actions
        let notifications = {
            let mut tables = self.tables.write().unwrap();
            let table = tables.get_mut(table_id).unwrap();
            table.setup_game()?;
            table.action_notifications()
        };

        // send notifications about dealer actions
        self.send_notifications(notifications, Some(table_id)).await;

        self.send_table_notifications(table_id).await;

        Ok(())
    }

    pub async fn send_notifications(
        &self,
        notifications: SerializedNotifications,
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
                debug!("removing broken client connection");
                broken_connections.push(notification.connection_id);
            };
        }

        if !broken_connections.is_empty() {
            // lock the state again to remove broken connections:
            if let Some(table_id) = table_id {
                let mut tables = self.tables.write().unwrap();
                let table = tables.get_mut(table_id).unwrap();
                for connection_id in broken_connections {
                    table.connections.remove(connection_id);
                }
            } else {
                for connection_id in broken_connections {
                    self.connections.write().unwrap().remove(connection_id);
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

        let tables = self.tables.read().unwrap();
        let table = tables
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

    pub async fn finish_game(&self, login_id: &str, table_id: &str) -> Result<(), GameError> {
        let result = {
            self.get_user(login_id)?;

            let mut tables = self.tables.write().unwrap();
            let table = tables
                .get_mut(table_id)
                .ok_or(GameError::NotFound("table id not found"))?;

            table.user_index(login_id).ok_or(GameError::NotFound(
                "user has not joined table at which game should be finished",
            ))?;

            table.finish_game()
        };

        if result.is_ok() {
            self.send_table_notifications(table_id).await;
        }

        result
    }

    pub fn check_user_can_connect(
        &self,
        login_id: &str,
        table_id: &str,
    ) -> Result<bool, GameError> {
        self.get_user(login_id)?;

        let tables = self.tables.read().unwrap();
        let table = tables
            .get(table_id)
            .ok_or(GameError::NotFound("table id not found"))?;

        table
            .user_index(login_id)
            .ok_or(GameError::NotFound("connecting user has not joined table"))?;

        Ok(true)
    }

    pub async fn add_user_global_connection(
        &self,
        login_id: String,
        sender: NotificationSenderHandle,
    ) {
        let _err = self.get_user(&login_id).map(|user| {
            self.connections.write().unwrap().add(user, sender);
        });
    }

    pub async fn add_user_table_connection(
        &self,
        login_id: String,
        table_id: String,
        sender: NotificationSenderHandle,
    ) {
        let mut notification = None;
        {
            let _err = self.get_user(&login_id).map(|user| {
                self.tables
                    .write()
                    .unwrap()
                    .get_mut(&table_id)
                    .map(|table| {
                        notification = table.connection_opened(user, sender);
                    })
            });
        }

        if let Some(notification) = notification {
            // send current state to newly connected user
            self.send_notifications(vec![notification], Some(&table_id))
                .await;
        }
    }

    pub async fn play_card(
        &self,
        login_id: &str,
        table_id: &str,
        card_index: usize,
    ) -> Result<(), GameError> {
        let table_notifications;
        let result;

        let phase_changed = {
            self.get_user(login_id)?;

            let mut tables = self.tables.write().unwrap();
            let table = tables
                .get_mut(table_id)
                .ok_or(GameError::NotFound("table id not found"))?;

            let player = table.user_index(login_id).ok_or(GameError::NotFound(
                "user has not joined table at which card should be played",
            ))?;

            let game = table
                .game
                .as_mut()
                .ok_or(GameError::Conflict("game not started yet"))?;

            let old_phase = game.state().phase;

            result = game
                .play_card(player, card_index)
                .map_err(GameError::Conflict);

            if result.is_ok() && game.state().phase == GamePhase::Finished {
                table.game_results.push(game.points());
            }
            drop(tables);

            let tables = self.tables.read().unwrap();
            let table = tables.get(table_id).unwrap();
            let game = table.game.as_ref().unwrap();
            let new_phase = game.state().phase();

            table_notifications = table.action_notifications();

            new_phase != old_phase
        };

        // send notifications about performed actions
        self.send_notifications(table_notifications, Some(table_id))
            .await;
        if phase_changed {
            self.send_table_notifications(table_id).await;
        }

        result
    }
}
