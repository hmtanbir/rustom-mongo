use crate::common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use rustom::models::user::Claims;
use serde_json::{Value, json};
use tower::Service;

fn generate_test_token(user_id: uuid::Uuid, role: i32) -> String {
    let secret =
        std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret_for_tests_123456789".to_string());
    let expiration = Utc::now()
        .checked_add_signed(Duration::seconds(3600))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        user_id,
        role,
        status: 1,
        exp: expiration as u64,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

async fn generate_db_token(db: &sqlx::PgPool, user_id: uuid::Uuid, role: i32) -> String {
    let email = format!("user_{}_{}@example.com", role, user_id);
    let pwd_digest = "$argon2id$v=19$m=19456,t=2,p=1$mIk38++6ZCEyzKo+edgXEw$/h0anRjDkzS46suJM6/P3+DySS3qp1+6jXtNjd6UMTs";
    sqlx::query("INSERT INTO users (id, name, email, password_digest, role, status) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(user_id)
        .bind("Test User")
        .bind(email)
        .bind(pwd_digest)
        .bind(role)
        .bind(1)
        .execute(db)
        .await
        .unwrap();

    generate_test_token(user_id, role)
}

#[tokio::test]
async fn test_user_registration() {
    let (app, _db) = common::setup_app().await;
    let gateway_key = common::get_gateway_key();

    let email = format!("test_{}@example.com", uuid::Uuid::new_v4());

    let payload = json!({
        "user": {
            "name": "Test User",
            "email": email,
            "password": "Password123!"
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/registration")
        .header("Content-Type", "application/json")
        .header("x-api-gateway-key", &gateway_key)
        .body(Body::from(payload.to_string()))
        .unwrap();

    let mut app = app;
    let response = app.call(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_get_me_authenticated() {
    let (app, db) = common::setup_app().await;
    let gateway_key = common::get_gateway_key();

    let user_id = uuid::Uuid::new_v4();
    let email = format!("me_{}@example.com", user_id);
    let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$mIk38++6ZCEyzKo+edgXEw$/h0anRjDkzS46suJM6/P3+DySS3qp1+6jXtNjd6UMTs";

    sqlx::query(
        r#"
        INSERT INTO users (id, name, email, password_digest, role, status)
        VALUES ($1, 'Me User', $2, $3, 1, 1)
        "#,
    )
    .bind(user_id)
    .bind(&email)
    .bind(password_hash)
    .execute(&db)
    .await
    .unwrap();

    let token = generate_test_token(user_id, 1);

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/users/me")
        .header("x-api-gateway-key", &gateway_key)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let mut app = app;
    let response = app.call(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_body: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(response_body["data"]["email"], email);
}

#[tokio::test]
async fn test_get_users_as_admin() {
    let (app, db) = common::setup_app().await;
    let gateway_key = common::get_gateway_key();

    let admin_id = uuid::Uuid::new_v4();
    let token = generate_db_token(&db, admin_id, 0).await; // 0 = Admin

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/users")
        .header("x-api-gateway-key", &gateway_key)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let mut app = app;
    let response = app.call(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_users_as_standard_user_is_forbidden() {
    let (app, db) = common::setup_app().await;
    let gateway_key = common::get_gateway_key();

    let user_id = uuid::Uuid::new_v4();
    let token = generate_db_token(&db, user_id, 1).await; // 1 = Standard User

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/users")
        .header("x-api-gateway-key", &gateway_key)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let mut app = app;
    let response = app.call(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_user_registration_duplicate_email() {
    let (app, db) = common::setup_app().await;
    let gateway_key = common::get_gateway_key();

    let email = format!("dup_{}@example.com", uuid::Uuid::new_v4());

    // Seed the first user
    let user_id = uuid::Uuid::new_v4();
    let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$mIk38++6ZCEyzKo+edgXEw$/h0anRjDkzS46suJM6/P3+DySS3qp1+6jXtNjd6UMTs";
    sqlx::query(
        r#"
        INSERT INTO users (id, name, email, password_digest, role, status)
        VALUES ($1, 'Existing User', $2, $3, 1, 1)
        "#,
    )
    .bind(user_id)
    .bind(&email)
    .bind(password_hash)
    .execute(&db)
    .await
    .unwrap();

    // Try to register again with same email
    let payload = json!({
        "user": {
            "name": "Another User",
            "email": email,
            "password": "Password123!"
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/registration")
        .header("Content-Type", "application/json")
        .header("x-api-gateway-key", &gateway_key)
        .body(Body::from(payload.to_string()))
        .unwrap();

    let mut app = app;
    let response = app.call(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_user_registration_validation_errors() {
    let (app, _db) = common::setup_app().await;
    let gateway_key = common::get_gateway_key();

    let payload = json!({
        "user": {
            "name": "", // Empty name
            "email": "invalid@",
            "password": "" // Empty password
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/registration")
        .header("Content-Type", "application/json")
        .header("x-api-gateway-key", &gateway_key)
        .body(Body::from(payload.to_string()))
        .unwrap();

    let mut app = app;
    let response = app.call(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_get_me_unauthorized() {
    let (app, _db) = common::setup_app().await;
    let gateway_key = common::get_gateway_key();

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/users/me")
        .header("x-api-gateway-key", &gateway_key)
        // Missing Authorization header
        .body(Body::empty())
        .unwrap();

    let mut app = app;
    let response = app.call(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_delete_user_as_admin_vs_standard_user() {
    let (app, db) = common::setup_app().await;
    let gateway_key = common::get_gateway_key();

    // Create target user to delete
    let target_user_id = uuid::Uuid::new_v4();
    let email = format!("target_{}@example.com", target_user_id);
    let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$mIk38++6ZCEyzKo+edgXEw$/h0anRjDkzS46suJM6/P3+DySS3qp1+6jXtNjd6UMTs";
    sqlx::query(
        r#"
        INSERT INTO users (id, name, email, password_digest, role, status)
        VALUES ($1, 'Target User', $2, $3, 1, 1)
        "#,
    )
    .bind(target_user_id)
    .bind(&email)
    .bind(password_hash)
    .execute(&db)
    .await
    .unwrap();

    // 1. Try to delete as standard user (forbidden)
    let standard_user_id = uuid::Uuid::new_v4();
    let standard_token = generate_db_token(&db, standard_user_id, 1).await; // 1 = Standard User

    let req_std = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/users/{}", target_user_id))
        .header("x-api-gateway-key", &gateway_key)
        .header("Authorization", format!("Bearer {}", standard_token))
        .body(Body::empty())
        .unwrap();

    let mut app_std = app.clone();
    let response_std = app_std.call(req_std).await.unwrap();
    assert_eq!(response_std.status(), StatusCode::FORBIDDEN);

    // 2. Delete as admin (success)
    let admin_id = uuid::Uuid::new_v4();
    let admin_token = generate_db_token(&db, admin_id, 0).await; // 0 = Admin

    let req_admin = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/users/{}", target_user_id))
        .header("x-api-gateway-key", &gateway_key)
        .header("Authorization", format!("Bearer {}", admin_token))
        .body(Body::empty())
        .unwrap();

    let mut app_admin = app;
    let response_admin = app_admin.call(req_admin).await.unwrap();
    assert_eq!(response_admin.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_update_user_password() {
    let (app, db) = common::setup_app().await;
    let gateway_key = common::get_gateway_key();

    let user_id = uuid::Uuid::new_v4();
    let email = format!("update_pwd_{}@example.com", user_id);
    let initial_hash = "$argon2id$v=19$m=19456,t=2,p=1$mIk38++6ZCEyzKo+edgXEw$/h0anRjDkzS46suJM6/P3+DySS3qp1+6jXtNjd6UMTs"; // for "password"

    sqlx::query(
        r#"
        INSERT INTO users (id, name, email, password_digest, role, status)
        VALUES ($1, 'Update Password User', $2, $3, 1, 1)
        "#,
    )
    .bind(user_id)
    .bind(&email)
    .bind(initial_hash)
    .execute(&db)
    .await
    .unwrap();

    let token = generate_test_token(user_id, 1);

    let payload = json!({
        "user": {
            "password": "Newpassword123!"
        }
    });

    let req = Request::builder()
        .method("PATCH")
        .uri("/api/v1/users/me")
        .header("Content-Type", "application/json")
        .header("x-api-gateway-key", &gateway_key)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::from(payload.to_string()))
        .unwrap();

    let mut app = app;
    let response = app.call(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify the password digest has changed in the database
    let updated_password_digest: String =
        sqlx::query_scalar("SELECT password_digest FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&db)
            .await
            .unwrap();

    assert_ne!(updated_password_digest, initial_hash);
}

#[tokio::test]
async fn test_admin_can_restore_deleted_user() {
    let (app, db) = common::setup_app().await;
    let gateway_key = common::get_gateway_key();

    // 1. Create a user
    let user_id = uuid::Uuid::new_v4();
    let email = format!("deleted_user_{}@example.com", user_id);
    let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$mIk38++6ZCEyzKo+edgXEw$/h0anRjDkzS46suJM6/P3+DySS3qp1+6jXtNjd6UMTs";

    sqlx::query(
        r#"
        INSERT INTO users (id, name, email, password_digest, role, status, deleted_at)
        VALUES ($1, 'Deleted User', $2, $3, 1, 1, NOW())
        "#,
    )
    .bind(user_id)
    .bind(&email)
    .bind(password_hash)
    .execute(&db)
    .await
    .unwrap();

    // 2. Perform patch update as admin to set deleted_at to null
    let admin_id = uuid::Uuid::new_v4();
    let admin_token = generate_db_token(&db, admin_id, 0).await; // 0 = Admin

    let payload = json!({
        "user": {
            "deleted_at": null
        }
    });

    let req = Request::builder()
        .method("PATCH")
        .uri(format!("/api/v1/users/{}", user_id))
        .header("Content-Type", "application/json")
        .header("x-api-gateway-key", &gateway_key)
        .header("Authorization", format!("Bearer {}", admin_token))
        .body(Body::from(payload.to_string()))
        .unwrap();

    let mut app = app;
    let response = app.call(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify it is null in database
    let deleted_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT deleted_at FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&db)
            .await
            .unwrap();

    assert!(deleted_at.is_none());
}

#[tokio::test]
async fn test_non_admin_cannot_update_deleted_at() {
    let (app, db) = common::setup_app().await;
    let gateway_key = common::get_gateway_key();

    // Create target user
    let user_id = uuid::Uuid::new_v4();
    let email = format!("user_to_check_{}@example.com", user_id);
    let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$mIk38++6ZCEyzKo+edgXEw$/h0anRjDkzS46suJM6/P3+DySS3qp1+6jXtNjd6UMTs";

    sqlx::query(
        r#"
        INSERT INTO users (id, name, email, password_digest, role, status, deleted_at)
        VALUES ($1, 'Test User', $2, $3, 1, 1, NOW())
        "#,
    )
    .bind(user_id)
    .bind(&email)
    .bind(password_hash)
    .execute(&db)
    .await
    .unwrap();

    // Perform patch update as self (non-admin, role=1) trying to update deleted_at
    let token = generate_test_token(user_id, 1);

    let payload = json!({
        "user": {
            "deleted_at": null
        }
    });

    let req = Request::builder()
        .method("PATCH")
        .uri("/api/v1/users/me")
        .header("Content-Type", "application/json")
        .header("x-api-gateway-key", &gateway_key)
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::from(payload.to_string()))
        .unwrap();

    let mut app = app;
    let response = app.call(req).await.unwrap();

    // Standard user gets UNAUTHORIZED because their user account is soft-deleted and fails auth DB lookup check
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
