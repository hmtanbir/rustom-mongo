use crate::errors::AppError;
use crate::infrastructure::JOBS_QUEUE;
use crate::models::JobPayload;
use futures_util::stream::StreamExt;
use lapin::{
    BasicProperties, Channel,
    options::{BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, BasicRejectOptions},
    types::FieldTable,
};

use async_trait::async_trait;
use std::sync::Arc;

/// Generic messaging queue service trait.
#[async_trait]
pub trait QueueService: Send + Sync {
    /// Publish a background task to the message broker.
    async fn publish_job(&self, job: &JobPayload) -> Result<(), AppError>;
}

/// RabbitMQ implementation of the QueueService trait.
#[derive(Clone)]
pub struct RabbitMQQueueService {
    channel: Channel,
}

impl RabbitMQQueueService {
    /// Create a new RabbitMQQueueService.
    pub fn new(channel: Channel) -> Self {
        Self { channel }
    }
}

#[async_trait]
impl QueueService for RabbitMQQueueService {
    async fn publish_job(&self, job: &JobPayload) -> Result<(), AppError> {
        tracing::debug!("Publishing job to queue: {:?}", job);

        let payload = serde_json::to_vec(job).map_err(|e| {
            AppError::Unexpected(anyhow::anyhow!("Failed to serialize job payload: {}", e))
        })?;

        let confirm = self
            .channel
            .basic_publish(
                "", // Default exchange
                JOBS_QUEUE,
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await
            .map_err(AppError::Queue)?
            .await
            .map_err(AppError::Queue)?;

        if !confirm.is_ack() {
            return Err(AppError::Queue(lapin::Error::IOError(std::sync::Arc::new(
                std::io::Error::other("Message was not acknowledged by RabbitMQ broker"),
            ))));
        }

        tracing::info!("Job {} published successfully.", job.job_id);
        Ok(())
    }
}

/// Dynamic trait object for QueueService.
pub type DynQueueService = Arc<dyn QueueService>;

/// Spawns a non-blocking Tokio background worker task to consume and process RabbitMQ messages.
pub fn start_queue_consumer(channel: Channel) {
    tokio::spawn(async move {
        tracing::info!("Starting background queue worker consumer...");

        let mut consumer = match channel
            .basic_consume(
                JOBS_QUEUE,
                "rustom_consumer_tag",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
        {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to register queue consumer: {:?}", e);
                return;
            }
        };

        while let Some(delivery_result) = consumer.next().await {
            let delivery = match delivery_result {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("Error in consumer delivery stream: {:?}", e);
                    continue;
                }
            };

            let data = &delivery.data;
            let payload: JobPayload = match serde_json::from_slice(data) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("Failed to deserialize received job payload: {:?}", e);
                    // Reject invalid messages without requeuing to prevent poison-pill loops
                    let _ = delivery.reject(BasicRejectOptions::default()).await;
                    continue;
                }
            };

            tracing::info!(
                "Worker received Job ID: {} [Type: {}]",
                payload.job_id,
                payload.job_type
            );

            // Execute asynchronous processing based on the job type
            let process_result = process_job(&payload).await;

            match process_result {
                Ok(_) => {
                    tracing::info!(
                        "Job {} completed successfully. Acknowledging...",
                        payload.job_id
                    );
                    if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                        tracing::error!("Failed to acknowledge message: {:?}", e);
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to process job {}: {:?}. Rejecting...",
                        payload.job_id,
                        e
                    );
                    // Reject the message without requeuing to prevent poison pill loops.
                    // In a production system, a Dead Letter Exchange (DLX) should be configured
                    // on the queue to capture these failed messages.
                    if let Err(re) = delivery.reject(BasicRejectOptions { requeue: false }).await {
                        tracing::error!("Failed to reject message: {:?}", re);
                    }
                }
            }
        }
    });
}

/// Processes a single background job payload.
async fn process_job(job: &JobPayload) -> Result<(), AppError> {
    // Mimic database or network processing latency without blocking the main worker thread
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    match job.job_type.as_str() {
        "email" => {
            let email_to = job
                .payload
                .get("to")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let email_body = job
                .payload
                .get("body")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            tracing::info!(
                "Sending email background job -> TO: {}, BODY: {}",
                email_to,
                email_body
            );
            Ok(())
        }
        "data_process" => {
            tracing::info!("Executing data_process job payload: {:?}", job.payload);
            Ok(())
        }
        unknown_type => {
            let error_msg = format!("Unrecognized job type: {}", unknown_type);
            tracing::warn!("{}", error_msg);
            Err(AppError::Unexpected(anyhow::anyhow!(error_msg)))
        }
    }
}
