use axum::{Json, extract::State, http::StatusCode};
use serde_json::{Value, json};

use crate::app_state::AppState;
use crate::config::API_RATE_LIMIT;
use crate::errors::AppError;
use crate::extractors::AppJson;
use crate::models::{UserPayloadWrapper, UserRegisterRequestDto};
use crate::services::SlackNotification;

#[utoipa::path(
    post,
    path = "/api/v1/registration",
    request_body = UserRegisterRequestDto,
    responses(
        (status = 201, description = "User successfully registered", body = crate::serializers::user_serializer::UserResponseDto),
        (status = 422, description = "Invalid input / validation failed", body = crate::serializers::user_serializer::ErrorResponseDto)
    ),
    tag = "Auth"
)]
pub async fn registration(
    State(state): State<AppState>,
    AppJson(payload_wrapper): AppJson<UserPayloadWrapper<UserRegisterRequestDto>>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let mut payload = payload_wrapper.into_inner();

    if payload.email.trim().is_empty()
        || payload.password.trim().is_empty()
        || payload.name.trim().is_empty()
    {
        return Err(AppError::InvalidInput(
            "Name, email and password are required".to_string(),
        ));
    }

    // Rate limiting: API_RATE_LIMIT requests per 60 seconds per email identifier
    let rate_limit_key = format!("rate_limit:register:{}", payload.email);
    if let Ok(count) = state
        .user_service
        .get_cache()
        .incr_with_ttl(&rate_limit_key, 60)
        .await
        && count > *API_RATE_LIMIT
    {
        return Err(AppError::Authorization(
            "Too many registration attempts. Please try again in a minute.".to_string(),
        ));
    }

    // Force role=1 (standard user) and status=1 (active) for public registration
    payload.role = Some(1);
    payload.status = Some(1);

    let user_dto = state.user_service.register(payload).await?;

    // Send Slack notification for user registration asynchronously
    let slack_message = format!(
        "New user registered: {} ({})",
        user_dto.name, user_dto.email
    );
    tokio::spawn(async move {
        let _ = SlackNotification::notify_registration(&slack_message).await;
    });

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "status": StatusCode::CREATED.as_u16(),
            "message": "Successfully data created",
            "data": user_dto
        })),
    ))
}
