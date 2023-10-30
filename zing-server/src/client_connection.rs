use std::sync::RwLock;

use zing_game::client_notification::ClientNotification;

use crate::{entities, util::random_id, ws_notifications::NotificationSenderHandle};

pub struct SerializedNotification {
    pub connection_id: String,
    pub msg: String,
    sender: NotificationSenderHandle,
}

impl SerializedNotification {
    pub async fn send(&self) -> Result<(), &'static str> {
        self.sender.send(self.msg.clone()).await
    }
}

pub type SerializedNotifications = Vec<SerializedNotification>;

pub struct ClientConnection {
    pub connection_id: String,
    user: entities::user::Model, // FIXME: should probably be just id/token?
    pub sender: NotificationSenderHandle,
    pub actions_sent: RwLock<usize>,
}

impl ClientConnection {
    pub fn new(user: entities::user::Model, sender: NotificationSenderHandle) -> Self {
        Self {
            connection_id: random_id(),
            user,
            sender,
            actions_sent: RwLock::new(0),
        }
    }

    pub fn client_login_token(&self) -> &str {
        &self.user.token
    }

    pub fn serialized_notification(&self, msg: String) -> SerializedNotification {
        SerializedNotification {
            connection_id: self.connection_id.clone(),
            msg,
            sender: self.sender.clone(),
        }
    }

    pub fn client_notification(
        &self,
        client_notification: &ClientNotification,
    ) -> SerializedNotification {
        self.serialized_notification(serde_json::to_string(client_notification).unwrap())
    }
}

#[derive(Default)]
pub struct ClientConnections(Vec<ClientConnection>);

impl ClientConnections {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn iter(&self) -> std::slice::Iter<'_, ClientConnection> {
        self.0.iter()
    }

    pub fn add(&mut self, user: entities::user::Model, sender: NotificationSenderHandle) {
        self.0.push(ClientConnection::new(user, sender));
    }

    pub fn last(&self) -> std::option::Option<&'_ ClientConnection> {
        self.0.last()
    }

    pub fn remove(&mut self, connection_id: String) {
        for (i, c) in self.iter().enumerate() {
            if c.connection_id == connection_id {
                self.0.remove(i);
                break;
            }
        }
    }

    pub fn remove_user_with_token(&mut self, login_token: &str) {
        self.0.retain(|c| c.user.token != login_token);
    }
}
