pub mod v1;

use crate::app_state::AppState;
use axum::Router;

pub fn routes() -> Router<AppState> {
    Router::new().nest("/v1", v1::routes())
}
