use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use futures_util::StreamExt;
use reqwest::cookie;
use std::collections::VecDeque;
use std::sync::Arc;
use tracing::{event, Level};
use tungstenite::client::IntoClientRequest;
use zing_game::card_action::CardAction;
use zing_game::client_notification::ClientNotification;
use zing_game::game::GameState;

#[derive(Resource)]
pub struct GameLogic {
    notifications: VecDeque<StateChange>,

    client: reqwest::Client,
    play_uri: String,
    ws_uri: http::Uri,
    login_cookie: String,
}

pub enum StateChange {
    GameStarted(GameState, usize),
    CardAction(CardAction),
}

impl GameLogic {
    pub fn new(
        base_url: &str,
        login_id: &str,
        table_id: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let jar = cookie::Jar::default();
        jar.add_cookie_str(
            &format!("login_id={}", login_id),
            &base_url.parse().unwrap(),
        );
        let client = reqwest::Client::builder()
            .cookie_provider(Arc::new(jar))
            .build()
            .unwrap();

        let play_uri = format!("{}/table/{}/game/play", base_url, table_id);

        let ws_uri = format!(
            "{}/table/{}/game/ws",
            base_url.replace("http", "ws"),
            table_id
        )
        .parse()?;

        let login_cookie = format!("login_id={}", login_id);

        Ok(Self {
            notifications: VecDeque::new(),
            client,
            play_uri,
            ws_uri,
            login_cookie,
        })
    }

    fn spawn_websocket_handler(
        &self,
        runtime: ResMut<TokioTasksRuntime>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut request = self.ws_uri.clone().into_client_request()?;
        request
            .headers_mut()
            .insert("Cookie", http::HeaderValue::from_str(&self.login_cookie)?);

        runtime.spawn_background_task(|mut ctx| async move {
            let result = tokio_tungstenite::connect_async(request).await;

            match result {
                Ok((ws_stream, _response)) => {
                    ws_stream
                        .fold(&mut ctx, |ctx, message| async {
                            let json = message.unwrap().into_text().unwrap();
                            if let Ok(client_notification) = serde_json::from_str(&json) {
                                ctx.run_on_main_thread(move |ctx| {
                                    let mut game_logic =
                                        ctx.world.get_resource_mut::<GameLogic>().unwrap();
                                    game_logic.handle_client_notification(client_notification);
                                })
                                .await;
                            }
                            ctx
                        })
                        .await;
                }
                Err(e) => {
                    event!(Level::ERROR, "Could not connect to websocket: {}", e);
                }
            };
        });

        Ok(())
    }

    pub fn handle_client_notification(&mut self, notification: ClientNotification) {
        match notification {
            ClientNotification::GameStatus(initial_state, we_are_player) => self
                .notifications
                .push_back(StateChange::GameStarted(initial_state, we_are_player)),
            ClientNotification::CardActions(actions) => self
                .notifications
                .extend(actions.into_iter().map(StateChange::CardAction)),
        }
    }

    pub fn get_next_state_change(&mut self) -> Option<StateChange> {
        self.notifications.pop_front()
    }

    pub fn play_card(&mut self, runtime: ResMut<TokioTasksRuntime>, card_index: usize) {
        let request = self
            .client
            .post(&self.play_uri)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(format!("{{ \"card_index\": {} }}", card_index));
        runtime.spawn_background_task(|_ctx| async move {
            match request.send().await {
                Err(err) => event!(Level::ERROR, "Rest API error trying to play card: {}", err),
                Ok(response) => {
                    event!(
                        Level::INFO,
                        "{} {}",
                        response.status(),
                        response.text().await.unwrap()
                    );
                }
            };
        });
    }
}

pub fn spawn_websocket_handler(game_logic: Res<GameLogic>, runtime: ResMut<TokioTasksRuntime>) {
    game_logic.spawn_websocket_handler(runtime).unwrap();
}
