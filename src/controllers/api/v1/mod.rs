pub mod registration_controller;
pub mod sessions_controller;
pub mod users_controller;

use crate::app_state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/registration", post(registration_controller::registration))
        .route("/sessions", post(sessions_controller::create))
        .route(
            "/users",
            get(users_controller::index).post(users_controller::create),
        )
        .route(
            "/users/me",
            get(users_controller::me).patch(users_controller::update_me),
        )
        .route(
            "/users/:id",
            get(users_controller::show)
                .patch(users_controller::update)
                .delete(users_controller::destroy),
        )
}
