use axum::{http, response::IntoResponse};

pub enum GameError {
    Unauthorized(&'static str),
    NotFound(&'static str),
    BadRequest(&'static str),
    Conflict(&'static str),
    DBError(&'static str),
}

impl IntoResponse for GameError {
    fn into_response(self) -> axum::response::Response {
        match self {
            GameError::Unauthorized(msg) => (http::StatusCode::UNAUTHORIZED, msg).into_response(),
            GameError::NotFound(msg) => (http::StatusCode::NOT_FOUND, msg).into_response(),
            GameError::BadRequest(msg) => (http::StatusCode::BAD_REQUEST, msg).into_response(),
            GameError::Conflict(msg) => (http::StatusCode::CONFLICT, msg).into_response(),
            GameError::DBError(msg) => {
                (http::StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
            }
        }
    }
}
