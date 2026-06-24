use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Representation of an asynchronous job payload to be placed in RabbitMQ.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct JobPayload {
    pub job_id: Uuid,
    pub job_type: String,
    pub payload: serde_json::Value,
}

/// Request DTO for creating a new background job.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateJobRequestDto {
    pub job_type: String,
    pub payload: serde_json::Value,
}

/// Response DTO confirming a job has been queued.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateJobResponseDto {
    pub status: String,
    pub job_id: Uuid,
}
