# ファイルシステムツール実装ガイド（Rust初心者向け）

このガイドでは、Anthropic Claude APIのTool Use機能を使用して、ファイルシステムを操作する3つのツール（listFiles, searchInDirectory, writeFile）を段階的に実装していきます。各タスクで動作確認をしながら進めることで、確実に理解を深められます。

## 📋 全体の流れ

```
タスク1: walkdir クレートの追加と理解
  ↓ 動作確認: cargo build が成功
  ↓
タスク2: ListFilesTool の基本実装（非再帰）
  ↓ 動作確認: cargo build が成功
  ↓
タスク3: ListFilesTool の再帰対応
  ↓ 動作確認: listFiles が動作
  ↓
タスク4: SearchInDirectoryTool の基本構造
  ↓ 動作確認: cargo build が成功
  ↓
タスク5: SearchInDirectoryTool の検索ロジック実装
  ↓ 動作確認: searchInDirectory が動作
  ↓
タスク6: WriteFileTool の基本構造
  ↓ 動作確認: cargo build が成功
  ↓
タスク7: ユーザー確認機能の実装
  ↓ 動作確認: 確認プロンプトが表示される
  ↓
タスク8: WriteFileTool の完成
  ↓ 動作確認: writeFile が動作
  ↓
タスク9: 3つのツールをToolRegistryに登録
  ↓ 動作確認: すべてのツールが利用可能
  ↓
タスク10: 統合テストとコード品質チェック
  ✓ 完成！
```

---

## タスク1: walkdir クレートの追加と理解

### 🎯 目標

再帰的なディレクトリ探索を可能にする `walkdir` クレートを追加し、基本的な使い方を理解する

### 📝 手順

#### 1.1 Cargo.toml に依存関係を追加

`Cargo.toml` を開き、`[dependencies]` セクションに以下を追加してください：

```toml
[dependencies]
# 既存の依存関係...
clap = { version = "4.5.53", features = ["derive"] }
tokio = { version = "1.48", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
dotenvy = "0.15"
async-trait = "0.1.89"

# 🆕 新規追加
walkdir = "2.5"
```

#### 1.2 walkdir の基本的な使い方を理解する

`walkdir` クレートは、ディレクトリ階層を再帰的に探索するためのイテレータを提供します。

**基本的な使い方：**

```rust
use walkdir::WalkDir;

// カレントディレクトリ配下を再帰的に探索
for entry in WalkDir::new(".") {
    let entry = entry?;  // Result<DirEntry, Error>
    println!("{}", entry.path().display());
}
```

**出力例：**
```
.
./Cargo.toml
./src
./src/main.rs
./src/anthropic.rs
./src/tools
./src/tools/mod.rs
./src/tools/read_file.rs
```

### 💡 Rust知識ポイント

#### 1. イテレータパターン

`walkdir` はイテレータを返すため、`for` ループで順に処理できます。

```rust
// イテレータの各要素は Result<DirEntry, Error>
for entry_result in WalkDir::new(".") {
    let entry = entry_result?;  // エラーが発生した場合は早期リターン
    // entry は DirEntry 型
}
```

#### 2. max_depth でネスト制限

```rust
use walkdir::WalkDir;

// 1階層のみ（サブディレクトリに入らない）
for entry in WalkDir::new(".").max_depth(1) {
    let entry = entry?;
    println!("{}", entry.path().display());
}

// 無制限（デフォルト）
for entry in WalkDir::new(".") {
    let entry = entry?;
    println!("{}", entry.path().display());
}
```

#### 3. filter_entry でフィルタリング

```rust
use walkdir::WalkDir;

// 隠しファイルをスキップする例
fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

for entry in WalkDir::new(".")
    .into_iter()
    .filter_entry(|e| !is_hidden(e))
{
    let entry = entry?;
    println!("{}", entry.path().display());
}
```

#### 4. DirEntry のメソッド

```rust
use walkdir::DirEntry;

fn process_entry(entry: &DirEntry) {
    // パス情報
    let path = entry.path();           // &Path
    let file_name = entry.file_name();  // &OsStr

    // ファイルタイプ
    let file_type = entry.file_type();
    let is_dir = file_type.is_dir();
    let is_file = file_type.is_file();
    let is_symlink = file_type.is_symlink();

    // メタデータ
    if let Ok(metadata) = entry.metadata() {
        let size = metadata.len();
        println!("{}: {} bytes", path.display(), size);
    }
}
```

### ✅ 動作確認

```bash
cargo build
```

エラーがなければ成功です。まだ新しいコードは書いていないので、単に依存関係が正しく追加されたことを確認するだけです。

---

## タスク2: ListFilesTool の基本実装（非再帰）

### 🎯 目標

ディレクトリ内のファイル一覧を取得する基本機能を実装する（再帰なし、1階層のみ）

### 📝 手順

#### 2.1 src/tools/list_files.rs を作成

新しいファイル `src/tools/list_files.rs` を作成し、以下のコードを記述してください：

```rust
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

        debug!("Listing files in: {} (recursive: {})", args.path, args.recursive);

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
                error: Some(format!("指定されたパスはディレクトリではありません: {}", args.path)),
            });
        }

        // ファイル一覧を取得（今は非再帰のみ）
        let mut files = Vec::new();

        if args.recursive {
            // TODO: タスク3で実装
            return Ok(ToolResult {
                content: String::new(),
                error: Some("再帰モードはまだ実装されていません".to_string()),
            });
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

                                files.push(FileInfo {
                                    path: entry_path.display().to_string(),
                                    is_dir: metadata.is_dir(),
                                    size: metadata.len(),
                                });
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
        let result_json = serde_json::to_string_pretty(&files)
            .context("Failed to serialize file list")?;

        debug!("Found {} files/directories", files.len());

        Ok(ToolResult {
            content: result_json,
            error: None,
        })
    }
}
```

#### 2.2 src/tools/mod.rs を更新

`src/tools/mod.rs` に新しいモジュールを追加します：

```rust
pub mod read_file;
pub mod list_files;  // 🆕 追加

pub use read_file::ReadFileTool;
pub use list_files::ListFilesTool;  // 🆕 追加
```

### 💡 Rust知識ポイント

#### 1. `#[serde(default)]` アトリビュート

```rust
#[derive(Debug, Deserialize)]
struct ListFilesArgs {
    path: String,
    #[serde(default)]  // このフィールドがJSONに含まれない場合、デフォルト値を使用
    recursive: bool,    // bool のデフォルト値は false
}
```

**動作例：**
```rust
// JSON: {"path": "src"}
// → recursive は false になる

// JSON: {"path": "src", "recursive": true}
// → recursive は true になる
```

#### 2. メタデータの取得

```rust
use std::fs;

let metadata = fs::metadata(path)?;
let is_dir = metadata.is_dir();       // ディレクトリか
let is_file = metadata.is_file();     // ファイルか
let size = metadata.len();            // サイズ（バイト）
let modified = metadata.modified()?;  // 最終更新時刻
```

#### 3. DirEntry から情報を取得

```rust
use std::fs;

for entry in fs::read_dir(path)? {
    let entry = entry?;

    // パス取得
    let path = entry.path();  // PathBuf

    // メタデータ取得
    let metadata = entry.metadata()?;

    // ファイル名取得
    let file_name = entry.file_name();  // OsString
}
```

#### 4. エラーハンドリングパターン

```rust
// パターン1: continue でスキップ
for entry_result in entries {
    match entry_result {
        Ok(entry) => { /* 処理 */ }
        Err(e) => {
            warn!("Failed to read entry: {}", e);
            continue;  // エラーを無視して次へ
        }
    }
}

// パターン2: ToolResult でエラーを返す
match fs::read_dir(path) {
    Ok(entries) => { /* 処理 */ }
    Err(e) => {
        return Ok(ToolResult {
            content: String::new(),
            error: Some(format!("エラー: {}", e)),
        });
    }
}
```

### ✅ 動作確認

```bash
cargo build
```

エラーがなければ成功です。まだツールを登録していないので、実際の動作確認はタスク9で行います。

---

## タスク3: ListFilesTool の再帰対応

### 🎯 目標

`recursive=true` の場合にサブディレクトリも含めてファイル一覧を取得できるようにする

### 📝 手順

#### 3.1 walkdir を使った再帰実装

`src/tools/list_files.rs` の `execute` メソッドを更新します。

以下の部分を置き換えてください：

**変更前：**
```rust
        // ファイル一覧を取得（今は非再帰のみ）
        let mut files = Vec::new();

        if args.recursive {
            // TODO: タスク3で実装
            return Ok(ToolResult {
                content: String::new(),
                error: Some("再帰モードはまだ実装されていません".to_string()),
            });
        } else {
```

**変更後：**
```rust
        // ファイル一覧を取得
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

                        files.push(FileInfo {
                            path: entry_path.display().to_string(),
                            is_dir: metadata.is_dir(),
                            size: metadata.len(),
                        });
                    }
                    Err(e) => {
                        warn!("Failed to read entry: {}", e);
                        continue;
                    }
                }
            }
        } else {
```

### 💡 Rust知識ポイント

#### 1. walkdir vs std::fs::read_dir の違い

| 機能 | `std::fs::read_dir` | `walkdir::WalkDir` |
|------|---------------------|-------------------|
| 探索範囲 | 1階層のみ | 再帰的（すべてのサブディレクトリ） |
| 順序 | 順不同 | デフォルトでソートされる |
| エラーハンドリング | シンプル | 柔軟（filter_entry等） |
| 性能 | 軽量 | やや重い（深い階層の場合） |

#### 2. イテレータの柔軟性

```rust
use walkdir::WalkDir;

// 基本
for entry in WalkDir::new(path) {
    let entry = entry?;
}

// 深さ制限
for entry in WalkDir::new(path).max_depth(2) {
    let entry = entry?;
}

// フィルタリング
for entry in WalkDir::new(path)
    .into_iter()
    .filter_entry(|e| !e.file_name().to_str().unwrap().starts_with('.'))
{
    let entry = entry?;
}
```

#### 3. 共通のパターン抽出

重複するコードを避けるため、エントリ処理を共通化できます：

```rust
// ヘルパー関数
fn process_entry(entry_path: &Path, metadata: &std::fs::Metadata) -> FileInfo {
    FileInfo {
        path: entry_path.display().to_string(),
        is_dir: metadata.is_dir(),
        size: metadata.len(),
    }
}

// 使用
files.push(process_entry(&entry_path, &metadata));
```

### ✅ 動作確認

```bash
cargo build
```

エラーがなければ成功です。

**実際の動作確認（タスク9の後）:**
```bash
cargo run -- "src ディレクトリの中身を教えて"
# → 非再帰モードで src/ 直下のファイルのみ表示

cargo run -- "プロジェクト全体のファイル一覧を教えて（再帰的に）"
# → 再帰モードですべてのファイルが表示
```

---

## タスク4: SearchInDirectoryTool の基本構造

### 🎯 目標

ファイル内容をキーワード検索するツールの骨格を作成する

### 📝 手順

#### 4.1 src/tools/search_in_directory.rs を作成

新しいファイル `src/tools/search_in_directory.rs` を作成し、以下のコードを記述してください：

```rust
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
        Ok(ToolResult {
            content: "検索機能はまだ実装されていません".to_string(),
            error: None,
        })
    }
}
```

#### 4.2 src/tools/mod.rs を更新

```rust
pub mod read_file;
pub mod list_files;
pub mod search_in_directory;  // 🆕 追加

pub use read_file::ReadFileTool;
pub use list_files::ListFilesTool;
pub use search_in_directory::SearchInDirectoryTool;  // 🆕 追加
```

### 💡 Rust知識ポイント

#### 1. 構造体フィールドのオプション化

```rust
// すべて必須
#[derive(Debug, Deserialize)]
struct SearchArgs {
    path: String,
    keyword: String,
}

// case_sensitive をオプションに
#[derive(Debug, Deserialize)]
struct SearchArgs {
    path: String,
    keyword: String,
    #[serde(default)]  // デフォルトは false
    case_sensitive: Option<bool>,
}
```

**本タスクでは：**
- シンプルさを優先して、常に大文字小文字を区別しない検索
- `case_sensitive` オプションは追加しない（Chapter 4で拡張可能）

#### 2. 検索結果の表現

```rust
// パターン1: シンプル（ファイルパスのみ）
#[derive(Debug, Serialize)]
struct SearchResult {
    files: Vec<String>,
}

// パターン2: 詳細（行番号と内容を含む）
#[derive(Debug, Serialize)]
struct SearchMatch {
    path: String,
    line_number: usize,
    line: String,
}
```

**本タスクでは：**
- パターン2を採用（より有用な情報を提供）

### ✅ 動作確認

```bash
cargo build
```

エラーがなければ成功です。

---

## タスク5: SearchInDirectoryTool の検索ロジック実装

### 🎯 目標

ファイル内容を実際にキーワード検索する機能を実装する

### 📝 手順

#### 5.1 execute メソッドの完成

`src/tools/search_in_directory.rs` の `execute` メソッドを更新します。

以下の部分を置き換えてください：

**変更前：**
```rust
        // TODO: タスク5で実装
        Ok(ToolResult {
            content: "検索機能はまだ実装されていません".to_string(),
            error: None,
        })
```

**変更後：**
```rust
        // 検索処理
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

            // ディレクトリはスキップ
            if entry.file_type().is_dir() {
                continue;
            }

            let file_path = entry.path();

            // ファイルを読み込み
            let content = match tokio::fs::read_to_string(file_path).await {
                Ok(c) => c,
                Err(_) => {
                    // バイナリファイルや権限エラーは静かにスキップ
                    debug!("Skipping file: {:?}", file_path);
                    continue;
                }
            };

            // 行単位で検索
            for (line_num, line) in content.lines().enumerate() {
                // 大文字小文字を区別しない検索
                if line.to_lowercase().contains(&keyword_lower) {
                    matches.push(SearchMatch {
                        path: file_path.display().to_string(),
                        line_number: line_num + 1,  // 1始まり
                        line: line.to_string(),
                    });
                }
            }
        }

        // 結果をJSON形式で返す
        let result_json = serde_json::to_string_pretty(&matches)
            .context("Failed to serialize search results")?;

        debug!("Found {} matches", matches.len());

        Ok(ToolResult {
            content: result_json,
            error: None,
        })
```

### 💡 Rust知識ポイント

#### 1. lines() イテレータ

```rust
let content = "line1\nline2\nline3";

// 各行を順に処理
for line in content.lines() {
    println!("{}", line);
}

// 行番号付き
for (line_num, line) in content.lines().enumerate() {
    println!("{}: {}", line_num + 1, line);  // 1始まりにする
}
```

#### 2. 大文字小文字を区別しない検索

```rust
let keyword = "ToolHandler";
let line = "impl ToolHandler for ReadFileTool {";

// ❌ 大文字小文字を区別する
if line.contains(keyword) {
    // マッチしない（"ToolHandler" != "toolhandler"）
}

// ✅ 大文字小文字を区別しない
if line.to_lowercase().contains(&keyword.to_lowercase()) {
    // マッチする
}
```

#### 3. バイナリファイルのスキップ

```rust
// read_to_string は UTF-8 でないファイルでエラーを返す
match tokio::fs::read_to_string(file_path).await {
    Ok(content) => {
        // テキストファイル → 検索処理
    }
    Err(_) => {
        // バイナリファイル、権限エラーなど → スキップ
        debug!("Skipping file: {:?}", file_path);
        continue;
    }
}
```

**この方法の利点：**
- 明示的なバイナリ判定が不要
- UTF-8 でないファイルを自動的にスキップ
- シンプルで堅牢

#### 4. 非同期処理との組み合わせ

```rust
// walkdir は同期イテレータ
for entry in WalkDir::new(path) {
    let entry = entry?;

    // tokio::fs は非同期関数
    let content = tokio::fs::read_to_string(entry.path()).await?;

    // 組み合わせ可能！
}
```

**注意：**
- `walkdir` 自体はブロッキングだが、ディレクトリ探索は高速なので問題ない
- ファイル読み込みは `tokio::fs` を使って非同期化

### ✅ 動作確認

```bash
cargo build
```

エラーがなければ成功です。

**実際の動作確認（タスク9の後）:**
```bash
cargo run -- "src ディレクトリで 'ToolHandler' という単語が使われているファイルを教えて"
# → マッチした行が表示される
```

---

## タスク6: WriteFileTool の基本構造

### 🎯 目標

ファイル書き込みツールの骨格を作成する

### 📝 手順

#### 6.1 src/tools/write_file.rs を作成

新しいファイル `src/tools/write_file.rs` を作成し、以下のコードを記述してください：

```rust
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::path::Path;
use tracing::{debug, warn};

use crate::anthropic::{Tool, ToolHandler, ToolResult};

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

        // TODO: タスク7-8で実装
        Ok(ToolResult {
            content: "ファイル書き込み機能はまだ実装されていません".to_string(),
            error: None,
        })
    }
}
```

#### 6.2 src/tools/mod.rs を更新

```rust
pub mod read_file;
pub mod list_files;
pub mod search_in_directory;
pub mod write_file;  // 🆕 追加

pub use read_file::ReadFileTool;
pub use list_files::ListFilesTool;
pub use search_in_directory::SearchInDirectoryTool;
pub use write_file::WriteFileTool;  // 🆕 追加
```

### 💡 Rust知識ポイント

#### 1. ファイル作成のAPI

```rust
use tokio::fs;

// シンプルな書き込み（ファイルが存在すれば上書き）
fs::write("test.txt", "Hello, World!").await?;

// ファイルを開いて書き込み（より柔軟）
use tokio::io::AsyncWriteExt;

let mut file = fs::File::create("test.txt").await?;
file.write_all(b"Hello, World!").await?;
```

#### 2. ディレクトリの作成

```rust
use tokio::fs;

// 単一ディレクトリ作成
fs::create_dir("new_dir").await?;

// 複数階層のディレクトリ作成
fs::create_dir_all("path/to/deep/dir").await?;  // 推奨
```

**違い：**
- `create_dir` - 親ディレクトリが存在しない場合はエラー
- `create_dir_all` - 必要なディレクトリをすべて作成（`mkdir -p` 相当）

#### 3. パスの親ディレクトリ取得

```rust
use std::path::Path;

let path = Path::new("src/tools/new_tool.rs");

// 親ディレクトリを取得
let parent = path.parent();  // Some("src/tools")

// 使用例
if let Some(parent) = path.parent() {
    tokio::fs::create_dir_all(parent).await?;
}
```

### ✅ 動作確認

```bash
cargo build
```

エラーがなければ成功です。

---

## タスク7: ユーザー確認機能の実装

### 🎯 目標

破壊的操作（ファイル作成・上書き）の前にユーザーの確認を取る機能を実装する

### 📝 手順

#### 7.1 ユーザー入力ヘルパー関数の追加

`src/tools/write_file.rs` の冒頭に以下のヘルパー関数を追加してください：

```rust
use std::io::{self, Write as IoWrite};

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
```

#### 7.2 execute メソッドの更新（一部）

`src/tools/write_file.rs` の `execute` メソッドを以下のように更新します：

**変更前：**
```rust
        let path = Path::new(&args.path);

        // TODO: タスク7-8で実装
        Ok(ToolResult {
            content: "ファイル書き込み機能はまだ実装されていません".to_string(),
            error: None,
        })
```

**変更後：**
```rust
        let path = Path::new(&args.path);

        // 既存ファイルの確認
        if path.exists() {
            warn!("File already exists: {}", args.path);

            // ユーザーに確認
            let message = format!("ファイル '{}' は既に存在します。上書きしますか？", args.path);
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

        // TODO: タスク8でファイル書き込みを実装
        Ok(ToolResult {
            content: format!("ファイル '{}' への書き込みが確認されました（実装予定）", args.path),
            error: None,
        })
```

### 💡 Rust知識ポイント

#### 1. stdout().flush() の必要性

```rust
use std::io::{self, Write};

// ❌ 悪い例：プロンプトがすぐに表示されない
print!("確認してください [y/N]: ");
let mut input = String::new();
io::stdin().read_line(&mut input)?;
// ユーザーが入力するまでプロンプトが表示されないことがある

// ✅ 良い例：flush() で即座に表示
print!("確認してください [y/N]: ");
io::stdout().flush()?;  // バッファをフラッシュ
let mut input = String::new();
io::stdin().read_line(&mut input)?;
```

**理由：**
- `print!` マクロは標準出力をバッファリングする
- `flush()` でバッファの内容を強制的に出力
- `println!` は自動的にフラッシュするが、`print!` はしない

#### 2. trim() で改行を除去

```rust
let mut input = String::new();
io::stdin().read_line(&mut input)?;

// read_line() は改行文字を含む
println!("入力: '{}'", input);  // "y\n"

// trim() で前後の空白・改行を除去
println!("入力: '{}'", input.trim());  // "y"
```

#### 3. 同期I/Oの使用

```rust
// ユーザー入力は同期I/Oを使う
use std::io;

fn prompt_user_confirmation(message: &str) -> Result<bool> {
    // std::io を使用（tokio::io ではない）
    io::stdin().read_line(&mut input)?;
    Ok(...)
}

// 非同期関数内でも使用可能
#[async_trait]
impl ToolHandler for WriteFileTool {
    async fn execute(&self, input: serde_json::Value) -> Result<ToolResult> {
        // 同期関数を呼び出し
        if !prompt_user_confirmation("確認")? {
            return Ok(...);
        }
        // ...
    }
}
```

**理由：**
- ユーザー入力は非同期化しにくい（待つしかない）
- `std::io` を使っても問題ない
- 非同期タスク内でブロッキングI/Oを呼ぶことは許容される

#### 4. セキュリティ：デフォルトは "No"

```rust
// ✅ 安全：明示的な 'y' のみ許可
if input.trim().to_lowercase() == "y" {
    // 実行
} else {
    // キャンセル
}

// ❌ 危険：デフォルトが "Yes"
if input.trim().to_lowercase() != "n" {
    // 実行（Enter だけでも実行される）
}
```

**設計原則：**
- 破壊的操作はデフォルトで拒否
- 明示的な同意のみを受け入れる

### ✅ 動作確認

```bash
cargo build
```

エラーがなければ成功です。

**実際の動作確認（タスク9の後）:**
```bash
cargo run -- "test.txt というファイルを作成して"
# → 確認プロンプトが表示される
# → 'y' を入力すると進む、他の文字だとキャンセル
```

---

## タスク8: WriteFileTool の完成

### 🎯 目標

実際にファイルを作成・書き込む機能を実装し、WriteFileToolを完成させる

### 📝 手順

#### 8.1 execute メソッドの完成

`src/tools/write_file.rs` の `execute` メソッドを完成させます。

以下の部分を置き換えてください：

**変更前：**
```rust
        // TODO: タスク8でファイル書き込みを実装
        Ok(ToolResult {
            content: format!("ファイル '{}' への書き込みが確認されました（実装予定）", args.path),
            error: None,
        })
```

**変更後：**
```rust
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
```

### 💡 Rust知識ポイント

#### 1. tokio::fs::create_dir_all の使い方

```rust
use tokio::fs;

// 例: "path/to/deep/dir/file.txt" を作成したい
let path = Path::new("path/to/deep/dir/file.txt");

// 親ディレクトリを取得
if let Some(parent) = path.parent() {
    // "path/to/deep/dir" を作成（中間ディレクトリも含む）
    fs::create_dir_all(parent).await?;
}

// ファイルを作成
fs::write(path, "content").await?;
```

**動作：**
- 既に存在するディレクトリに対しても成功（エラーにならない）
- 必要な中間ディレクトリをすべて作成
- Linux の `mkdir -p` と同じ動作

#### 2. path.parent() の扱い

```rust
use std::path::Path;

let path1 = Path::new("src/tools/new_file.rs");
path1.parent()  // Some("src/tools")

let path2 = Path::new("file.txt");
path2.parent()  // Some("")（カレントディレクトリ）

let path3 = Path::new("/");
path3.parent()  // None
```

**パターン：**
```rust
if let Some(parent) = path.parent() {
    if !parent.as_os_str().is_empty() {
        // 親ディレクトリが存在する場合のみ作成
        tokio::fs::create_dir_all(parent).await?;
    }
}
```

#### 3. tokio::fs::write の使い方

```rust
use tokio::fs;

// 文字列を書き込み
fs::write("test.txt", "Hello, World!").await?;

// バイト列を書き込み
fs::write("test.bin", b"\x00\x01\x02\x03").await?;

// String を書き込み
let content = String::from("Hello");
fs::write("test.txt", content).await?;

// &str を書き込み
let content = "Hello";
fs::write("test.txt", content).await?;
```

**特徴：**
- ファイルが存在すれば上書き
- ファイルが存在しなければ新規作成
- シンプルで使いやすい

#### 4. エラーハンドリングの最終形

```rust
// パターン：各ステップでToolResultとしてエラーを返す
match tokio::fs::create_dir_all(parent).await {
    Ok(_) => { /* 成功 */ }
    Err(e) => {
        return Ok(ToolResult {
            content: String::new(),
            error: Some(format!("エラー: {}", e)),
        });
    }
}

match tokio::fs::write(&path, &content).await {
    Ok(_) => {
        Ok(ToolResult {
            content: "成功".to_string(),
            error: None,
        })
    }
    Err(e) => {
        Ok(ToolResult {
            content: String::new(),
            error: Some(format!("エラー: {}", e)),
        })
    }
}
```

### ✅ 動作確認

```bash
cargo build
```

エラーがなければ成功です。

**実際の動作確認（タスク9の後）:**
```bash
# テスト1: 新規ファイル作成
cargo run -- "test.txt というファイルを作成して、内容は 'Hello, World!' にしてください"
# → 確認プロンプト表示 → 'y' 入力 → ファイル作成

# テスト2: 深い階層のファイル作成
cargo run -- "test/deep/dir/example.md というファイルを作成してください"
# → 親ディレクトリも自動作成

# テスト3: 既存ファイルの上書き
cargo run -- "test.txt を上書きしてください"
# → 上書き確認プロンプト表示
```

---

## タスク9: 3つのツールをToolRegistryに登録

### 🎯 目標

main.rs で3つの新しいツールを登録し、すべて利用可能にする

### 📝 手順

#### 9.1 main.rs を更新

`src/main.rs` を開き、以下の変更を行います：

**変更箇所1: use 宣言**

```rust
use tools::{ReadFileTool, ListFilesTool, SearchInDirectoryTool, WriteFileTool};  // 🆕 追加
```

**変更箇所2: ツール登録**

```rust
    // ツールを登録
    let mut tool_registry = ToolRegistry::new();
    tool_registry.register(ReadFileTool::schema(), ReadFileTool::new());
    tool_registry.register(ListFilesTool::schema(), ListFilesTool::new());  // 🆕 追加
    tool_registry.register(SearchInDirectoryTool::schema(), SearchInDirectoryTool::new());  // 🆕 追加
    tool_registry.register(WriteFileTool::schema(), WriteFileTool::new());  // 🆕 追加

    tracing::info!(
        "Registered tools: readFile, listFiles, searchInDirectory, writeFile"  // 🆕 更新
    );
```

### 💡 Rust知識ポイント

#### 1. モジュールシステム

```rust
// src/tools/mod.rs で公開
pub use read_file::ReadFileTool;
pub use list_files::ListFilesTool;
pub use search_in_directory::SearchInDirectoryTool;
pub use write_file::WriteFileTool;

// src/main.rs でインポート
use tools::{ReadFileTool, ListFilesTool, SearchInDirectoryTool, WriteFileTool};
```

**省略記法：**
```rust
// 個別にインポート
use tools::ReadFileTool;
use tools::ListFilesTool;
// ...

// まとめてインポート（推奨）
use tools::{ReadFileTool, ListFilesTool, SearchInDirectoryTool, WriteFileTool};
```

#### 2. ToolRegistry の動作

```rust
let mut registry = ToolRegistry::new();

// ツールを登録
registry.register(
    ReadFileTool::schema(),    // Tool 構造体（スキーマ情報）
    ReadFileTool::new()        // ToolHandler trait を実装した構造体
);

// LLMにスキーマを渡す
let schemas = registry.get_schemas();

// ツールを実行
let result = registry.execute("readFile", input).await?;
```

### ✅ 動作確認

```bash
cargo build
cargo run
```

**実際のテスト：**

```bash
# テスト1: listFiles
cargo run
> src ディレクトリの中身を教えて
# → ファイル一覧が表示される

# テスト2: searchInDirectory
> src ディレクトリで 'ToolHandler' という文字列を探して
# → マッチした行が表示される

# テスト3: writeFile
> test.txt というファイルを作成して、内容は 'Hello from Claude!' にして
# → 確認プロンプト → 'y' → ファイル作成

# テスト4: 複合タスク
> src ディレクトリにある Rust ファイルを探して、その中から 'async' という単語を検索してください
# → listFiles → searchInDirectory の順で実行される
```

---

## タスク10: 統合テストとコード品質チェック

### 🎯 目標

すべてのツールが正しく動作し、コード品質が保たれていることを確認する

### 📝 手順

#### 10.1 実際のユースケースでテスト

以下のテストケースを順番に実行してください：

**テストケース1: ファイル探索（非再帰）**
```bash
cargo run
> カレントディレクトリの中身を教えて
```

**期待される動作：**
- `listFiles` ツールが呼ばれる
- カレントディレクトリ直下のファイル・ディレクトリが表示される

**テストケース2: ファイル探索（再帰）**
```bash
> プロジェクト全体のファイルをすべて教えて（再帰的に）
```

**期待される動作：**
- `listFiles` ツールが `recursive: true` で呼ばれる
- すべてのサブディレクトリを含むファイル一覧が表示される

**テストケース3: キーワード検索**
```bash
> src ディレクトリで 'async' という単語が使われているファイルを教えて
```

**期待される動作：**
- `searchInDirectory` ツールが呼ばれる
- マッチした行番号と内容が表示される

**テストケース4: ファイル作成（新規）**
```bash
> hello.txt というファイルを作成して、内容は 'Hello, World!' にしてください
```

**期待される動作：**
- `writeFile` ツールが呼ばれる
- ユーザー確認プロンプトが表示される
- 'y' を入力するとファイルが作成される

**テストケース5: ファイル作成（深い階層）**
```bash
> test/deep/directory/example.md というファイルを作成してください
```

**期待される動作：**
- 親ディレクトリが自動的に作成される
- ファイルが正常に作成される

**テストケース6: 複合タスク**
```bash
> Cargo.toml を読んで、依存クレートの一覧を summary.txt に書き出してください
```

**期待される動作：**
- `readFile` → `writeFile` の順で実行される
- summary.txt が作成される

#### 10.2 コード品質チェック

##### 10.2.1 フォーマットチェック

```bash
cargo fmt
```

**確認：**
- コードが自動的にフォーマットされる
- 一貫したスタイルが保たれる

##### 10.2.2 Linter実行

```bash
cargo clippy -- -D warnings
```

**確認：**
- 警告がゼロであることを確認
- もし警告がある場合は修正する

**よくある警告と修正：**

```rust
// 警告: needless_return
fn example() -> i32 {
    return 42;  // ❌
}

// 修正
fn example() -> i32 {
    42  // ✅
}

// 警告: unused_variable
let unused = 42;  // ❌

// 修正
let _unused = 42;  // ✅（明示的に未使用を示す）

// 警告: single_match
match value {
    Some(x) => println!("{}", x),
    _ => {}
}  // ❌

// 修正
if let Some(x) = value {
    println!("{}", x);
}  // ✅
```

##### 10.2.3 ビルド（リリースモード）

```bash
cargo build --release
```

**確認：**
- エラーなくビルドが完了
- 最適化されたバイナリが生成される

#### 10.3 セキュリティ検証

##### 10.3.1 ディレクトリトラバーサル攻撃のテスト

```bash
cargo run
> ../../../etc/passwd を読んでください
```

**期待される動作：**
- 現時点では読めてしまう（Chapter 3では基本機能のみ）
- Chapter 4でパス検証を追加予定

##### 10.3.2 バイナリファイルのスキップ確認

```bash
# バイナリファイルを含むディレクトリで検索
> target ディレクトリで 'main' を検索してください
```

**期待される動作：**
- バイナリファイルは自動的にスキップされる
- テキストファイルのみ検索される

#### 10.4 ログ確認

```bash
RUST_LOG=debug cargo run
```

**確認するログ：**
- ツール実行のログが出力される
- エラーや警告が適切にログされる

**期待されるログ例：**
```
DEBUG executing listFiles tool with input: {"path": "src"}
DEBUG listing files in: src (recursive: false)
DEBUG found 5 files/directories
```

### ✅ 最終確認チェックリスト

以下の項目をすべて確認してください：

- [ ] **listFiles が動作する（非再帰）**
  ```bash
  cargo run -- "src ディレクトリの中身を教えて"
  ```

- [ ] **listFiles が動作する（再帰）**
  ```bash
  cargo run -- "プロジェクト全体のファイル一覧を教えて"
  ```

- [ ] **searchInDirectory が動作する**
  ```bash
  cargo run -- "src で 'ToolHandler' を検索して"
  ```

- [ ] **writeFile が動作する（新規ファイル）**
  ```bash
  cargo run -- "test.txt を作成して"
  ```

- [ ] **writeFile が動作する（確認プロンプト）**
  - ユーザー確認プロンプトが表示される
  - 'y' を入力すると実行される
  - 'n' や Enter を入力するとキャンセルされる

- [ ] **writeFile が動作する（ディレクトリ自動作成）**
  ```bash
  cargo run -- "deep/nested/dir/file.txt を作成して"
  ```

- [ ] **複数ツールの組み合わせが動作する**
  ```bash
  cargo run -- "Cargo.toml を読んで summary.txt に書いて"
  ```

- [ ] **cargo fmt が警告なく完了する**
  ```bash
  cargo fmt --check
  ```

- [ ] **cargo clippy が警告なく完了する**
  ```bash
  cargo clippy -- -D warnings
  ```

- [ ] **cargo build が成功する**
  ```bash
  cargo build
  ```

- [ ] **cargo build --release が成功する**
  ```bash
  cargo build --release
  ```

- [ ] **ログが適切に出力される**
  ```bash
  RUST_LOG=debug cargo run
  ```

### 🐛 トラブルシューティング

#### 問題1: "ディレクトリが見つかりません" エラー

**症状：**
```
error: Some("ディレクトリが見つかりません: src")
```

**原因：**
- カレントディレクトリが間違っている
- 相対パスの解釈が異なる

**解決策：**
```bash
# カレントディレクトリを確認
pwd

# プロジェクトルートに移動
cd /path/to/coding-agent-example

# 再実行
cargo run
```

#### 問題2: "ユーザー入力の読み取りに失敗しました" エラー

**症状：**
- 確認プロンプトでエラーが発生

**原因：**
- 標準入力がリダイレクトされている
- パイプ経由で実行している

**解決策：**
```bash
# ❌ パイプ経由（失敗）
echo "yes" | cargo run

# ✅ 通常の実行（成功）
cargo run
```

#### 問題3: cargo clippy で警告が出る

**症状：**
```
warning: unused import: `Context`
```

**解決策：**
```rust
// 未使用のインポートを削除
// use anyhow::Context;  // ❌ 削除

// または使用する
let result = something().context("error message")?;  // ✅
```

#### 問題4: バイナリファイルで検索が遅い

**症状：**
- `searchInDirectory` が大きなバイナリファイルで遅い

**解決策（Chapter 4で実装予定）：**
```rust
// ファイルサイズ制限を追加
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;  // 10MB

let metadata = tokio::fs::metadata(&file_path).await?;
if metadata.len() > MAX_FILE_SIZE {
    debug!("Skipping large file: {:?}", file_path);
    continue;
}
```

---

## 🎉 完成！

おめでとうございます！Chapter 3 のすべてのタスクを完了しました。

### 達成できたこと

✅ **3つの新しいツールの実装**
- `listFiles` - ディレクトリ探索（再帰/非再帰対応）
- `searchInDirectory` - キーワード検索
- `writeFile` - ファイル作成（ユーザー確認付き）

✅ **Rustスキルの習得**
- `walkdir` クレートの使用
- 非同期ファイルI/O（`tokio::fs`）
- ユーザー入力の処理（`std::io`）
- PathBuf と Path の使い分け

✅ **セキュリティ意識**
- ユーザー確認による破壊的操作の防止
- バイナリファイルの自動スキップ
- エラーハンドリングの徹底

✅ **コード品質**
- フォーマット済み（cargo fmt）
- Linter警告ゼロ（cargo clippy）
- 適切なログ出力（tracing）

### この時点でのエージェントの能力

```
1. 読む
   - readFile: ファイル内容の取得

2. 探す
   - listFiles: ディレクトリ構造の把握
   - searchInDirectory: キーワード検索

3. 書く
   - writeFile: 新規ファイルの作成
```

### 次のステップ（Chapter 4）

Chapter 4 では、さらに高度な機能を実装します：

#### 4.1 editFile ツール（最重要）
- 既存ファイルの部分編集
- 行範囲指定による置換
- 差分表示とユーザー確認

#### 4.2 セキュリティ強化
- パス検証（`canonicalize()`）
- ファイルサイズ制限
- ホワイトリスト方式の実装

#### 4.3 UX改善
- `dialoguer` クレートの導入
- カラフルなターミナル出力
- プログレスバーの表示

#### 4.4 テストの充実化
- ユニットテスト
- 統合テスト
- エッジケースのカバレッジ

### 学習リソース

**次に学ぶべきこと：**
- [Rust Async Book](https://rust-lang.github.io/async-book/) - 非同期プログラミングの深掘り
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/) - 設計パターン
- [The rustup book](https://rust-lang.github.io/rustup/) - Rustツールチェーン

**参考プロジェクト：**
- `codex.md` - より高度なTUI実装
- `docs/ch2/ch2_tool_use.md` - Tool Useの基礎

---

これでChapter 3の実装ガイドは完了です。各タスクを順番に進めることで、ファイルシステムツールの実装とRustプログラミングのスキルを習得できたはずです。

次は実際にコードを書いて、動くコーディングエージェントを完成させてください！
