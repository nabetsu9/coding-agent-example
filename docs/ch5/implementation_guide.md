# システムプロンプトと設定管理 実装ガイド（Rust初心者向け）

このガイドでは、コーディングエージェントにシステムプロンプトと設定管理機能を追加する方法を、タスクごとに分割して段階的に実装していきます。

## 📋 全体の流れ

```
タスク1: システムプロンプトの概念を理解する
  ↓
タスク2: toml, dirs 依存関係を追加
  ↓ 動作確認: cargo build が成功
  ↓
タスク3: Config構造体を定義（基本構造）
  ↓ 動作確認: cargo build が成功
  ↓
タスク4: 設定パス解決を実装
  ↓ 動作確認: cargo build が成功
  ↓
タスク5: Config load/save を実装
  ↓ 動作確認: ユニットテストが通る
  ↓
タスク6: システムプロンプトビルダーを作成
  ↓ 動作確認: cargo build が成功
  ↓
タスク7: MessageRequest に system フィールドを追加
  ↓ 動作確認: cargo build が成功
  ↓
タスク8: APIクライアントを更新
  ↓ 動作確認: cargo build が成功
  ↓
タスク9: main.rs に統合
  ↓ 動作確認: システムプロンプト付きでAPI呼び出しできる
  ↓
タスク10: 制御文字サニタイズを実装
  ↓ 動作確認: ユニットテストが通る
  ↓
タスク11: 対話的モデル切り替えを実装
  ↓ 動作確認: /model コマンドが動作する
  ↓
タスク12: E2Eテストと検証
  ✓ 完成！
```

---

## タスク1: システムプロンプトの概念を理解する

### 🎯 目標
システムプロンプトとは何か、なぜ必要かを理解する

### 📝 概念説明

#### システムプロンプトとは？

システムプロンプトは、LLMに「どのように振る舞うべきか」を伝える特別なメッセージです。

```
┌──────────────────────────────────────┐
│  System Prompt                       │  ← 毎回のリクエストに含まれる
│  "You are a coding assistant..."     │     行動指針・制約・ルール
├──────────────────────────────────────┤
│  User Message                        │  ← ユーザーからの実際の指示
│  "Create a new file..."              │
└──────────────────────────────────────┘
```

#### なぜ必要か？

**システムプロンプトなしの場合:**
```
User: "tools/writeFile.goを参考に、tools/copyFile.goを作成して"

LLM: （writeFile.goを読まずに）こんな感じでいいですか？
     → 既存のスタイルと違う実装になる
```

**システムプロンプトありの場合:**
```
System: "Before implementing, you MUST use readFile on reference files"

User: "tools/writeFile.goを参考に、tools/copyFile.goを作成して"

LLM: まずwriteFile.goを読みます...
     → readFile("tools/writeFile.go")
     → 既存のスタイルに合わせた実装
```

#### Anthropic API での指定方法

```rust
// Anthropic Messages API のリクエスト構造
{
    "model": "claude-sonnet-4-5-20250514",
    "max_tokens": 1024,
    "system": "You are a coding assistant...",  // ← ここがシステムプロンプト
    "messages": [
        {"role": "user", "content": "Hello"}
    ]
}
```

### ✅ 確認ポイント

- [ ] システムプロンプトがユーザーメッセージと別であることを理解した
- [ ] なぜ情報収集を強制する必要があるか理解した
- [ ] Anthropic APIの `system` フィールドを理解した

---

## タスク2: toml, dirs 依存関係を追加

### 🎯 目標
設定管理に必要なクレートを追加する

### 📝 手順

#### 2.1 Cargo.toml を編集

`Cargo.toml` の `[dependencies]` セクションに以下を追加：

```toml
[dependencies]
# 既存の依存関係...

# 設定管理（Chapter 5で追加）
toml = "0.8"
dirs = "5.0"
```

### 💡 Rust知識ポイント

**1. `toml` クレート**
- TOML形式のファイルを読み書きするライブラリ
- `serde` と組み合わせて構造体との変換が可能
- Rustエコシステムでは設定ファイルにTOMLが標準的

```rust
// 使用例
let config: Config = toml::from_str(&content)?;  // TOML → 構造体
let content = toml::to_string_pretty(&config)?;  // 構造体 → TOML
```

**2. `dirs` クレート**
- OS依存のディレクトリパスを取得するライブラリ
- クロスプラットフォーム対応（Windows/macOS/Linux）

```rust
// 使用例
let home = dirs::home_dir();  // Some("/Users/username") or None
// Windows: C:\Users\username
// macOS:   /Users/username
// Linux:   /home/username
```

**3. なぜJSONではなくTOMLか？**

| 形式 | コメント | Rust標準 | 可読性 |
|------|---------|---------|--------|
| JSON | 不可 | ❌ | 普通 |
| TOML | 可能 | ✅ | 高い |
| YAML | 可能 | ❌ | 高い |

Cargo.toml自体がTOML形式であり、Rustエコシステムでは設定ファイルにTOMLを使うのが一般的です。

### ✅ 動作確認

```bash
cargo build
```

**期待される結果:**
```
   Compiling toml v0.8.x
   Compiling dirs v5.0.x
   Compiling coding-agent-example v0.1.0
    Finished dev [unoptimized + debuginfo] target(s)
```

---

## タスク3: Config構造体を定義（基本構造）

### 🎯 目標
設定を保持する構造体を定義する

### 📝 手順

#### 3.1 新しいファイルを作成

`src/config.rs` を新規作成します。

#### 3.2 基本構造を記述

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub model: ModelConfig,

    #[serde(default)]
    pub agent: AgentConfig,
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    #[serde(default = "default_model")]
    pub default: String,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
}

// デフォルト値を返す関数
fn default_model() -> String {
    "claude-sonnet-4-5-20250514".to_string()
}

fn default_max_iterations() -> usize {
    10
}

// Default トレイトの実装
impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default: default_model(),
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: default_max_iterations(),
        }
    }
}
```

#### 3.3 main.rs でモジュールを読み込む

`src/main.rs` の先頭（use文の前）に追加：

```rust
mod config;
```

### 💡 Rust知識ポイント

**1. `#[derive(Default)]` vs `impl Default`**

```rust
// derive を使う場合（すべてのフィールドが Default を実装している必要あり）
#[derive(Default)]
pub struct Config { ... }

// 手動で実装する場合（カスタムデフォルト値が必要な場合）
impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default: "claude-sonnet-4-5-20250514".to_string(),
        }
    }
}
```

**2. `#[serde(default)]` アトリビュート**

```rust
#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]  // TOMLにこのフィールドがなければ Default::default() を使う
    pub model: ModelConfig,
}
```

これにより、設定ファイルに一部のフィールドが欠けていても正常にパースできます。

**3. `#[serde(default = "関数名")]`**

```rust
#[serde(default = "default_model")]
pub default: String,

fn default_model() -> String {
    "claude-sonnet-4-5-20250514".to_string()
}
```

フィールドが存在しない場合に呼ばれる関数を指定できます。

**4. なぜ `Clone` が必要か？**

```rust
#[derive(Clone)]
pub struct Config { ... }
```

- 設定を複数の場所で使い回す場合にコピーが必要
- `clone()` メソッドで明示的にコピーを作成できる
- 所有権を移動させずにデータを共有できる

### ✅ 動作確認

```bash
cargo build
```

エラーがなければ成功です。

---

## タスク4: 設定パス解決を実装

### 🎯 目標
設定ファイルのパス（`~/.codex/config.toml`）を解決する

### 📝 手順

#### 4.1 Config構造体に関連関数を追加

`src/config.rs` の `impl Default for AgentConfig` の後に以下を追加：

```rust
impl Config {
    /// Get the codex home directory (~/.codex)
    pub fn codex_home() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .context("Could not determine home directory")?;
        Ok(home.join(".codex"))
    }

    /// Get the config file path (~/.codex/config.toml)
    pub fn config_path() -> Result<PathBuf> {
        let codex_home = Self::codex_home()?;
        Ok(codex_home.join("config.toml"))
    }
}
```

### 💡 Rust知識ポイント

**1. `PathBuf` と `Path`**

| 型 | 説明 | 類似 |
|----|------|------|
| `PathBuf` | 所有権を持つパス | `String` |
| `Path` | パスへの参照 | `&str` |

```rust
let path: PathBuf = PathBuf::from("/home/user");
let path_ref: &Path = path.as_path();
```

**2. `PathBuf::join()`**

```rust
let home = PathBuf::from("/home/user");
let config = home.join(".codex").join("config.toml");
// 結果: /home/user/.codex/config.toml

// Windows の場合は自動的に \ が使われる
// C:\Users\user\.codex\config.toml
```

`join()` はOSに応じたパス区切り文字を使用するため、クロスプラットフォーム対応になります。

**3. `dirs::home_dir()` の戻り値**

```rust
pub fn home_dir() -> Option<PathBuf>
```

- 成功時: `Some(PathBuf)`
- 失敗時: `None`（ホームディレクトリが不明な場合）

`Option` を `Result` に変換するために `.context()` を使用：

```rust
let home = dirs::home_dir()
    .context("Could not determine home directory")?;
// Option<PathBuf> → Result<PathBuf, anyhow::Error>
```

**4. `Self` キーワード**

```rust
impl Config {
    pub fn config_path() -> Result<PathBuf> {
        let codex_home = Self::codex_home()?;  // Self = Config
        // ...
    }
}
```

`impl` ブロック内で `Self` は実装対象の型（この場合 `Config`）を指します。

### ✅ 動作確認

```bash
cargo build
```

エラーがなければ成功です。

---

## タスク5: Config load/save を実装

### 🎯 目標
設定ファイルの読み込みと保存機能を実装する

### 📝 手順

#### 5.1 load と save メソッドを追加

`src/config.rs` の `impl Config` ブロック内に以下を追加：

```rust
    /// Load configuration from file (or use defaults if not found)
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            tracing::debug!("Config file not found at {:?}, using defaults", path);
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)
            .context("Failed to read config file")?;

        let config: Config = toml::from_str(&content)
            .context("Failed to parse config file")?;

        tracing::info!("Loaded config from {:?}", path);
        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        std::fs::write(&path, content)
            .context("Failed to write config file")?;

        tracing::info!("Saved config to {:?}", path);
        Ok(())
    }
```

#### 5.2 ユニットテストを追加

`src/config.rs` の末尾に以下を追加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.model.default, "claude-sonnet-4-5-20250514");
        assert_eq!(config.agent.max_iterations, 10);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();

        // TOML文字列からデシリアライズできることを確認
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.model.default, config.model.default);
    }

    #[test]
    fn test_partial_config_parsing() {
        // 一部のフィールドが欠けていても動作することを確認
        let toml_str = r#"
[model]
default = "claude-haiku-3-5-20241022"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.model.default, "claude-haiku-3-5-20241022");
        assert_eq!(config.agent.max_iterations, 10);  // デフォルト値が使われる
    }
}
```

### 💡 Rust知識ポイント

**1. `std::fs::read_to_string()`**

```rust
let content = std::fs::read_to_string(&path)?;
```

ファイル全体を `String` として読み込みます。

**2. `std::fs::write()`**

```rust
std::fs::write(&path, content)?;
```

文字列をファイルに書き込みます（既存ファイルは上書き）。

**3. `std::fs::create_dir_all()`**

```rust
std::fs::create_dir_all(parent)?;
```

親ディレクトリを含めて再帰的にディレクトリを作成します（`mkdir -p` 相当）。

**4. `path.parent()`**

```rust
if let Some(parent) = path.parent() {
    // parent は &Path 型
}
```

パスの親ディレクトリを取得します。ルートの場合は `None` が返ります。

**5. `toml::from_str()` と `toml::to_string_pretty()`**

```rust
// TOML文字列 → 構造体
let config: Config = toml::from_str(&content)?;

// 構造体 → TOML文字列（整形済み）
let content = toml::to_string_pretty(&config)?;
```

**6. `#[cfg(test)]` アトリビュート**

```rust
#[cfg(test)]
mod tests {
    // このモジュールはテスト実行時のみコンパイルされる
}
```

条件付きコンパイル（Conditional Compilation）の一種で、`cargo test` 実行時のみ有効になります。

**7. `r#"..."#` 生文字列リテラル**

```rust
let toml_str = r#"
[model]
default = "claude-haiku"
"#;
```

エスケープなしで複数行の文字列を書けます。TOML/JSON/SQLなどを埋め込む際に便利です。

### ✅ 動作確認

```bash
cargo test config
```

**期待される結果:**
```
running 3 tests
test config::tests::test_default_config ... ok
test config::tests::test_config_serialization ... ok
test config::tests::test_partial_config_parsing ... ok

test result: ok. 3 passed; 0 failed
```

---

## タスク6: システムプロンプトビルダーを作成

### 🎯 目標
エージェントの行動指針となるシステムプロンプトを生成する関数を作成する

### 📝 手順

#### 6.1 新しいファイルを作成

`src/system_prompt.rs` を新規作成します。

#### 6.2 システムプロンプトを記述

```rust
/// Build the system prompt for the coding agent
pub fn build_system_prompt() -> String {
    r#"You are a Rust coding assistant with access to file system tools.

## Critical Rules (Non-Negotiable)
1. NEVER assume or guess file contents, names, or locations - You must explore to understand them
2. Information gathering is MANDATORY before implementation - Guessing leads to immediate failure
3. Before using writeFile or editFile, you MUST have used readFile on reference files
4. NEVER ask for permission between steps - Proceed automatically through the entire workflow
5. Complete the entire task in one continuous flow - No pausing for confirmation

## Why Information Gathering is Critical
- File structures vary: What you expect vs. what exists are often different
- Extensions matter: .rs vs .ts vs .go affects implementation
- Directory layout matters: Different projects have different organization
- Assumption costs: Guessing wrong means complete rework

## Execution Protocol
When you receive a request, follow this mandatory sequence and proceed automatically:

### Step 1: Information Gathering (Required, but proceed automatically)
- Discover project structure: Use 'listFiles' to understand what files exist
- Use 'readFile': Read ALL reference files mentioned in the request
- Use 'searchInDirectory': Find related files when unsure about locations
- Verify reality: What you discover often differs from assumptions

**Internal Verification (check silently, do not ask user):**
□ Have I discovered the project structure when needed?
□ Have I read the reference file contents with readFile?
□ Do I understand the existing code structure?

### Step 2: Implementation (Proceed automatically after Step 1)
- Use 'writeFile' for new file creation
- Use 'editFile' for existing file modification
- Complete all related changes

**IMPORTANT: Proceed from Step 1 to Step 2 automatically without asking for permission.**

## Common Mistakes to Avoid
- FORBIDDEN: Guessing file names (e.g., assuming "todo.rs" exists without checking)
- FORBIDDEN: Guessing file extensions (e.g., assuming .js when it might be .ts)
- FORBIDDEN: Guessing directory structure (e.g., assuming files are in "src/" without checking)
- FORBIDDEN: Seeing "refer to X file" and implementing without actually reading X
- FORBIDDEN: Using your knowledge to guess file contents
- FORBIDDEN: Skipping the readFile step because the task seems simple
- FORBIDDEN: Asking "Should I proceed with implementation?" after information gathering

## Available Tools
- readFile: Read file contents by path
- writeFile: Create new files (requires user confirmation)
- editFile: Modify existing files (requires reading first)
- listFiles: List directory contents
- searchInDirectory: Search for text patterns in files

## Your Responsibility
Complete the entire task following this protocol in one continuous flow.
No shortcuts, no assumptions, no guessing, and no asking for permission between steps."#.to_string()
}
```

#### 6.3 main.rs でモジュールを読み込む

`src/main.rs` の先頭に追加：

```rust
mod system_prompt;
```

### 💡 Rust知識ポイント

**1. 生文字列リテラル `r#"..."#`**

```rust
r#"
This is a raw string.
It can span multiple lines.
No need to escape "quotes" or \backslashes.
"#
```

- `r#"` で開始し `"#` で終了
- エスケープシーケンスが無効になる
- 複数行の文字列をそのまま書ける
- `#` の数を増やすことで、文字列内に `"#` を含めることも可能: `r##"..."##`

**2. `.to_string()` メソッド**

```rust
r#"..."#.to_string()
```

生文字列リテラル（`&'static str` 型）を `String` 型に変換します。

- `&str`: 借用された文字列スライス（不変）
- `String`: 所有権を持つ文字列（ヒープ上に確保）

関数から文字列を返す場合、`String` を返すのが一般的です。

**3. なぜシステムプロンプトは英語で書くのか？**

実験によると、GPT-4.1-nano/mini などの軽量モデルでは、日本語の長文プロンプトだと指示の一部が無視されることがあります。英語で書くことで、より確実に指示が伝わります。

### ✅ 動作確認

```bash
cargo build
```

エラーがなければ成功です。

---

## タスク7: MessageRequest に system フィールドを追加

### 🎯 目標
Anthropic APIへのリクエストにシステムプロンプトを含められるようにする

### 📝 手順

#### 7.1 MessageRequest 構造体を修正

`src/anthropic.rs` の `MessageRequest` 構造体を以下のように修正：

**変更前:**
```rust
/// Request structure for Messages API
#[derive(Debug, Serialize)]
struct MessageRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}
```

**変更後:**
```rust
/// Request structure for Messages API
#[derive(Debug, Serialize)]
struct MessageRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}
```

### 💡 Rust知識ポイント

**1. `Option<T>` 型**

```rust
system: Option<String>,
```

- 値が「ある」か「ない」かを表現する型
- `Some(value)`: 値がある
- `None`: 値がない

```rust
let with_system: Option<String> = Some("You are...".to_string());
let without_system: Option<String> = None;
```

**2. `#[serde(skip_serializing_if = "Option::is_none")]`**

```rust
#[serde(skip_serializing_if = "Option::is_none")]
system: Option<String>,
```

`None` の場合、JSONに含めません：

```rust
// system = Some("prompt")
{"model": "...", "system": "prompt", "messages": [...]}

// system = None
{"model": "...", "messages": [...]}  // system フィールドなし
```

これにより、システムプロンプトを使わない場合でも既存のAPIと互換性を保てます。

**3. `Option::is_none()` 関数**

```rust
impl<T> Option<T> {
    pub fn is_none(&self) -> bool {
        matches!(*self, None)
    }
}
```

`skip_serializing_if` には `&T -> bool` を返す関数を指定します。

### ✅ 動作確認

```bash
cargo build
```

エラーがなければ成功です。

---

## タスク8: APIクライアントを更新

### 🎯 目標
システムプロンプトを受け取れるように API クライアントを更新する

### 📝 手順

#### 8.1 create_message メソッドのシグネチャを更新

`src/anthropic.rs` の `create_message` メソッドを修正：

**変更前:**
```rust
    pub async fn create_message(
        &self,
        model: &str,
        max_tokens: u32,
        user_message: &str,
    ) -> Result<MessageResponse> {
```

**変更後:**
```rust
    pub async fn create_message(
        &self,
        model: &str,
        max_tokens: u32,
        user_message: &str,
        system: Option<String>,
    ) -> Result<MessageResponse> {
```

#### 8.2 MessageRequest の構築を更新

同じメソッド内の `MessageRequest` 構築部分を修正：

**変更前:**
```rust
        let request = MessageRequest {
            model: model.to_string(),
            max_tokens,
            messages: vec![Message {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
        };
```

**変更後:**
```rust
        let request = MessageRequest {
            model: model.to_string(),
            max_tokens,
            messages: vec![Message {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
            system,
        };
```

### 💡 Rust知識ポイント

**1. `Option<String>` を引数として受け取る**

```rust
pub async fn create_message(
    &self,
    // ...
    system: Option<String>,  // 呼び出し側が Some() か None を渡す
) -> Result<MessageResponse>
```

呼び出し方：

```rust
// システムプロンプトあり
client.create_message(&model, max_tokens, &message, Some(prompt)).await?;

// システムプロンプトなし
client.create_message(&model, max_tokens, &message, None).await?;
```

**2. 構造体のフィールド名省略**

```rust
MessageRequest {
    model: model.to_string(),
    max_tokens,  // max_tokens: max_tokens の省略形
    // ...
    system,      // system: system の省略形
}
```

変数名とフィールド名が同じ場合、`: 値` を省略できます。

### ✅ 動作確認

```bash
cargo build
```

コンパイラが呼び出し側の修正を要求するエラーを出す可能性があります。次のタスクで修正します。

---

## タスク9: main.rs に統合

### 🎯 目標
設定ファイルの読み込みとシステムプロンプトの使用を main.rs に統合する

### 📝 手順

#### 9.1 use 文を追加

`src/main.rs` の先頭の use 文に追加：

```rust
use config::Config;
use system_prompt::build_system_prompt;
```

#### 9.2 main 関数を更新

`src/main.rs` の main 関数を以下のように更新：

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // ロギング初期化
    tracing_subscriber::fmt()
        .with_env_filter("coding_agent_example=debug")
        .init();

    // .envファイルのロード（失敗時も継続）
    dotenvy::dotenv().ok();

    // 設定ファイルの読み込み
    let config = Config::load()
        .context("Failed to load configuration")?;

    // CLI引数のパース
    let args = Args::parse();

    // APIキーの検証
    if args.api_key.is_empty() {
        anyhow::bail!("ANTHROPIC_API_KEY is required. Set via environment variable or --api-key flag.");
    }

    // モデルの決定（CLI引数 > 設定ファイル）
    let model = if args.model == "claude-3-5-sonnet-20241022" {
        // デフォルト値のままなら設定ファイルの値を使う
        config.model.default.clone()
    } else {
        args.model.clone()
    };

    tracing::info!("Using model: {}", model);
    tracing::info!("Sending message to Claude API");

    // システムプロンプトの構築
    let system_prompt = build_system_prompt();

    // Anthropic APIクライアントの作成
    let client = AnthropicClient::new(args.api_key);

    // メッセージの送信（システムプロンプト付き）
    let response = client
        .create_message(&model, args.max_tokens, &args.message, Some(system_prompt))
        .await
        .context("Failed to communicate with Claude API")?;

    // レスポンスの表示
    for content in &response.content {
        if content.content_type == "text" {
            println!("{}", content.text);
        }
    }

    // トークン使用量の表示
    tracing::info!(
        "Tokens used: {} input, {} output",
        response.usage.input_tokens,
        response.usage.output_tokens
    );

    Ok(())
}
```

### 💡 Rust知識ポイント

**1. 設定の優先順位の実装**

```rust
let model = if args.model == "claude-3-5-sonnet-20241022" {
    config.model.default.clone()
} else {
    args.model.clone()
};
```

CLI引数がデフォルト値のままなら設定ファイルの値を使用します。これにより：
- CLI引数で指定 → その値を使用
- CLI引数なし → 設定ファイルの値を使用
- 設定ファイルもなし → デフォルト値

**2. `.clone()` の使用**

```rust
config.model.default.clone()
```

`String` は `Copy` トレイトを実装していないため、値をコピーするには `clone()` を呼ぶ必要があります。

**3. `Some(system_prompt)` の渡し方**

```rust
client.create_message(&model, args.max_tokens, &args.message, Some(system_prompt)).await?;
```

`Option<String>` を期待する引数に `String` を渡すため、`Some()` でラップします。

### ✅ 動作確認

```bash
cargo run -- "Hello, Claude!"
```

**期待される結果:**
- システムプロンプトがAPIに送られる（デバッグログで確認可能）
- Claudeからの応答が表示される

デバッグログを有効にして確認：
```bash
RUST_LOG=debug cargo run -- "Hello"
```

---

## タスク10: 制御文字サニタイズを実装

### 🎯 目標
LLM出力に含まれる制御文字を除去する関数を実装する

### 📝 手順

#### 10.1 ユーティリティ関数を追加

`src/tools/mod.rs` に以下を追加（または新しく `src/utils.rs` を作成）：

```rust
/// Remove control characters from string (except \n, \r, \t)
pub fn sanitize_content(content: &str) -> String {
    content
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
        .collect()
}

#[cfg(test)]
mod sanitize_tests {
    use super::*;

    #[test]
    fn test_sanitize_preserves_normal_text() {
        let input = "Hello, World!\nNew line\tTab";
        let result = sanitize_content(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_sanitize_removes_control_chars() {
        let input = "Hello\x06World";  // \x06 is ACK control character
        let result = sanitize_content(input);
        assert_eq!(result, "HelloWorld");
    }

    #[test]
    fn test_sanitize_preserves_newlines() {
        let input = "Line1\nLine2\r\nLine3";
        let result = sanitize_content(input);
        assert_eq!(result, input);
    }
}
```

#### 10.2 write_file.rs で使用

`src/tools/write_file.rs` のファイル書き込み前にサニタイズを適用：

```rust
// 変更前
tokio::fs::write(&args.path, &args.content).await?;

// 変更後
use crate::tools::sanitize_content;
let sanitized = sanitize_content(&args.content);
tokio::fs::write(&args.path, &sanitized).await?;
```

同様に `edit_file.rs` にも適用します。

### 💡 Rust知識ポイント

**1. `.chars()` メソッド**

```rust
content.chars()
```

文字列を `char` のイテレータに変換します。UTF-8のマルチバイト文字も正しく1文字として扱います。

**2. `.filter()` メソッド**

```rust
.filter(|c| !c.is_control() || *c == '\n')
```

クロージャが `true` を返す要素のみを残します。

**3. クロージャと参照**

```rust
|c| !c.is_control() || *c == '\n'
// c は &char 型
// *c で char 型に逆参照
```

イテレータの `filter()` はクロージャに参照を渡すため、値を比較するには `*` で逆参照が必要です。

**4. `.is_control()` メソッド**

```rust
c.is_control()
```

Unicode制御文字（`\x00`〜`\x1F`、`\x7F`〜`\x9F`）かどうかを判定します。

**5. `.collect()` メソッド**

```rust
.collect::<String>()
// または型推論で
.collect()
```

イテレータの要素を集めて新しいコレクションを作成します。`char` のイテレータから `String` を作成できます。

### ✅ 動作確認

```bash
cargo test sanitize
```

**期待される結果:**
```
running 3 tests
test tools::sanitize_tests::test_sanitize_preserves_normal_text ... ok
test tools::sanitize_tests::test_sanitize_removes_control_chars ... ok
test tools::sanitize_tests::test_sanitize_preserves_newlines ... ok

test result: ok. 3 passed
```

---

## タスク11: 対話的モデル切り替えを実装

### 🎯 目標
実行中に `/model` コマンドでモデルを切り替えられるようにする

### 📝 手順

この機能は、REPLモード（対話モード）がある場合に実装します。
現在の単発実行モードでは、CLI引数で指定する形になります。

#### 11.1 コマンド解析関数を追加（将来のREPLモード用）

`src/main.rs` に以下を追加（または別ファイルに）：

```rust
/// Handle special commands
fn handle_command(input: &str, config: &mut Config) -> Option<String> {
    let input = input.trim();

    // /model コマンド
    if input.starts_with("/model ") {
        let new_model = input.trim_start_matches("/model ").trim();
        if new_model.is_empty() {
            return Some(format!("Current model: {}", config.model.default));
        }

        config.model.default = new_model.to_string();
        if let Err(e) = config.save() {
            return Some(format!("Model switched to: {} (failed to save: {})", new_model, e));
        }
        return Some(format!("Model switched to: {} (saved)", new_model));
    }

    // /config コマンド
    if input == "/config" {
        return Some(format!("Current configuration:\n{:#?}", config));
    }

    // コマンドではない
    None
}
```

### 💡 Rust知識ポイント

**1. `&mut` 参照**

```rust
fn handle_command(input: &str, config: &mut Config) -> Option<String>
```

`&mut` はミュータブルな借用で、値を変更できます。

```rust
config.model.default = new_model.to_string();  // 変更可能
```

**2. `starts_with()` と `trim_start_matches()`**

```rust
if input.starts_with("/model ") {
    let new_model = input.trim_start_matches("/model ").trim();
}
```

- `starts_with()`: プレフィックスの存在を確認
- `trim_start_matches()`: プレフィックスを削除

**3. `{:#?}` フォーマット指定子**

```rust
format!("{:#?}", config)
```

- `{:?}`: Debug形式（1行）
- `{:#?}`: Debug形式（整形・改行あり）

```
// {:?} の出力
Config { model: ModelConfig { default: "claude-sonnet" }, agent: AgentConfig { max_iterations: 10 } }

// {:#?} の出力
Config {
    model: ModelConfig {
        default: "claude-sonnet",
    },
    agent: AgentConfig {
        max_iterations: 10,
    },
}
```

### ✅ 動作確認

現在は単発実行モードなので、CLI引数でモデルを指定：

```bash
# デフォルトモデル
cargo run -- "Hello"

# モデル指定
cargo run -- --model claude-haiku-3-5-20241022 "Hello"
```

---

## タスク12: E2Eテストと検証

### 🎯 目標
全ての機能が正しく動作することを確認する

### 📝 検証項目

#### 12.1 設定ファイルの動作確認

```bash
# 設定ファイルの場所を確認
cat ~/.codex/config.toml

# 設定ファイルがなければ作成される
cargo run -- "test" 2>&1 | grep -i config
```

#### 12.2 システムプロンプトの動作確認

```bash
# デバッグログでシステムプロンプトが送られていることを確認
RUST_LOG=debug cargo run -- "List the files in the current directory"
```

期待される動作：
- エージェントが `listFiles` ツールを使用してディレクトリを確認
- 推測ではなく実際のファイル構造を報告

#### 12.3 ツール使用の確認

```bash
# 複数ファイルに関わるタスクを依頼
cargo run -- "Read the contents of Cargo.toml"
```

期待される動作：
- エージェントが `readFile` を使用
- ファイル内容を正確に報告

#### 12.4 エラーハンドリングの確認

```bash
# 存在しないファイルを読む
cargo run -- "Read the contents of nonexistent.txt"

# APIキーなし
ANTHROPIC_API_KEY="" cargo run -- "test"
```

### ✅ 最終確認チェックリスト

- [ ] `cargo build` が成功する
- [ ] `cargo test` がすべて成功する
- [ ] `cargo clippy -- -D warnings` がエラーなし
- [ ] `cargo fmt` を実行済み
- [ ] 設定ファイルが `~/.codex/config.toml` に作成される
- [ ] システムプロンプトがAPIリクエストに含まれる
- [ ] エージェントが情報収集を先に行う
- [ ] 制御文字がファイルに書き込まれない

---

## 🎉 完成！

おめでとうございます！システムプロンプトと設定管理機能が完成しました。

### 📊 実装の全体像

```
src/
├── main.rs              # CLIエントリーポイント、設定読み込み
├── anthropic.rs         # APIクライアント（system フィールド追加）
├── config.rs            # 設定管理モジュール
│   ├── Config構造体
│   ├── load() / save()
│   └── config_path()
├── system_prompt.rs     # システムプロンプトビルダー
│   └── build_system_prompt()
└── tools/
    ├── mod.rs           # sanitize_content() 追加
    ├── write_file.rs    # サニタイズ適用
    └── edit_file.rs     # サニタイズ適用

~/.codex/
└── config.toml          # ユーザー設定
```

### 🚀 次のステップ

#### Phase 6: ツール呼び出し対応の改善
エージェントが複数のツールを連続して呼び出せるようにする（マルチターン対話）

#### Phase 7: メモリ・コンテキスト管理
会話履歴の管理と長期記憶の実装

---

## 📚 学んだRustの概念まとめ

### 新しい概念（Chapter 5）

| 概念 | 用途 | 説明 |
|------|------|------|
| `dirs` クレート | ホームディレクトリ | クロスプラットフォームパス解決 |
| `toml` クレート | 設定ファイル | TOML形式の読み書き |
| `#[serde(default)]` | デシリアライズ | 欠けているフィールドにデフォルト値 |
| `#[serde(skip_serializing_if)]` | シリアライズ | 条件付きでフィールドを除外 |
| 生文字列 `r#"..."#` | 長文文字列 | エスケープ不要の複数行文字列 |
| `.chars().filter().collect()` | 文字列変換 | イテレータによる文字列処理 |
| `PathBuf::join()` | パス操作 | クロスプラットフォームパス結合 |
| `Option<T>` | 任意の値 | `Some(value)` または `None` |

### 復習した概念

| 概念 | Chapter | Chapter 5での使用 |
|------|---------|------------------|
| `anyhow::Context` | 1 | 設定ファイル読み込みエラー |
| `serde::{Serialize, Deserialize}` | 1-4 | Config構造体 |
| モジュール (`mod`) | 1 | config, system_prompt モジュール |
| `Result<T, E>` | 全章 | 設定操作全般 |
| ユニットテスト | 1 | 設定のテスト |

---

## 💡 参考リソース

### 公式ドキュメント
- [toml crate](https://docs.rs/toml/latest/toml/)
- [dirs crate](https://docs.rs/dirs/latest/dirs/)
- [serde attributes](https://serde.rs/attributes.html)

### プロンプトエンジニアリング
- [Prompt Engineering Guide](https://www.promptingguide.ai/)
- [Anthropic Prompt Engineering](https://docs.anthropic.com/en/docs/build-with-claude/prompt-engineering)

### プロジェクト内ドキュメント
- `docs/codex.md` - Codexの設計パターン
- `docs/ch5/ch5_system_prompt.md` - この章の概要ドキュメント
