use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::{
    extract::{FromRequest, Path, RequestParts},
    http,
    response::Html,
    routing::{get, post},
    Extension, Json, Router,
};
use cookie::SameSite;
use serde::Deserialize;
use state::{ErrorResponse, State};
use tower_cookies::{Cookie, CookieManagerLayer, Cookies};

mod state;

#[tokio::main]
async fn main() {
    //let tables = HashMap::new();
    let state = Arc::new(Mutex::new(State::default()));

    let app = Router::new()
        .route("/", get(index))
        .route("/login", post(login).get(whoami).delete(logout))
        .route("/table", post(create_table))
        .route("/table/:table_id", post(join_table).delete(leave_table))
        .route("/table/:table_id/game", post(start_game))
        .layer(Extension(state))
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
    Json(login_request): Json<LoginRequest>,
    Extension(state): Extension<Arc<Mutex<State>>>,
    cookies: Cookies,
) -> Result<String, ErrorResponse> {
    let mut state = state.lock().unwrap();
    let user_name = login_request.name;
    if user_name.is_empty() {
        return Err((http::StatusCode::BAD_REQUEST, "name must not be empty"));
    }
    let login_id = state.login(user_name.clone());
    println!("Logged in {} as {}", user_name, login_id);

    // TODO: log out if USERNAME_COOKIE is already set (and valid)

    let mut login_cookie = Cookie::new(USERNAME_COOKIE, login_id);
    login_cookie.set_same_site(SameSite::Strict);
    cookies.add(login_cookie);
    Ok(user_name)
}

async fn logout(cookies: Cookies) {
    let mut login_cookie = Cookie::new(USERNAME_COOKIE, "");
    login_cookie.set_same_site(SameSite::Strict);
    cookies.remove(login_cookie);
}

struct LoginID(String);

#[async_trait]
impl<B> FromRequest<B> for LoginID
where
    B: Send,
{
    type Rejection = (http::StatusCode, &'static str);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let cookies = Cookies::from_request(req).await?;

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

async fn index() -> Html<&'static str> {
    Html(std::include_str!("../assets/index.html"))
}

async fn whoami(
    Extension(state): Extension<Arc<Mutex<State>>>,
    login_id: LoginID,
) -> Result<String, ErrorResponse> {
    let state = state.lock().unwrap();
    match state.whoami(login_id.0) {
        Some(user_name) => Ok(user_name),
        None => Err((http::StatusCode::UNAUTHORIZED, "no valid login cookie")),
    }
}

async fn create_table(
    login_id: LoginID,
    Extension(state): Extension<Arc<Mutex<State>>>,
) -> Result<String, ErrorResponse> {
    let mut state = state.lock().unwrap();
    state.create_table(login_id.0)
}

async fn join_table(
    login_id: LoginID,
    Path(table_id): Path<String>,
    Extension(state): Extension<Arc<Mutex<State>>>,
) -> Result<(), ErrorResponse> {
    let mut state = state.lock().unwrap();
    state.join_table(login_id.0, table_id)
}

async fn leave_table(
    login_id: LoginID,
    Path(table_id): Path<String>,
    Extension(state): Extension<Arc<Mutex<State>>>,
) -> Result<(), ErrorResponse> {
    let mut state = state.lock().unwrap();
    state.leave_table(login_id.0, table_id)
}

async fn start_game(
    login_id: LoginID,
    Path(table_id): Path<String>,
    Extension(state): Extension<Arc<Mutex<State>>>,
) -> Result<(), ErrorResponse> {
    let mut state = state.lock().unwrap();
    state.start_game(login_id.0, table_id)
}
