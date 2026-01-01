use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, warn};

use crate::anthropic::{Tool, ToolHandler, ToolResult};

/// readFile ツールの引数
#[derive(Debug, Deserialize)]
struct ReadFileArgs {
    path: String,
}

/// readFile ツールの実装
pub struct ReadFileTool;

impl ReadFileTool {
    pub fn new() -> Self {
        Self
    }

    /// ツールのスキーマ定義を返す
    pub fn schema() -> Tool {
        Tool {
            name: "readFile".to_string(),
            description:
                "指定されたパスのファイル内容を読み込みます。相対パスまたは絶対パスを指定できます。"
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "読み込むファイルのパス（例: README.md, src/main.rs）"
                    }
                },
                "required": ["path"]
            }),
        }
    }
}

#[async_trait]
impl ToolHandler for ReadFileTool {
    async fn execute(&self, input: serde_json::Value) -> Result<ToolResult> {
        debug!("Executing readFile tool with input: {:?}", input);

        // 引数をパース
        let args: ReadFileArgs =
            serde_json::from_value(input).context("Failed to parse readFile arguments")?;

        debug!("Reading file: {}", args.path);

        // パスのバリデーション
        let path = PathBuf::from(&args.path);

        // ファイルが存在しない場合
        if !path.exists() {
            warn!("File not found: {}", args.path);
            return Ok(ToolResult {
                content: String::new(),
                error: Some(format!("ファイルが見つかりません: {}", args.path)),
            });
        }

        // ファイル読み込み
        match fs::read_to_string(&path).await {
            Ok(content) => {
                debug!(
                    "Successfully read {} bytes from {}",
                    content.len(),
                    args.path
                );
                Ok(ToolResult {
                    content,
                    error: None,
                })
            }
            Err(e) => {
                warn!("Failed to read file {}: {}", args.path, e);
                Ok(ToolResult {
                    content: String::new(),
                    error: Some(format!("ファイルの読み込みに失敗しました: {}", e)),
                })
            }
        }
    }
}
