use std::sync::{Arc, RwLock};

use zing_game::client_notification::ClientNotification;

use crate::{user::User, util::random_id, ws_notifications::NotificationSenderHandle};

pub type SerializedNotification = (String, String, NotificationSenderHandle);
pub type SerializedNotifications = Vec<SerializedNotification>;

pub struct ClientConnection {
    pub connection_id: String,
    pub user: Arc<User>,
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
}

impl ClientConnection {
    pub fn serialized_notification(&self, msg: String) -> SerializedNotification {
        (self.connection_id.clone(), msg, self.sender.clone())
    }

    pub fn client_notification(
        &self,
        client_notification: &ClientNotification,
    ) -> SerializedNotification {
        self.serialized_notification(serde_json::to_string(client_notification).unwrap())
    }
}
