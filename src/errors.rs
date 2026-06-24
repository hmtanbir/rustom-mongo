use crate::services::slack_notification::SlackNotification;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

/// Centralized application error enum mapping domain errors to HTTP status codes.
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Internal database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Message queue error: {0}")]
    Queue(#[from] lapin::Error),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Authorization failed: {0}")]
    Authorization(String),

    #[error("Invalid request input: {0}")]
    InvalidInput(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::Database(err) => {
                tracing::error!("Database error occurred: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal database error".to_string(),
                )
            }
            AppError::Cache(err) => {
                tracing::error!("Cache error occurred: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal cache error".to_string(),
                )
            }
            AppError::Queue(err) => {
                tracing::error!("Queue error occurred: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal queue error".to_string(),
                )
            }
            AppError::Authentication(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Authorization(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::InvalidInput(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()), // Matching Rails unprocessable_content
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::Unexpected(err) => {
                tracing::error!("Unexpected application error: {:?}", err);
                let msg = err.to_string();

                // Spawn a background task to send Slack notification
                let slack_msg = format!("[Error] Exception occurred\nMessage: {}\n", msg);
                tokio::spawn(async move {
                    let _ = SlackNotification::notify_error(&slack_msg).await;
                });

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An unexpected error occurred".to_string(),
                )
            }
        };

        let body = Json(json!({
            "status": status.as_u16(),
            "message": error_message,
            "data": null
        }));

        (status, body).into_response()
    }
}
