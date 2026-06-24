use reqwest::Client;
use serde_json::json;

pub struct SlackNotification;

impl SlackNotification {
    /// Notify on unexpected errors
    pub async fn notify_error(message: &str) -> Result<(), reqwest::Error> {
        let webhook_url = std::env::var("SLACK_ERROR_WEBHOOK_URL").unwrap_or_default();
        if webhook_url.is_empty() {
            return Ok(());
        }
        Self::send(&webhook_url, message).await
    }

    /// Notify on successful registration
    pub async fn notify_registration(message: &str) -> Result<(), reqwest::Error> {
        let webhook_url = std::env::var("SLACK_REGISTRATION_WEBHOOK_URL").unwrap_or_default();
        if webhook_url.is_empty() {
            return Ok(());
        }
        Self::send(&webhook_url, message).await
    }

    async fn send(webhook_url: &str, text: &str) -> Result<(), reqwest::Error> {
        let client = Client::new();
        let payload = json!({
            "text": text
        });

        let response = client.post(webhook_url).json(&payload).send().await?;

        if !response.status().is_success() {
            tracing::warn!("Failed to send Slack notification: {:?}", response.status());
        }

        Ok(())
    }
}
