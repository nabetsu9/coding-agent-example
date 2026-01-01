# Tool Use 実装ガイド（Rust初心者向け）

このガイドでは、Anthropic Claude APIのTool Use機能を使用してCLIツールを拡張する方法を、タスクごとに分割して段階的に実装していきます。各タスクで動作確認をしながら進めることで、確実に理解を深められます。

## 📋 全体の流れ

```
タスク1: Tool Use の仕組みを理解する
  ↓
タスク2: ContentBlock を列挙型に変更する
  ↓ 動作確認: cargo build が成功
  ↓
タスク3: Message 構造体を拡張する
  ↓ 動作確認: cargo build が成功
  ↓
タスク4: Tool 定義構造体を実装する
  ↓ 動作確認: cargo build が成功
  ↓
タスク5: ToolHandler trait を設計する
  ↓ 動作確認: cargo build が成功
  ↓
タスク6: ReadFileTool を実装する
  ↓ 動作確認: cargo build が成功
  ↓
タスク7: ToolRegistry を実装する
  ↓ 動作確認: cargo build が成功
  ↓
タスク8: create_message_with_tools を実装する
  ↓ 動作確認: cargo build が成功
  ↓
タスク9: Agentic Loop を実装する
  ↓ 動作確認: API呼び出しが成功
  ↓
タスク10: main.rs の統合とテスト
  ✓ 完成！
```

---

## タスク1: Tool Use の仕組みを理解する

### 🎯 目標
Anthropic API の Tool Use 機能の仕組みを理解する

### 📝 手順

#### 1.1 Anthropic と OpenAI の違いを学ぶ

参考となるZenn記事（Go + OpenAI）との主な違いを理解しましょう。

| 項目 | OpenAI (Zenn記事) | Anthropic (本プロジェクト) |
|------|-------------------|---------------------------|
| ツール定義 | `tools` 配列 | `tools` 配列（同じ） |
| パラメータ定義 | `parameters` フィールド | `input_schema` フィールド |
| LLMの応答 | `tool_calls` 配列 | `tool_use` content block |
| 結果の返却 | `tool` role のメッセージ | `user` role の `tool_result` block |

#### 1.2 stop_reason="tool_use" の意味

APIレスポンスの `stop_reason` フィールドで次の動作を判断します:

```rust
match response.stop_reason.as_deref() {
    Some("tool_use") => {
        // LLMがツールを使いたい
        // → ツールを実行して結果を返す必要がある
    }
    Some("end_turn") => {
        // 会話が完了
        // → 最終応答を表示
    }
    _ => {
        // その他のケース
    }
}
```

#### 1.3 content block の種類

Anthropic API では、メッセージの `content` に複数種類のブロックを含めることができます:

**text block:**
```json
{
  "type": "text",
  "text": "こんにちは！"
}
```

**tool_use block（LLMからアプリへ）:**
```json
{
  "type": "tool_use",
  "id": "toolu_01A09q90qw90lq917835lq9",
  "name": "readFile",
  "input": {
    "path": "README.md"
  }
}
```

**tool_result block（アプリからLLMへ）:**
```json
{
  "type": "tool_result",
  "tool_use_id": "toolu_01A09q90qw90lq917835lq9",
  "content": "{\"content\": \"ファイルの内容...\"}",
  "is_error": false
}
```

### 💡 Rust知識ポイント

**Tagged Union（タグ付き共用体）**

複数の型を持つことができる列挙型（enum）を使用します。`#[serde(tag = "type")]` アトリビュートにより、JSON の `"type"` フィールドに基づいて自動的に適切なバリアントにデシリアライズされます。

```rust
#[derive(Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
}
```

---

## タスク2: ContentBlock を列挙型に変更する

### 🎯 目標
現在の ContentBlock struct を enum に変更し、複数の content タイプをサポートする

### 📝 手順

#### 2.1 現在のコードを確認

`src/anthropic.rs` の ContentBlock 定義を確認します:

```rust
// 現在の実装（struct）
#[derive(Debug, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}
```

この実装では `text` タイプのブロックしか扱えません。

#### 2.2 enum に書き換える

`src/anthropic.rs` の ContentBlock を以下のように書き換えます:

**変更前:**
```rust
#[derive(Debug, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}
```

**変更後:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text {
        text: String,
    },

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
```

### 💡 Rust知識ポイント

**1. `#[serde(tag = "type")]`**

これは「外部タグ付き」（externally tagged）列挙型と呼ばれます。JSON の `"type"` フィールドの値に基づいて、どのバリアントかを判断します。

**2. `#[serde(rename = "text")]`**

Rust のバリアント名（`Text`）と JSON のフィールド名（`"text"`）が異なる場合に使用します。

**3. `Clone` の追加**

後で content block をコピーする必要があるため、`Clone` trait を derive します。

**4. `Serialize` の追加**

tool_result を JSON にシリアライズするため、`Serialize` trait を追加します。

**5. `#[serde(skip_serializing_if = "Option::is_none")]`**

`is_error` が `None` の場合、JSON に出力しないようにします。これにより、不要なフィールドを省略できます。

**6. `serde_json::Value`**

任意の JSON 値を表す型です。ツールの入力パラメータは動的に変わるため、この型を使用します。

#### 2.3 main.rs のコードを更新

ContentBlock が enum になったため、`main.rs` の表示部分を更新します:

**変更前:**
```rust
// レスポンスの表示
for content in &response.content {
    if content.content_type == "text" {
        println!("{}", content.text);
    }
}
```

**変更後:**
```rust
// レスポンスの表示
for content in &response.content {
    if let ContentBlock::Text { text } = content {
        println!("{}", text);
    }
}
```

### 💡 Rust知識ポイント

**`if let` パターンマッチ**

`if let` は、特定のパターンにマッチする場合のみコードを実行する構文です。

```rust
// match を使った場合（冗長）
match content {
    ContentBlock::Text { text } => println!("{}", text),
    _ => {}
}

// if let を使った場合（簡潔）
if let ContentBlock::Text { text } = content {
    println!("{}", text);
}
```

### ✅ 動作確認

```bash
cargo build
```

**期待される結果:**
```
   Compiling coding-agent-example v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 2.34s
```

エラーなくビルドが成功すればOKです！

---

## タスク3: Message 構造体を拡張する

### 🎯 目標
会話履歴を管理するため、Message 構造体を導入する

### 📝 手順

#### 3.1 現在の問題点

現在、`MessageRequest` では messages を以下のように定義しています:

```rust
struct MessageRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,  // ← この Message 構造体が未定義
}
```

しかし、実際には単純な構造体ではなく、content が文字列またはブロック配列のどちらかになる場合があります。

#### 3.2 Message と MessageContent を定義

`src/anthropic.rs` に以下の構造体を追加します（MessageRequest の **前** に追加）:

```rust
/// メッセージ（会話の1ターン）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,  // "user" または "assistant"
    pub content: MessageContent,
}

/// メッセージの内容（文字列 or ブロック配列）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}
```

### 💡 Rust知識ポイント

**1. `#[serde(untagged)]`**

`MessageContent` は「タグなし」（untagged）列挙型です。JSON に `"type"` フィールドがなく、値の構造だけで判断します。

```json
// Text バリアント
"こんにちは"

// Blocks バリアント
[
  { "type": "text", "text": "こんにちは" },
  { "type": "tool_use", "id": "...", "name": "readFile", "input": {...} }
]
```

**2. 役割（role）**

- `"user"`: ユーザーからのメッセージ（質問、ツール結果を含む）
- `"assistant"`: Claude からのメッセージ（応答、ツール使用要求を含む）

#### 3.3 MessageRequest を更新

既存の `MessageRequest` 内部の `Message` 構造体を削除し、上で定義した `Message` を使用します。

**変更箇所は特にありません**（既に `Vec<Message>` を使用していれば）。ただし、Message が正しく import されていることを確認してください。

#### 3.4 既存の create_message との互換性維持

既存のコードが動き続けるよう、ヘルパーメソッドを追加します:

```rust
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
```

### 💡 Rust知識ポイント

**`impl Into<String>`**

`Into<String>` trait を実装する任意の型（`&str`, `String` など）を受け入れます。

```rust
Message::user_text("Hello");        // &str
Message::user_text(String::from("Hello"));  // String
```

両方とも動作します。

### ✅ 動作確認

```bash
cargo build
```

既存のコードがコンパイルエラーにならないことを確認してください。

---

## タスク4: Tool 定義構造体を実装する

### 🎯 目標
ツールのスキーマと実行結果を表す構造体を定義する

### 📝 手順

#### 4.1 Tool 構造体の追加

`src/anthropic.rs` に以下を追加します:

```rust
/// Tool definition for the API
#[derive(Debug, Clone, Serialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}
```

### 💡 Rust知識ポイント

**`serde_json::Value`**

任意の JSON 値を表現できる型です。JSON Schema は動的な構造なので、この型を使用します。

```rust
use serde_json::json;

let schema = json!({
    "type": "object",
    "properties": {
        "path": {
            "type": "string",
            "description": "ファイルパス"
        }
    },
    "required": ["path"]
});
```

#### 4.2 ToolResult 構造体の追加

```rust
/// ツール実行結果
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
```

**設計のポイント:**

- `content`: 正常時の結果（例: ファイルの内容）
- `error`: エラー時のメッセージ（例: "File not found"）

**どちらか一方のみ設定されます。**

#### 4.3 MessageRequest に tools フィールドを追加

```rust
#[derive(Debug, Serialize)]
struct MessageRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,  // ← 追加
}
```

### 💡 Rust知識ポイント

**`#[serde(skip_serializing_if = "Option::is_none")]`**

`tools` が `None` の場合、JSON に含めません。これにより、ツールを使わない場合は従来通りのリクエストになります。

```rust
// tools が None の場合
{
  "model": "claude-sonnet-4-5",
  "max_tokens": 1024,
  "messages": [...]
  // "tools" フィールドは出力されない
}

// tools が Some の場合
{
  "model": "claude-sonnet-4-5",
  "max_tokens": 1024,
  "messages": [...],
  "tools": [...]
}
```

### ✅ 動作確認

```bash
cargo build
```

---

## タスク5: ToolHandler trait を設計する

### 🎯 目標
ツールの実行ロジックを定義するための trait を作成する

### 📝 手順

#### 5.1 async-trait の追加

`Cargo.toml` に async-trait を追加します:

```toml
[dependencies]
# 既存の依存関係...
async-trait = "0.1"
```

### 💡 Rust知識ポイント

**async trait の問題**

Rust では、trait のメソッドを `async` にすることができません（コンパイラの制限）。

```rust
// これはコンパイルエラー
trait ToolHandler {
    async fn execute(&self, input: serde_json::Value) -> Result<ToolResult>;
}
```

**async-trait の解決策**

`async-trait` クレートを使用すると、async メソッドを持つ trait を定義できます。

```rust
use async_trait::async_trait;

#[async_trait]
trait ToolHandler {
    async fn execute(&self, input: serde_json::Value) -> Result<ToolResult>;
}
```

内部的には、`async fn` を `Box<dyn Future>` に変換します。

#### 5.2 ToolHandler trait の定義

`src/anthropic.rs` に以下を追加します:

```rust
use async_trait::async_trait;

/// ツール実行のための trait
#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn execute(&self, input: serde_json::Value) -> Result<ToolResult>;
}
```

### 💡 Rust知識ポイント

**`Send + Sync` の意味**

- `Send`: スレッド間で所有権を移動できる
- `Sync`: 複数のスレッドから参照できる

非同期処理では、タスクが別のスレッドに移動する可能性があるため、これらの trait が必要です。

**なぜ必要？**

```rust
// Tokio ランタイムでタスクをスポーンする場合
tokio::spawn(async move {
    handler.execute(input).await  // handler は Send + Sync である必要がある
});
```

### ✅ 動作確認

```bash
cargo build
```

---

## タスク6: ReadFileTool を実装する

### 🎯 目標
最初の具体的なツール `readFile` を実装する

### 📝 手順

#### 6.1 tools ディレクトリの作成

```bash
mkdir -p src/tools
```

#### 6.2 src/tools/mod.rs の作成

```rust
pub mod read_file;

pub use read_file::ReadFileTool;
```

#### 6.3 src/tools/read_file.rs の作成

新しいファイル `src/tools/read_file.rs` を作成し、以下のコードを記述します:

```rust
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
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
            description: "指定されたパスのファイル内容を読み込みます。相対パスまたは絶対パスを指定できます。".to_string(),
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
        let args: ReadFileArgs = serde_json::from_value(input)
            .context("Failed to parse readFile arguments")?;

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
                debug!("Successfully read {} bytes from {}", content.len(), args.path);
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
```

### 💡 Rust知識ポイント

**1. `tokio::fs::read_to_string`**

非同期でファイルを読み込む関数です。`.await` で完了を待ちます。

```rust
// 同期版（ブロックする）
std::fs::read_to_string("file.txt")?;

// 非同期版（他のタスクを実行できる）
tokio::fs::read_to_string("file.txt").await?;
```

**2. `serde_json::from_value`**

`serde_json::Value` から型付き構造体にデシリアライズします。

```rust
let input = json!({ "path": "README.md" });
let args: ReadFileArgs = serde_json::from_value(input)?;
println!("{}", args.path);  // "README.md"
```

**3. エラーを ToolResult として返す**

重要な原則: **ツール実行のエラーはパニックさせず、ToolResult として返す**

```rust
// ❌ BAD: エラーを propagate
fs::read_to_string(&path).await?;

// ✅ GOOD: エラーを ToolResult として返す
match fs::read_to_string(&path).await {
    Ok(content) => Ok(ToolResult { content, error: None }),
    Err(e) => Ok(ToolResult {
        content: String::new(),
        error: Some(format!("エラー: {}", e)),
    }),
}
```

Claude はエラー内容を読んで適切に対応できます。

#### 6.4 main.rs で tools モジュールを宣言

`src/main.rs` の先頭に追加:

```rust
mod anthropic;
mod tools;  // ← 追加

use anthropic::AnthropicClient;
use tools::ReadFileTool;  // ← 追加
```

### ✅ 動作確認

```bash
cargo build
```

警告があっても、エラーがなければOKです。

---

## タスク7: ToolRegistry を実装する

### 🎯 目標
複数のツールを管理するレジストリを実装する

### 📝 手順

#### 7.1 ToolRegistry 構造体の追加

`src/anthropic.rs` に以下を追加します:

```rust
use std::collections::HashMap;

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
    pub fn register<T: ToolHandler + 'static>(
        &mut self,
        schema: Tool,
        handler: T,
    ) {
        let name = schema.name.clone();
        self.schemas.push(schema);
        self.tools.insert(name, Box::new(handler));
    }

    /// 登録されているツールのスキーマ一覧を取得
    pub fn get_schemas(&self) -> Vec<Tool> {
        self.schemas.clone()
    }

    /// ツールを実行
    pub async fn execute(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<ToolResult> {
        let handler = self.tools.get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", name))?;

        handler.execute(input).await
    }
}
```

### 💡 Rust知識ポイント

**1. `HashMap<String, Box<dyn ToolHandler>>`**

- `HashMap`: キー（ツール名）から値（ツールハンドラ）への高速マッピング
- `Box<dyn ToolHandler>`: trait オブジェクト（動的ディスパッチ）
- `dyn`: dynamic（動的）を意味し、実行時に実際の型が決まる

**なぜ Box？**

trait オブジェクトはサイズが不定なので、ヒープに配置する必要があります。

```rust
// ❌ サイズ不定なのでコンパイルエラー
let handler: dyn ToolHandler = ...;

// ✅ Box でヒープに配置
let handler: Box<dyn ToolHandler> = Box::new(ReadFileTool::new());
```

**2. `'static` ライフタイム**

```rust
pub fn register<T: ToolHandler + 'static>(...)
```

`'static` は「プログラムの終了まで有効」という意味です。ツールハンドラは一度登録したら削除しないため、この制約を課します。

**3. `ok_or_else` メソッド**

`Option<T>` を `Result<T, E>` に変換します。

```rust
let handler = self.tools.get(name)        // Option<&Box<dyn ToolHandler>>
    .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", name))?;  // Result
```

- `Some(handler)` → `Ok(handler)`
- `None` → `Err(anyhow::anyhow!(...))`

### ✅ 動作確認

```bash
cargo build
```

---

## タスク8: create_message_with_tools を実装する

### 🎯 目標
ツールをサポートした新しい API 呼び出しメソッドを実装する

### 📝 手順

#### 8.1 create_message_with_tools メソッドの追加

`src/anthropic.rs` の `impl AnthropicClient` ブロック内に以下のメソッドを追加します:

```rust
impl AnthropicClient {
    // 既存の create_message メソッド...

    /// ツールをサポートしたメッセージ作成
    pub async fn create_message_with_tools(
        &self,
        model: &str,
        max_tokens: u32,
        messages: Vec<Message>,
        tools: Option<Vec<Tool>>,
    ) -> Result<MessageResponse> {
        debug!("Preparing request to Anthropic API with tools");
        debug!(?model, ?max_tokens, messages_count = messages.len(), "Request parameters");

        let request = MessageRequest {
            model: model.to_string(),
            max_tokens,
            messages,
            tools,
        };

        let response = self.client
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
```

### 💡 Rust知識ポイント

このメソッドは既存の `create_message` とほぼ同じですが、`tools` パラメータを受け取る点が異なります。

**設計のポイント:**

- 既存のメソッドはそのまま残す（互換性維持）
- 新しいメソッドは `tools` を `Option` で受け取る
- 将来的には、既存のメソッドを新しいメソッドで実装し直すことも可能

```rust
pub async fn create_message(...) -> Result<MessageResponse> {
    self.create_message_with_tools(
        model,
        max_tokens,
        vec![Message::user_text(user_message)],
        None,  // tools なし
    ).await
}
```

### ✅ 動作確認

```bash
cargo build
```

---

## タスク9: Agentic Loop を実装する

### 🎯 目標
ツールを自動的に実行し、結果を Claude に返す反復ループを実装する

### 📝 手順

#### 9.1 会話結果を表す構造体の追加

`src/anthropic.rs` に以下を追加:

```rust
/// 会話の結果（ツール実行を含む）
pub struct ConversationResult {
    pub response: MessageResponse,
    pub conversation: Vec<Message>,
    pub iterations: usize,
}
```

#### 9.2 execute_with_tools メソッドの実装

`src/anthropic.rs` の `impl AnthropicClient` ブロック内に追加:

```rust
impl AnthropicClient {
    // 既存のメソッド...

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
            let response = self.create_message_with_tools(
                model,
                max_tokens,
                conversation.clone(),
                Some(tool_registry.get_schemas()),
            ).await?;

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
        bail!("Max iterations ({}) reached without final response", max_iterations);
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
                let content = serde_json::to_string(&result)
                    .context("Failed to serialize tool result")?;

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
```

### 💡 Rust知識ポイント

**1. `for iteration in 0..max_iterations`**

0 から `max_iterations - 1` まで反復します。

```rust
for iteration in 0..5 {
    println!("{}", iteration);  // 0, 1, 2, 3, 4
}
```

**2. `.clone()` の使用**

会話履歴を API に送る際、所有権を移動せずにコピーを渡します。

```rust
let response = self.create_message_with_tools(
    model,
    max_tokens,
    conversation.clone(),  // コピーを渡す
    Some(tool_registry.get_schemas()),
).await?;

// conversation はまだ使用可能
conversation.push(...);
```

**3. `as_deref()` メソッド**

`Option<String>` を `Option<&str>` に変換します。

```rust
let stop_reason: Option<String> = Some("tool_use".to_string());
stop_reason.as_deref() == Some("tool_use")  // true
```

**4. `if let` でのパターンマッチ**

```rust
if let ContentBlock::ToolUse { id, name, input } = block {
    // ToolUse バリアントの場合のみ実行
}
```

**5. `bail!` マクロ**

エラーを返して関数を終了します。

```rust
bail!("Max iterations ({}) reached", max_iterations);
// 以下と同じ
// return Err(anyhow::anyhow!("Max iterations ({}) reached", max_iterations));
```

### ✅ 動作確認

```bash
cargo build
```

---

## タスク10: main.rs の統合とテスト

### 🎯 目標
すべての実装を統合し、実際に動作させる

### 📝 手順

#### 10.1 main.rs を更新

`src/main.rs` を以下のように書き換えます:

```rust
mod anthropic;
mod tools;

use anyhow::Result;
use clap::Parser;
use anthropic::{AnthropicClient, ToolRegistry, MessageContent, ContentBlock};
use tools::ReadFileTool;

/// Anthropic Claude CLI Agent
#[derive(Debug, Parser)]
#[clap(author, version, about = "Anthropic Claude CLI Agent with Tool Use")]
struct Args {
    /// User message/prompt to send to Claude
    #[arg(value_name = "MESSAGE")]
    message: String,

    /// Anthropic API key (can also be set via ANTHROPIC_API_KEY env var)
    #[arg(long, env = "ANTHROPIC_API_KEY")]
    api_key: String,

    /// Model to use
    #[arg(long, short = 'm', default_value = "claude-3-5-sonnet-20241022")]
    model: String,

    /// Maximum tokens to generate
    #[arg(long, default_value = "1024")]
    max_tokens: u32,

    /// Maximum tool use iterations
    #[arg(long, default_value = "5")]
    max_iterations: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    // ロギング初期化
    tracing_subscriber::fmt()
        .with_env_filter("coding_agent_example=debug")
        .init();

    // .envファイルのロード（失敗時も継続）
    dotenvy::dotenv().ok();

    // CLI引数のパース
    let args = Args::parse();

    // APIキーの検証
    if args.api_key.is_empty() {
        anyhow::bail!("ANTHROPIC_API_KEY is required. Set via environment variable or --api-key flag.");
    }

    tracing::info!("Starting Anthropic Claude CLI with Tool Use");

    // Anthropic APIクライアントの作成
    let client = AnthropicClient::new(args.api_key);

    // ToolRegistry の作成
    let mut tool_registry = ToolRegistry::new();
    tool_registry.register(ReadFileTool::schema(), ReadFileTool::new());

    tracing::info!("Registered tools: readFile");

    // ツールを使った会話を実行
    let result = client
        .execute_with_tools(
            &args.model,
            args.max_tokens,
            &args.message,
            &tool_registry,
            args.max_iterations,
        )
        .await?;

    // レスポンスの表示
    println!("\n--- Claude's Response ---");
    for block in &result.response.content {
        if let ContentBlock::Text { text } = block {
            println!("{}", text);
        }
    }

    // メタデータの表示
    println!("\n--- Metadata ---");
    println!("Iterations: {}", result.iterations);
    println!("Input tokens: {}", result.response.usage.input_tokens);
    println!("Output tokens: {}", result.response.usage.output_tokens);

    Ok(())
}
```

### 💡 Rust知識ポイント

**1. 新しい CLI 引数**

`max_iterations`: ツール実行の最大反復回数を指定できます。

```bash
cargo run -- "Cargo.tomlを読んで" --max-iterations 10
```

**2. tracing::info!**

構造化ログを出力します。

```rust
tracing::info!("Registered tools: readFile");
// 出力: 2025-01-02T12:00:00.123Z  INFO coding_agent_example: Registered tools: readFile
```

#### 10.2 動作確認

まず、テスト用のファイルを作成します:

```bash
echo "これはテスト用のファイルです。
Tool Use が正しく動作しているか確認します。

内容:
- Rust で実装
- Anthropic Claude API を使用
- readFile ツールでこのファイルを読み込み" > sample.txt
```

プログラムを実行します:

```bash
cargo run -- "sample.txtの内容を読んで教えてください"
```

**期待される出力:**
```
2025-01-02T12:00:00.123Z  INFO coding_agent_example: Starting Anthropic Claude CLI with Tool Use
2025-01-02T12:00:00.234Z  INFO coding_agent_example: Registered tools: readFile
2025-01-02T12:00:00.345Z  INFO coding_agent_example: Iteration 1/5
2025-01-02T12:00:01.123Z  INFO coding_agent_example: Executing tools...
2025-01-02T12:00:01.234Z  INFO coding_agent_example: Executing tool: readFile
2025-01-02T12:00:01.345Z  INFO coding_agent_example: Tool 'readFile' executed successfully
2025-01-02T12:00:01.456Z  INFO coding_agent_example: Iteration 2/5
2025-01-02T12:00:02.123Z  INFO coding_agent_example: Conversation completed in 2 iterations

--- Claude's Response ---
sample.txtの内容を読み込みました。以下がファイルの内容です：

これはテスト用のファイルです。
Tool Use が正しく動作しているか確認します。

内容:
- Rust で実装
- Anthropic Claude API を使用
- readFile ツールでこのファイルを読み込み

--- Metadata ---
Iterations: 2
Input tokens: 450
Output tokens: 95
```

#### 10.3 エラーケースのテスト

存在しないファイルを指定した場合:

```bash
cargo run -- "存在しないファイル.txtを読んでください"
```

**期待される出力:**
```
--- Claude's Response ---
申し訳ありませんが、指定されたファイルを読み込むことができませんでした。

エラー: ファイルが見つかりません: 存在しないファイル.txt

ファイルパスが正しいか、ファイルが存在するかをご確認ください。
```

Claude はエラーメッセージを理解し、適切に応答します。

#### 10.4 ツールなしのケース

ツールを必要としない質問をした場合:

```bash
cargo run -- "Rustについて簡単に説明してください"
```

**期待される出力:**
```
--- Claude's Response ---
Rustは、メモリ安全性とパフォーマンスを重視したシステムプログラミング言語です...

--- Metadata ---
Iterations: 1
Input tokens: 120
Output tokens: 200
```

ツールが呼び出されず、1回の反復で完了します。

---

## タスク10.5: コード品質チェック

### フォーマット確認

```bash
cargo fmt
```

自動的にコードが整形されます。

### Linter実行

```bash
cargo clippy -- -D warnings
```

**期待される結果:**
```
    Checking coding-agent-example v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 1.23s
```

警告やエラーがある場合は、メッセージに従って修正します。

---

## 🎉 完成！

おめでとうございます！Anthropic Claude APIのTool Use機能を使ったコーディングエージェントが完成しました。

### 📊 実装の全体像

```
src/
├── main.rs                # CLIエントリーポイント + ToolRegistry統合
├── anthropic.rs           # APIクライアント + Agentic Loop
└── tools/
    ├── mod.rs             # ツールモジュール
    └── read_file.rs       # readFile ツール実装
```

### 🚀 達成できたこと

- ✅ Tool Use の理解 - LLMがツールを使う仕組みの習得
- ✅ ContentBlock enum - 複数の content タイプのサポート
- ✅ Message 構造体 - 会話履歴の管理
- ✅ ToolHandler trait - 型安全なツール実装
- ✅ ToolRegistry - 拡張可能なツール管理
- ✅ readFile ツール - ファイルシステムアクセス
- ✅ Agentic Loop - 自動的なツール実行と結果のフィードバック
- ✅ エラーハンドリング - Claude が理解できるエラー応答

### 📚 学んだRustの概念まとめ

#### 型システム
- ✅ Tagged enum (`#[serde(tag = "type")]`)
- ✅ Untagged enum (`#[serde(untagged)]`)
- ✅ Trait object (`Box<dyn ToolHandler>`)
- ✅ `Option<T>` と `Result<T, E>`

#### 非同期処理
- ✅ `async-trait` クレート
- ✅ `tokio::fs` for async file I/O
- ✅ `.await` での非同期処理の待機

#### エラーハンドリング
- ✅ `anyhow::Result`
- ✅ `.context()` でエラー情報の追加
- ✅ `bail!` マクロ
- ✅ エラーを値として返すパターン

#### コレクション
- ✅ `HashMap<K, V>`
- ✅ `Vec<T>`

### 🔧 次のステップ（Chapter 3）

Chapter 3 では、以下のツールを追加してエージェントの能力を大幅に拡張します:

1. **writeFile**: ファイルの作成・上書き
2. **listFiles**: ディレクトリ内のファイル一覧
3. **searchInDirectory**: キーワード検索

これにより、nebulaは「読む」「書く」「探す」能力を獲得し、本格的なコーディングエージェントへと進化します。

---

## トラブルシューティング

### よくあるエラーと対処法

**エラー1: Tool not found**
```
Error: Tool not found: readFile
```
**原因:** ToolRegistry にツールが登録されていません
**対処法:**
```rust
tool_registry.register(ReadFileTool::schema(), ReadFileTool::new());
```

**エラー2: Failed to parse readFile arguments**
```
Error: Failed to parse readFile arguments
```
**原因:** LLMが不正な入力を送信しています
**対処法:** ツールの `description` を詳細にして、LLMが正しく使えるようにします

**エラー3: Max iterations reached**
```
Error: Max iterations (5) reached without final response
```
**原因:** ツールの実行結果が LLM に正しく伝わっていません
**対処法:**
- ToolResult の JSON シリアライゼーションを確認
- エラーメッセージが適切か確認

---

質問があれば、いつでもお気軽にどうぞ！🦀
