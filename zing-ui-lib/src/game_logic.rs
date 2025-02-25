use bevy::prelude::*;
use std::collections::VecDeque;
use zing_game::card_action::CardAction;
use zing_game::client_notification::ClientNotification;
use zing_game::game::GameState;

#[cfg(not(target_family = "wasm"))]
use {
    bevy_tokio_tasks::{TokioTasksPlugin, TokioTasksRuntime},
    futures_util::StreamExt,
    reqwest::cookie,
    std::sync::Arc,
    tungstenite::client::IntoClientRequest,
};

#[cfg(not(target_family = "wasm"))]
pub type TasksRuntime = TokioTasksRuntime;

#[cfg(target_family = "wasm")]
use {
    std::sync::{mpsc, mpsc::Receiver, Mutex},
    wasm_bindgen::prelude::*,
    wasm_bindgen_futures::JsFuture,
    web_sys::{ErrorEvent, MessageEvent, WebSocket},
    web_sys::{Request, RequestInit, Response},
};

#[cfg(target_family = "wasm")]
#[derive(Resource)]
pub struct TasksRuntime {
    notification_receiver: Mutex<Receiver<ClientNotification>>,
}

#[cfg(target_family = "wasm")]
impl TasksRuntime {
    fn new() -> Self {
        let (_dropped_sender, dangling_receiver) = mpsc::channel();
        Self {
            notification_receiver: Mutex::new(dangling_receiver),
        }
    }
}

pub struct GameLogicPlugin {
    pub base_url: String,
    pub login_id: String,
    pub table_id: String,
}

impl Plugin for GameLogicPlugin {
    fn build(&self, app: &mut App) {
        let game_logic = GameLogic::new(&self.base_url, &self.login_id, &self.table_id).unwrap();

        #[cfg(not(target_family = "wasm"))]
        app.insert_resource(game_logic)
            .add_plugins(TokioTasksPlugin::default())
            .add_systems(Startup, spawn_websocket_handler);

        #[cfg(target_family = "wasm")]
        app.insert_resource(game_logic)
            .insert_resource(TasksRuntime::new())
            .add_systems(Startup, spawn_websocket_handler)
            .add_systems(Update, receive_client_notifications);
    }
}

#[derive(Resource)]
pub struct GameLogic {
    notifications: VecDeque<StateChange>,

    #[cfg(not(target_family = "wasm"))]
    client: reqwest::Client,
    play_uri: String,
    #[cfg(not(target_family = "wasm"))]
    ws_uri: http::Uri,
    #[cfg(target_family = "wasm")]
    ws_uri: String,
    #[cfg(not(target_family = "wasm"))]
    login_cookie: String,
}

pub enum StateChange {
    GameStarted(GameState, usize),
    CardAction(CardAction),
}

impl GameLogic {
    #[cfg(not(target_family = "wasm"))]
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
            "{}/table/{}/ws",
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

    #[cfg(target_family = "wasm")]
    pub fn new(
        base_url: &str,
        _login_id: &str,
        table_id: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let play_uri = format!("{}/table/{}/game/play", base_url, table_id);

        let ws_uri = format!(
            "{}/table/{}/ws",
            base_url.replace("http", "ws"),
            table_id
        );

        Ok(Self {
            notifications: VecDeque::new(),
            play_uri,
            ws_uri,
        })
    }

    #[cfg(not(target_family = "wasm"))]
    fn spawn_websocket_handler(
        &self,
        runtime: ResMut<TasksRuntime>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut request = self.ws_uri.clone().into_client_request()?;
        request.headers_mut().insert(
            http::header::COOKIE,
            http::HeaderValue::from_str(&self.login_cookie)?,
        );

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
                    error!("Could not connect to websocket: {}", e);
                }
            };
        });

        Ok(())
    }

    #[cfg(target_family = "wasm")]
    fn spawn_websocket_handler(&self, mut runtime: ResMut<TasksRuntime>) {
        let (sender, receiver) = mpsc::channel();
        runtime.notification_receiver = Mutex::new(receiver);

        // nice: the browser already manages the login cookie for us
        let ws = WebSocket::new(&self.ws_uri).unwrap();

        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(client_notification) =
                serde_wasm_bindgen::from_value::<ClientNotification>(e.data())
            {
                debug!(
                    "message event, received ClientNotification: {:?}",
                    client_notification
                );
                if sender.send(client_notification).is_err() {
                    error!("could not send ClientNotification via mspc channel");
                }
            } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                debug!("message event, received Text: {:?}", txt);
                if let Ok(client_notification) = serde_json::from_str(&String::from(txt)) {
                    if sender.send(client_notification).is_err() {
                        error!("could not send ClientNotification via mspc channel");
                    }
                }
            } else {
                debug!("message event, received: {:?}", e.data());
            }
        });
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
            error!("error event: {:?}", e);
        });
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();
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

    #[cfg(not(target_family = "wasm"))]
    pub fn play_card(&mut self, runtime: ResMut<TasksRuntime>, card_index: usize) {
        let request = self
            .client
            .post(&self.play_uri)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(format!("{{ \"card_index\": {} }}", card_index));
        runtime.spawn_background_task(|_ctx| async move {
            match request.send().await {
                Err(err) => error!("Rest API error trying to play card: {}", err),
                Ok(response) => {
                    debug!("{} {}", response.status(), response.text().await.unwrap());
                }
            };
        });
    }

    #[cfg(target_family = "wasm")]
    pub fn play_card(
        &mut self,
        _runtime: ResMut<TasksRuntime>,
        card_index: usize,
    ) {
        use bevy::tasks::AsyncComputeTaskPool;

        let opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_body(&JsValue::from_str(&format!(
            "{{ \"card_index\": {} }}",
            card_index
        )));

        let request = Request::new_with_str_and_init(&self.play_uri, &opts).unwrap();

        request
            .headers()
            .set(http::header::CONTENT_TYPE.as_str(), "application/json").unwrap();

        let thread_pool = AsyncComputeTaskPool::get();
        let task = thread_pool.spawn(async move {
            let window = web_sys::window().unwrap();
            let resp_value = JsFuture::from(window.fetch_with_request(&request))
                .await
                .unwrap();

            let resp: Response = resp_value.dyn_into().unwrap();

            let json = JsFuture::from(resp.json().unwrap()).await;

            debug!("API response from playing card: {:?}", json);
        });
        task.detach();
    }
}

//#[cfg(not(target_family = "wasm"))]
pub fn spawn_websocket_handler(game_logic: Res<GameLogic>, runtime: ResMut<TasksRuntime>) {
    let _ = game_logic.spawn_websocket_handler(runtime);
}

#[cfg(target_family = "wasm")]
pub fn receive_client_notifications(mut game_logic: ResMut<GameLogic>, runtime: Res<TasksRuntime>) {
    if let Ok(receiver) = runtime.notification_receiver.lock() {
        let r = receiver.try_recv();
        if let Ok(client_notification) = r {
            debug!("received client notification: {:?}", client_notification);
            game_logic.handle_client_notification(client_notification);
        }
    } else {
        error!("could not access receiver");
    }
}
