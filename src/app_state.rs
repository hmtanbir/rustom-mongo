use crate::config::AppConfig;
use crate::services::{DynQueueService, UserService};

/// Struct containing shared application dependencies.
#[derive(Clone)]
pub struct AppState {
    /// Service logic for user management & password validation.
    pub user_service: UserService,
    /// Publisher service for pushing jobs to RabbitMQ.
    pub queue_publisher: DynQueueService,
    /// Environment variables configuration.
    pub config: AppConfig,
}
