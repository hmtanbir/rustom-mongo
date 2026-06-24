use crate::models::user::User;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// User details response serialized format.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserSerializer {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    #[schema(example = "user")]
    pub role: String,
    #[schema(example = "active")]
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl From<User> for UserSerializer {
    fn from(user: User) -> Self {
        let role_str = crate::models::user::ROLES_MAP
            .iter()
            .find(|&(_, &val)| val == user.role)
            .map(|(key, _)| key.clone())
            .unwrap_or_else(|| user.role.to_string());

        let status_str = crate::models::user::STATUSES_MAP
            .iter()
            .find(|&(_, &val)| val == user.status)
            .map(|(key, _)| key.clone())
            .unwrap_or_else(|| user.status.to_string());

        UserSerializer {
            id: user.id,
            name: user.name,
            email: user.email,
            role: role_str,
            status: status_str,
            created_at: user.created_at,
            updated_at: user.updated_at,
            deleted_at: user.deleted_at,
        }
    }
}

/// Wrapper response containing user details.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserResponseDto {
    pub status: u16,
    pub message: String,
    pub data: UserSerializer,
}

/// Wrapper response containing session token details.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SessionResponseDto {
    pub status: u16,
    pub message: String,
    pub data: crate::models::UserLoginResponseDto,
}

/// Standard error response wrapper.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponseDto {
    pub status: u16,
    pub message: String,
    pub data: Option<serde_json::Value>,
}
