use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Wrapper to support Rails `wrap_parameters` behavior.
/// This seamlessly accepts both `{"user": {...}}` and unwrapped payloads.
#[derive(Clone, Debug)]
pub enum UserPayloadWrapper<T> {
    Wrapped { user: T },
    Unwrapped(T),
}

impl<'de, T> serde::Deserialize<'de> for UserPayloadWrapper<T>
where
    T: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        if let Some(obj) = value.as_object()
            && let Some(user_val) = obj.get("user")
        {
            // If the "user" key exists, parse its contents as T
            let inner = T::deserialize(user_val.clone()).map_err(serde::de::Error::custom)?;
            return Ok(UserPayloadWrapper::Wrapped { user: inner });
        }

        // Fallback: parse the whole object as T
        let inner = T::deserialize(value).map_err(serde::de::Error::custom)?;
        Ok(UserPayloadWrapper::Unwrapped(inner))
    }
}

impl<T> UserPayloadWrapper<T> {
    pub fn into_inner(self) -> T {
        match self {
            Self::Wrapped { user } => user,
            Self::Unwrapped(inner) => inner,
        }
    }
}

use crate::utils::parse_yaml_map;
use serde::Deserializer;
use std::collections::HashMap;
use std::sync::LazyLock;

pub static ROLES_MAP: LazyLock<HashMap<String, i32>> = LazyLock::new(|| {
    let paths = [
        "src/config/data/roles.yml",
        "config/data/roles.yml",
        "roles.yml",
    ];
    for path in &paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            return parse_yaml_map(&content);
        }
    }
    let mut m = HashMap::new();
    m.insert("admin".to_string(), 0);
    m.insert("user".to_string(), 1);
    m
});

pub static STATUSES_MAP: LazyLock<HashMap<String, i32>> = LazyLock::new(|| {
    let paths = [
        "src/config/data/statuses.yml",
        "config/data/statuses.yml",
        "statuses.yml",
    ];
    for path in &paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            return parse_yaml_map(&content);
        }
    }
    let mut m = HashMap::new();
    m.insert("inactive".to_string(), 0);
    m.insert("active".to_string(), 1);
    m
});

fn deserialize_role<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RoleInput {
        String(String),
        Int(i32),
    }

    let opt = Option::<RoleInput>::deserialize(deserializer)?;
    match opt {
        Some(RoleInput::String(s)) => {
            let key = s.to_lowercase();
            if let Some(&val) = ROLES_MAP.get(&key) {
                Ok(Some(val))
            } else {
                Err(serde::de::Error::custom(format!("Invalid role: {}", s)))
            }
        }
        Some(RoleInput::Int(i)) => {
            if ROLES_MAP.values().any(|&v| v == i) {
                Ok(Some(i))
            } else {
                Err(serde::de::Error::custom(format!(
                    "Invalid role integer value: {}",
                    i
                )))
            }
        }
        None => Ok(None),
    }
}

fn deserialize_status<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StatusInput {
        String(String),
        Int(i32),
    }

    let opt = Option::<StatusInput>::deserialize(deserializer)?;
    match opt {
        Some(StatusInput::String(s)) => {
            let key = s.to_lowercase();
            if let Some(&val) = STATUSES_MAP.get(&key) {
                Ok(Some(val))
            } else {
                Err(serde::de::Error::custom(format!("Invalid status: {}", s)))
            }
        }
        Some(StatusInput::Int(i)) => {
            if STATUSES_MAP.values().any(|&v| v == i) {
                Ok(Some(i))
            } else {
                Err(serde::de::Error::custom(format!(
                    "Invalid status integer value: {}",
                    i
                )))
            }
        }
        None => Ok(None),
    }
}

fn deserialize_deleted_at<'de, D>(
    deserializer: D,
) -> Result<Option<Option<DateTime<Utc>>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<DateTime<Utc>>::deserialize(deserializer)?;
    Ok(Some(opt))
}

/// Core domain representation of a User in the database.
#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    /// Unique identifier for user.
    pub id: Uuid,
    /// Email address of user.
    pub email: String,
    /// Full name of the user.
    pub name: String,
    /// Password hash stored securely.
    pub password_digest: String,
    /// Role of the user (0 = Admin, 1 = User).
    pub role: i32,
    /// Status of the user (0 = Inactive, 1 = Active).
    pub status: i32,
    /// Date time when the user registration occurred.
    pub created_at: DateTime<Utc>,
    /// Date time when the user last updated details.
    pub updated_at: DateTime<Utc>,
    /// Date time when the user was soft-deleted.
    pub deleted_at: Option<DateTime<Utc>>,
}

impl User {
    pub fn is_inactive(&self) -> bool {
        self.deleted_at.is_some() || self.status != 1
    }

    pub fn is_admin(&self) -> bool {
        self.role == 0
    }
}

/// Request schema for user registration. (Permitted params)
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)] // Enforce strong parameters
pub struct UserRegisterRequestDto {
    #[schema(example = "John Doe")]
    pub name: String,
    #[schema(example = "user@example.com")]
    pub email: String,
    #[schema(example = "SecretPassword123")]
    pub password: String,
    #[serde(default, deserialize_with = "deserialize_role")]
    pub role: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_status")]
    pub status: Option<i32>,
}

/// Request schema for user login authentication.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UserLoginRequestDto {
    #[schema(example = "user@example.com")]
    pub email: String,
    #[schema(example = "SecretPassword123")]
    pub password: String,
}

/// Request schema for user creation by admin (allows role assignment).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UserCreateRequestDto {
    pub name: String,
    pub email: String,
    pub password: String,
    #[serde(default, deserialize_with = "deserialize_role")]
    pub role: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_status")]
    pub status: Option<i32>,
}

/// Request schema for user updates (Permitted params).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct UserUpdateRequestDto {
    pub name: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    #[serde(default, deserialize_with = "deserialize_role")]
    pub role: Option<i32>, // Only processed if current user is admin
    #[serde(default, deserialize_with = "deserialize_status")]
    pub status: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_deleted_at")]
    pub deleted_at: Option<Option<DateTime<Utc>>>,
}

/// Response schema for successful user authentication.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserLoginResponseDto {
    pub token: String,
}

/// JWT Token claim layout.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: Uuid,
    pub role: i32,
    pub status: i32,
    pub exp: u64,
}

/// Pagination parameters.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub role: Option<String>, // 'admin' or 'user'
    pub deleted: Option<bool>,
}

impl PaginationParams {
    pub fn get_page(&self) -> u32 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn get_per_page(&self) -> u32 {
        self.per_page.unwrap_or(10).clamp(1, 100)
    }

    pub fn offset(&self) -> u32 {
        (self.get_page() - 1) * self.get_per_page()
    }
}

/// Paginated response metadata.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub status: u16,
    pub message: String,
    pub data: Vec<T>,
    pub current_page: u32,
    pub per_page: u32,
    pub total_pages: u32,
    pub total_count: u32,
    pub next_page: Option<u32>,
    pub prev_page: Option<u32>,
}
