use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::User;
use chrono::Utc;
use mongodb::{Client, Database, bson::doc};
use uuid::Uuid;

/// Initialize and configure the MongoDB connection.
pub async fn init_db(config: &AppConfig) -> Result<Database, AppError> {
    tracing::info!("Initializing MongoDB connection pool...");

    let client = Client::with_uri_str(&config.database_url)
        .await
        .map_err(|e| {
            AppError::Unexpected(anyhow::anyhow!("Failed to connect to MongoDB: {}", e))
        })?;

    let db = client.default_database().ok_or_else(|| {
        AppError::Unexpected(anyhow::anyhow!("No default database found in MongoDB URI"))
    })?;

    tracing::info!("Ensuring unique index on users email...");
    let collection = db.collection::<User>("users");
    let index_model = mongodb::IndexModel::builder()
        .keys(doc! { "email": 1 })
        .options(
            mongodb::options::IndexOptions::builder()
                .unique(true)
                .build(),
        )
        .build();

    collection.create_index(index_model).await.map_err(|e| {
        AppError::Unexpected(anyhow::anyhow!("Failed to create email index: {}", e))
    })?;

    tracing::info!("Checking if seed data is needed...");
    let count = collection
        .count_documents(doc! {})
        .await
        .map_err(|e| AppError::Unexpected(anyhow::anyhow!("Failed to count users: {}", e)))?;

    if count == 0 {
        tracing::info!("Seeding database with default admin user...");
        let admin = User {
            id: Uuid::new_v4(),
            name: "Admin User".to_string(),
            email: "admin@rustom.project".to_string(),
            password_digest: "$argon2id$v=19$m=19456,t=2,p=1$mIk38++6ZCEyzKo+edgXEw$/h0anRjDkzS46suJM6/P3+DySS3qp1+6jXtNjd6UMTs".to_string(),
            role: 0,
            status: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };
        match collection.insert_one(admin).await {
            Ok(_) => {}
            Err(e) => {
                if let mongodb::error::ErrorKind::Write(mongodb::error::WriteFailure::WriteError(
                    write_error,
                )) = &*e.kind
                    && write_error.code == 11000
                {
                    tracing::info!("Seed user already exists due to concurrent insert.");
                    return Ok(db);
                }
                return Err(AppError::Unexpected(anyhow::anyhow!(
                    "Failed to insert seed user: {}",
                    e
                )));
            }
        }
    } else {
        tracing::info!("Seeding skipped: users collection already contains records.");
    }

    Ok(db)
}
