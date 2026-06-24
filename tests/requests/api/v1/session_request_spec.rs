use crate::common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::Service;

#[tokio::test]
async fn test_successful_login() {
    let (app, db) = common::setup_app().await;
    let gateway_key = common::get_gateway_key();

    // Insert a fresh user directly into the database to guarantee they exist for login
    let user_id = uuid::Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);
    // Hardcoded Argon2 hash for 'password'
    let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$mIk38++6ZCEyzKo+edgXEw$/h0anRjDkzS46suJM6/P3+DySS3qp1+6jXtNjd6UMTs";

    sqlx::query(
        r#"
        INSERT INTO users (id, name, email, password_digest, role, status)
        VALUES ($1, 'Login Test User', $2, $3, 1, 1)
        "#,
    )
    .bind(user_id)
    .bind(&email)
    .bind(password_hash)
    .execute(&db)
    .await
    .unwrap();

    let payload = json!({
        "email": email,
        "password": "password"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/sessions")
        .header("Content-Type", "application/json")
        .header("x-api-gateway-key", &gateway_key)
        .body(Body::from(payload.to_string()))
        .unwrap();

    let mut app = app;
    let response = app.call(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_body: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(response_body["data"]["token"].is_string());
}

#[tokio::test]
async fn test_invalid_login() {
    let (app, _db) = common::setup_app().await;
    let gateway_key = common::get_gateway_key();

    let payload = json!({
        "email": "doesnotexist@example.com",
        "password": "wrongpassword"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/sessions")
        .header("Content-Type", "application/json")
        .header("x-api-gateway-key", &gateway_key)
        .body(Body::from(payload.to_string()))
        .unwrap();

    let mut app = app;
    let response = app.call(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_missing_api_gateway_key() {
    let (app, _db) = common::setup_app().await;

    let payload = json!({
        "email": "test@example.com",
        "password": "password"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/sessions")
        .header("Content-Type", "application/json")
        // Missing x-api-gateway-key header
        .body(Body::from(payload.to_string()))
        .unwrap();

    let mut app = app;
    let response = app.call(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_login_inactive_or_deleted_user() {
    let (app, db) = common::setup_app().await;
    let gateway_key = common::get_gateway_key();

    // Insert an inactive user directly into the database (status = 0)
    let user_id = uuid::Uuid::new_v4();
    let email = format!("inactive_{}@example.com", user_id);
    let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$mIk38++6ZCEyzKo+edgXEw$/h0anRjDkzS46suJM6/P3+DySS3qp1+6jXtNjd6UMTs";

    sqlx::query(
        r#"
        INSERT INTO users (id, name, email, password_digest, role, status)
        VALUES ($1, 'Inactive User', $2, $3, 1, 0)
        "#,
    )
    .bind(user_id)
    .bind(&email)
    .bind(password_hash)
    .execute(&db)
    .await
    .unwrap();

    let payload = json!({
        "email": email,
        "password": "password"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/sessions")
        .header("Content-Type", "application/json")
        .header("x-api-gateway-key", &gateway_key)
        .body(Body::from(payload.to_string()))
        .unwrap();

    let mut app = app;
    let response = app.call(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_suspended_user() {
    let (app, db) = common::setup_app().await;
    let gateway_key = common::get_gateway_key();

    // Insert a suspended user directly into the database (status = 2)
    let user_id = uuid::Uuid::new_v4();
    let email = format!("suspended_{}@example.com", user_id);
    let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$mIk38++6ZCEyzKo+edgXEw$/h0anRjDkzS46suJM6/P3+DySS3qp1+6jXtNjd6UMTs";

    sqlx::query(
        r#"
        INSERT INTO users (id, name, email, password_digest, role, status)
        VALUES ($1, 'Suspended User', $2, $3, 1, 2)
        "#,
    )
    .bind(user_id)
    .bind(&email)
    .bind(password_hash)
    .execute(&db)
    .await
    .unwrap();

    let payload = json!({
        "email": email,
        "password": "password"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/sessions")
        .header("Content-Type", "application/json")
        .header("x-api-gateway-key", &gateway_key)
        .body(Body::from(payload.to_string()))
        .unwrap();

    let mut app = app;
    let response = app.call(req).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
