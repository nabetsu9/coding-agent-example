use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, Write};
use std::path::Path;
use tokio::fs;
use tracing::{debug, warn};

use crate::anthropic::{Tool, ToolHandler, ToolResult};

#[derive(Debug, Deserialize)]
pub struct EditFileArgs {
    /// 編集する既存ファイルのパス
    pub path: String,
    /// ファイル全体を上書きする新しい完全な内容
    pub new_content: String,
}

/// editFile ツール
pub struct EditFileTool;

impl EditFileTool {
    /// 新しいインスタンスを作成
    pub fn new() -> Self {
        Self
    }

    /// ツールのスキーマ定義を返す
    pub fn schema() -> Tool {
        Tool {
            name: "editFile".to_string(),
            description: "既存ファイルの内容を完全に上書きします。\
                          重要: ファイルを破壊しないために、必ず以下のワークフローに従ってください:\n\
                          1. 'readFile'を使用して現在の完全な内容を取得する\n\
                          2. 思考プロセスで、読み取った内容を基に新しいファイルの完全版を構築する\n\
                          3. このツールを使用して完全な新しい内容を書き込む\n\
                          部分的な編集には使用しないでください。常にファイル全体の内容を提供してください。\
                          実行前にユーザーの許可を求めます。"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "編集する既存ファイルのパス"
                    },
                    "new_content": {
                        "type": "string",
                        "description": "ファイル全体を上書きする新しい完全な内容"
                    }
                },
                "required": ["path", "new_content"]
            }),
        }
    }
}

impl EditFileTool {
    // ... 既存のメソッド（new, schema）...

    /// ファイルが存在するかチェック
    fn check_file_exists(path: &str) -> Result<(), String> {
        if !Path::new(path).exists() {
            return Err(
                "ファイルが存在しません。新しいファイルの作成にはwriteFileを使用してください。"
                    .to_string(),
            );
        }

        if !Path::new(path).is_file() {
            return Err(format!("{} はファイルではありません。", path));
        }

        Ok(())
    }

    /// ユーザーに確認を求める
    fn prompt_user_confirmation(path: &str) -> Result<bool> {
        // 1. プロンプトを表示
        print!(
            "\n既存ファイルを編集します: {}\n実行してもよろしいですか？ [y/N]: ",
            path
        );

        // 2. バッファをフラッシュ（即座に表示）
        io::stdout()
            .flush()
            .context("標準出力のフラッシュに失敗しました")?;

        // 3. ユーザー入力を読み取り
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .context("ユーザー入力の読み取りに失敗しました")?;

        // 4. 入力を検証（"y" または "Y" のみ許可）
        let confirmed = input.trim().to_lowercase() == "y";

        Ok(confirmed)
    }
}

#[async_trait]
impl ToolHandler for EditFileTool {
    async fn execute(&self, input: Value) -> Result<ToolResult> {
        debug!("Executing editFile tool");

        // 1. 入力をパース
        let args: EditFileArgs =
            serde_json::from_value(input).context("editFile: 引数のパースに失敗しました")?;

        debug!(
            "editFile args: path={}, content_length={}",
            args.path,
            args.new_content.len()
        );

        // 2. ファイルが存在するかチェック
        if let Err(error_msg) = Self::check_file_exists(&args.path) {
            warn!("editFile: ファイル存在チェック失敗: {}", error_msg);
            return Ok(ToolResult {
                content: String::new(),
                error: Some(error_msg),
            });
        }

        // 3. ユーザーに確認
        match Self::prompt_user_confirmation(&args.path) {
            Ok(true) => {
                debug!("editFile: ユーザーが承認しました");
            }
            Ok(false) => {
                warn!("editFile: ユーザーによってキャンセルされました");
                return Ok(ToolResult {
                    content: String::new(),
                    error: Some("ユーザーによってキャンセルされました".to_string()),
                });
            }
            Err(e) => {
                warn!("editFile: ユーザー確認中にエラー: {}", e);
                return Ok(ToolResult {
                    content: String::new(),
                    error: Some(format!("ユーザー確認中にエラーが発生しました: {}", e)),
                });
            }
        }

        // 4. ファイルを完全に上書き
        match fs::write(&args.path, &args.new_content).await {
            Ok(_) => {
                debug!("editFile: ファイルを正常に更新しました: {}", args.path);
                Ok(ToolResult {
                    content: format!("ファイル {} を正常に更新しました", args.path),
                    error: None,
                })
            }
            Err(e) => {
                warn!("editFile: ファイルの書き込みに失敗: {}", e);
                Ok(ToolResult {
                    content: String::new(),
                    error: Some(format!("ファイルの書き込みに失敗しました: {}", e)),
                })
            }
        }
    }
}
