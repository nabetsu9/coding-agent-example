use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn execute(&self, input: serde_json::Value) -> Result<ToolResult>;
}

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
    #[allow(dead_code)]
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: MessageContent::Text(text.into()),
        }
    }

    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

    /// ツールをサポートしたメッセージ作成
    pub async fn create_message_with_tools(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<Message>,
        tools: Option<Vec<Tool>>,
    ) -> Result<MessageResponse> {
        debug!("Preparing request to Anthropic API with tools");
        debug!(
            ?model,
            ?max_tokens,
            messages_count = messages.len(),
            "Request parameters"
        );

        let request = MessageRequest {
            model: model.to_string(),
            max_tokens,
            messages,
            tools,
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

    /// ツールを使った会話（Agentic Loop）
    pub async fn execute_with_tools(
        &self,
        model: &str,
        max_tokens: u32,
        user_message: &str,
        tool_registry: &ToolRegistry,
        max_iterations: usize,
    ) -> Result<ConversationResult> {
        // 会話履歴を初期化
        let mut conversation = vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text(user_message.to_string()),
        }];

        // 最大反復回数までループ
        for iteration in 0..max_iterations {
            info!("Iteration {}/{}", iteration + 1, max_iterations);

            // APIを呼び出す
            let response = self
                .create_message_with_tools(
                    model,
                    max_tokens,
                    conversation.clone(),
                    Some(tool_registry.get_schemas()),
                )
                .await?;

            // アシスタントのメッセージを会話履歴に追加
            conversation.push(Message {
                role: "assistant".to_string(),
                content: MessageContent::Blocks(response.content.clone()),
            });

            // stop_reason をチェック
            if response.stop_reason.as_deref() != Some("tool_use") {
                // ツール使用がない → 最終応答
                info!("Conversation completed in {} iterations", iteration + 1);
                return Ok(ConversationResult {
                    response,
                    conversation,
                    iterations: iteration + 1,
                });
            }

            // ツールを実行
            info!("Executing tools...");
            let tool_results = self.execute_tools(&response.content, tool_registry).await?;

            // ツール結果を会話履歴に追加
            conversation.push(Message {
                role: "user".to_string(),
                content: MessageContent::Blocks(tool_results),
            });
        }

        // 最大反復回数に到達
        bail!(
            "Max iterations ({}) reached without final response",
            max_iterations
        );
    }

    /// content blocks からツールを抽出して実行
    async fn execute_tools(
        &self,
        content_blocks: &[ContentBlock],
        tool_registry: &ToolRegistry,
    ) -> Result<Vec<ContentBlock>> {
        let mut results = Vec::new();

        for block in content_blocks {
            if let ContentBlock::ToolUse { id, name, input } = block {
                info!("Executing tool: {}", name);

                // ツールを実行
                let result = tool_registry.execute(name, input.clone()).await?;

                // 結果を JSON にシリアライズ
                let content =
                    serde_json::to_string(&result).context("Failed to serialize tool result")?;

                // tool_result block を作成
                results.push(ContentBlock::ToolResult {
                    tool_use_id: id.clone(),
                    content,
                    is_error: result.error.as_ref().map(|_| true),
                });

                info!("Tool '{}' executed successfully", name);
            }
        }

        Ok(results)
    }
}

/// ツールのレジストリ（登録・管理・実行）
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn ToolHandler>>,
    schemas: Vec<Tool>,
}

impl ToolRegistry {
    /// 新しいレジストリを作成
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            schemas: Vec::new(),
        }
    }

    /// ツールを登録
    pub fn register<T: ToolHandler + 'static>(&mut self, schema: Tool, handler: T) {
        let name = schema.name.clone();
        self.schemas.push(schema);
        self.tools.insert(name, Box::new(handler));
    }

    /// 登録されているツールのスキーマ一覧を取得
    pub fn get_schemas(&self) -> Vec<Tool> {
        self.schemas.clone()
    }

    /// ツールを実行
    pub async fn execute(&self, name: &str, input: serde_json::Value) -> Result<ToolResult> {
        let handler = self
            .tools
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", name))?;

        handler.execute(input).await
    }
}

/// 会話の結果（ツール実行を含む）
pub struct ConversationResult {
    pub response: MessageResponse,
    #[allow(dead_code)]
    pub conversation: Vec<Message>,
    pub iterations: usize,
}
