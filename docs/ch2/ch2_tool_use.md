# Chapter 2: Tool Use (Function Calling) でLLMの能力を拡張する

このドキュメントでは、Anthropic Claude APIのTool Use機能を使用してCLIツールを拡張し、LLMに「道具」を与える方法について説明します。

## 1. Tool Use の概要

### Tool Use（Function Calling）とは何か

Tool Use（旧称: Function Calling）は、LLMが外部のツールや機能を呼び出すことができる仕組みです。LLMは与えられたタスクを達成するために、どのツールをいつ使うべきかを自律的に判断し、適切なパラメータでツールを呼び出します。

**従来の会話の流れ:**
```
User → LLM → テキスト応答
```

**Tool Use対応後の流れ:**
```
User → LLM → ツール使用要求 → アプリ側でツール実行 → 結果をLLMに返却 → LLMが最終回答
```

### Tool Useの利点

- **外部システム連携**: ファイルシステム、データベース、API呼び出しが可能
- **リアルタイム情報**: 現在の時刻、天気、株価などの動的データにアクセス
- **計算実行**: 複雑な数値計算や処理をプログラムで実行
- **自律的判断**: どの機能をいつ使用するかをLLMが適切に判断

### Anthropic と OpenAI の違い

本プロジェクトの参考となるZenn記事（Go + OpenAI）との主な違い:

| 項目 | OpenAI API | Anthropic API |
|------|-----------|---------------|
| パラメータ名 | `functions` → `tools` | `tools` |
| スキーマフィールド | `parameters` | `input_schema` |
| レスポンス形式 | `tool_calls` 配列 | `tool_use` content block |
| ツール結果 | `tool` role のメッセージ | `tool_result` content block |
| 並列実行 | 順次実行が基本 | ネイティブサポート |
| Strict モード | - | 厳密なスキーマ検証 |

## 2. Anthropic API の Tool Use 仕様

### tools 配列の構造

APIリクエストに `tools` パラメータを追加することで、LLMが使用できるツールを定義します。

```rust
#[derive(Debug, Clone, Serialize)]
pub struct Tool {
    pub name: String,           // ツール名（英数字とアンダースコア、64文字以内）
    pub description: String,    // ツールの説明（重要！）
    pub input_schema: serde_json::Value,  // JSON Schema
}
```

**JSON Schema の例:**
```json
{
  "type": "object",
  "properties": {
    "path": {
      "type": "string",
      "description": "読み込むファイルのパス"
    }
  },
  "required": ["path"]
}
```

### tool_use content block

LLMがツールを使いたい場合、レスポンスに `tool_use` タイプの content block が含まれます。

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

### tool_result content block

ツール実行結果は、`user` role のメッセージとして `tool_result` content block を含めて返します。

```json
{
  "type": "tool_result",
  "tool_use_id": "toolu_01A09q90qw90lq917835lq9",
  "content": "{\"content\": \"ファイルの内容...\", \"error\": null}",
  "is_error": false
}
```

### stop_reason の扱い

レスポンスの `stop_reason` フィールドで次の動作を判断します:

- `"tool_use"`: LLMがツールを使いたい → ツールを実行して結果を返す
- `"end_turn"`: 会話が完了 → 最終応答を表示
- `"max_tokens"`: トークン制限到達 → エラーハンドリング

## 3. アーキテクチャ設計

### ツール定義パターン（Schema + Handler）

ツールは「スキーマ」と「実行ロジック」の組み合わせで定義します。

```rust
/// ツールの実行ロジック
#[async_trait::async_trait]
pub trait ToolHandler: Send + Sync {
    async fn execute(&self, input: serde_json::Value) -> Result<ToolResult>;
}

/// ツール実行結果
#[derive(Debug, Serialize)]
pub struct ToolResult {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
```

### ToolHandler trait の設計

非同期処理に対応した trait として設計します:

- **`async fn execute`**: ツールの実行ロジック
- **`Send + Sync`**: マルチスレッド環境で安全に使用可能
- **`Result<ToolResult>`**: エラーを ToolResult として返す（panic させない）

### ToolRegistry パターン

複数のツールを管理するレジストリパターンを採用します。

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn ToolHandler>>,
    schemas: Vec<Tool>,
}

impl ToolRegistry {
    pub fn register<T: ToolHandler + 'static>(&mut self, schema: Tool, handler: T);
    pub fn get_schemas(&self) -> Vec<Tool>;
    pub async fn execute(&self, name: &str, input: serde_json::Value) -> Result<ToolResult>;
}
```

**利点:**
- ツールの動的登録が可能
- 型安全な実装
- 拡張が容易

### Agentic Loop の実装フロー

```
1. User Input
   ↓
2. API Call with tools[] array
   ↓
3. Response with stop_reason="tool_use"?
   ├─ No  → 最終応答を表示して終了
   └─ Yes → 4へ
   ↓
4. Extract tool_use blocks
   ↓
5. Execute tools via ToolRegistry
   ↓
6. Create tool_result blocks
   ↓
7. Add to conversation as user message
   ↓
8. 2に戻る（最大反復回数まで）
```

**重要なポイント:**
- 会話履歴を保持し続ける（context management）
- 最大反復回数を設定してループを防ぐ
- 各ツール呼び出しをログに記録

## 4. 依存関係と技術選定

### 必須依存関係の追加

**Cargo.toml に追加:**
```toml
[dependencies]
async-trait = "0.1"    # async trait のサポート
serde_json = "1.0"     # JSON Schema 処理（既存）
tokio = { version = "1.48", features = ["fs"] }  # 非同期ファイルI/O
```

### async-trait の役割

Rustの trait では async メソッドを直接定義できないため、`async-trait` クレートを使用します。

```rust
// async-trait を使わない場合（エラー）
trait ToolHandler {
    async fn execute(&self, input: serde_json::Value) -> Result<ToolResult>;  // ❌
}

// async-trait を使う場合（正しい）
#[async_trait::async_trait]
trait ToolHandler: Send + Sync {
    async fn execute(&self, input: serde_json::Value) -> Result<ToolResult>;  // ✅
}
```

### JSON Schema の構築

`serde_json::json!` マクロを使って JSON Schema を簡潔に記述できます。

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

## 5. readFile ツールの実装例

### 機能説明

`readFile` ツールは、指定されたパスのファイル内容を読み込み、テキストとして返します。

**入力:**
```json
{
  "path": "README.md"
}
```

**出力（成功時）:**
```json
{
  "content": "ファイルの内容...",
  "error": null
}
```

**出力（エラー時）:**
```json
{
  "content": "",
  "error": "File not found: README.md"
}
```

### セキュリティ考慮事項（path validation）

ファイルシステムへのアクセスには、セキュリティリスクが伴います。

**実装すべき検証:**

1. **ディレクトリトラバーサル対策**
   ```rust
   // 悪意のあるパス例: ../../etc/passwd
   let canonical = path.canonicalize().context("Invalid path")?;
   let cwd = std::env::current_dir()?;
   if !canonical.starts_with(&cwd) {
       // 現在のディレクトリ外へのアクセスを拒否
       return Ok(ToolResult {
           content: String::new(),
           error: Some("Access denied: path outside working directory".to_string()),
       });
   }
   ```

2. **ファイル存在チェック**
   ```rust
   if !path.exists() {
       return Ok(ToolResult {
           content: String::new(),
           error: Some(format!("File not found: {}", path.display())),
       });
   }
   ```

3. **読み取り権限の確認**
   - `tokio::fs::read_to_string` が自動的にエラーを返すため、追加の実装は不要

### エラーハンドリング戦略

**重要な原則: エラーをパニックさせない**

```rust
// ❌ BAD: エラーを propagate すると agentic loop が壊れる
pub async fn execute(&self, input: serde_json::Value) -> Result<ToolResult> {
    let content = tokio::fs::read_to_string(path).await?;  // エラー時にpanic
    Ok(ToolResult { content, error: None })
}

// ✅ GOOD: エラーを ToolResult として返す
pub async fn execute(&self, input: serde_json::Value) -> Result<ToolResult> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => Ok(ToolResult { content, error: None }),
        Err(e) => Ok(ToolResult {
            content: String::new(),
            error: Some(format!("Failed to read file: {}", e)),
        }),
    }
}
```

**理由:**
- LLMはエラー内容を理解して適切に対応できる
- Agentic loop が途中で停止しない
- ユーザーにとって分かりやすいエラーメッセージ

## 6. 次のステップ

### 他のツールの追加方法

新しいツールを追加する際のパターン:

1. **ツール構造体を定義** (`src/tools/新ツール名.rs`)
2. **ToolHandler trait を実装**
3. **schema() メソッドを実装**
4. **ToolRegistry に登録**

**例: writeFile ツールの追加**
```rust
// src/tools/write_file.rs
pub struct WriteFileTool;

impl WriteFileTool {
    pub fn schema() -> Tool {
        Tool {
            name: "writeFile".to_string(),
            description: "指定されたパスに内容を書き込みます".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["path", "content"]
            }),
        }
    }
}

#[async_trait]
impl ToolHandler for WriteFileTool {
    async fn execute(&self, input: serde_json::Value) -> Result<ToolResult> {
        // 実装...
    }
}

// main.rs で登録
tool_registry.register(WriteFileTool::schema(), WriteFileTool::new());
```

### Chapter 3 の予告

次のChapterでは、以下のツールを追加してエージェントの能力を拡張します:

- **writeFile**: 新規ファイルの作成
- **listFiles**: ディレクトリ内のファイル一覧取得
- **searchInDirectory**: ディレクトリ内のキーワード検索

これにより、nebulaは「ファイルを読む」だけでなく、「ファイルを作成する」「ファイルを検索する」能力を獲得し、本格的なコーディングエージェントへと進化します。

## 参考リソース

### Anthropic API ドキュメント
- [Tool Use Overview](https://docs.anthropic.com/en/docs/build-with-claude/tool-use)
- [Implement Tool Use](https://docs.anthropic.com/en/docs/build-with-claude/tool-use#specifying-tools)

### Rust関連
- [async-trait Documentation](https://docs.rs/async-trait/latest/async_trait/)
- [serde_json::json! macro](https://docs.rs/serde_json/latest/serde_json/macro.json.html)
- [tokio::fs](https://docs.rs/tokio/latest/tokio/fs/)

### プロジェクト内ドキュメント
- `codex.md` - Codex の技術スタック詳細
- `docs/ch1/ch1_cli.md` - CLI実装ガイド
- `docs/ch1/implementation_guide.md` - Anthropic API連携実装ガイド

## まとめ

この Chapter 2 では、LLMに「道具」を与える Tool Use 機能について学びました。

**達成できること:**
- ✅ Tool Use の概念と仕組みの理解
- ✅ Anthropic API の Tool Use 仕様の把握
- ✅ ToolHandler trait による型安全な設計
- ✅ ToolRegistry パターンによる拡張性の確保
- ✅ readFile ツールによるファイルシステムアクセス
- ✅ セキュリティを考慮した実装
- ✅ エラーハンドリング戦略の確立

次の `implementation_guide.md` では、これらの概念を実際のRustコードとして段階的に実装していきます。
