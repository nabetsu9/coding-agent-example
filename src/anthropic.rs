use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// メッセージの内容（文字列 or ブロック配列）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

/// Request structure for Messages API
#[derive(Debug, Serialize)]
struct MessageRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String, // "user" または "assistant"
    pub content: MessageContent,
}

impl Message {
    /// テキストメッセージを作成（便利メソッド）
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: MessageContent::Text(text.into()),
        }
    }

    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: MessageContent::Text(text.into()),
        }
    }
}

/// Response structure
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // API response fields - not all are currently used
pub struct MessageResponse {
    pub id: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },

    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Tool definition for the API
#[derive(Debug, Clone, Serialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// ツール実行結果
/// どちらか一方のみ設定される
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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
            messages: vec![Message::user_text(user_message)],
            tools: None,
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
