use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::{
    Claims, PaginatedResponse, PaginationParams, User, UserCreateRequestDto, UserLoginRequestDto,
    UserLoginResponseDto, UserRegisterRequestDto, UserUpdateRequestDto,
};
use crate::serializers::user_serializer::UserSerializer;
use crate::services::cache::DynCacheService;

#[derive(Clone)]
pub struct UserService {
    db: PgPool,
    cache: DynCacheService,
    config: AppConfig,
}

impl UserService {
    pub fn new(db: PgPool, cache: DynCacheService, config: AppConfig) -> Self {
        Self { db, cache, config }
    }

    pub fn get_cache(&self) -> &DynCacheService {
        &self.cache
    }

    fn validate_password(&self, password: &str) -> Result<(), AppError> {
        if password.len() < 8 {
            return Err(AppError::InvalidInput(
                "Password must be at least 8 characters long".to_string(),
            ));
        }
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_numeric());
        let has_special = password.chars().any(|c| !c.is_alphanumeric());

        if !has_uppercase || !has_lowercase || !has_digit || !has_special {
            return Err(AppError::InvalidInput(
                "Password must contain at least one uppercase letter, one lowercase letter, one digit, and one special character".to_string()
            ));
        }
        Ok(())
    }

    pub async fn register(&self, dto: UserRegisterRequestDto) -> Result<UserSerializer, AppError> {
        self.validate_password(&dto.password)?;

        let existing = sqlx::query_as::<_, User>("SELECT id, name, email, password_digest, role, status, created_at, updated_at, deleted_at FROM users WHERE email = $1 AND deleted_at IS NULL")
            .bind(&dto.email)
            .fetch_optional(&self.db)
            .await?;

        if existing.is_some() {
            return Err(AppError::Conflict(
                "Email is already registered".to_string(),
            ));
        }

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_digest = argon2
            .hash_password(dto.password.as_bytes(), &salt)
            .map_err(|e| AppError::Authentication(format!("Password hashing failure: {}", e)))?
            .to_string();

        let mut tx = self.db.begin().await?;
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (name, email, password_digest, role, status)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, email, password_digest, role, status, created_at, updated_at, deleted_at
            "#,
        )
        .bind(&dto.name)
        .bind(&dto.email)
        .bind(&password_digest)
        .bind(dto.role.unwrap_or(1))
        .bind(dto.status.unwrap_or(1))
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        self.invalidate_users_index().await;

        Ok(UserSerializer::from(user))
    }

    pub async fn login(&self, dto: UserLoginRequestDto) -> Result<UserLoginResponseDto, AppError> {
        let user_result = sqlx::query_as::<_, User>("SELECT id, name, email, password_digest, role, status, created_at, updated_at, deleted_at FROM users WHERE email = $1 AND deleted_at IS NULL")
            .bind(&dto.email)
            .fetch_optional(&self.db)
            .await?;

        let (user, is_valid) = match user_result {
            Some(u) => {
                let parsed_hash = PasswordHash::new(&u.password_digest).map_err(|e| {
                    AppError::Authentication(format!("Invalid password hash representation: {}", e))
                })?;
                let is_valid = Argon2::default()
                    .verify_password(dto.password.as_bytes(), &parsed_hash)
                    .is_ok();
                (Some(u), is_valid)
            }
            None => {
                // Dummy hash for "password" to mitigate timing attacks
                let dummy_hash = "$argon2id$v=19$m=19456,t=2,p=1$mIk38++6ZCEyzKo+edgXEw$/h0anRjDkzS46suJM6/P3+DySS3qp1+6jXtNjd6UMTs";
                let parsed_hash = PasswordHash::new(dummy_hash).unwrap();
                let is_valid = Argon2::default()
                    .verify_password(dto.password.as_bytes(), &parsed_hash)
                    .is_ok();
                (None, is_valid)
            }
        };

        if !is_valid || user.is_none() {
            return Err(AppError::Authentication(
                "Invalid email or password".to_string(),
            ));
        }

        let user = user.unwrap();

        if user.is_inactive() {
            return Err(AppError::Authentication(
                "User is inactive or suspended".to_string(),
            ));
        }

        let exp = Utc::now()
            .checked_add_signed(chrono::Duration::seconds(
                self.config.jwt_expiration_seconds as i64,
            ))
            .ok_or_else(|| AppError::Unexpected(anyhow::anyhow!("Time calculation overflow")))?
            .timestamp() as u64;

        let claims = Claims {
            user_id: user.id,
            role: user.role,
            status: user.status,
            exp,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )
        .map_err(|e| AppError::Authentication(format!("Token signing failure: {}", e)))?;

        Ok(UserLoginResponseDto { token })
    }

    pub async fn get_users_paginated(
        &self,
        params: PaginationParams,
    ) -> Result<PaginatedResponse<UserSerializer>, AppError> {
        let version = self
            .cache
            .get("users_index_version")
            .await
            .unwrap_or_default()
            .unwrap_or_else(|| "0".to_string());

        let cache_key = format!(
            "users_index/{}/{}/{}/{}/{}",
            params.role.as_deref().unwrap_or("all"),
            params.deleted.unwrap_or(false),
            params.get_page(),
            params.get_per_page(),
            version
        );

        if let Ok(Some(cached_str)) = self.cache.get(&cache_key).await
            && let Ok(cached) =
                serde_json::from_str::<PaginatedResponse<UserSerializer>>(&cached_str)
        {
            return Ok(cached);
        }

        let role_filter = params
            .role
            .as_deref()
            .map(|r| if r == "admin" { 0 } else { 1 });
        let deleted_filter = params.deleted.unwrap_or(false);

        let mut query = "SELECT id, name, email, password_digest, role, status, created_at, updated_at, deleted_at FROM users WHERE 1=1".to_string();
        let mut count_query = "SELECT COUNT(*) FROM users WHERE 1=1".to_string();

        if deleted_filter {
            query.push_str(" AND deleted_at IS NOT NULL");
            count_query.push_str(" AND deleted_at IS NOT NULL");
        } else {
            query.push_str(" AND deleted_at IS NULL");
            count_query.push_str(" AND deleted_at IS NULL");
        }

        let mut bind_params = 1;

        if role_filter.is_some() {
            query.push_str(&format!(" AND role = ${}", bind_params));
            count_query.push_str(&format!(" AND role = ${}", bind_params));
            bind_params += 1;
        }

        let mut count_q = sqlx::query_as::<_, (i64,)>(&count_query);
        if let Some(r) = role_filter {
            count_q = count_q.bind(r);
        }
        let total_count = count_q.fetch_one(&self.db).await?;
        let total_count = total_count.0 as u32;

        let total_pages = if total_count == 0 {
            1
        } else {
            (total_count as f32 / params.get_per_page() as f32).ceil() as u32
        };

        query.push_str(&format!(
            " ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
            bind_params,
            bind_params + 1
        ));

        let mut users_q = sqlx::query_as::<_, User>(&query);
        if let Some(r) = role_filter {
            users_q = users_q.bind(r);
        }
        let users = users_q
            .bind(params.get_per_page() as i64)
            .bind(params.offset() as i64)
            .fetch_all(&self.db)
            .await?;

        let response = PaginatedResponse {
            status: 200,
            message: "Successfully data fetched".to_string(),
            data: users.into_iter().map(UserSerializer::from).collect(),
            current_page: params.get_page(),
            per_page: params.get_per_page(),
            total_pages,
            total_count,
            next_page: if params.get_page() < total_pages {
                Some(params.get_page() + 1)
            } else {
                None
            },
            prev_page: if params.get_page() > 1 {
                Some(params.get_page() - 1)
            } else {
                None
            },
        };

        if let Ok(json_str) = serde_json::to_string(&response) {
            let ttl = std::env::var("API_CACHE_TTL")
                .unwrap_or_else(|_| "3600".to_string())
                .parse()
                .unwrap_or(3600);
            let _ = self.cache.set(&cache_key, &json_str, ttl).await;
        }

        Ok(response)
    }

    pub async fn get_user(&self, user_id: Uuid) -> Result<UserSerializer, AppError> {
        let cache_key = format!("user:profile:{}", user_id);

        if let Ok(Some(cached_str)) = self.cache.get(&cache_key).await
            && let Ok(cached_user) = serde_json::from_str::<UserSerializer>(&cached_str)
        {
            return Ok(cached_user);
        }

        let user = sqlx::query_as::<_, User>("SELECT id, name, email, password_digest, role, status, created_at, updated_at, deleted_at FROM users WHERE id = $1 AND deleted_at IS NULL")
            .bind(user_id)
            .fetch_optional(&self.db)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let user_dto = UserSerializer::from(user);

        if let Ok(json_str) = serde_json::to_string(&user_dto) {
            let ttl = std::env::var("API_CACHE_TTL")
                .unwrap_or_else(|_| "3600".to_string())
                .parse()
                .unwrap_or(3600);
            let _ = self.cache.set(&cache_key, &json_str, ttl).await;
        }

        Ok(user_dto)
    }

    pub async fn create_user(&self, dto: UserCreateRequestDto) -> Result<UserSerializer, AppError> {
        self.validate_password(&dto.password)?;

        let existing = sqlx::query_as::<_, User>("SELECT id, name, email, password_digest, role, status, created_at, updated_at, deleted_at FROM users WHERE email = $1 AND deleted_at IS NULL")
            .bind(&dto.email)
            .fetch_optional(&self.db)
            .await?;

        if existing.is_some() {
            return Err(AppError::Conflict(
                "Email is already registered".to_string(),
            ));
        }

        let salt = SaltString::generate(&mut OsRng);
        let password_digest = Argon2::default()
            .hash_password(dto.password.as_bytes(), &salt)
            .map_err(|e| AppError::Authentication(format!("Hashing error: {}", e)))?
            .to_string();

        let mut tx = self.db.begin().await?;
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (name, email, password_digest, role, status)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, email, password_digest, role, status, created_at, updated_at, deleted_at
            "#,
        )
        .bind(&dto.name)
        .bind(&dto.email)
        .bind(&password_digest)
        .bind(dto.role.unwrap_or(1))
        .bind(dto.status.unwrap_or(1))
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        self.invalidate_users_index().await;

        Ok(UserSerializer::from(user))
    }

    pub async fn update_user(
        &self,
        user_id: Uuid,
        dto: UserUpdateRequestDto,
    ) -> Result<UserSerializer, AppError> {
        let mut tx = self.db.begin().await?;

        let existing = sqlx::query_as::<_, User>("SELECT id, name, email, password_digest, role, status, created_at, updated_at, deleted_at FROM users WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        if existing.deleted_at.is_some() && dto.deleted_at != Some(None) {
            return Err(AppError::NotFound("User not found".to_string()));
        }

        if let Some(ref new_email) = dto.email
            && new_email != &existing.email
        {
            let email_exists = sqlx::query_as::<_, User>("SELECT id, name, email, password_digest, role, status, created_at, updated_at, deleted_at FROM users WHERE email = $1 AND id != $2 AND deleted_at IS NULL")
                    .bind(new_email)
                    .bind(user_id)
                    .fetch_optional(&mut *tx)
                    .await?;

            if email_exists.is_some() {
                return Err(AppError::Conflict(
                    "Email is already registered by another user".to_string(),
                ));
            }
        }

        let password_digest = if let Some(ref pwd) = dto.password {
            self.validate_password(pwd)?;
            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();
            Some(
                argon2
                    .hash_password(pwd.as_bytes(), &salt)
                    .map_err(|e| {
                        AppError::Authentication(format!("Password hashing failure: {}", e))
                    })?
                    .to_string(),
            )
        } else {
            None
        };

        let has_deleted_at = dto.deleted_at.is_some();
        let deleted_at_val = dto.deleted_at.flatten();

        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET name = COALESCE($1, name),
                email = COALESCE($2, email),
                role = COALESCE($3, role),
                status = COALESCE($4, status),
                password_digest = COALESCE($5, password_digest),
                deleted_at = CASE WHEN $6 THEN $7 ELSE deleted_at END,
                updated_at = NOW()
            WHERE id = $8
            RETURNING id, name, email, password_digest, role, status, created_at, updated_at, deleted_at
            "#,
        )
        .bind(&dto.name)
        .bind(&dto.email)
        .bind(dto.role)
        .bind(dto.status)
        .bind(&password_digest)
        .bind(has_deleted_at)
        .bind(deleted_at_val)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        let cache_key = format!("user:profile:{}", user_id);
        let _ = self.cache.delete(&cache_key).await;
        self.invalidate_users_index().await;

        Ok(UserSerializer::from(user))
    }

    async fn invalidate_users_index(&self) {
        let _ = self
            .cache
            .set(
                "users_index_version",
                &Utc::now().timestamp().to_string(),
                86400,
            )
            .await;
    }

    pub async fn soft_delete_user(&self, user_id: Uuid) -> Result<(), AppError> {
        let mut tx = self.db.begin().await?;

        let result =
            sqlx::query("UPDATE users SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL")
                .bind(user_id)
                .execute(&mut *tx)
                .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "User not found or already deleted".to_string(),
            ));
        }

        tx.commit().await?;

        let cache_key = format!("user:profile:{}", user_id);
        let _ = self.cache.delete(&cache_key).await;
        self.invalidate_users_index().await;

        Ok(())
    }
}
