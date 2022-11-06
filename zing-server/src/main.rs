use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::{
    extract::{FromRequest, Query, RequestParts, Path},
    http,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use state::State;
use tower_cookies::{Cookie, CookieManagerLayer, Cookies};

mod state;

#[tokio::main]
async fn main() {
    //let tables = HashMap::new();
    let state = Arc::new(Mutex::new(State::default()));

    let app = Router::new()
        .route("/login", get(login))
        .route("/logout", get(logout))
        .route("/table", post(create_table))
        .route("/table/:table_id", post(join_table).delete(leave_table))
        .layer(Extension(state))
        .layer(CookieManagerLayer::new());

    // run it with hyper on localhost:3000
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
    Query(login_request): Query<LoginRequest>,
    Extension(state): Extension<Arc<Mutex<State>>>,
    cookies: Cookies,
) {
    let mut state = state.lock().unwrap();
    let login_id = state.login(login_request.name.clone());
    println!("Logged in {} as {}", login_request.name, login_id);

    cookies.add(Cookie::new(USERNAME_COOKIE, login_id));
}

async fn logout(cookies: Cookies) {
    cookies.remove(Cookie::new(USERNAME_COOKIE, ""));
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

async fn create_table(
    login_id: LoginID,
    Extension(state): Extension<Arc<Mutex<State>>>,
) -> Result<String, (http::StatusCode, &'static str)> {
    let mut state = state.lock().unwrap();
    state.create_table(login_id.0)
}

async fn join_table(
    login_id: LoginID,
    Path(table_id): Path<String>,
    Extension(state): Extension<Arc<Mutex<State>>>,
) -> Result<(), (http::StatusCode, &'static str)> {
    let mut state = state.lock().unwrap();
    state.join_table(login_id.0, table_id)
}

async fn leave_table(
    login_id: LoginID,
    Path(table_id): Path<String>,
    Extension(state): Extension<Arc<Mutex<State>>>,
) -> Result<(), (http::StatusCode, &'static str)> {
    let mut state = state.lock().unwrap();
    state.leave_table(login_id.0, table_id)
}
