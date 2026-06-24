use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::errors::AppError;
use crate::extractors::AppJson;
use crate::middleware::AuthenticatedUser;
use crate::models::{
    PaginationParams, UserCreateRequestDto, UserPayloadWrapper, UserUpdateRequestDto,
};
use crate::policies::UserPolicy;

#[utoipa::path(
    get,
    path = "/api/v1/users",
    tag = "Users",
    params(
        PaginationParams
    ),
    responses(
        (status = 200, description = "Successfully fetched paginated list of users", body = crate::models::PaginatedResponse<crate::serializers::user_serializer::UserSerializer>),
        (status = 401, description = "Unauthorized", body = crate::serializers::user_serializer::ErrorResponseDto),
        (status = 403, description = "Forbidden (Admin only)", body = crate::serializers::user_serializer::ErrorResponseDto)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn index(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, AppError> {
    if !UserPolicy::index(&claims) {
        return Err(AppError::Authorization("unauthorized".to_string()));
    }

    let paginated_res = state.user_service.get_users_paginated(params).await?;

    Ok(Json(serde_json::to_value(paginated_res).map_err(|e| {
        AppError::Unexpected(anyhow::anyhow!(
            "Failed to serialize paginated response: {}",
            e
        ))
    })?))
}

#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    tag = "Users",
    params(
        ("id" = Uuid, Path, description = "User ID to fetch")
    ),
    responses(
        (status = 200, description = "User fetched successfully", body = crate::serializers::user_serializer::UserResponseDto),
        (status = 401, description = "Unauthorized", body = crate::serializers::user_serializer::ErrorResponseDto),
        (status = 403, description = "Forbidden", body = crate::serializers::user_serializer::ErrorResponseDto),
        (status = 404, description = "User not found", body = crate::serializers::user_serializer::ErrorResponseDto)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn show(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    if !UserPolicy::show(&claims, id) {
        return Err(AppError::Authorization("unauthorized".to_string()));
    }

    let user_dto = state.user_service.get_user(id).await?;

    Ok(Json(json!({
        "status": StatusCode::OK.as_u16(),
        "message": "Successfully data fetched",
        "data": user_dto
    })))
}

#[utoipa::path(
    get,
    path = "/api/v1/users/me",
    tag = "Users",
    responses(
        (status = 200, description = "Current user profile fetched successfully", body = crate::serializers::user_serializer::UserResponseDto),
        (status = 401, description = "Unauthorized", body = crate::serializers::user_serializer::ErrorResponseDto)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn me(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
) -> Result<Json<Value>, AppError> {
    let user_dto = state.user_service.get_user(claims.user_id).await?;

    Ok(Json(json!({
        "status": StatusCode::OK.as_u16(),
        "message": "Successfully data fetched",
        "data": user_dto
    })))
}

#[utoipa::path(
    patch,
    path = "/api/v1/users/me",
    tag = "Users",
    request_body = UserUpdateRequestDto,
    responses(
        (status = 200, description = "Current user profile updated successfully", body = crate::serializers::user_serializer::UserResponseDto),
        (status = 401, description = "Unauthorized", body = crate::serializers::user_serializer::ErrorResponseDto),
        (status = 422, description = "Validation failed / Invalid input", body = crate::serializers::user_serializer::ErrorResponseDto)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn update_me(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    AppJson(payload_wrapper): AppJson<UserPayloadWrapper<UserUpdateRequestDto>>,
) -> Result<Json<Value>, AppError> {
    let mut payload = payload_wrapper.into_inner();

    // Normal users cannot change their own role or status, ensure they are stripped
    if claims.role != 0 {
        payload.role = None;
        payload.status = None;
        payload.deleted_at = None;
    }

    let user_dto = state
        .user_service
        .update_user(claims.user_id, payload)
        .await?;

    Ok(Json(json!({
        "status": StatusCode::OK.as_u16(),
        "message": "Successfully data updated",
        "data": user_dto
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/users",
    tag = "Users",
    request_body = UserCreateRequestDto,
    responses(
        (status = 201, description = "User created successfully", body = crate::serializers::user_serializer::UserResponseDto),
        (status = 401, description = "Unauthorized", body = crate::serializers::user_serializer::ErrorResponseDto),
        (status = 403, description = "Forbidden (Admin only)", body = crate::serializers::user_serializer::ErrorResponseDto),
        (status = 422, description = "Validation failed / Invalid input", body = crate::serializers::user_serializer::ErrorResponseDto)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn create(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    AppJson(payload_wrapper): AppJson<UserPayloadWrapper<UserCreateRequestDto>>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    if claims.role != 0 {
        return Err(AppError::Authorization(
            "Only admins can create users directly".to_string(),
        ));
    }

    let payload = payload_wrapper.into_inner();
    let user_dto = state.user_service.create_user(payload).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "status": StatusCode::CREATED.as_u16(),
            "message": "Successfully data created",
            "data": user_dto
        })),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/users/{id}",
    tag = "Users",
    params(
        ("id" = Uuid, Path, description = "User ID to update")
    ),
    request_body = UserUpdateRequestDto,
    responses(
        (status = 200, description = "User updated successfully", body = crate::serializers::user_serializer::UserResponseDto),
        (status = 401, description = "Unauthorized", body = crate::serializers::user_serializer::ErrorResponseDto),
        (status = 403, description = "Forbidden", body = crate::serializers::user_serializer::ErrorResponseDto),
        (status = 404, description = "User not found", body = crate::serializers::user_serializer::ErrorResponseDto),
        (status = 422, description = "Validation failed / Invalid input", body = crate::serializers::user_serializer::ErrorResponseDto)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn update(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Path(id): Path<Uuid>,
    AppJson(payload_wrapper): AppJson<UserPayloadWrapper<UserUpdateRequestDto>>,
) -> Result<Json<Value>, AppError> {
    if !UserPolicy::update(&claims, id) {
        return Err(AppError::Authorization("unauthorized".to_string()));
    }

    let mut payload = payload_wrapper.into_inner();

    // Strip role and status updates if user is not admin
    if claims.role != 0 {
        payload.role = None;
        payload.status = None;
        payload.deleted_at = None;
    }

    let user_dto = state.user_service.update_user(id, payload).await?;

    Ok(Json(json!({
        "status": StatusCode::OK.as_u16(),
        "message": "Successfully data updated",
        "data": user_dto
    })))
}

#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}",
    tag = "Users",
    params(
        ("id" = Uuid, Path, description = "User ID to delete")
    ),
    responses(
        (status = 200, description = "User deleted successfully", body = crate::serializers::user_serializer::UserResponseDto),
        (status = 401, description = "Unauthorized", body = crate::serializers::user_serializer::ErrorResponseDto),
        (status = 403, description = "Forbidden (Admin only)", body = crate::serializers::user_serializer::ErrorResponseDto),
        (status = 404, description = "User not found", body = crate::serializers::user_serializer::ErrorResponseDto)
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn destroy(
    State(state): State<AppState>,
    AuthenticatedUser(claims): AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    if !UserPolicy::destroy(&claims) {
        return Err(AppError::Authorization("unauthorized".to_string()));
    }

    state.user_service.soft_delete_user(id).await?;

    Ok(Json(json!({
        "status": StatusCode::OK.as_u16(),
        "message": "Successfully data deleted",
        "data": serde_json::Value::Null
    })))
}
