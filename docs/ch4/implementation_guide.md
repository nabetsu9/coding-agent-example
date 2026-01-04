# editFile ツール 実装ガイド（Rust初心者向け）

このガイドでは、既存ファイルを安全に編集する `editFile` ツールを、タスクごとに分割して段階的に実装していきます。各タスクで動作確認をしながら進めることで、確実に理解を深められます。

## 📋 全体の流れ

```
タスク1: editFile の要件を理解する
  ↓
タスク2: edit_file.rs モジュールを作成
  ↓ 動作確認: cargo build が成功
  ↓
タスク3: EditFileArgs 構造体を定義
  ↓ 動作確認: cargo build が成功
  ↓
タスク4: EditFileTool 構造体とスキーマを作成
  ↓ 動作確認: cargo build が成功
  ↓
タスク5: ファイル存在チェックのロジックを実装
  ↓ 動作確認: cargo build が成功
  ↓
タスク6: ユーザー確認機能を実装
  ↓ 動作確認: cargo build が成功
  ↓
タスク7: ToolHandler trait を実装
  ↓ 動作確認: cargo build が成功
  ↓
タスク8: ToolRegistry に登録
  ↓ 動作確認: ツール一覧に表示
  ↓
タスク9: 基本的な編集をテスト
  ↓ 動作確認: ファイル編集が成功
  ↓
タスク10: Read-Modify-Write パターンのテスト
  ✓ 完成！
```

---

## タスク1: editFile の要件を理解する

### 🎯 目標
editFile ツールが何をするのか、なぜそのように設計されているのかを理解する

### 📝 手順

#### 1.1 editFile vs writeFile の違いを確認

以下の表を参照し、役割の違いを理解してください。

| 機能 | writeFile | editFile |
|------|-----------|----------|
| 用途 | 新規ファイル作成 | 既存ファイル編集 |
| 前提条件 | ファイルが存在しない | ファイルが**存在する** |
| エラー時 | 既存ファイルがあれば保護 | ファイルがなければエラー |

**重要：** この違いにより、誤操作を防ぎます。

#### 1.2 Read-Modify-Write パターンの理解

editFile は単独では機能しません。以下のワークフローで使用されます：

```
1. readFile で現在の内容を読む
   ↓
2. LLM がメモリ内で変更を加える
   ↓
3. editFile で完全な新しい内容を書き込む
```

**例：**
```
ユーザー: "config.tomlのmax_tokensを2048に変更して"
  ↓
Agent: readFile("config.toml")
  ↓
Agent: （メモリ内で変更）
  ↓
Agent: editFile("config.toml", "model = ...\nmax_tokens = 2048\n")
```

#### 1.3 全体置換 vs 部分編集の理解

**部分編集（採用しない理由）：**
```rust
// 行番号指定での置換
edit_lines(path, start_line, end_line, new_content)
```

- ❌ 構文エラーを引き起こしやすい
- ❌ インデントの不整合
- ❌ 実装が複雑

**全体置換（本プロジェクトで採用）：**
```rust
// ファイル全体を完全に置き換え
edit_file(path, complete_new_content)
```

- ✅ 安全（構文エラーのリスクが低い）
- ✅ シンプルな実装
- ✅ LLM に適している（全体のコンテキストを理解）

### 💡 Rust知識ポイント

このタスクは概念的な理解のため、Rustの新しい知識は特にありません。

### ✅ 確認事項

- [ ] writeFile と editFile の違いを理解した
- [ ] Read-Modify-Write パターンを理解した
- [ ] 全体置換を採用する理由を理解した

---

## タスク2: edit_file.rs モジュールを作成

### 🎯 目標
新しいツールモジュール用のファイルを作成し、プロジェクトに組み込む

### 📝 手順

#### 2.1 新しいファイルを作成

`src/tools/edit_file.rs` という新しいファイルを作成します。

```bash
touch src/tools/edit_file.rs
```

#### 2.2 基本的なインポートを記述

`src/tools/edit_file.rs` を開いて、以下を記述してください：

```rust
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{debug, warn};

use crate::anthropic::{Tool, ToolHandler, ToolResult};
```

#### 2.3 モジュールシステムに登録

`src/tools/mod.rs` を開いて、以下を **追加** してください：

```rust
mod read_file;
mod write_file;
mod list_files;
mod search_in_directory;
mod edit_file;  // 新しく追加

pub use read_file::ReadFileTool;
pub use write_file::WriteFileTool;
pub use list_files::ListFilesTool;
pub use search_in_directory::SearchInDirectoryTool;
pub use edit_file::EditFileTool;  // 新しく追加
```

### 💡 Rust知識ポイント

**1. モジュールシステム**

```rust
mod edit_file;  // src/tools/edit_file.rs を読み込む
```

- `mod` 宣言は、別のファイルをモジュールとして読み込みます
- ファイル名とモジュール名は一致させる必要があります
- `edit_file.rs` → `mod edit_file;`

**2. pub use による再エクスポート**

```rust
pub use edit_file::EditFileTool;
```

- `use` だけでは、このモジュール内でしか使えません
- `pub use` により、外部（main.rs など）から `tools::EditFileTool` としてアクセス可能になります

**Go言語との違い：**

| Go | Rust |
|----|------|
| パッケージ名がディレクトリ名 | モジュール名がファイル名 |
| `import "nebula/tools"` | `mod edit_file;` + `pub use` |
| 自動的にエクスポート | `pub` キーワードが必要 |

### ✅ 動作確認

```bash
cargo build
```

**期待される結果:**
```
   Compiling coding-agent-example v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 1.23s
```

エラーなくビルドが成功すればOKです（警告は無視してOK）。

---

## タスク3: EditFileArgs 構造体を定義

### 🎯 目標
ツールの入力パラメータを表す構造体を作成する

### 📝 手順

#### 3.1 EditFileArgs 構造体を定義

`src/tools/edit_file.rs` に以下を追加してください：

```rust
/// editFile ツールの引数
#[derive(Debug, Deserialize)]
pub struct EditFileArgs {
    /// 編集する既存ファイルのパス
    pub path: String,
    /// ファイル全体を上書きする新しい完全な内容
    pub new_content: String,
}
```

### 💡 Rust知識ポイント

**1. `#[derive(Debug, Deserialize)]`**

```rust
#[derive(Debug, Deserialize)]
pub struct EditFileArgs { ... }
```

- `Debug`: デバッグ出力が可能になります（`println!("{:?}", args)` など）
- `Deserialize`: JSONから自動的にこの構造体に変換できるようになります

**2. ドキュメントコメント `///`**

```rust
/// editFile ツールの引数
pub struct EditFileArgs { ... }
```

- `//` は通常のコメント
- `///` はドキュメントコメント（`cargo doc` で生成されるドキュメントに含まれる）

**3. 構造体フィールドの可視性**

```rust
pub path: String,  // pub をつけると外部から読める
```

- `pub` がないと、モジュール外からアクセスできません
- 今回は `ToolHandler` の実装で使うため `pub` が必要です

**JSON からの自動変換の仕組み：**

入力JSON:
```json
{
  "path": "src/main.rs",
  "new_content": "fn main() { ... }"
}
```

自動的に以下に変換されます:
```rust
EditFileArgs {
    path: "src/main.rs".to_string(),
    new_content: "fn main() { ... }".to_string(),
}
```

### ✅ 動作確認

```bash
cargo build
```

エラーがなければ成功です。

---

## タスク4: EditFileTool 構造体とスキーマを作成

### 🎯 目標
ツール構造体を定義し、Claude APIに送信するJSON Schemaを作成する

### 📝 手順

#### 4.1 EditFileTool 構造体を定義

`src/tools/edit_file.rs` に以下を追加してください：

```rust
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
```

### 💡 Rust知識ポイント

**1. ユニット構造体（Unit Struct）**

```rust
pub struct EditFileTool;  // フィールドがない構造体
```

- データを持たない構造体
- メソッドの入れ物として使われる
- メモリをほぼ消費しない（ゼロサイズ型）

**通常の構造体との違い：**

```rust
// 通常の構造体（データを持つ）
struct Person {
    name: String,
    age: u32,
}

// ユニット構造体（データを持たない）
struct EditFileTool;
```

**2. 関連関数（Associated Function）**

```rust
impl EditFileTool {
    pub fn new() -> Self { ... }      // 関連関数
    pub fn schema() -> Tool { ... }   // 関連関数
}
```

- `self` を受け取らない関数
- `EditFileTool::new()` のように呼び出す
- コンストラクタパターンとして使われる

**メソッドとの違い：**

```rust
impl EditFileTool {
    // 関連関数（self なし）
    pub fn new() -> Self { Self }

    // メソッド（&self あり）
    pub fn execute(&self, input: Value) -> Result<ToolResult> { ... }
}

// 呼び出し方
let tool = EditFileTool::new();  // 関連関数
tool.execute(input);             // メソッド
```

**3. `json!` マクロ**

```rust
use serde_json::json;

let schema = json!({
    "type": "object",
    "properties": { ... }
});
```

- JSON構造を直接Rustコードとして書ける
- コンパイル時に型チェックされる
- `serde_json::Value` 型が生成される

**4. 文字列の複数行表記**

```rust
"既存ファイルの内容を完全に上書きします。\
 重要: ファイルを破壊しないために..."
```

- `\` で改行を無視して連結
- 読みやすくするための記法

**5. `description` フィールドの重要性**

この `description` は、LLMがツールを正しく使用するための最も重要な情報源です。

**良い description の要素：**
1. **What（何をする）** - 完全に上書きする
2. **How（どう使う）** - Read-Modify-Writeパターン
3. **Why（なぜ）** - ファイルを破壊しないため
4. **Constraints（制約）** - 部分的な編集は不可
5. **Safety（安全性）** - ユーザー許可が必要

### ✅ 動作確認

```bash
cargo build
```

エラーがなければ成功です。

---

## タスク5: ファイル存在チェックのロジックを実装

### 🎯 目標
ファイルが存在するかを確認し、存在しない場合は適切なエラーを返すロジックを実装する

### 📝 手順

#### 5.1 ヘルパー関数を作成

`src/tools/edit_file.rs` に以下を追加してください：

```rust
use std::path::Path;

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
            return Err(
                format!("{} はファイルではありません。", path)
            );
        }

        Ok(())
    }
}
```

### 💡 Rust知識ポイント

**1. `std::path::Path`**

```rust
use std::path::Path;

let path = Path::new("src/main.rs");  // &Path 型
```

- ファイルパスを扱うための標準ライブラリの型
- `&str` や `String` から作成できる
- クロスプラットフォーム対応（Windows/Linux/macOS）

**よく使うメソッド：**

```rust
let path = Path::new("src/main.rs");

path.exists()    // ファイル/ディレクトリが存在するか
path.is_file()   // ファイルか
path.is_dir()    // ディレクトリか
path.is_absolute() // 絶対パスか
path.extension() // 拡張子を取得（Option<&OsStr>）
```

**2. `Result<(), String>` 型**

```rust
fn check_file_exists(path: &str) -> Result<(), String>
```

- `Result<T, E>` の特殊なケース
- `T = ()` → 成功時に値を返さない
- `E = String` → エラー時はエラーメッセージを返す

**使い方：**

```rust
match EditFileTool::check_file_exists(&path) {
    Ok(()) => { /* ファイルが存在する */ }
    Err(error_msg) => { /* error_msg にエラーメッセージ */ }
}
```

**3. プライベートメソッド**

```rust
fn check_file_exists(...) -> ...  // pub がない
```

- `pub` をつけないと、モジュール内でのみ使用可能
- ヘルパー関数として内部実装に使う
- 外部には公開しない

**4. `format!` マクロ**

```rust
format!("{} はファイルではありません。", path)
```

- 文字列を整形して `String` を作成
- `println!` と似ているが、表示ではなく文字列を返す
- `{}` にはDebugやDisplayトレイトを実装した値を埋め込める

**5. 早期リターンパターン**

```rust
if !condition {
    return Err(...);  // エラーの場合は即座に返す
}

Ok(())  // 全てのチェックが通ったら成功
```

- ガード節（Guard Clause）とも呼ばれる
- ネストを浅くして読みやすくする

### ✅ 動作確認

```bash
cargo build
```

警告が出るかもしれませんが（未使用の関数など）、エラーがなければOKです。

---

## タスク6: ユーザー確認機能を実装

### 🎯 目標
ファイル編集前にユーザーに確認を求める機能を実装する

### 📝 手順

#### 6.1 ユーザー確認の関数を実装

`src/tools/edit_file.rs` に以下を追加してください：

```rust
use std::io::{self, Write};

impl EditFileTool {
    // ... 既存のメソッド（new, schema, check_file_exists）...

    /// ユーザーに確認を求める
    fn prompt_user_confirmation(path: &str) -> Result<bool> {
        // 1. プロンプトを表示
        print!("\n既存ファイルを編集します: {}\n実行してもよろしいですか？ [y/N]: ", path);

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
```

### 💡 Rust知識ポイント

**1. `std::io::Write` トレイト**

```rust
use std::io::{self, Write};

io::stdout().flush()?;
```

- `io::stdout()` は標準出力を表すハンドル
- `flush()` メソッドでバッファの内容を即座に表示
- `Write` トレイトをインポートしないと `flush()` メソッドが使えない

**なぜ `flush()` が必要か：**

```rust
// flush() なし
print!("入力してください: ");
// バッファリングされて表示されないかもしれない
io::stdin().read_line(&mut input)?;

// flush() あり
print!("入力してください: ");
io::stdout().flush()?;  // 即座に表示される
io::stdin().read_line(&mut input)?;
```

**2. `print!` vs `println!`**

```rust
print!("改行なし");    // 改行なし
println!("改行あり");  // 自動的に改行
```

- `print!` は改行を追加しない
- プロンプト表示には `print!` を使う（ユーザー入力と同じ行に）

**3. `stdin().read_line()`**

```rust
let mut input = String::new();
io::stdin().read_line(&mut input)?;
```

- ユーザーの入力を `input` に読み込む
- **改行文字も含まれる** ため、`trim()` が必要
- `mut` が必要（変更可能な変数）

**4. `.trim().to_lowercase()`**

```rust
let confirmed = input.trim().to_lowercase() == "y";
```

- `trim()` - 前後の空白・改行を除去
  ```rust
  "  hello\n".trim()  // → "hello"
  ```

- `to_lowercase()` - 小文字に変換
  ```rust
  "Hello".to_lowercase()  // → "hello"
  ```

- メソッドチェーン - 連続して呼び出せる

**5. デフォルト値の設計**

```rust
if input.trim().to_lowercase() == "y" {
    // 明示的な "y" のみ許可
} else {
    // それ以外は全て "No"
}
```

**セキュリティの観点：**
- デフォルトは「安全側」（No）
- 明示的な同意（"y"）がない限り実行しない
- これにより誤操作を防ぐ

**6. エラーハンドリングの `.context()`**

```rust
io::stdout()
    .flush()
    .context("標準出力のフラッシュに失敗しました")?;
```

- エラーが発生した場合、説明文を追加
- どこで失敗したかが分かりやすくなる

**writeFile との比較：**

`src/tools/write_file.rs` にも同様のユーザー確認機能があります。パターンを統一することで保守性が向上します。

### ✅ 動作確認

```bash
cargo build
```

警告は無視してOKです。エラーがなければ成功です。

---

## タスク7: ToolHandler trait を実装

### 🎯 目標
`ToolHandler` trait を実装して、editFile ツールを実行可能にする

### 📝 手順

#### 7.1 ToolHandler trait を実装

`src/tools/edit_file.rs` に以下を追加してください：

```rust
use tokio::fs;

#[async_trait]
impl ToolHandler for EditFileTool {
    async fn execute(&self, input: Value) -> Result<ToolResult> {
        debug!("Executing editFile tool");

        // 1. 入力をパース
        let args: EditFileArgs = serde_json::from_value(input)
            .context("editFile: 引数のパースに失敗しました")?;

        debug!("editFile args: path={}, content_length={}", args.path, args.new_content.len());

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
```

### 💡 Rust知識ポイント

**1. `#[async_trait]` マクロ**

```rust
#[async_trait]
impl ToolHandler for EditFileTool {
    async fn execute(&self, input: Value) -> Result<ToolResult> { ... }
}
```

- `async fn` を trait で使うために必要
- `async_trait` クレートが提供
- Rustの制限を回避するためのマクロ

**2. `serde_json::from_value()`**

```rust
let args: EditFileArgs = serde_json::from_value(input)?;
```

- `serde_json::Value` から `EditFileArgs` 構造体に変換
- 型を明示すると自動的に変換される
- 変換に失敗すると `Err` を返す

**入力の流れ：**
```
JSON文字列 → serde_json::Value → EditFileArgs構造体
{"path": "..."} → Value::Object(...) → EditFileArgs { path: "..." }
```

**3. `if let` によるパターンマッチ**

```rust
if let Err(error_msg) = Self::check_file_exists(&args.path) {
    // エラーの場合のみ実行
    return Ok(ToolResult { ... });
}
```

- `Result<(), String>` の `Err` のみを処理
- `Ok` の場合は何もせず続行

**通常の `match` との比較：**

```rust
// if let（エラーのみ処理）
if let Err(error_msg) = result {
    // エラー処理
}

// match（両方処理）
match result {
    Ok(()) => { /* 成功時 */ }
    Err(error_msg) => { /* エラー時 */ }
}
```

**4. `match` による詳細なエラーハンドリング**

```rust
match Self::prompt_user_confirmation(&args.path) {
    Ok(true) => { /* ユーザーがYes */ }
    Ok(false) => { /* ユーザーがNo */ }
    Err(e) => { /* エラー */ }
}
```

- `Result<bool, Error>` の3つのケースを処理
- `Ok(true)` と `Ok(false)` を区別

**5. `tokio::fs::write()`**

```rust
use tokio::fs;

fs::write(&args.path, &args.new_content).await?;
```

- 非同期でファイルに書き込み
- ファイルを完全に上書き（既存の内容は削除される）
- `.await` で完了を待つ

**`std::fs::write` との違い：**

| `std::fs::write` | `tokio::fs::write` |
|-----------------|-------------------|
| 同期（ブロッキング） | 非同期（ノンブロッキング） |
| `.await` 不要 | `.await` 必須 |
| 他の処理をブロック | 他の処理を継続可能 |

**6. ログレベルの使い分け**

```rust
debug!("editFile args: ...");   // デバッグ情報
warn!("editFile: ユーザーによってキャンセル");  // 警告
```

| レベル | 用途 | 例 |
|-------|------|-----|
| `debug!` | デバッグ情報 | 引数の内容、実行ステップ |
| `info!` | 通常の情報 | 正常な処理の完了 |
| `warn!` | 警告 | キャンセル、失敗 |
| `error!` | エラー | 致命的な問題 |

**7. エラーを `ToolResult` として返す重要性**

```rust
// ✅ GOOD: エラーを ToolResult として返す
match fs::write(...).await {
    Ok(_) => Ok(ToolResult { content: "成功", error: None }),
    Err(e) => Ok(ToolResult { content: "", error: Some(...) }),
}

// ❌ BAD: エラーを伝播させる（agentic loopが停止）
fs::write(...).await?;  // エラー時に関数が終了
Ok(ToolResult { ... })
```

**なぜこの違いが重要か：**
- LLMはエラーメッセージを読んで適切に対応できる
- Agentic loop が途中で停止しない
- ユーザーにとって分かりやすいエラーメッセージ

### ✅ 動作確認

```bash
cargo build
```

エラーがなければ成功です！

---

## タスク8: ToolRegistry に登録

### 🎯 目標
editFile ツールをエージェントが使えるようにToolRegistryに登録する

### 📝 手順

#### 8.1 main.rs でインポート

`src/main.rs` の冒頭付近に、以下が既にあることを確認してください：

```rust
use crate::tools::{
    EditFileTool,          // 新しく追加
    ListFilesTool,
    ReadFileTool,
    SearchInDirectoryTool,
    WriteFileTool,
};
```

もし `EditFileTool` がない場合は追加してください。

#### 8.2 ToolRegistry に登録

`src/main.rs` の `main` 関数内で、ツール登録部分を見つけて `EditFileTool` を追加してください：

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // ... ロギング初期化など ...

    // ツールレジストリの作成
    let mut tool_registry = ToolRegistry::new();

    // 既存のツール登録
    tool_registry.register(ReadFileTool::schema(), ReadFileTool::new());
    tool_registry.register(WriteFileTool::schema(), WriteFileTool::new());
    tool_registry.register(ListFilesTool::schema(), ListFilesTool::new());
    tool_registry.register(SearchInDirectoryTool::schema(), SearchInDirectoryTool::new());

    // editFile ツールを登録（新しく追加）
    tool_registry.register(EditFileTool::schema(), EditFileTool::new());

    // ... 残りのコード ...
}
```

#### 8.3 利用可能ツールの表示を更新

`main.rs` で利用可能なツールを表示している箇所を見つけて、`editFile` を追加してください：

```rust
println!("Available tools: readFile, writeFile, listFiles, searchInDirectory, editFile");
```

または、動的に表示している場合はそのままでOKです。

### 💡 Rust知識ポイント

**1. `use` 文の整理**

```rust
use crate::tools::{
    EditFileTool,
    ListFilesTool,
    ReadFileTool,
    SearchInDirectoryTool,
    WriteFileTool,
};
```

- 複数の型を1つの `use` 文でインポート
- 中括弧 `{}` で囲む
- アルファベット順に並べると読みやすい

**2. `register()` メソッドの呼び出し**

```rust
tool_registry.register(
    EditFileTool::schema(),  // スキーマ（Claude APIに送信）
    EditFileTool::new()      // ハンドラ（実行ロジック）
);
```

- 第一引数: `Tool` 構造体（JSON Schema）
- 第二引数: `ToolHandler` を実装した型のインスタンス

**内部で何が起こっているか：**

```rust
// ToolRegistry の内部（参考）
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn ToolHandler>>,  // 名前 → ハンドラ
    schemas: Vec<Tool>,                            // スキーマのリスト
}

impl ToolRegistry {
    pub fn register<T: ToolHandler + 'static>(
        &mut self,
        schema: Tool,
        handler: T
    ) {
        self.schemas.push(schema.clone());
        self.tools.insert(schema.name.clone(), Box::new(handler));
    }
}
```

### ✅ 動作確認

#### ビルド確認

```bash
cargo build
```

エラーがなければ成功です。

#### 実行して確認

```bash
cargo run -- "利用可能なツールを教えて"
```

**期待される出力例：**
```
Available tools: readFile, writeFile, listFiles, searchInDirectory, editFile
```

または、Claude が「editFileというツールが使えます」と説明してくれます。

---

## タスク9: 基本的な編集をテスト

### 🎯 目標
実際にファイルを編集して、editFile ツールが正しく動作することを確認する

### 📝 手順

#### 9.1 テストファイルを作成

テスト用のファイルを作成します。

**Linux/macOS の場合:**
```bash
echo "Hello World" > test.txt
```

**Windows の場合:**
```bash
echo Hello World > test.txt
```

#### 9.2 ファイル内容を確認

```bash
cat test.txt  # または type test.txt (Windows)
```

**出力:**
```
Hello World
```

#### 9.3 エージェントを実行

```bash
cargo run -- "test.txtの内容をHello Nebulaに変更してください"
```

**期待される動作：**

```
Assistant is using tools...
Executing tool: readFile with arguments: {"path":"test.txt"}
Tool 'readFile' executed with result: {"content":"Hello World\n","error":null}

Assistant is using tools...
Executing tool: editFile with arguments: {"path":"test.txt","new_content":"Hello Nebula\n"}

既存ファイルを編集します: test.txt
実行してもよろしいですか？ [y/N]:
```

**ここで `y` を入力してEnterを押してください。**

```
Tool 'editFile' executed with result: {"content":"ファイル test.txt を正常に更新しました","error":null}
Assistant: test.txtの内容を「Hello World」から「Hello Nebula」に変更しました。
```

#### 9.4 結果を確認

```bash
cat test.txt  # または type test.txt (Windows)
```

**期待される出力:**
```
Hello Nebula
```

### 💡 動作のポイント

**Read-Modify-Write パターンが自動実行されている：**

1. **Read** - LLMが「まずファイルを読む必要がある」と判断
   ```
   readFile("test.txt") → "Hello World"
   ```

2. **Modify** - LLMがメモリ内で新しい内容を構築
   ```
   思考: "Hello World を Hello Nebula に変更"
   ```

3. **Write** - LLMがeditFileを呼び出す
   ```
   editFile("test.txt", "Hello Nebula")
   ```

**ユーザー確認が機能している：**
- editFile実行前に確認プロンプトが表示される
- "y" を入力しないと実行されない
- セキュリティが確保されている

### 🐛 よくあるエラーと対処法

**エラー1: ファイルが存在しません**
```
error: "ファイルが存在しません。新しいファイルの作成にはwriteFileを使用してください。"
```

**原因:** test.txt が作成されていないか、パスが間違っている

**対処法:**
```bash
# ファイルを作成
echo "Hello World" > test.txt

# 現在のディレクトリを確認
pwd
ls test.txt
```

**エラー2: Permission denied**
```
error: "ファイルの書き込みに失敗しました: Permission denied"
```

**原因:** ファイルに書き込み権限がない

**対処法:**
```bash
# 権限を確認
ls -l test.txt

# 権限を付与（Linux/macOS）
chmod 644 test.txt
```

**エラー3: readFileが実行されない**
```
Assistant: editFileで直接変更します...
```

**原因:** description が不十分、またはモデルが Read-Modify-Write を理解していない

**対処法:**
- description フィールドを再確認
- より明示的な指示を与える: "まずファイルを読んでから編集してください"

### ✅ 動作確認

- [ ] テストファイルを作成できた
- [ ] エージェントがreadFileとeditFileを順番に呼び出した
- [ ] ユーザー確認プロンプトが表示された
- [ ] ファイルの内容が正しく変更された

---

## タスク10: Read-Modify-Write パターンのテスト

### 🎯 目標
より複雑なシナリオでRead-Modify-Writeパターンが正しく動作することを確認する

### 📝 手順

#### 10.1 複数行のファイルを作成

```bash
cat > config.toml << EOF
model = "claude-3-5-sonnet-20241022"
max_tokens = 1024
api_key = "your-api-key"
EOF
```

**または Windows の場合:**
```cmd
(
echo model = "claude-3-5-sonnet-20241022"
echo max_tokens = 1024
echo api_key = "your-api-key"
) > config.toml
```

#### 10.2 内容を確認

```bash
cat config.toml
```

**出力:**
```
model = "claude-3-5-sonnet-20241022"
max_tokens = 1024
api_key = "your-api-key"
```

#### 10.3 複雑な編集をテスト

```bash
cargo run -- "config.tomlのmax_tokensを2048に変更してください"
```

**期待される動作フロー:**

```
1. LLM判断: "まずconfig.tomlを読む必要がある"
   ↓
2. ツール実行: readFile("config.toml")
   ↓
   結果:
   {
     "content": "model = \"claude-3-5-sonnet-20241022\"\nmax_tokens = 1024\napi_key = \"your-api-key\"\n"
   }
   ↓
3. LLM思考: "max_tokensを1024から2048に変更した完全な内容を構築"
   ↓
4. ツール実行: editFile({
     "path": "config.toml",
     "new_content": "model = \"claude-3-5-sonnet-20241022\"\nmax_tokens = 2048\napi_key = \"your-api-key\"\n"
   })
   ↓
5. ユーザー確認: "実行してもよろしいですか？ [y/N]:"
   ↓ (y を入力)
6. 完了: ファイル更新
```

#### 10.4 結果を確認

```bash
cat config.toml
```

**期待される出力:**
```
model = "claude-3-5-sonnet-20241022"
max_tokens = 2048
api_key = "your-api-key"
```

**重要なポイント:**
- ✅ `max_tokens` が 1024 → 2048 に変更されている
- ✅ 他の行は変更されていない
- ✅ ファイル構造が保たれている

#### 10.5 行の追加をテスト

```bash
cargo run -- "config.tomlに temperature = 0.7 という行を追加してください"
```

**期待される動作:**

```
1. readFile("config.toml")
2. LLM: 既存の内容に temperature = 0.7 を追加
3. editFile で完全な新しい内容を書き込み
4. ユーザー確認
5. 完了
```

#### 10.6 最終結果を確認

```bash
cat config.toml
```

**期待される出力:**
```
model = "claude-3-5-sonnet-20241022"
max_tokens = 2048
api_key = "your-api-key"
temperature = 0.7
```

### 💡 Read-Modify-Write パターンの利点

**1. 安全性**
- LLMはファイル全体を理解してから編集
- 部分的な変更で構文エラーを起こすリスクが低い
- 意図しない削除を防ぐ

**2. 予測可能性**
```
readFile → ファイル全体を読む
  ↓
LLM思考 → 完全な新しい内容を構築
  ↓
editFile → 全体を置き換え
```

各ステップが明確で理解しやすい。

**3. LLMの特性に適している**
- LLMは「全体のコンテキスト」を理解するのが得意
- 部分的な編集指示（行番号など）よりも、完全な内容を生成する方が得意

**4. エラー回復が容易**
- エラーが発生した場合も、LLMは全体を見て修正できる
- 部分編集だと「どこが間違っているか」を特定するのが難しい

### 🐛 よくある問題と対処法

**問題1: ファイルの一部が削除される**

```toml
# 元のファイル
model = "claude-3-5-sonnet-20241022"
max_tokens = 1024
api_key = "your-api-key"

# 編集後（問題）
max_tokens = 2048
```

**原因:** LLMがファイル全体を読まずに編集しようとした

**対処法:**
1. `description` フィールドを確認
2. より明示的な指示: "まずreadFileで全内容を読んでから..."
3. `max_iterations` を増やす（複数ターンの余地を与える）

**問題2: ユーザー確認が表示されない**

**原因:** 標準入出力のバッファリング問題

**対処法:**
```rust
// flush() が正しく呼ばれているか確認
io::stdout().flush()?;
```

**問題3: ファイルが空になる**

**原因:** `new_content` が空文字列

**対処法:**
- LLMの応答を確認
- デバッグログを有効化: `RUST_LOG=debug cargo run -- "..."`

### ✅ 最終確認チェックリスト

**機能テスト:**
- [ ] 単純な内容変更が成功する
- [ ] 複数行ファイルの一部変更が成功する
- [ ] 行の追加が成功する
- [ ] ファイルの構造が保たれる

**エラーハンドリング:**
- [ ] 存在しないファイルで適切なエラーが表示される
- [ ] ユーザーが "n" を入力するとキャンセルされる
- [ ] 権限エラーが適切に表示される

**Read-Modify-Write パターン:**
- [ ] readFile が先に実行される
- [ ] editFile がその後実行される
- [ ] ファイル全体が置き換えられる
- [ ] 他の行が保持される

**ユーザー体験:**
- [ ] ユーザー確認プロンプトが表示される
- [ ] "y" を入力するまで実行されない
- [ ] 成功/失敗のメッセージが明確

### 🎉 完成！

おめでとうございます！editFile ツールが完成しました。

**達成できたこと:**
- ✅ editFile ツールの実装
- ✅ ファイル存在チェック
- ✅ ユーザー確認機能
- ✅ Read-Modify-Write パターンの実現
- ✅ 安全なファイル編集
- ✅ エラーハンドリング

**実装の全体像:**

```
src/tools/edit_file.rs
  ├── EditFileArgs        # 引数構造体
  ├── EditFileTool        # ツール構造体
  │   ├── new()          # コンストラクタ
  │   ├── schema()       # JSON Schema定義
  │   ├── check_file_exists()      # ヘルパー関数
  │   ├── prompt_user_confirmation() # ヘルパー関数
  │   └── execute()      # ToolHandler実装（メイン処理）
  └── ToolHandler trait 実装
```

---

## 次のステップ: Chapter 5 への展望

### 現在のエージェントの能力

Chapter 4 完了時点で、エージェントは以下のことができます：

**ファイル操作:**
- ✅ readFile - ファイルを読む
- ✅ writeFile - 新規ファイルを作成
- ✅ editFile - 既存ファイルを編集
- ✅ listFiles - ディレクトリ一覧
- ✅ searchInDirectory - コード検索

**基本的なワークフロー:**
- ✅ Read-Modify-Write パターン
- ✅ 複数ツールの連続実行（agentic loop）
- ✅ ユーザー確認による安全性

### まだ足りないもの

しかし、本格的なコーディングエージェントにはまだ以下が不足しています：

**1. 計画的な思考プロセス**
```
現在: "ファイルを編集して" → すぐに編集
理想: プロジェクト調査 → 計画立案 → 実装 → 検証
```

**2. プロジェクト全体の理解**
```
現在: 個別のファイルを操作
理想: プロジェクト構造を理解し、関連ファイルを把握
```

**3. コード品質の維持**
```
現在: 編集するだけ
理想: cargo fmt → cargo clippy → cargo test
```

**4. 役割と制約の明確化**
```
現在: ツールを使えるだけ
理想: "Rustコーディングアシスタント"としての自覚
```

### Chapter 5 で学ぶこと

次の Chapter では、システムプロンプトの設計と実装により、エージェントに「思考プロセス」と「役割」を与えます。

---

## 📚 参考リソース

### Rust関連

**標準ライブラリ:**
- [std::path::Path](https://doc.rust-lang.org/std/path/struct.Path.html) - パス操作
- [std::fs](https://doc.rust-lang.org/std/fs/) - ファイル操作
- [std::io](https://doc.rust-lang.org/std/io/) - 入出力

**非同期ライブラリ:**
- [tokio::fs](https://docs.rs/tokio/latest/tokio/fs/) - 非同期ファイルI/O
- [async-trait](https://docs.rs/async-trait/) - async trait のサポート

**シリアライゼーション:**
- [serde](https://docs.rs/serde/) - シリアライゼーションフレームワーク
- [serde_json](https://docs.rs/serde_json/) - JSON処理

### Anthropic API

- [Tool Use Documentation](https://docs.anthropic.com/en/docs/build-with-claude/tool-use)
- [Best Practices for Tool Definitions](https://docs.anthropic.com/en/docs/build-with-claude/tool-use#best-practices-for-tool-definitions)

### プロジェクト内ドキュメント

- `docs/ch4/ch4_editfile.md` - editFileの概念的説明
- `docs/ch3/ch3_filesystem_tools.md` - ファイルシステムツールの概要
- `docs/ch3/implementation_guide.md` - writeFile の実装参考
- `codex.md` - より高度な実装の参考

### Rustプログラミング

- [The Rust Programming Language](https://doc.rust-lang.org/book/) - 公式ガイド
- [Rust By Example](https://doc.rust-lang.org/rust-by-example/) - 実例で学ぶ
- [Async Book](https://rust-lang.github.io/async-book/) - 非同期プログラミング

---

## まとめ

この Chapter 4 実装ガイドでは、editFile ツールを10のタスクに分けて段階的に実装しました。

**学んだRustの概念:**

### 基本概念
- ✅ モジュールシステム（mod, pub use）
- ✅ 構造体とユニット構造体
- ✅ Derive マクロ（Debug, Deserialize）

### パス操作
- ✅ `std::path::Path` と `PathBuf`
- ✅ `exists()`, `is_file()` メソッド

### ファイルI/O
- ✅ `tokio::fs::write()` - 非同期ファイル書き込み
- ✅ 同期 vs 非同期の使い分け

### ユーザー入力
- ✅ `std::io::stdin()` と `read_line()`
- ✅ `stdout().flush()` の重要性
- ✅ `trim()` と `to_lowercase()`

### エラーハンドリング
- ✅ `Result<(), String>` パターン
- ✅ `if let` によるパターンマッチ
- ✅ `match` による詳細なエラー処理
- ✅ エラーを `ToolResult` として返す重要性

### async/await
- ✅ `#[async_trait]` マクロ
- ✅ `.await` の使用
- ✅ 非同期関数の実装

### デザインパターン
- ✅ Read-Modify-Write パターン
- ✅ 全体置換アプローチ
- ✅ ユーザー確認フロー
- ✅ 安全性優先の設計

**次のChapterで学ぶこと:**
- システムプロンプトの設計
- より高度な思考プロセス
- プロジェクト全体の理解
- 本格的なコーディングエージェントへの進化

それでは、Chapter 5 でお会いしましょう！🦀