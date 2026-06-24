use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::Claims;
use crate::services::EncryptionService;
use axum::{
    Extension, async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use jsonwebtoken::{DecodingKey, Validation, decode};

/// Extractor type to enforce and inspect JWT authenticated users.
pub struct AuthenticatedUser(pub Claims);

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Extension(config) = Extension::<AppConfig>::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                AppError::Unexpected(anyhow::anyhow!(
                    "AppConfig was not injected via Router extensions"
                ))
            })?;

        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AppError::Authentication("Missing Authorization token".to_string()))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::Authentication(
                "Invalid Authorization header format. Expected Bearer token format".to_string(),
            ));
        }

        let mut token = auth_header[7..].to_string();

        // If encryption is enabled, try to decrypt the token.
        // The user might be sending the entire encrypted response.
        if EncryptionService::encryption_enabled()
            && let Ok(decrypted_string) = EncryptionService::decrypt(&token)
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&decrypted_string)
            && let Some(t) = parsed
                .get("data")
                .and_then(|d| d.get("token"))
                .and_then(|t| t.as_str())
        {
            token = t.to_string();
        }

        let token_data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| {
            AppError::Authentication("Invalid or expired user session token".to_string())
        })?;

        // Get state extension to verify user against database
        let Extension(state) =
            Extension::<crate::app_state::AppState>::from_request_parts(parts, state)
                .await
                .map_err(|_| {
                    AppError::Unexpected(anyhow::anyhow!(
                        "AppState was not injected via Router extensions"
                    ))
                })?;

        // Fetch the user and verify status
        let user = state
            .user_service
            .get_user(token_data.claims.user_id)
            .await
            .map_err(|_| {
                AppError::Authentication("User record not found or suspended".to_string())
            })?;

        // Ensure user status is active and not deleted
        if user.status != "active" || user.deleted_at.is_some() {
            return Err(AppError::Authentication(
                "Your account has been deactivated or suspended".to_string(),
            ));
        }

        Ok(AuthenticatedUser(token_data.claims))
    }
}
