use axum::{extract::Request, middleware::Next, response::Response};
use std::env;
use std::sync::LazyLock;

use crate::errors::AppError;

static API_GATEWAY_KEY_VAL: LazyLock<String> =
    LazyLock::new(|| env::var("API_GATEWAY_KEY").unwrap_or_default());

static API_GATEWAY_ERROR_MESSAGE_VAL: LazyLock<String> = LazyLock::new(|| {
    env::var("API_GATEWAY_ERROR_MESSAGE")
        .unwrap_or_else(|_| "Invalid User API Gateway Key".to_string())
});

static APP_ENV: LazyLock<String> =
    LazyLock::new(|| env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()));

pub async fn verify_api_gateway_key(req: Request, next: Next) -> Result<Response, AppError> {
    let expected_key = &*API_GATEWAY_KEY_VAL;
    let error_message = &*API_GATEWAY_ERROR_MESSAGE_VAL;

    if expected_key.is_empty() {
        if *APP_ENV == "production" {
            return Err(AppError::Authorization(error_message.clone()));
        }
        return Ok(next.run(req).await);
    }

    if let Some(provided_key) = req.headers().get("x-api-gateway-key")
        && let Ok(key_str) = provided_key.to_str()
    {
        use subtle::ConstantTimeEq;
        if key_str.as_bytes().ct_eq(expected_key.as_bytes()).into() {
            return Ok(next.run(req).await);
        }
    }

    Err(AppError::Authorization(error_message.clone()))
}
