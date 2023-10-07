use std::{ops::Deref, sync::Arc};

use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, Path, State, WebSocketUpgrade},
    http::request::Parts,
    response::IntoResponse,
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
    let state = Arc::new(ZingState::default());

    let app = Router::new()
        .nest_service("/", ServeFile::new("zing-server/assets/index.html"))
        .route("/login", post(login).get(whoami).delete(logout))
        .route("/table", post(create_table).get(list_tables))
        .route("/ws", get(global_ws_handler))
        .route(
            "/table/:table_id",
            post(join_table).get(get_table_info).delete(leave_table),
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

const LOGIN_COOKIE: &str = "login_id";

async fn login(
    State(state): State<Arc<ZingState>>,
    cookies: Cookies,
    Json(login_request): Json<LoginRequest>,
) -> Result<String, GameError> {
    let user_name = login_request.name;
    if user_name.is_empty() {
        return Err(GameError::BadRequest("name must not be empty"));
    }
    let login_token = state.login(&user_name);
    info!("Logged in {} as {}", user_name, login_token);

    // TODO: report error if LOGIN_COOKIE is already set (and valid)?

    let mut login_cookie = Cookie::new(LOGIN_COOKIE, login_token);
    login_cookie.set_same_site(SameSite::Strict);
    cookies.add(login_cookie);
    Ok(user_name)
}

async fn logout(
    State(state): State<Arc<ZingState>>,
    LoginToken(login_token): LoginToken,
    cookies: Cookies,
) -> Result<(), GameError> {
    let user_name = state.whoami(&login_token);
    state.logout(&login_token)?;
    info!("Logged out {}", user_name.unwrap());

    let mut login_cookie = Cookie::new(LOGIN_COOKIE, "");
    login_cookie.set_same_site(SameSite::Strict);
    cookies.remove(login_cookie);
    Ok(())
}

struct LoginToken(String);

#[async_trait]
impl<S> FromRequestParts<S> for LoginToken
where
    S: Send + Sync,
{
    type Rejection = GameError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let cookies = Cookies::from_request_parts(parts, state)
            .await
            .map_err(|_| GameError::Unauthorized("internal error trying to extract cookies"))?;

        let login_token = cookies
            .get(LOGIN_COOKIE)
            .ok_or(GameError::Unauthorized("login first (id cookie missing)"))?
            .value()
            .to_string();

        Ok(LoginToken(login_token))
    }
}

async fn whoami(
    State(state): State<Arc<ZingState>>,
    LoginToken(login_token): LoginToken,
) -> Result<String, GameError> {
    match state.whoami(&login_token) {
        Some(user_name) => Ok(user_name),
        None => Err(GameError::Unauthorized("no valid login cookie")),
    }
}

async fn create_table(
    LoginToken(login_token): LoginToken,
    State(state): State<Arc<ZingState>>,
) -> Result<impl IntoResponse, GameError> {
    state.create_table(&login_token)
}

async fn list_tables(
    LoginToken(login_token): LoginToken,
    State(state): State<Arc<ZingState>>,
) -> Result<impl IntoResponse, GameError> {
    state.list_tables(&login_token)
}

async fn get_table_info(
    LoginToken(login_token): LoginToken,
    Path(table_id): Path<String>,
    State(state): State<Arc<ZingState>>,
) -> Result<impl IntoResponse, GameError> {
    state.get_table_info(&login_token, &table_id)
}

async fn join_table(
    LoginToken(login_token): LoginToken,
    Path(table_id): Path<String>,
    State(state): State<Arc<ZingState>>,
) -> Result<impl IntoResponse, GameError> {
    ZingState::join_table(state.deref(), &login_token, &table_id).await
}

async fn leave_table(
    LoginToken(login_token): LoginToken,
    Path(table_id): Path<String>,
    State(state): State<Arc<ZingState>>,
) -> Result<(), GameError> {
    state.leave_table(&login_token, &table_id)
}

async fn start_game(
    LoginToken(login_token): LoginToken,
    Path(table_id): Path<String>,
    State(state): State<Arc<ZingState>>,
) -> Result<(), GameError> {
    ZingState::start_game(state.deref(), &login_token, &table_id).await
}

async fn game_status(
    LoginToken(login_token): LoginToken,
    Path(table_id): Path<String>,
    State(state): State<Arc<ZingState>>,
) -> Result<Json<GameState>, GameError> {
    state.game_status(&login_token, &table_id)
}

async fn finish_game(
    LoginToken(login_token): LoginToken,
    Path(table_id): Path<String>,
    State(state): State<Arc<ZingState>>,
) -> Result<(), GameError> {
    ZingState::finish_game(state.deref(), &login_token, &table_id).await
}

#[derive(Deserialize)]
struct GameAction {
    card_index: usize,
}

async fn play_card(
    LoginToken(login_token): LoginToken,
    Path(table_id): Path<String>,
    State(state): State<Arc<ZingState>>,
    Json(game_action): Json<GameAction>,
) -> Result<(), GameError> {
    ZingState::play_card(
        state.deref(),
        &login_token,
        &table_id,
        game_action.card_index,
    )
    .await
}

async fn global_ws_handler(
    LoginToken(login_token): LoginToken,
    State(state): State<Arc<ZingState>>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, GameError> {
    state.get_user(&login_token)?;

    Ok(ws.on_upgrade(move |socket| {
        let sender = NotificationSenderHandle::new(socket);

        add_user_global_connection(state, login_token, sender)
    }))
}

async fn add_user_global_connection(
    state: Arc<ZingState>,
    login_token: String,
    sender: NotificationSenderHandle,
) {
    ZingState::add_user_global_connection(state.deref(), login_token, sender).await
}

async fn table_ws_handler(
    LoginToken(login_token): LoginToken,
    Path(table_id): Path<String>,
    State(state): State<Arc<ZingState>>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, GameError> {
    state.check_user_can_connect(&login_token, &table_id)?;

    Ok(ws.on_upgrade(move |socket| {
        let sender = NotificationSenderHandle::new(socket);

        add_user_table_connection(state, login_token, table_id, sender)
    }))
}

async fn add_user_table_connection(
    state: Arc<ZingState>,
    login_token: String,
    table_id: String,
    sender: NotificationSenderHandle,
) {
    ZingState::add_user_table_connection(state.deref(), login_token, table_id, sender).await
}
