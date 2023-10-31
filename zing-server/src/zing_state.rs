use axum::Json;
use entities::prelude::*;
use sea_orm::{prelude::*, ActiveValue, Condition, QueryOrder};
use std::{collections::HashMap, sync::RwLock};
use tracing::debug;
use zing_game::game::{GamePhase, GameState};

use crate::{
    client_connection::{ClientConnections, SerializedNotifications},
    entities,
    game_error::GameError,
    table::{LoadedTable, TableInfo},
    util::random_id,
    ws_notifications::NotificationSenderHandle,
};

#[derive(Default)]
pub struct ZingState {
    tables: RwLock<HashMap<String, LoadedTable>>,
    connections: RwLock<ClientConnections>,
    db_conn: DatabaseConnection,
}

impl ZingState {
    pub async fn new(db_conn: DatabaseConnection) -> Self {
        Self {
            db_conn,
            ..Default::default()
        }
    }

    pub async fn get_user_with_token(
        &self,
        login_token: &str,
    ) -> Result<entities::user::Model, GameError> {
        let user = User::find()
            .filter(entities::user::Column::Token.eq(login_token))
            .one(&self.db_conn)
            .await
            .map_err(|_| GameError::DBError("DB error"))?;

        user.ok_or(GameError::Unauthorized("user not found (bad id cookie)"))
    }

    pub async fn login(&self, user_name: &str) -> Result<String, GameError> {
        let login_token = random_id();

        User::insert(entities::user::ActiveModel {
            name: ActiveValue::Set(user_name.to_owned()),
            token: ActiveValue::Set(login_token.clone()),
            logged_in: ActiveValue::Set(true),
            ..Default::default()
        })
        .exec_without_returning(&self.db_conn)
        .await
        .map_err(|_| GameError::DBError("DB insert failed unexpectedly"))?;

        Ok(login_token)
    }

    pub async fn logout(&self, user: entities::user::Model) -> Result<String, GameError> {
        let token = user.token.clone();
        let user_name = user.name.clone();

        // mark as logged out
        let mut user: entities::user::ActiveModel = user.into();
        user.logged_in = ActiveValue::Set(false);
        user.update(&self.db_conn)
            .await
            .map_err(|_| GameError::DBError("DB update failed unexpectedly"))?;

        // close websocket connections
        self.connections
            .write()
            .unwrap()
            .remove_user_with_token(&token);

        let mut tables = self.tables.write().unwrap();
        for tc in tables.values_mut() {
            tc.connections.remove_user_with_token(&token);
        }

        // FIXME: remove table if all users have logged out (taking into account loaded tables)

        Ok(user_name)
    }

    pub async fn create_table(
        &self,
        user: entities::user::Model,
    ) -> Result<Json<TableInfo>, GameError> {
        let table = LoadedTable::create_for_user(user, &self.db_conn).await?;

        let table_info = table.table_info();

        self.tables.write().unwrap().insert(table.token(), table);

        Ok(Json(table_info))
    }

    async fn ensure_loaded_table(&self, table: entities::table::Model) {
        let loaded = self.tables.read().unwrap().get(&table.token).is_some();

        if !loaded {
            let token = table.token.clone();
            let loaded = LoadedTable::new_from_db(table, &self.db_conn).await;
            let mut tables = self.tables.write().unwrap();
            tables.insert(token.clone(), loaded);
        }
    }

    async fn table_info(&self, table: entities::table::Model) -> TableInfo {
        let token = table.token.clone();

        self.ensure_loaded_table(table).await;

        let tables = self.tables.read().unwrap();
        let loaded = tables.get(&token).expect("we have just loaded the table");
        loaded.table_info()
    }

    pub async fn list_tables(
        &self,
        user: entities::user::Model,
    ) -> Result<Json<Vec<TableInfo>>, GameError> {
        let mut result = Vec::new();

        for (table, _table_join) in Table::find()
            .find_also_related(TableJoin)
            .filter(entities::table_join::Column::UserId.eq(user.id))
            .order_by_asc(entities::table::Column::CreatedAt)
            .all(&self.db_conn)
            .await
            .map_err(|_| GameError::DBError("DB query failed unexpectedly"))?
        {
            result.push(self.table_info(table).await);
        }

        Ok(Json(result))
    }

    async fn find_table_with_token(
        &self,
        token: &str,
    ) -> Result<entities::table::Model, GameError> {
        {
            let tables = self.tables.read().unwrap();
            if let Some(loaded) = tables.get(token) {
                return Ok(loaded.table());
            }
        }

        Table::find()
            .filter(entities::table::Column::Token.eq(token))
            .one(&self.db_conn)
            .await
            .map_err(|_| GameError::DBError("DB query failed unexpectedly"))?
            .ok_or(GameError::NotFound("table not found by token"))
    }

    pub async fn get_table_info(&self, token: &str) -> Result<Json<TableInfo>, GameError> {
        let table = self.find_table_with_token(token).await?;

        let result = self.table_info(table).await;

        Ok(Json(result))
    }

    async fn send_table_notifications(&self, token: &str) {
        if let Ok(table) = self.find_table_with_token(token).await {
            let table_info = self.table_info(table).await;

            let notifications = {
                // loaded_table_info() will have loaded the table
                // FIXME: isn't it dangerous to lock both tables and connections at the same time?!
                // (it would not be if we made sure that every time this happens, the order is the same)
                let tables = self.tables.read().unwrap();
                let loaded = tables.get(token).unwrap();
                self.connections
                    .read()
                    .unwrap()
                    .iter()
                    .filter_map(|c| {
                        loaded.player_index(c.client_login_token()).map(|_| {
                            c.serialized_notification(serde_json::to_string(&table_info).unwrap())
                        })
                    })
                    .collect()
            };

            self.send_notifications(notifications, None).await;
        }
    }

    pub async fn join_table(
        &self,
        user: &entities::user::Model,
        table_token: &str,
    ) -> Result<Json<TableInfo>, GameError> {
        let table = self.find_table_with_token(table_token).await?;

        {
            self.ensure_loaded_table(table.clone()).await;

            let tables = self.tables.read().unwrap();
            if tables
                .get(table_token)
                .expect("must be loaded now")
                .games_have_started()
            {
                return Err(GameError::Conflict(
                    "cannot join a table after games have started",
                ));
            }
        }

        let table_pos = table
            .find_related(TableJoin)
            .count(&self.db_conn)
            .await
            .map_err(|_| GameError::DBError("DB query failed unexpectedly"))?;

        TableJoin::insert(entities::table_join::ActiveModel {
            user_id: ActiveValue::Set(user.id),
            table_id: ActiveValue::Set(table.id),
            table_pos: ActiveValue::Set(table_pos.try_into().unwrap()),
        })
        .exec_without_returning(&self.db_conn)
        .await
        // TODO: discriminate between a generic DB error vs. a constraint violation?
        .map_err(|_| GameError::Conflict("trying to join table again"))?;

        {
            let mut tables = self.tables.write().unwrap();
            let loaded_table = tables.get_mut(table_token).expect("must be loaded now");
            loaded_table.user_joined(user);
        }

        self.send_table_notifications(table_token).await;

        self.get_table_info(table_token).await
    }

    pub async fn leave_table(
        &self,
        user: &entities::user::Model,
        table_token: &str,
    ) -> Result<(), GameError> {
        let player_index = self.user_index_at_table(user, table_token).await?;

        let table_id = {
            // scope for locked self.tables
            let mut tables = self.tables.write().unwrap();
            let table = tables
                .get_mut(table_token)
                .ok_or(GameError::NotFound("table id not found"))?;

            if table.games_have_started() {
                return Err(GameError::Conflict(
                    "cannot leave a table after games have started",
                ));
            }

            table.user_left(player_index);

            table.table().id
        };

        // remove user from table
        TableJoin::delete_many()
            .filter(
                Condition::all()
                    .add(entities::table_join::Column::TableId.eq(table_id))
                    .add(entities::table_join::Column::UserId.eq(user.id)),
            )
            .exec(&self.db_conn)
            .await
            .map_err(|_| GameError::DBError("DB error (DELETE from table_join)"))?;

        // correct table positions of other users if necessary
        // FIXME: could we use an auto_increment value in order to make this
        // unnecessary (we only require orderable values, and currently
        // the below would even have a theoretical bug w.r.t. concurrency /
        // non-atomicity with the above)? another improvement would be a
        // single update statement that uses an appropriate decrement expression!
        let positions_to_change = TableJoin::find()
            .filter(
                Condition::all()
                    .add(entities::table_join::Column::TableId.eq(table_id))
                    .add(
                        entities::table_join::Column::TablePos
                            .gt(i32::try_from(player_index).unwrap()),
                    ),
            )
            .all(&self.db_conn)
            .await
            .map_err(|_| GameError::DBError("DB error (SELECT from table_join)"))?;
        for table_join in positions_to_change {
            let old_pos = table_join.table_pos;
            let mut table_join: entities::table_join::ActiveModel = table_join.into();
            table_join.table_pos = ActiveValue::Set(old_pos - 1);
            table_join.update(&self.db_conn).await.map_err(|_| {
                GameError::DBError("DB error (UPDATE table_join, decreasing table_pos)")
            })?;
        }

        // finally, remove tables that no longer have logged in users
        let table = self.find_table_with_token(table_token).await?;
        let table_has_logged_in_users = table
            .find_related(User)
            .filter(entities::user::Column::LoggedIn.eq(true))
            .one(&self.db_conn)
            .await
            .unwrap()
            .is_some();

        if !table_has_logged_in_users {
            table.delete(&self.db_conn).await.map_err(|_| {
                GameError::DBError("DB error (DELETE table without logged in users)")
            })?;
        }

        Ok(())
    }

    pub async fn start_game(
        &self,
        user: &entities::user::Model,
        table_token: &str,
    ) -> Result<(), GameError> {
        self.user_index_at_table(user, table_token).await?;

        // start a game (sync code), collect initial game status notifications
        let (game_json, notifications) = {
            {
                // scope for locked self.tables
                let tables = self.tables.read().unwrap();
                let loaded = tables
                    .get(table_token)
                    .expect("we have just loaded the table");

                loaded.player_index(&user.token).ok_or(GameError::NotFound(
                    "user has not joined table at which game should start",
                ))?;
            }

            {
                // scope for locked self.tables
                let mut tables = self.tables.write().unwrap();
                let loaded = tables.get_mut(table_token).unwrap();
                loaded.start_game()?;

                (loaded.game_json(), loaded.initial_game_status_messages())
            }
        };

        // send initial card notifications
        self.send_notifications(notifications, Some(table_token))
            .await;

        // finally, perform first dealer card actions
        let notifications = {
            let mut tables = self.tables.write().unwrap();
            let loaded = tables.get_mut(table_token).unwrap();
            loaded.setup_game()?;
            loaded.action_notifications()
        };

        // send notifications about dealer actions
        self.send_notifications(notifications, Some(table_token))
            .await;

        self.send_table_notifications(table_token).await;

        Table::update_many()
            .col_expr(entities::table::Column::Game, Expr::value(game_json))
            .filter(entities::table::Column::Token.eq(table_token))
            .exec(&self.db_conn)
            .await
            .map_err(|_| GameError::DBError("DB error (UPDATE table.game)"))?;

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

    pub async fn game_status(
        &self,
        user: &entities::user::Model,
        table_token: &str,
    ) -> Result<Json<GameState>, GameError> {
        let table = self.find_table_with_token(table_token).await?;

        self.ensure_loaded_table(table).await;

        let tables = self.tables.read().unwrap();
        let loaded = tables
            .get(table_token)
            .expect("we have just loaded the table");

        loaded
            .player_index(&user.token)
            .ok_or(GameError::NotFound("user has not joined table"))?;

        loaded
            .game_status(&user.token)
            .map_or(Err(GameError::NotFound("no game active")), |game| {
                Ok(Json(game))
            })
    }

    pub async fn finish_game(
        &self,
        user: &entities::user::Model,
        table_token: &str,
    ) -> Result<(), GameError> {
        self.user_index_at_table(user, table_token).await?;

        // scope for locked self.tables
        let result = {
            let mut tables = self.tables.write().unwrap();
            let loaded = tables
                .get_mut(table_token)
                .expect("we have just loaded the table");

            loaded.finish_game()
        };

        if result.is_ok() {
            self.send_table_notifications(table_token).await;
        }

        Table::update_many()
            .col_expr(
                entities::table::Column::Game,
                Expr::value(None::<sea_orm::prelude::Json>),
            )
            .filter(entities::table::Column::Token.eq(table_token))
            .exec(&self.db_conn)
            .await
            .map_err(|_| GameError::DBError("DB error (UPDATE table.game)"))?;

        result
    }

    pub async fn user_index_at_table(
        &self,
        user: &entities::user::Model,
        table_token: &str,
    ) -> Result<usize, GameError> {
        let table = self.find_table_with_token(table_token).await?;

        self.ensure_loaded_table(table).await;

        let tables = self.tables.read().unwrap();
        let table = tables
            .get(table_token)
            .expect("we have just loaded the table");

        table
            .player_index(&user.token)
            .ok_or(GameError::NotFound("user has not joined table"))
    }

    pub async fn add_user_global_connection(
        &self,
        user: entities::user::Model,
        sender: NotificationSenderHandle,
    ) {
        self.connections.write().unwrap().add(user, sender);
    }

    pub async fn add_user_table_connection(
        &self,
        user: entities::user::Model,
        table_id: String,
        sender: NotificationSenderHandle,
    ) {
        let mut notification = None;
        if let Some(table) = self.tables.write().unwrap().get_mut(&table_id) {
            notification = table.connection_opened(user, sender);
        }

        if let Some(notification) = notification {
            // send current state to newly connected user
            self.send_notifications(vec![notification], Some(&table_id))
                .await;
        }
    }

    pub async fn play_card(
        &self,
        user: &entities::user::Model,
        table_token: &str,
        card_index: usize,
    ) -> Result<(), GameError> {
        let table_notifications;
        let result;

        let (game_json, phase_changed) = {
            let player_index = self.user_index_at_table(user, table_token).await?;

            let mut tables = self.tables.write().unwrap();
            let table = tables
                .get_mut(table_token)
                .ok_or(GameError::NotFound("table id not found"))?;

            let game = table
                .game
                .as_mut()
                .ok_or(GameError::Conflict("game not started yet"))?;

            let old_phase = game.state().phase;

            result = game
                .play_card(player_index, card_index)
                .map_err(GameError::Conflict);

            if result.is_ok() && game.state().phase == GamePhase::Finished {
                table.game_results.push(game.points());
            }
            drop(tables);

            let tables = self.tables.read().unwrap();
            let table = tables.get(table_token).unwrap();
            let game = table.game.as_ref().unwrap();
            let new_phase = game.state().phase();

            table_notifications = table.action_notifications();

            (table.game_json(), new_phase != old_phase)
        };

        // send notifications about performed actions
        self.send_notifications(table_notifications, Some(table_token))
            .await;
        if phase_changed {
            self.send_table_notifications(table_token).await;
        }

        Table::update_many()
            .col_expr(entities::table::Column::Game, Expr::value(game_json))
            .filter(entities::table::Column::Token.eq(table_token))
            .exec(&self.db_conn)
            .await
            .map_err(|_| GameError::DBError("DB error (UPDATE table.game)"))?;

        result
    }
}
