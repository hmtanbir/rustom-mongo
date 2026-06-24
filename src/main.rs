#![warn(clippy::all)]

use anyhow::Context;
use std::sync::Arc;

use rustom::app_state;
use rustom::config;
use rustom::config::routes;
use rustom::infrastructure;
use rustom::services;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Load configuration
    let config = config::AppConfig::from_env()
        .map_err(|e| anyhow::anyhow!("Configuration loading failure: {}", e))?;

    // 2. Initialize logging & tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("info,rustom=debug,tower_http=debug")
            }),
        )
        .init();

    tracing::info!("Starting Rustom API boilerplate template...");

    // 3. Initialize Postgres connection pool (runs migrations inside)
    let db = infrastructure::init_db(&config)
        .await
        .context("Failed to initialize PostgreSQL database")?;

    // 4. Initialize Redis cache pool
    let redis = infrastructure::init_redis(&config)
        .context("Failed to initialize Redis cache connection")?;

    // 5. Initialize RabbitMQ connection and channel
    let (_rabbitmq_conn, rabbitmq_channel) = infrastructure::init_rabbitmq(&config)
        .await
        .context("Failed to initialize RabbitMQ connection")?;

    // 6. Start the RabbitMQ background worker consumer
    services::start_queue_consumer(rabbitmq_channel.clone());

    // 7. Instantiate services
    let cache_service =
        Arc::new(services::RedisCacheService::new(redis)) as services::DynCacheService;
    let user_service = services::UserService::new(db, cache_service, config.clone());
    let queue_publisher = Arc::new(services::RabbitMQQueueService::new(rabbitmq_channel))
        as services::DynQueueService;

    // 8. Bootstrap state and routes
    let state = app_state::AppState {
        user_service,
        queue_publisher,
        config: config.clone(),
    };

    let app = routes::create_router(state);

    // 9. Start Axum server listening on specified port
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind to server address: {}", addr))?;

    tracing::info!("Rustom API server running on http://{}", addr);
    tracing::info!(
        "Swagger documentation available at http://{}/api-docs",
        addr
    );

    axum::serve(listener, app)
        .await
        .context("Server execution encountered error")?;

    Ok(())
}
