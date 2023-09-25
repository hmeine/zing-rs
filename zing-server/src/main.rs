use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, Path, State, WebSocketUpgrade},
    http::{self, request::Parts},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use cookie::SameSite;
use game_error::GameError;
use serde::Deserialize;
use tower_cookies::{Cookie, CookieManagerLayer, Cookies};
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;
use ws_notifications::NotificationSenderHandle;
use zing_game::game::GameState;
use zing_state::ZingState;

mod client_connection;
mod game_error;
mod table;
mod user;
mod util;
mod ws_notifications;
mod zing_state;

#[shuttle_runtime::main]
async fn axum() -> shuttle_axum::ShuttleAxum {
    let state = Arc::new(RwLock::new(ZingState::default()));

    let app = Router::new()
        .route("/", get(index))
        .route("/login", post(login).get(whoami).delete(logout))
        .route("/table", post(create_table).get(list_tables))
        .route("/ws", get(global_ws_handler))
        .route(
            "/table/:table_id",
            post(join_table).get(get_table).delete(leave_table),
        )
        .route(
            "/table/:table_id/game",
            post(start_game).get(game_status).delete(finish_game),
        )
        .route("/table/:table_id/game/play", post(play_card))
        .route("/table/:table_id/ws", get(table_ws_handler))
        .nest_service(
            "/zing_ui_lib.js",
            ServeFile::new("zing-ui-lib/pkg/zing_ui_lib.js"),
        )
        .nest_service(
            "/zing_ui_lib_bg.wasm",
            ServeFile::new("zing-ui-lib/pkg/zing_ui_lib_bg.wasm"),
        )
        .nest_service("/assets", ServeDir::new("zing-ui-lib/assets"))
        .with_state(state)
        .layer(CookieManagerLayer::new());

    Ok(app.into())
}

#[derive(Deserialize)]
struct LoginRequest {
    name: String,
}

const USERNAME_COOKIE: &str = "login_id";

async fn login(
    State(state): State<Arc<RwLock<ZingState>>>,
    cookies: Cookies,
    Json(login_request): Json<LoginRequest>,
) -> Result<String, GameError> {
    let mut state = state.write().unwrap();
    let user_name = login_request.name;
    if user_name.is_empty() {
        return Err(GameError::BadRequest("name must not be empty"));
    }
    let login_id = state.login(&user_name);
    info!("Logged in {} as {}", user_name, login_id);

    // TODO: log out if USERNAME_COOKIE is already set (and valid)

    let mut login_cookie = Cookie::new(USERNAME_COOKIE, login_id);
    login_cookie.set_same_site(SameSite::Strict);
    cookies.add(login_cookie);
    Ok(user_name)
}

async fn logout(
    State(state): State<Arc<RwLock<ZingState>>>,
    login_id: LoginID,
    cookies: Cookies,
) -> Result<(), GameError> {
    let mut state = state.write().unwrap();
    let user_name = state.whoami(&login_id.0);
    state.logout(&login_id.0)?;
    info!("Logged out {}", user_name.unwrap());

    let mut login_cookie = Cookie::new(USERNAME_COOKIE, "");
    login_cookie.set_same_site(SameSite::Strict);
    cookies.remove(login_cookie);
    Ok(())
}

struct LoginID(String);

#[async_trait]
impl<S> FromRequestParts<S> for LoginID
where
    S: Send + Sync,
{
    type Rejection = (http::StatusCode, &'static str);

    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let cookies = Cookies::from_request_parts(req, state).await?;

        let login_id = cookies
            .get(USERNAME_COOKIE)
            .ok_or((
                http::StatusCode::UNAUTHORIZED,
                "login first (id cookie missing)",
            ))?
            .value()
            .to_string();

        Ok(LoginID(login_id))
    }
}

#[cfg(not(debug_assertions))]
async fn index() -> Html<&'static str> {
    Html(std::include_str!("../assets/index.html"))
}

#[cfg(debug_assertions)]
async fn index() -> Result<Html<String>, http::StatusCode> {
    Ok(Html(
        std::fs::read_to_string("zing-server/assets/index.html")
            .or(Err(http::StatusCode::NOT_FOUND))?,
    ))
}

async fn whoami(
    State(state): State<Arc<RwLock<ZingState>>>,
    login_id: LoginID,
) -> Result<String, GameError> {
    let state = state.read().unwrap();
    match state.whoami(&login_id.0) {
        Some(user_name) => Ok(user_name),
        None => Err(GameError::Unauthorized("no valid login cookie")),
    }
}

async fn create_table(
    login_id: LoginID,
    State(state): State<Arc<RwLock<ZingState>>>,
) -> Result<impl IntoResponse, GameError> {
    let mut state = state.write().unwrap();
    state.create_table(&login_id.0)
}

async fn list_tables(
    login_id: LoginID,
    State(state): State<Arc<RwLock<ZingState>>>,
) -> Result<impl IntoResponse, GameError> {
    let state = state.read().unwrap();
    state.list_tables(&login_id.0)
}

async fn get_table(
    login_id: LoginID,
    Path(table_id): Path<String>,
    State(state): State<Arc<RwLock<ZingState>>>,
) -> Result<impl IntoResponse, GameError> {
    let state = state.read().unwrap();
    state.get_table(&login_id.0, &table_id)
}

async fn join_table(
    login_id: LoginID,
    Path(table_id): Path<String>,
    State(state): State<Arc<RwLock<ZingState>>>,
) -> Result<impl IntoResponse, GameError> {
    ZingState::join_table(state.deref(), &login_id.0, &table_id).await
}

async fn leave_table(
    login_id: LoginID,
    Path(table_id): Path<String>,
    State(state): State<Arc<RwLock<ZingState>>>,
) -> Result<(), GameError> {
    let mut state = state.write().unwrap();
    state.leave_table(&login_id.0, &table_id)
}

async fn start_game(
    login_id: LoginID,
    Path(table_id): Path<String>,
    State(state): State<Arc<RwLock<ZingState>>>,
) -> Result<(), GameError> {
    ZingState::start_game(state.deref(), &login_id.0, &table_id).await
}

async fn game_status(
    login_id: LoginID,
    Path(table_id): Path<String>,
    State(state): State<Arc<RwLock<ZingState>>>,
) -> Result<Json<GameState>, GameError> {
    let state = state.read().unwrap();
    state.game_status(&login_id.0, &table_id)
}

async fn finish_game(
    login_id: LoginID,
    Path(table_id): Path<String>,
    State(state): State<Arc<RwLock<ZingState>>>,
) -> Result<(), GameError> {
    ZingState::finish_game(state.deref(), &login_id.0, &table_id).await
}

#[derive(Deserialize)]
struct GameAction {
    card_index: usize,
}

async fn play_card(
    login_id: LoginID,
    Path(table_id): Path<String>,
    State(state): State<Arc<RwLock<ZingState>>>,
    Json(game_action): Json<GameAction>,
) -> Result<(), GameError> {
    ZingState::play_card(
        state.deref(),
        &login_id.0,
        &table_id,
        game_action.card_index,
    )
    .await
}

async fn global_ws_handler(
    login_id: LoginID,
    State(state): State<Arc<RwLock<ZingState>>>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, GameError> {
    state
        .read()
        .unwrap()
        .get_user(&login_id.0)?;

    Ok(ws.on_upgrade(move |socket| {
        let sender = NotificationSenderHandle::new(socket);

        add_user_global_connection(state, login_id.0, sender)
    }))
}

async fn add_user_global_connection(
    state: Arc<RwLock<ZingState>>,
    login_id: String,
    sender: NotificationSenderHandle,
) {
    ZingState::add_user_global_connection(state.deref(), login_id, sender).await
}

async fn table_ws_handler(
    login_id: LoginID,
    Path(table_id): Path<String>,
    State(state): State<Arc<RwLock<ZingState>>>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, GameError> {
    state
        .read()
        .unwrap()
        .check_user_can_connect(&login_id.0, &table_id)?;

    Ok(ws.on_upgrade(move |socket| {
        let sender = NotificationSenderHandle::new(socket);

        add_user_table_connection(state, login_id.0, table_id, sender)
    }))
}

async fn add_user_table_connection(
    state: Arc<RwLock<ZingState>>,
    login_id: String,
    table_id: String,
    sender: NotificationSenderHandle,
) {
    ZingState::add_user_table_connection(state.deref(), login_id, table_id, sender).await
}
