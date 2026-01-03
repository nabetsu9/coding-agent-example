use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use tracing::{debug, warn};

use crate::anthropic::{Tool, ToolHandler, ToolResult};

/// listFiles ツールの引数
#[derive(Debug, Deserialize)]
struct ListFilesArgs {
    path: String,
    #[serde(default)]
    recursive: bool,
}

/// ファイル情報
#[derive(Debug, Serialize)]
struct FileInfo {
    path: String,
    is_dir: bool,
    size: u64,
}

/// listFiles ツールの実装
pub struct ListFilesTool;

impl ListFilesTool {
    pub fn new() -> Self {
        Self
    }

    /// ツールのスキーマ定義を返す
    pub fn schema() -> Tool {
        Tool {
            name: "listFiles".to_string(),
            description: "指定されたディレクトリ内のファイルとディレクトリの一覧を取得します。recursiveがtrueの場合、サブディレクトリも含めます。".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "一覧を取得するディレクトリのパス（例: src, ., ./docs）"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "サブディレクトリも含めて再帰的に一覧を取得するか（デフォルト: false）"
                    }
                },
                "required": ["path"]
            }),
        }
    }
}

#[async_trait]
impl ToolHandler for ListFilesTool {
    async fn execute(&self, input: serde_json::Value) -> Result<ToolResult> {
        debug!("Executing listFiles tool with input: {:?}", input);

        // 引数をパース
        let args: ListFilesArgs =
            serde_json::from_value(input).context("Failed to parse listFiles arguments")?;

        debug!(
            "Listing files in: {} (recursive: {})",
            args.path, args.recursive
        );

        let path = Path::new(&args.path);

        // ディレクトリが存在しない場合
        if !path.exists() {
            warn!("Directory not found: {}", args.path);
            return Ok(ToolResult {
                content: String::new(),
                error: Some(format!("ディレクトリが見つかりません: {}", args.path)),
            });
        }

        // ファイルの場合はエラー
        if !path.is_dir() {
            warn!("Path is not a directory: {}", args.path);
            return Ok(ToolResult {
                content: String::new(),
                error: Some(format!(
                    "指定されたパスはディレクトリではありません: {}",
                    args.path
                )),
            });
        }

        // ファイル一覧を取得（今は非再帰のみ）
        let mut files = Vec::new();

        if args.recursive {
            // 再帰モード: walkdir を使用
            use walkdir::WalkDir;

            for entry_result in WalkDir::new(path) {
                match entry_result {
                    Ok(entry) => {
                        let entry_path = entry.path();
                        let metadata = match entry.metadata() {
                            Ok(m) => m,
                            Err(e) => {
                                warn!("Failed to get metadata for {:?}: {}", entry_path, e);
                                continue;
                            }
                        };

                        files.push(process_entry(&entry_path, &metadata))
                    }
                    Err(e) => {
                        warn!("Failed to read entry: {}", e);
                        continue;
                    }
                }
            }
        } else {
            // 非再帰モード: std::fs::read_dir を使用
            match std::fs::read_dir(path) {
                Ok(entries) => {
                    for entry_result in entries {
                        match entry_result {
                            Ok(entry) => {
                                let entry_path = entry.path();
                                let metadata = match entry.metadata() {
                                    Ok(m) => m,
                                    Err(e) => {
                                        warn!("Failed to get metadata for {:?}: {}", entry_path, e);
                                        continue;
                                    }
                                };

                                files.push(process_entry(&entry_path, &metadata))
                            }
                            Err(e) => {
                                warn!("Failed to read entry: {}", e);
                                continue;
                            }
                        }
                    }
                }
                Err(e) => {
                    return Ok(ToolResult {
                        content: String::new(),
                        error: Some(format!("ディレクトリの読み込みに失敗しました: {}", e)),
                    });
                }
            }
        }

        // 結果をJSON形式で返す
        let result_json =
            serde_json::to_string_pretty(&files).context("Failed to serialize file list")?;

        debug!("Found {} files/directories", files.len());

        Ok(ToolResult {
            content: result_json,
            error: None,
        })
    }
}

fn process_entry(entry_path: &Path, metadata: &std::fs::Metadata) -> FileInfo {
    FileInfo {
        path: entry_path.display().to_string(),
        is_dir: metadata.is_dir(),
        size: metadata.len(),
    }
}
