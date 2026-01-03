use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::io::{self, Write as IoWrite};
use std::path::Path;
use tracing::{debug, warn};

use crate::anthropic::{Tool, ToolHandler, ToolResult};

/// ユーザーに確認を求める
///
/// # Returns
/// - `Ok(true)` - ユーザーが 'y' または 'Y' を入力
/// - `Ok(false)` - ユーザーがそれ以外を入力
/// - `Err(_)` - 入力の読み取りに失敗
fn prompt_user_confirmation(message: &str) -> Result<bool> {
    // 1. プロンプトを表示
    print!("{} [y/N]: ", message);

    // 2. バッファをフラッシュ（即座に表示）
    io::stdout().flush().context("Failed to flush stdout")?;

    // 3. ユーザー入力を読み取り
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("Failed to read user input")?;

    // 4. 入力を検証（'y' または 'Y' のみ許可）
    Ok(input.trim().to_lowercase() == "y")
}

/// writeFile ツールの引数
#[derive(Debug, Deserialize)]
struct WriteFileArgs {
    path: String,
    content: String,
}

/// writeFile ツールの実装
pub struct WriteFileTool;

impl WriteFileTool {
    pub fn new() -> Self {
        Self
    }

    /// ツールのスキーマ定義を返す
    pub fn schema() -> Tool {
        Tool {
            name: "writeFile".to_string(),
            description: "指定されたパスに新しいファイルを作成し、内容を書き込みます。親ディレクトリが存在しない場合は自動で作成します。既存ファイルが存在する場合は確認を求めます。".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "作成するファイルの完全なパス（例: test.txt, src/new_file.rs）"
                    },
                    "content": {
                        "type": "string",
                        "description": "ファイルに書き込む内容"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }
}

#[async_trait]
impl ToolHandler for WriteFileTool {
    async fn execute(&self, input: serde_json::Value) -> Result<ToolResult> {
        debug!("Executing writeFile tool with input: {:?}", input);

        // 引数をパース
        let args: WriteFileArgs =
            serde_json::from_value(input).context("Failed to parse writeFile arguments")?;

        debug!("Writing to file: {}", args.path);

        let path = Path::new(&args.path);

        if path.exists() {
            warn!("File already exists: {}", args.path);

            let message = format!(
                "ファイル '{}' は既に存在します。上書きしますか？",
                args.path
            );
            match prompt_user_confirmation(&message) {
                Ok(true) => {
                    debug!("User confirmed overwrite");
                }
                Ok(false) => {
                    debug!("User cancelled");
                    return Ok(ToolResult {
                        content: String::new(),
                        error: Some("ユーザーによりキャンセルされました".to_string()),
                    });
                }
                Err(e) => {
                    return Ok(ToolResult {
                        content: String::new(),
                        error: Some(format!("ユーザー入力の読み取りに失敗しました: {}", e)),
                    });
                }
            }
        } else {
            // 新規ファイルの場合も確認
            let message = format!("ファイル '{}' を作成しますか？", args.path);
            match prompt_user_confirmation(&message) {
                Ok(true) => {
                    debug!("User confirmed file creation");
                }
                Ok(false) => {
                    debug!("User cancelled");
                    return Ok(ToolResult {
                        content: String::new(),
                        error: Some("ユーザーによりキャンセルされました".to_string()),
                    });
                }
                Err(e) => {
                    return Ok(ToolResult {
                        content: String::new(),
                        error: Some(format!("ユーザー入力の読み取りに失敗しました: {}", e)),
                    });
                }
            }
        }

        // 親ディレクトリの作成
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                debug!("Creating parent directory: {:?}", parent);
                match tokio::fs::create_dir_all(parent).await {
                    Ok(_) => {
                        debug!("Parent directory created successfully");
                    }
                    Err(e) => {
                        return Ok(ToolResult {
                            content: String::new(),
                            error: Some(format!("ディレクトリの作成に失敗しました: {}", e)),
                        });
                    }
                }
            }
        }
        // ファイル書き込み
        match tokio::fs::write(&path, &args.content).await {
            Ok(_) => {
                debug!("File written successfully: {}", args.path);
                Ok(ToolResult {
                    content: format!(
                        "ファイル '{}' を作成しました（{}バイト）",
                        args.path,
                        args.content.len()
                    ),
                    error: None,
                })
            }
            Err(e) => {
                warn!("Failed to write file {}: {}", args.path, e);
                Ok(ToolResult {
                    content: String::new(),
                    error: Some(format!("ファイルの書き込みに失敗しました: {}", e)),
                })
            }
        }
    }
}
