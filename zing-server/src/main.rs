use std::{collections::HashMap, sync::{Mutex, Arc}};

use axum::{extract::Query, routing::get, Extension, Json, Router};
use rand::distributions::{Alphanumeric, DistString};
use serde::Deserialize;

#[derive(Default)]
struct User {
    name: String,
    tables: Vec<String>,
}

#[derive(Default)]
struct Table {
    users: Vec<String>,
}

#[derive(Default)]
struct State {
    users: HashMap<String, User>,
    tables: HashMap<String, Table>,
}

pub fn random_id() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 16)
}

#[tokio::main]
async fn main() {
    //let tables = HashMap::new();
    let state = Arc::new(Mutex::new(State::default()));

    let app = Router::new().route("/", get(login)).layer(Extension(state));

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

async fn login(Query(login_request): Query<LoginRequest>, Extension(state): Extension<Arc<Mutex<State>>>) {
    let id = random_id();
    let mut state = state.lock().unwrap();
    println!("Logged in {} as {}", login_request.name, id);
    state.users.insert(
        id,
        User {
            name: login_request.name,
            ..Default::default()
        },
    );
}
