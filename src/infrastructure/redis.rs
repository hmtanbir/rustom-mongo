use crate::config::AppConfig;
use crate::errors::AppError;
use deadpool_redis::{Config, Pool, Runtime};

/// Redis Connection Pool alias.
pub type RedisPool = Pool;

/// Initialize and configure the Redis connection pool.
pub fn init_redis(config: &AppConfig) -> Result<RedisPool, AppError> {
    tracing::info!(
        "Initializing Redis connection pool at {}...",
        config.redis_url
    );

    let cfg = Config::from_url(&config.redis_url);
    let pool = cfg
        .create_pool(Some(Runtime::Tokio1))
        .map_err(|e| AppError::Cache(format!("Failed to create Redis pool: {}", e)))?;

    tracing::info!("Redis pool is ready.");
    Ok(pool)
}
