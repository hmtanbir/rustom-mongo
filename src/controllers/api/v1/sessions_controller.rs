use axum::{Json, extract::State, http::StatusCode};
use serde_json::{Value, json};

use crate::app_state::AppState;
use crate::config::API_RATE_LIMIT;
use crate::errors::AppError;
use crate::extractors::AppJson;
use crate::models::{UserLoginRequestDto, UserPayloadWrapper};

#[utoipa::path(
    post,
    path = "/api/v1/sessions",
    request_body = UserLoginRequestDto,
    responses(
        (status = 200, description = "Successfully logged in", body = crate::serializers::user_serializer::SessionResponseDto),
        (status = 401, description = "Invalid credentials / authentication failed", body = crate::serializers::user_serializer::ErrorResponseDto),
        (status = 422, description = "Invalid input / fields missing", body = crate::serializers::user_serializer::ErrorResponseDto)
    ),
    tag = "Auth"
)]

pub async fn create(
    State(state): State<AppState>,
    AppJson(payload_wrapper): AppJson<UserPayloadWrapper<UserLoginRequestDto>>,
) -> Result<Json<Value>, AppError> {
    let payload = payload_wrapper.into_inner();

    if payload.email.trim().is_empty() || payload.password.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "invalid email or password".to_string(),
        ));
    }

    // Rate limiting: API_RATE_LIMIT requests per 60 seconds per email identifier
    let rate_limit_key = format!("rate_limit:login:{}", payload.email);
    if let Ok(count) = state
        .user_service
        .get_cache()
        .incr_with_ttl(&rate_limit_key, 60)
        .await
        && count > *API_RATE_LIMIT
    {
        return Err(AppError::Authorization(
            "Too many login attempts. Please try again in a minute.".to_string(),
        ));
    }

    // Attempt login via service
    let login_res = state.user_service.login(payload).await?;

    Ok(Json(json!({
        "status": StatusCode::OK.as_u16(),
        "message": "Successfully data fetched",
        "data": {
            "token": login_res.token
        }
    })))
}
