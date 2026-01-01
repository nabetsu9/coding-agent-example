use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Request structure for Messages API
#[derive(Debug, Serialize)]
struct MessageRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

/// Response structure
#[derive(Debug, Deserialize)]
#[allow(dead_code)]  // API response fields - not all are currently used
pub struct MessageResponse {
    pub id: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
pub struct ContentBlock {
    // `type` はRustの予約語なので `content_type` という名前にして、JSON上では `"type"` として扱う
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Anthropic API client
pub struct AnthropicClient {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl AnthropicClient {
    /// Create new Anthropic API client
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.anthropic.com/v1".to_string(),
            client: reqwest::Client::new(),
        }
    }
    /// Send a message to Claude (non-streaming)
    pub async fn create_message(
        &self,
        model: &str,
        max_tokens: u32,
        user_message: &str,
    ) -> Result<MessageResponse> {
        debug!("Preparing request to Anthropic API");
        debug!(?model, ?max_tokens, "Request parameters");

        let request = MessageRequest {
            model: model.to_string(),
            max_tokens,
            messages: vec![Message {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
        };

        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Anthropic API")?;

        let status = response.status();
        debug!(?status, "Received response from Anthropic API");

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("API request failed with status {}: {}", status, error_text);
        }

        let message_response = response
            .json::<MessageResponse>()
            .await
            .context("Failed to parse API response")?;

        info!("Successfully received response from Claude");

        Ok(message_response)
    }
}
