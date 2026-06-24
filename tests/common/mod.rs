use axum::Router;
use sqlx::PgPool;
use std::sync::Arc;

use rustom::app_state::AppState;
use rustom::config::AppConfig;
use rustom::config::routes::create_router;
use rustom::errors::AppError;
use rustom::models::job::JobPayload;
use rustom::services::{DynCacheService, DynQueueService, QueueService, UserService};

pub struct MockCache;

#[async_trait::async_trait]
impl rustom::services::cache::CacheService for MockCache {
    async fn get(&self, _key: &str) -> Result<Option<String>, AppError> {
        Ok(None)
    }
    async fn set(&self, _key: &str, _value: &str, _ttl_seconds: u64) -> Result<(), AppError> {
        Ok(())
    }
    async fn delete(&self, _key: &str) -> Result<(), AppError> {
        Ok(())
    }
    async fn incr_with_ttl(&self, _key: &str, _ttl_seconds: u64) -> Result<i64, AppError> {
        Ok(1)
    }
}

pub struct MockQueue;

#[async_trait::async_trait]
impl QueueService for MockQueue {
    async fn publish_job(&self, _job: &JobPayload) -> Result<(), AppError> {
        Ok(())
    }
}

pub async fn setup_app() -> (Router, PgPool) {
    // Note: To disable encryption for tests, set API_PAYLOAD_ENCRYPTION_ENABLED=false
    // in the environment running the tests (e.g. in .env.test or CI config).
    // Mutating std::env::set_var here is unsafe in Rust 2024 and causes data races.

    // Load .env.test thread-safely
    static INIT_DOTENV: std::sync::Once = std::sync::Once::new();
    INIT_DOTENV.call_once(|| {
        let _ = dotenvy::from_filename(".env.test");
    });

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        let user = std::env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string());
        let pass = std::env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "postgres".to_string());
        let host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
        let db_name = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "rustom_test".to_string());
        format!("postgres://{}:{}@{}:{}/{}", user, pass, host, port, db_name)
    });

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| {
        let pass = std::env::var("REDIS_PASSWORD")
            .expect("REDIS_PASSWORD must be set in .env.development");
        let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = std::env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string());
        let db = std::env::var("REDIS_DB").unwrap_or_else(|_| "0".to_string());
        format!("redis://:{}@{}:{}/{}", pass, host, port, db)
    });

    let rabbitmq_url = std::env::var("RABBITMQ_URL").unwrap_or_else(|_| {
        let user = std::env::var("RABBITMQ_USER").unwrap_or_else(|_| "guest".to_string());
        let pass = std::env::var("RABBITMQ_PASSWORD").unwrap_or_else(|_| "guest".to_string());
        let host = std::env::var("RABBITMQ_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = std::env::var("RABBITMQ_PORT").unwrap_or_else(|_| "5672".to_string());
        let vhost = std::env::var("RABBITMQ_VHOST").unwrap_or_else(|_| "%2f".to_string());
        format!("amqp://{}:{}@{}:{}/{}", user, pass, host, port, vhost)
    });

    let config = AppConfig {
        host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
        port: std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .unwrap_or(3000),
        database_url,
        redis_url,
        rabbitmq_url,
        jwt_secret: std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "secret_for_tests_123456789".to_string()),
        jwt_expiration_seconds: std::env::var("JWT_EXPIRATION_SECONDS")
            .unwrap_or_else(|_| "3600".to_string())
            .parse()
            .unwrap_or(3600),
        domain_name: std::env::var("DOMAIN_NAME")
            .unwrap_or_else(|_| "http://localhost:3000".to_string()),
    };

    let db = rustom::infrastructure::init_db(&config)
        .await
        .expect("Failed to initialize test DB and run migrations");

    // Clean all tables exactly once at the start of the test suite run to prevent race conditions
    static DB_CLEANED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if DB_CLEANED
        .compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        )
        .is_ok()
    {
        sqlx::query("TRUNCATE TABLE users CASCADE;")
            .execute(&db)
            .await
            .expect("Failed to truncate tables");
    }

    let cache_service = Arc::new(MockCache) as DynCacheService;
    let queue_publisher = Arc::new(MockQueue) as DynQueueService;
    let user_service = UserService::new(db.clone(), cache_service, config.clone());

    let state = AppState {
        user_service,
        queue_publisher,
        config,
    };

    (create_router(state), db)
}

pub fn get_gateway_key() -> String {
    std::env::var("API_GATEWAY_KEY").unwrap_or_default()
}
