use crate::config::AppConfig;
use crate::errors::AppError;
use lapin::{
    Channel, Connection, ConnectionProperties, options::QueueDeclareOptions, types::FieldTable,
};

/// Name of the message queue for processing background tasks.
pub const JOBS_QUEUE: &str = "rustom_jobs_queue";

/// Initialize RabbitMQ connection and channel, declaring the target queue.
pub async fn init_rabbitmq(config: &AppConfig) -> Result<(Connection, Channel), AppError> {
    tracing::info!(
        "Initializing RabbitMQ connection at {}...",
        config.rabbitmq_url
    );

    let conn = Connection::connect(&config.rabbitmq_url, ConnectionProperties::default())
        .await
        .map_err(AppError::Queue)?;

    let channel: Channel = conn.create_channel().await.map_err(AppError::Queue)?;
    channel
        .confirm_select(lapin::options::ConfirmSelectOptions::default())
        .await
        .map_err(AppError::Queue)?;

    tracing::info!("Declaring queue: {}", JOBS_QUEUE);
    let _: lapin::Queue = channel
        .queue_declare(
            JOBS_QUEUE,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .map_err(AppError::Queue)?;

    tracing::info!("RabbitMQ is ready.");
    Ok((conn, channel))
}
