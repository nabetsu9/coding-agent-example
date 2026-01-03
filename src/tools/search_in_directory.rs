use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use tracing::{debug, warn};

use crate::anthropic::{Tool, ToolHandler, ToolResult};

/// searchInDirectory ツールの引数
#[derive(Debug, Deserialize)]
struct SearchInDirectoryArgs {
    path: String,
    keyword: String,
}

/// 検索結果の1件
#[derive(Debug, Serialize)]
struct SearchMatch {
    path: String,
    line_number: usize,
    line: String,
}

/// searchInDirectory ツールの実装
pub struct SearchInDirectoryTool;

impl SearchInDirectoryTool {
    pub fn new() -> Self {
        Self
    }

    /// ツールのスキーマ定義を返す
    pub fn schema() -> Tool {
        Tool {
            name: "searchInDirectory".to_string(),
            description: "指定されたディレクトリ配下のファイルをキーワード検索し、マッチした行を返します。大文字小文字は区別しません。".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "検索を開始するディレクトリのパス"
                    },
                    "keyword": {
                        "type": "string",
                        "description": "検索するキーワード"
                    }
                },
                "required": ["path", "keyword"]
            }),
        }
    }
}

#[async_trait]
impl ToolHandler for SearchInDirectoryTool {
    async fn execute(&self, input: serde_json::Value) -> Result<ToolResult> {
        debug!("Executing searchInDirectory tool with input: {:?}", input);

        // 引数をパース
        let args: SearchInDirectoryArgs =
            serde_json::from_value(input).context("Failed to parse searchInDirectory arguments")?;

        debug!("Searching for '{}' in: {}", args.keyword, args.path);

        let path = Path::new(&args.path);

        // ディレクトリが存在しない場合
        if !path.exists() {
            warn!("Directory not found: {}", args.path);
            return Ok(ToolResult {
                content: String::new(),
                error: Some(format!("ディレクトリが見つかりません: {}", args.path)),
            });
        }

        // TODO: タスク5で実装
        let mut matches = Vec::new();
        let keyword_lower = args.keyword.to_lowercase();

        use walkdir::WalkDir;

        for entry_result in WalkDir::new(path) {
            let entry = match entry_result {
                Ok(e) => e,
                Err(e) => {
                    warn!("Failed to read entry: {}", e);
                    continue;
                }
            };
            if entry.file_type().is_dir() {
                continue;
            }

            let file_path = entry.path();

            let content = match tokio::fs::read_to_string(file_path).await {
                Ok(c) => c,
                Err(_) => {
                    // バイナリファイルや権限エラーは静かにスキップ
                    debug!("Skipping file: {:?}", file_path);
                    continue;
                }
            };

            for (line_num, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&keyword_lower) {
                    matches.push(SearchMatch {
                        path: file_path.display().to_string(),
                        line_number: line_num + 1,
                        line: line.to_string(),
                    });
                }
            }
        }

        let result_json =
            serde_json::to_string_pretty(&matches).context("Failed to serialize serach results")?;

        debug!("Found {} matches", matches.len());

        Ok(ToolResult {
            content: result_json,
            error: None,
        })
    }
}
