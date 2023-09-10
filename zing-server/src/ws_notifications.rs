use axum::extract::ws::{Message, WebSocket};
use tokio::sync::mpsc;
use tracing::debug;

struct NotificationSender {
    receiver: mpsc::Receiver<Notification>,
    socket: WebSocket,
}

struct Notification {
    json: String,
}

impl NotificationSender {
    fn new(receiver: mpsc::Receiver<Notification>, socket: WebSocket) -> Self {
        Self { receiver, socket }
    }

    async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            if self.socket.send(Message::Text(msg.json)).await.is_err() {
                debug!("*** NotificationSender: WebSocket send() failed ***");
                // connection closed, finish actor
                break;
            }
        }
    }
}

#[derive(Clone)]
pub struct NotificationSenderHandle {
    sender: mpsc::Sender<Notification>,
}

impl NotificationSenderHandle {
    pub fn new(socket: WebSocket) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let mut actor = NotificationSender::new(receiver, socket);
        tokio::spawn(async move { actor.run().await });

        Self { sender }
    }

    pub async fn send(&self, json: String) -> Result<(), &'static str> {
        self.sender
            .send(Notification { json })
            .await
            .map_err(|_tokio_err| "could not contact NotificationSender")
    }
}
