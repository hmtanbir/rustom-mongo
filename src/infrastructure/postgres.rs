use crate::config::AppConfig;
use crate::errors::AppError;
use sqlx::{PgPool, postgres::PgPoolOptions};

/// Initialize and configure the PostgreSQL connection pool.
/// Automatically runs pending database migrations upon establishment.
pub async fn init_db(config: &AppConfig) -> Result<PgPool, AppError> {
    tracing::info!("Initializing PostgreSQL connection pool...");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&config.database_url)
        .await
        .map_err(AppError::Database)?;

    tracing::info!("Applying pending migrations...");
    let mut migrator = sqlx::migrate!("./db/migrations");
    migrator.set_ignore_missing(true);
    let _: () = migrator
        .run(&pool)
        .await
        .map_err(|e| AppError::Unexpected(anyhow::anyhow!("Migration failed: {}", e)))?;

    tracing::info!("PostgreSQL database is ready.");

    // Seed data if the table is empty
    tracing::info!("Checking if seed data is needed...");
    if let Ok(count) = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
    {
        if count.0 == 0 {
            tracing::info!("Seeding database with default users...");
            let seed_sql = include_str!("../../db/seeds/20260612000001_seed_users.sql");
            let _ = sqlx::query(seed_sql)
                .execute(&pool)
                .await
                .map_err(|e| tracing::warn!("Failed to execute seed data: {}", e));
        } else {
            tracing::info!("Seeding skipped: users table already contains records.");
        }
    }

    Ok(pool)
}
