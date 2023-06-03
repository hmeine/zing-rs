use bevy::prelude::{ResMut, Resource};
use bevy_tokio_tasks::TokioTasksRuntime;
use reqwest::cookie;
use std::collections::VecDeque;
use std::sync::Arc;
use zing_game::card_action::CardAction;
use zing_game::client_notification::ClientNotification;
use zing_game::game::GameState;

#[derive(Resource)]
pub struct GameLogic {
    notifications: VecDeque<StateChange>,
    client: reqwest::Client,
    play_uri: String,
}

pub enum StateChange {
    GameStarted(GameState, usize),
    CardAction(CardAction),
}

impl GameLogic {
    pub fn new(base_url: &str, login_id: &str, table_id: &str) -> Self {
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

        Self {
            notifications: VecDeque::new(),
            client,
            play_uri,
        }
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
                Err(err) => println!("Rest API error trying to play card: {}", err),
                Ok(response) => {
                    println!("{} {}", response.status(), response.text().await.unwrap());
                }
            };
        });
    }
}
