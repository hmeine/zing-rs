use std::sync::{Arc, RwLock};

use crate::{ws_notifications::NotificationSenderHandle, user::User};

pub type SerializedNotification = (String, String, NotificationSenderHandle);
pub type SerializedNotifications = Vec<SerializedNotification>;

pub struct ClientConnection {
    pub connection_id: String,
    pub user: Arc<User>,
    pub sender: NotificationSenderHandle,
    pub actions_sent: RwLock<usize>,
}

impl ClientConnection {
    pub fn new(connection_id: String, user: Arc<User>, sender: NotificationSenderHandle) -> Self {
        Self {
            connection_id,
            user,
            sender,
            actions_sent: RwLock::new(0),
        }
    }
}

impl ClientConnection {
    pub fn notification(&self, msg: String) -> SerializedNotification {
        (self.connection_id.clone(), msg, self.sender.clone())
    }
}
