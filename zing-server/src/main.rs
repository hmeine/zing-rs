use std::sync::Arc;

use axum::{
    extract::{FromRequestParts, Path, State, WebSocketUpgrade},
    http::request::Parts,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use game_error::GameError;
use migration::MigratorTrait;
use sea_orm::SqlxPostgresConnector;
use serde::Deserialize;
use tower_cookies::cookie::SameSite;
use tower_cookies::{Cookie, CookieManagerLayer, Cookies};
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;
use ws_notifications::NotificationSenderHandle;
use zing_game::game::GameState;
use zing_state::ZingState;

mod client_connection;
mod entities;
mod game_error;
mod table;
mod util;
mod ws_notifications;
mod zing_state;

#[shuttle_runtime::main]
async fn axum(#[shuttle_shared_db::Postgres] pool: sqlx::PgPool) -> shuttle_axum::ShuttleAxum {
    let conn = SqlxPostgresConnector::from_sqlx_postgres_pool(pool);

    migration::Migrator::up(&conn, None)
        .await
        .expect("DB migration failed");

    let state = Arc::new(ZingState::new(conn).await);

    let app = Router::new()
        .route_service("/", ServeFile::new("zing-server/assets/index.html"))
        .route("/login", post(login).get(whoami).delete(logout))
        .route("/table", post(create_table).get(list_tables))
        .route("/ws", get(global_ws_handler))
        .route(
            "/table/{table_id}",
            post(join_table).get(get_table_info).delete(leave_table),
        )
        .route(
            "/table/{table_id}/game",
            post(start_game).get(game_status).delete(finish_game),
        )
        .route("/table/{table_id}/game/play", post(play_card))
        .route("/table/{table_id}/ws", get(table_ws_handler))
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
    let login_token = state.login(&user_name).await?;
    info!("Logged in {} as {}", user_name, login_token);

    // TODO: report error if LOGIN_COOKIE is already set (and valid)?

    let mut login_cookie = Cookie::new(LOGIN_COOKIE, login_token);
    login_cookie.set_same_site(SameSite::Strict);
    cookies.add(login_cookie);
    Ok(user_name)
}

async fn logout(
    State(state): State<Arc<ZingState>>,
    AuthenticatedUser(user): AuthenticatedUser,
    cookies: Cookies,
) -> Result<(), GameError> {
    state.logout(user.clone()).await?;
    info!("Logged out {}", user.name);

    let mut login_cookie = Cookie::new(LOGIN_COOKIE, "");
    login_cookie.set_same_site(SameSite::Strict);
    cookies.remove(login_cookie);
    Ok(())
}

struct LoginToken(String);

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

struct AuthenticatedUser(entities::user::Model);

impl FromRequestParts<Arc<ZingState>> for AuthenticatedUser {
    type Rejection = GameError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<ZingState>,
    ) -> Result<Self, Self::Rejection> {
        let LoginToken(login_token) = LoginToken::from_request_parts(parts, state).await?;

        let user = state.get_user_with_token(&login_token).await?;

        Ok(AuthenticatedUser(user))
    }
}

async fn whoami(AuthenticatedUser(user): AuthenticatedUser) -> Result<String, GameError> {
    Ok(user.name.clone())
}

async fn create_table(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<Arc<ZingState>>,
) -> Result<impl IntoResponse, GameError> {
    state.create_table(user).await
}

async fn list_tables(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<Arc<ZingState>>,
) -> Result<impl IntoResponse, GameError> {
    state.list_tables(user).await
}

async fn get_table_info(
    AuthenticatedUser(_user): AuthenticatedUser,
    Path(table_id): Path<String>,
    State(state): State<Arc<ZingState>>,
) -> Result<impl IntoResponse, GameError> {
    state.get_table_info(&table_id).await
}

async fn join_table(
    AuthenticatedUser(user): AuthenticatedUser,
    Path(table_id): Path<String>,
    State(state): State<Arc<ZingState>>,
) -> Result<impl IntoResponse, GameError> {
    state.join_table(&user, &table_id).await
}

async fn leave_table(
    AuthenticatedUser(user): AuthenticatedUser,
    Path(table_id): Path<String>,
    State(state): State<Arc<ZingState>>,
) -> Result<(), GameError> {
    state.leave_table(&user, &table_id).await
}

async fn start_game(
    AuthenticatedUser(user): AuthenticatedUser,
    Path(table_id): Path<String>,
    State(state): State<Arc<ZingState>>,
) -> Result<(), GameError> {
    state.start_game(&user, &table_id).await
}

async fn game_status(
    AuthenticatedUser(user): AuthenticatedUser,
    Path(table_id): Path<String>,
    State(state): State<Arc<ZingState>>,
) -> Result<Json<GameState>, GameError> {
    state.game_status(&user, &table_id).await
}

async fn finish_game(
    AuthenticatedUser(user): AuthenticatedUser,
    Path(table_id): Path<String>,
    State(state): State<Arc<ZingState>>,
) -> Result<(), GameError> {
    state.finish_game(&user, &table_id).await
}

#[derive(Deserialize)]
struct GameAction {
    card_index: usize,
}

async fn play_card(
    AuthenticatedUser(user): AuthenticatedUser,
    Path(table_id): Path<String>,
    State(state): State<Arc<ZingState>>,
    Json(game_action): Json<GameAction>,
) -> Result<(), GameError> {
    state
        .play_card(&user, &table_id, game_action.card_index)
        .await
}

async fn global_ws_handler(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<Arc<ZingState>>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, GameError> {
    Ok(ws.on_upgrade(move |socket| {
        let sender = NotificationSenderHandle::new(socket);

        async move { state.add_user_global_connection(user, sender).await }
    }))
}

async fn table_ws_handler(
    AuthenticatedUser(user): AuthenticatedUser,
    Path(table_id): Path<String>,
    State(state): State<Arc<ZingState>>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, GameError> {
    state.user_index_at_table(&user.clone(), &table_id).await?;

    Ok(ws.on_upgrade(move |socket| {
        let sender = NotificationSenderHandle::new(socket);

        async move {
            state
                .add_user_table_connection(user, table_id, sender)
                .await
        }
    }))
}
