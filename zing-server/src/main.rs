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
use serde::Deserialize;
use state::{ErrorResponse, ZingState};
use tower_cookies::{Cookie, CookieManagerLayer, Cookies};
use tracing::info;
use ws_notifications::NotificationSenderHandle;
use zing_game::game::GameState;

mod state;
mod ws_notifications;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = Arc::new(RwLock::new(ZingState::default()));

    let app = Router::new()
        .route("/", get(index))
        .route("/login", post(login).get(whoami).delete(logout))
        .route("/table", post(create_table).get(list_tables))
        .route(
            "/table/:table_id",
            post(join_table).get(get_table).delete(leave_table),
        )
        .route(
            "/table/:table_id/game",
            post(start_game).get(game_status).delete(finish_game),
        )
        .route("/table/:table_id/game/play", post(play_card))
        .route("/table/:table_id/game/ws", get(ws_handler)) // /game/ws or just /ws?
        .with_state(state)
        .layer(CookieManagerLayer::new());

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
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
) -> Result<String, ErrorResponse> {
    let mut state = state.write().unwrap();
    let user_name = login_request.name;
    if user_name.is_empty() {
        return Err((http::StatusCode::BAD_REQUEST, "name must not be empty"));
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
) -> Result<(), ErrorResponse> {
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
) -> Result<String, ErrorResponse> {
    let state = state.read().unwrap();
    match state.whoami(&login_id.0) {
        Some(user_name) => Ok(user_name),
        None => Err((http::StatusCode::UNAUTHORIZED, "no valid login cookie")),
    }
}

async fn create_table(
    login_id: LoginID,
    State(state): State<Arc<RwLock<ZingState>>>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let mut state = state.write().unwrap();
    state.create_table(&login_id.0)
}

async fn list_tables(
    login_id: LoginID,
    State(state): State<Arc<RwLock<ZingState>>>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let state = state.read().unwrap();
    state.list_tables(&login_id.0)
}

async fn get_table(
    login_id: LoginID,
    Path(table_id): Path<String>,
    State(state): State<Arc<RwLock<ZingState>>>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let state = state.read().unwrap();
    state.get_table(&login_id.0, &table_id)
}

async fn join_table(
    login_id: LoginID,
    Path(table_id): Path<String>,
    State(state): State<Arc<RwLock<ZingState>>>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let mut state = state.write().unwrap();
    state.join_table(&login_id.0, &table_id)
}

async fn leave_table(
    login_id: LoginID,
    Path(table_id): Path<String>,
    State(state): State<Arc<RwLock<ZingState>>>,
) -> Result<(), ErrorResponse> {
    let mut state = state.write().unwrap();
    state.leave_table(&login_id.0, &table_id)
}

async fn start_game(
    login_id: LoginID,
    Path(table_id): Path<String>,
    State(state): State<Arc<RwLock<ZingState>>>,
) -> Result<(), ErrorResponse> {
    ZingState::start_game(state.deref(), &login_id.0, &table_id).await
}

async fn game_status(
    login_id: LoginID,
    Path(table_id): Path<String>,
    State(state): State<Arc<RwLock<ZingState>>>,
) -> Result<Json<GameState>, ErrorResponse> {
    let state = state.read().unwrap();
    state.game_status(&login_id.0, &table_id)
}

async fn finish_game(
    login_id: LoginID,
    Path(table_id): Path<String>,
    State(state): State<Arc<RwLock<ZingState>>>,
) -> Result<(), ErrorResponse> {
    let mut state = state.write().unwrap();
    state.finish_game(&login_id.0, &table_id)
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
) -> Result<(), ErrorResponse> {
    ZingState::play_card(
        state.deref(),
        &login_id.0,
        &table_id,
        game_action.card_index,
    )
    .await
}

async fn ws_handler(
    login_id: LoginID,
    Path(table_id): Path<String>,
    State(state): State<Arc<RwLock<ZingState>>>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, ErrorResponse> {
    state
        .read()
        .unwrap()
        .check_user_can_connect(&login_id.0, &table_id)?;

    Ok(ws.on_upgrade(move |socket| {
        let sender = NotificationSenderHandle::new(socket);

        add_user_connection(state, login_id.0, table_id, sender)
    }))
}

async fn add_user_connection(
    state: Arc<RwLock<ZingState>>,
    login_id: String,
    table_id: String,
    sender: NotificationSenderHandle,
) {
    ZingState::add_user_connection(state.deref(), login_id, table_id, sender).await
}
