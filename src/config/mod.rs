pub mod routes;

pub use crate::utils::API_RATE_LIMIT;
use serde::Deserialize;

/// Global application configuration loaded from environment variables.
#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    /// The host on which the Axum server will run.
    pub host: String,
    /// The port on which the Axum server will listen.
    pub port: u16,
    /// Connection string for PostgreSQL database.
    pub database_url: String,
    /// Connection string for Redis caching server.
    pub redis_url: String,
    /// Connection string for RabbitMQ messaging broker.
    pub rabbitmq_url: String,
    /// Secret key used to sign and verify JWT authentication tokens.
    pub jwt_secret: String,
    /// Duration in seconds for JWT tokens to remain valid.
    pub jwt_expiration_seconds: u64,
    /// Allowed domain(s) for CORS.
    pub domain_name: String,
}

impl AppConfig {
    /// Load settings from environment variables and dotenv file.
    pub fn from_env() -> Result<Self, config::ConfigError> {
        // Determine application environment (default to "development")
        let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".into());
        eprintln!("Loading configuration for environment: {}", app_env);

        // Try to load variables from environment-specific .env file
        let env_file = format!(".env.{}", app_env);
        if dotenvy::from_filename(&env_file).is_err() {
            // Fallback to default .env if environment-specific file is not found
            let _ = dotenvy::dotenv();
        }

        let mut builder = config::Config::builder().add_source(config::Environment::default());

        // Construct database URL from individual POSTGRES_* env vars if available
        if let (Ok(user), Ok(pass), Ok(db), Ok(host), Ok(port)) = (
            std::env::var("POSTGRES_USER"),
            std::env::var("POSTGRES_PASSWORD"),
            std::env::var("POSTGRES_DB"),
            std::env::var("POSTGRES_HOST"),
            std::env::var("POSTGRES_PORT"),
        ) {
            builder = builder.set_default(
                "database_url",
                format!("postgres://{}:{}@{}:{}/{}", user, pass, host, port, db),
            )?;
        }

        // Construct Redis URL from individual REDIS_* env vars if available
        if let (Ok(pass), Ok(host), Ok(port), Ok(db)) = (
            std::env::var("REDIS_PASSWORD"),
            std::env::var("REDIS_HOST"),
            std::env::var("REDIS_PORT"),
            std::env::var("REDIS_DB"),
        ) {
            builder = builder.set_default(
                "redis_url",
                format!("redis://:{}@{}:{}/{}", pass, host, port, db),
            )?;
        }

        // Construct RabbitMQ URL from individual RABBITMQ_* env vars if available
        if let (Ok(user), Ok(pass), Ok(host), Ok(port), Ok(vhost)) = (
            std::env::var("RABBITMQ_USER"),
            std::env::var("RABBITMQ_PASSWORD"),
            std::env::var("RABBITMQ_HOST"),
            std::env::var("RABBITMQ_PORT"),
            std::env::var("RABBITMQ_VHOST"),
        ) {
            builder = builder.set_default(
                "rabbitmq_url",
                format!("amqp://{}:{}@{}:{}/{}", user, pass, host, port, vhost),
            )?;
        }

        let default_domain =
            std::env::var("DOMAIN_NAME").unwrap_or_else(|_| "http://localhost:3000".to_string());
        builder = builder.set_default("domain_name", default_domain)?;

        builder.build()?.try_deserialize()
    }
}
