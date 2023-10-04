use std::sync::{Arc, RwLock};

use zing_game::client_notification::ClientNotification;

use crate::{user::User, util::random_id, ws_notifications::NotificationSenderHandle};

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
    user: Arc<User>,
    pub sender: NotificationSenderHandle,
    pub actions_sent: RwLock<usize>,
}

impl ClientConnection {
    pub fn new(user: Arc<User>, sender: NotificationSenderHandle) -> Self {
        Self {
            connection_id: random_id(),
            user,
            sender,
            actions_sent: RwLock::new(0),
        }
    }

    pub fn client_login_id(&self) -> &str {
        &self.user.login_id
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

    pub fn add(&mut self, user: Arc<User>, sender: NotificationSenderHandle) {
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

    pub fn remove_user(&mut self, login_id: &str) {
        self.0.retain(|c| c.user.login_id != login_id);
    }
}
