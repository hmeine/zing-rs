use axum::extract::ws::{Message, Utf8Bytes, WebSocket};
use tokio::sync::mpsc;
use tokio::time::{self, Duration};
use tracing::debug;

const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);

struct NotificationSender {
    receiver: mpsc::Receiver<Notification>,
    socket: WebSocket,
}

struct Notification {
    json: Utf8Bytes,
}

impl NotificationSender {
    fn new(receiver: mpsc::Receiver<Notification>, socket: WebSocket) -> Self {
        Self { receiver, socket }
    }

    async fn run(&mut self) {
        let mut keepalive = time::interval(KEEPALIVE_INTERVAL);
        keepalive.tick().await;

        loop {
            tokio::select! {
                maybe_msg = self.receiver.recv() => {
                    let Some(msg) = maybe_msg else {
                        break;
                    };

                    if self.socket.send(Message::Text(msg.json)).await.is_err() {
                        debug!("*** NotificationSender: WebSocket send() failed ***");
                        break;
                    }
                }
                _ = keepalive.tick() => {
                    if self.socket.send(Message::Ping(Vec::new().into())).await.is_err() {
                        debug!("*** NotificationSender: WebSocket ping() failed ***");
                        break;
                    }
                }
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
            .send(Notification {
                json: Utf8Bytes::from(json),
            })
            .await
            .map_err(|_tokio_err| "could not contact NotificationSender")
    }
}
