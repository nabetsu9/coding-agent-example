# Anthropic API連携 実装ガイド（Rust初心者向け）

このガイドでは、Anthropic Claude APIと連携するCLIツールを、タスクごとに分割して段階的に実装していきます。各タスクで動作確認をしながら進めることで、確実に理解を深められます。

## 📋 全体の流れ

```
タスク1: 現在のエラーを理解する
  ↓
タスク2: main.rsの文法エラーを修正（最小限）
  ↓ 動作確認: cargo build が成功
  ↓
タスク3: CLI引数をAnthropic API用に変更
  ↓ 動作確認: --help が正しく表示
  ↓
タスク4: 非同期処理に対応
  ↓ 動作確認: cargo run が成功
  ↓
タスク5: anthropic.rs モジュールを作成（基本構造）
  ↓ 動作確認: cargo build が成功
  ↓
タスク6: APIリクエスト/レスポンス構造体を定義
  ↓ 動作確認: cargo build が成功
  ↓
タスク7: AnthropicClient を実装
  ↓ 動作確認: cargo build が成功
  ↓
タスク8: main.rs から anthropic.rs を呼び出す
  ↓ 動作確認: 実際にAPIが呼べる
  ↓
タスク9: エラーハンドリングとロギングを改善
  ↓ 動作確認: エラーメッセージが分かりやすい
  ↓
タスク10: コード品質チェック
  ✓ 完成！
```

---

## タスク1: 現在のエラーを理解する

### 🎯 目標
現在のコードがなぜコンパイルエラーになるのかを理解する

### 📝 手順

#### 1.1 エラーを確認
```bash
cargo build
```

#### 1.2 エラーメッセージを読む
以下のようなエラーが表示されます：

```
error: cannot find derive macro `Parser` in this scope
error: cannot find attribute `command` in this scope
error: cannot find attribute `arg` in this scope
```

### 💡 Rust知識ポイント

**Derive マクロとは？**
- Rustでは `#[derive(...)]` を使って、構造体に自動的に機能を追加できます
- `Parser` は clap クレートが提供する derive マクロで、CLI引数のパース機能を自動生成します
- **重要**: derive マクロを使うには、そのクレートで `features = ["derive"]` を有効にする必要があります

**現在の問題:**
- `Cargo.toml` で `clap = { version = "4.5.53", features = ["derive"] }` と設定されているので、この問題は既に解決済み

#### 1.3 main.rs の問題箇所を確認

`src/main.rs` の23行目を見てください：
```rust
apiKey = os.Getenv("ANTHROPIC_API_KEY");
```

### ❌ 問題点
これは **Go言語** の構文です！Rustでは動作しません。

**Go言語（間違い）:**
```go
apiKey = os.Getenv("ANTHROPIC_API_KEY")
```

**Rust（正しいパターン - 基本形）:**
```rust
// パターン1: unwrap_or_default() を使う（エラー時は空文字列）
let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();

// パターン2: Result<T, E> を返す関数内で使う（推奨）
let api_key = std::env::var("ANTHROPIC_API_KEY")
    .context("ANTHROPIC_API_KEY environment variable not set")?;
```

### 💡 重要: `unwrap()` を避けるべき理由

**❌ 避けるべき:**
```rust
let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap();
```

**なぜダメ？**
- 環境変数が設定されていない場合、プログラムがパニック（クラッシュ）します
- エラーメッセージが分かりにくい: `thread 'main' panicked at 'called Result::unwrap() on an Err value'`
- 適切なエラーハンドリングができません

**✅ 正しいパターン:**
```rust
// 1. unwrap_or_default() - エラー時はデフォルト値
let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();

// 2. ? オペレータ - エラーを呼び出し元に返す（推奨）
let api_key = std::env::var("ANTHROPIC_API_KEY")
    .context("ANTHROPIC_API_KEY environment variable not set")?;

// 3. match で明示的にハンドリング
let api_key = match std::env::var("ANTHROPIC_API_KEY") {
    Ok(key) => key,
    Err(_) => {
        eprintln!("Error: ANTHROPIC_API_KEY not set");
        return Err(anyhow::anyhow!("API key required"));
    }
};
```

### 📚 Rust vs Go の違い

| 項目 | Go | Rust |
|------|-----|------|
| 変数宣言 | `変数名 := 値` または `var 変数名 型` | `let 変数名 = 値;` |
| 命名規則 | キャメルケース (`apiKey`) | スネークケース (`api_key`) |
| 環境変数取得 | `os.Getenv("KEY")` | `std::env::var("KEY")` |
| エラー処理 | 戻り値で `(値, error)` | `Result<T, E>` 型 + `?` |
| パニック回避 | - | `unwrap()` を避け、`?` や `match` を使う |

---

## タスク2: main.rsの文法エラーを修正（最小限）

### 🎯 目標
まずはコンパイルが通る状態にする

### 📝 手順

#### 2.1 main.rsを開く
`src/main.rs` をエディタで開きます。

#### 2.2 18-28行目を以下に置き換える

**変更前:**
```rust
fn main() {
    // load environment variables from .env file
    dotenv().expect(".env file not found");
    let args = Args::parse();
    // 環境変数からAPIキーを取得
    apiKey = os.Getenv("ANTHROPIC_API_KEY");

    for _ in 0..args.count {
        println!("Hello {}!", args.name);
    }
}
```

**変更後:**
```rust
fn main() {
    // load environment variables from .env file
    dotenvy::dotenv().ok();

    let args = Args::parse();

    // 環境変数からAPIキーを取得
    let _api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();

    for _ in 0..args.count {
        println!("Hello {}!", args.name);
    }
}
```

### 💡 Rust知識ポイント

**1. モジュールパスの明示: `dotenvy::dotenv()`**
- `use dotenvy::dotenv;` でインポートしていれば `dotenv()` だけでOK
- インポートしていない場合は `dotenvy::dotenv()` とフルパスで書く必要があります

**2. `.ok()` vs `.expect()`**
- `.expect("message")` は失敗時にプログラムを停止（パニック）させます
- `.ok()` は失敗を無視して `None` を返し、プログラムは継続します
- codex.mdの推奨では `expect()` は禁止されています

**3. `let _api_key`（アンダースコアプレフィックス）**
- 変数名の前に `_` をつけると「この変数は今は使わないけど宣言だけする」という意味
- Rustコンパイラは使われない変数に警告を出しますが、`_` をつけることで警告を抑制できます

**4. `.unwrap_or_default()`**
- `Result<T, E>` や `Option<T>` から値を取り出すメソッド
- 成功時は値を返し、失敗時はデフォルト値（空文字列 `""`）を返します
- **注意:** `.unwrap()` よりは安全ですが、エラーを無視してしまうため、本番コードでは `?` オペレータや明示的なエラーハンドリングが推奨されます
- タスク2では「まずコンパイルを通す」ことが目標なので一時的に使用し、タスク4で適切なエラーハンドリングに改善します

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

## タスク3: CLI引数をAnthropic API用に変更

### 🎯 目標
CLIの引数をAnthropic APIに適した形に変更する

### 📝 手順

#### 3.1 Args構造体を書き換える

`src/main.rs` の6-16行目を以下に置き換えます：

**変更前:**
```rust
/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    name: String,

    /// Number of times to greet
    #[arg(short, long, default_value_t = 1)]
    count: u8,
}
```

**変更後:**
```rust
/// Anthropic Claude CLI Agent
#[derive(Parser, Debug)]
#[command(author, version, about = "Anthropic Claude CLI Agent")]
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
}
```

### 💡 Rust知識ポイント

**1. アトリビュート（属性）とは？**
- `#[...]` で囲まれた記述をアトリビュートと呼びます
- コンパイラやマクロに対する指示を書きます

**2. `#[arg(...)]` の各オプション**

| オプション | 意味 | 例 |
|----------|------|-----|
| `short` | 短いオプション名（1文字） | `-m` |
| `long` | 長いオプション名 | `--model` |
| `env` | 環境変数からも読み込む | `env = "API_KEY"` |
| `default_value` | デフォルト値 | `default_value = "100"` |
| `value_name` | ヘルプでの表示名 | `MESSAGE` |

**3. 位置引数（Positional Argument）**
```rust
#[arg(value_name = "MESSAGE")]
message: String,
```
- `short` も `long` もつけないと、位置で指定する引数になります
- `cargo run -- "こんにちは"` のように使います

**4. 型の違い: `u8` vs `u32`**
- `u8`: 0〜255 の符号なし整数（8ビット）
- `u32`: 0〜4,294,967,295 の符号なし整数（32ビット）
- トークン数は大きくなる可能性があるので `u32` を使います

### ✅ 動作確認

```bash
cargo run -- --help
```

**期待される結果:**
```
Anthropic Claude CLI Agent

Usage: coding-agent-example [OPTIONS] <MESSAGE>

Arguments:
  <MESSAGE>  User message/prompt to send to Claude

Options:
      --api-key <API_KEY>      Anthropic API key [env: ANTHROPIC_API_KEY]
  -m, --model <MODEL>          Model to use [default: claude-3-5-sonnet-20241022]
      --max-tokens <MAX_TOKENS>  Maximum tokens to generate [default: 1024]
  -h, --help                   Print help
  -V, --version                Print version
```

---

## タスク4: 非同期処理に対応

### 🎯 目標
Tokioを使って非同期処理を有効化する

### 📝 手順

#### 4.1 importsを追加

`src/main.rs` の先頭（1行目）に以下を追加：

**変更前:**
```rust
use clap::Parser;
use dotenvy::dotenv;
use std::env;
```

**変更後:**
```rust
use clap::Parser;
use anyhow::{Context, Result};
```

### 💡 Rust知識ポイント

**anyhowクレートとは？**
- エラーハンドリングを簡単にするライブラリ
- `Result<T, E>` の `E` 部分を `anyhow::Error` に統一できます
- `.context("説明")` でエラーに説明を追加できます

**Result型のエイリアス**
```rust
use anyhow::Result;
// これで Result<T> と書くだけで anyhow::Result<T, anyhow::Error> を意味する
```

#### 4.2 main関数を非同期化

**変更前:**
```rust
fn main() {
    // ...
}
```

**変更後:**
```rust
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

    println!("Message: {}", args.message);
    println!("Model: {}", args.model);
    println!("Max tokens: {}", args.max_tokens);

    Ok(())
}
```

### 💡 Rust知識ポイント

**1. `#[tokio::main]` マクロ**
- 非同期ランタイムを自動的にセットアップします
- `async fn main()` を使えるようにします
- 内部では以下のようなコードに展開されます：
  ```rust
  fn main() {
      tokio::runtime::Runtime::new()
          .unwrap()  // ⚠️ マクロ内部でのみ使用（ユーザーコードでは避ける）
          .block_on(async {
              // async fn main() の中身
          })
  }
  ```

  **注意:** マクロが自動生成するコード内では `.unwrap()` が使われていますが、これは「ランタイムの初期化失敗 = プログラムが動かない」という致命的な状況でのみ許容されます。**あなたが書くコードでは `.unwrap()` を避けてください。**

**2. `async fn` とは？**
- 非同期関数を定義するキーワード
- APIリクエストなど、時間がかかる処理を待つ間に他の処理ができます
- `.await` で非同期処理の完了を待ちます

**3. `-> Result<()>`**
- この関数は成功時に `Ok(())` を返します
- `()` は「何も値がない」という意味（Unit型）
- エラー時は `Err(...)` を返します

**4. `anyhow::bail!` マクロ**
- エラーメッセージを作って即座に関数から返ります
- 以下と同じ意味です：
  ```rust
  return Err(anyhow::anyhow!("エラーメッセージ"));
  ```

**5. tracing_subscriber**
- ログを表示するための初期化コード
- `RUST_LOG=debug` 環境変数でログレベルを制御できます

### ✅ 動作確認

```bash
cargo run -- "こんにちは、Claude！"
```

**期待される結果:**
```
Message: こんにちは、Claude！
Model: claude-3-5-sonnet-20241022
Max tokens: 1024
```

エラーケースも確認：
```bash
ANTHROPIC_API_KEY="" cargo run -- "test"
```

**期待される結果:**
```
Error: ANTHROPIC_API_KEY is required. Set via environment variable or --api-key flag.
```

---

## タスク5: anthropic.rs モジュールを作成（基本構造）

### 🎯 目標
新しいファイル `src/anthropic.rs` を作成し、基本構造を定義する

### 📝 手順

#### 5.1 新しいファイルを作成

`src/anthropic.rs` という新しいファイルを作成します。

#### 5.2 基本構造を記述

```rust
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Anthropic API client
pub struct AnthropicClient {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl AnthropicClient {
    /// Create new Anthropic API client
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.anthropic.com/v1".to_string(),
            client: reqwest::Client::new(),
        }
    }
}
```

### 💡 Rust知識ポイント

**1. `pub` キーワード**
- `pub` をつけると、他のモジュールから使えるようになります（公開）
- つけないと、そのモジュール内でしか使えません（プライベート）

```rust
pub struct AnthropicClient { ... }  // 他のファイルから使える
struct PrivateStruct { ... }         // このファイル内でしか使えない
```

**2. 構造体（struct）**
```rust
pub struct AnthropicClient {
    api_key: String,      // フィールド1
    base_url: String,     // フィールド2
    client: reqwest::Client,  // フィールド3
}
```
- データをまとめて管理する仕組み
- 他の言語の「クラス」に似ています（ただしメソッドは別に定義）

**3. impl ブロック**
```rust
impl AnthropicClient {
    pub fn new(api_key: String) -> Self { ... }
}
```
- 構造体にメソッド（関数）を実装します
- `Self` は `AnthropicClient` 自身を指します

**4. コンストラクタパターン: `new()`**
- Rustには特別なコンストラクタ構文はありません
- 慣習的に `new()` という関連関数を作ります
- `Self { ... }` で新しいインスタンスを作成して返します

**5. `reqwest::Client::new()`**
- HTTPリクエストを送るためのクライアントを作成
- 再利用可能なので、構造体のフィールドとして保持します

#### 5.3 main.rsでモジュールを読み込む

`src/main.rs` の先頭（use文の前）に追加：

```rust
mod anthropic;
use anthropic::AnthropicClient;
```

### 💡 Rust知識ポイント

**モジュールシステム**

```rust
mod anthropic;  // src/anthropic.rs を読み込む
use anthropic::AnthropicClient;  // AnthropicClient を使えるようにする
```

- `mod anthropic;` は「anthropic.rs というファイルをモジュールとして読み込む」という宣言
- `use anthropic::AnthropicClient;` で、`AnthropicClient` を短い名前で使えるようになります

### ✅ 動作確認

```bash
cargo build
```

**期待される結果:**
```
   Compiling coding-agent-example v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 1.23s
```

警告が出るかもしれませんが（未使用のコードなど）、エラーがなければOKです。

---

## タスク6: APIリクエスト/レスポンス構造体を定義

### 🎯 目標
Anthropic APIとやり取りするためのデータ構造を定義する

### 📝 手順

#### 6.1 src/anthropic.rs に構造体を追加

`AnthropicClient` 構造体の **前** に、以下を追加します：

```rust
/// Request structure for Messages API
#[derive(Debug, Serialize)]
struct MessageRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

/// Response structure
#[derive(Debug, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
```

### 💡 Rust知識ポイント

**1. `#[derive(Debug, Serialize)]`**
- `Debug`: `println!("{:?}", value)` でデバッグ表示できるようにします
- `Serialize`: この構造体をJSONに変換できるようにします（送信用）
- `Deserialize`: JSONからこの構造体に変換できるようにします（受信用）

**2. `Vec<T>` 型**
```rust
messages: Vec<Message>,
```
- 動的配列（他の言語のList/Array）
- `Message` 型の要素を複数持てます
- `Vec<T>` の `T` はジェネリック型パラメータ

**3. `Option<T>` 型**
```rust
pub stop_reason: Option<String>,
```
- 値があるかないかを表現する型
- `Some(値)` または `None` のどちらか
- 他の言語の `null` に相当しますが、型安全です

**4. `#[serde(rename = "type")]`**
```rust
#[serde(rename = "type")]
pub content_type: String,
```
- JSONのフィールド名と構造体のフィールド名が異なる場合に使います
- `type` はRustの予約語なので `content_type` という名前にして、JSON上では `"type"` として扱います

**5. 可視性の使い分け**
```rust
struct MessageRequest { ... }       // private（このファイル内でのみ使用）
pub struct MessageResponse { ... }  // public（main.rsから使える）
```
- リクエスト構造体は内部実装なので private
- レスポンス構造体は外部に返すので public

### ✅ 動作確認

```bash
cargo build
```

エラーがなければ成功です。

---

## タスク7: AnthropicClient を実装

### 🎯 目標
実際にAPIリクエストを送信するメソッドを実装する

### 📝 手順

#### 7.1 create_message メソッドを追加

`impl AnthropicClient` ブロック内に、以下のメソッドを追加：

```rust
    /// Send a message to Claude (non-streaming)
    pub async fn create_message(
        &self,
        model: &str,
        max_tokens: u32,
        user_message: &str,
    ) -> Result<MessageResponse> {
        debug!("Preparing request to Anthropic API");
        debug!(?model, ?max_tokens, "Request parameters");

        let request = MessageRequest {
            model: model.to_string(),
            max_tokens,
            messages: vec![Message {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
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
```

### 💡 Rust知識ポイント

**1. `&self` パラメータ**
```rust
pub async fn create_message(&self, ...) -> Result<MessageResponse>
```
- メソッドの第一引数は必ず `self` 関連
- `&self`: 借用（読み取り専用）
- `&mut self`: ミュータブルな借用（変更可能）
- `self`: 所有権を奪う（メソッド呼び出し後に使えなくなる）

**2. `&str` vs `String`**
```rust
model: &str,          // 文字列スライス（借用）
user_message: &str,   // 文字列スライス（借用）
```

| 型 | 説明 | 使い分け |
|----|------|---------|
| `String` | 所有権を持つ文字列 | 文字列を保持する |
| `&str` | 文字列への参照 | 引数として受け取る |

**3. `.to_string()` メソッド**
```rust
model: model.to_string(),
```
- `&str` から `String` を作成します
- メモリを新しく確保してコピーします

**4. `vec!` マクロ**
```rust
messages: vec![Message { ... }],
```
- ベクタ（配列）を簡単に作成するマクロ
- `vec![要素1, 要素2, ...]` と書きます

**5. メソッドチェーン**
```rust
let response = self.client
    .post(...)
    .header(...)
    .header(...)
    .json(...)
    .send()
    .await?;
```
- Rustでは各メソッドが `self` を返すことで、連続して呼び出せます
- 読みやすく、fluent APIパターンと呼ばれます

**6. `.await` と `?`**
```rust
.send()
.await
.context("...")?;
```
- `.await`: 非同期処理が完了するまで待つ
- `?`: エラーの場合は早期リターンする（エラー伝播）
- この2つはよくセットで使います

**7. `format!` マクロ**
```rust
format!("{}/messages", self.base_url)
```
- 文字列を整形して新しい `String` を作成
- `println!` と似ていますが、表示ではなく文字列を返します

**8. HTTPヘッダー**
```rust
.header("x-api-key", &self.api_key)
.header("anthropic-version", "2023-06-01")
.header("content-type", "application/json")
```
- Anthropic APIの要求仕様に従ったヘッダー
- `x-api-key`: 認証用のAPIキー
- `anthropic-version`: API バージョン指定
- `content-type`: リクエストボディの形式

**9. `.json::<MessageResponse>()`**
```rust
let message_response = response
    .json::<MessageResponse>()
    .await?;
```
- レスポンスのJSONを `MessageResponse` 構造体に変換
- `::<Type>` はターボフィッシュ構文と呼ばれ、型を明示的に指定します

### ✅ 動作確認

```bash
cargo build
```

警告があっても、エラーがなければOKです。

---

## タスク8: main.rs から anthropic.rs を呼び出す

### 🎯 目標
実際にAnthropic APIを呼び出して、レスポンスを表示する

### 📝 手順

#### 8.1 main関数を更新

`src/main.rs` の main関数を以下に書き換えます：

```rust
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

    tracing::info!("Sending message to Claude API");

    // Anthropic APIクライアントの作成
    let client = AnthropicClient::new(args.api_key);

    // メッセージの送信
    let response = client
        .create_message(&args.model, args.max_tokens, &args.message)
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

**1. `for` ループと参照**
```rust
for content in &response.content {
    // ...
}
```
- `&` をつけることで、ベクタの各要素を借用します
- `&` がないと所有権が移動してしまい、ループ後に `response.content` が使えなくなります

**2. フィールドアクセス**
```rust
content.content_type
content.text
response.usage.input_tokens
```
- `.` でフィールドにアクセスします
- ネストした構造体も `.` で辿れます

**3. `tracing::info!` マクロ**
```rust
tracing::info!(
    "Tokens used: {} input, {} output",
    response.usage.input_tokens,
    response.usage.output_tokens
);
```
- ログメッセージを出力します
- `println!` と似た構文ですが、構造化ログとして記録されます

### ✅ 動作確認

#### 1. ビルド確認
```bash
cargo build
```

#### 2. 実行（環境変数を使う場合）
```bash
cargo run -- "こんにちは、Claudeさん！"
```

**期待される結果:**
```
2025-12-31T04:30:00.123Z  INFO coding_agent_example: Sending message to Claude API
こんにちは！お元気ですか？私はClaude、Anthropicが開発したAIアシスタントです。
2025-12-31T04:30:01.456Z  INFO coding_agent_example: Tokens used: 25 input, 42 output
```

#### 3. APIキーを明示的に指定
```bash
cargo run -- --api-key "sk-ant-..." "Rustについて教えて"
```

#### 4. モデルを指定
```bash
cargo run -- --model "claude-3-5-sonnet-20241022" "俳句を作って"
```

#### 5. デバッグログを有効化
```bash
RUST_LOG=debug cargo run -- "テスト"
```

### 🐛 よくあるエラーと対処法

**エラー1: 401 Unauthorized**
```
Error: API request failed with status 401 Unauthorized
```
**原因:** APIキーが間違っているか、設定されていません
**対処法:**
- `.env` ファイルの `ANTHROPIC_API_KEY` を確認
- 環境変数が正しく読み込まれているか確認（`echo $ANTHROPIC_API_KEY`）

**エラー2: Failed to send request**
```
Error: Failed to send request to Anthropic API
```
**原因:** ネットワーク接続の問題
**対処法:**
- インターネット接続を確認
- プロキシ設定が必要な場合は環境変数 `HTTP_PROXY` を設定

**エラー3: Failed to parse API response**
```
Error: Failed to parse API response
```
**原因:** レスポンス構造体の定義がAPI仕様と一致していません
**対処法:**
- Anthropic APIのドキュメントを確認
- デバッグログで実際のレスポンスを確認（`RUST_LOG=debug`）

---

## タスク9: エラーハンドリングとロギングを改善

### 🎯 目標
より分かりやすいエラーメッセージとログを実装する

現在の実装で、既に以下のベストプラクティスが適用されています：
- ✅ `anyhow::Result` によるエラーハンドリング
- ✅ `.context()` でエラーに説明を追加
- ✅ `tracing` によるログ出力
- ✅ `unwrap()` / `expect()` を避ける

### 📝 追加の改善点

#### 9.1 APIキーのマスキング（セキュリティ向上）

デバッグログにAPIキーが表示されないようにします。

`src/anthropic.rs` の `AnthropicClient::new()` を修正：

```rust
pub fn new(api_key: String) -> Self {
    // APIキーの最初の数文字だけログに記録（セキュリティのため）
    let masked_key = if api_key.len() > 8 {
        format!("{}...", &api_key[..8])
    } else {
        "***".to_string()
    };
    debug!("Creating Anthropic client with key: {}", masked_key);

    Self {
        api_key,
        base_url: "https://api.anthropic.com/v1".to_string(),
        client: reqwest::Client::new(),
    }
}
```

### 💡 Rust知識ポイント

**文字列スライス**
```rust
&api_key[..8]  // 最初の8文字を取得
```
- `[開始..終了]` で範囲を指定
- `[..8]` は「0から8文字目まで」
- `[8..]` は「8文字目以降すべて」

### ✅ 動作確認

```bash
RUST_LOG=debug cargo run -- "test"
```

ログに `Creating Anthropic client with key: sk-ant-a...` のように表示されればOK。

---

## タスク10: コード品質チェック

### 🎯 目標
Rustのベストプラクティスに従っているか確認する

### 📝 手順

#### 10.1 フォーマットチェック
```bash
cargo fmt
```

自動的にコードが整形されます。

#### 10.2 Linter実行
```bash
cargo clippy -- -D warnings
```

**期待される結果:**
```
    Checking coding-agent-example v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 1.23s
```

警告やエラーがある場合は、メッセージに従って修正します。

### 💡 よくあるClippy警告と対処法

**警告: `needless_return`**
```rust
// 悪い例
fn example() -> i32 {
    return 42;
}

// 良い例
fn example() -> i32 {
    42
}
```
Rustでは最後の式が自動的に返されます。

**警告: `redundant_field_names`**
```rust
// 悪い例
let client = AnthropicClient { api_key: api_key };

// 良い例
let client = AnthropicClient { api_key };
```
フィールド名と変数名が同じ場合は省略できます。

#### 10.3 テストビルド（リリースモード）
```bash
cargo build --release
```

最適化されたバイナリが `target/release/` に生成されます。

#### 10.4 バイナリサイズ確認
```bash
ls -lh target/release/coding-agent-example
```

### ✅ 最終確認チェックリスト

- [ ] `cargo build` が成功する
- [ ] `cargo clippy -- -D warnings` がエラーなし
- [ ] `cargo fmt` を実行済み
- [ ] `cargo run -- "test"` でAPIが正常に動作する
- [ ] `cargo run -- --help` でヘルプが表示される
- [ ] エラーメッセージが分かりやすい
- [ ] ログが適切に出力される
- [ ] APIキーが `.env` ファイルで管理されている
- [ ] `.gitignore` に `.env` が含まれている

---

## 🎉 完成！

おめでとうございます！Anthropic Claude APIと連携するCLIツールが完成しました。

### 📊 実装の全体像

```
src/
├── main.rs           # CLIエントリーポイント
│   ├── Args構造体    # CLI引数の定義
│   └── main関数      # 全体の流れ制御
└── anthropic.rs      # Anthropic API連携
    ├── AnthropicClient  # APIクライアント
    ├── MessageRequest   # リクエスト構造体
    └── MessageResponse  # レスポンス構造体
```

### 🚀 次のステップ

#### Phase 2: ストリーミング対応
現在の実装では、レスポンスが全て完了してから表示されます。
ストリーミング機能を追加すると、ChatGPTのようにリアルタイムで文字が表示されるようになります。

#### Phase 3: REPLモード
対話型のインターフェースを追加し、連続した会話ができるようにします。

---

## 📚 学んだRustの概念まとめ

### 基本概念
- ✅ 所有権と借用（`&self`, `&str`）
- ✅ 変数宣言（`let`, `mut`）
- ✅ 型システム（`String`, `&str`, `u32`, `Option<T>`, `Result<T, E>`）

### 構文
- ✅ 構造体（`struct`）
- ✅ 列挙型（`enum`） - Result, Option
- ✅ トレイト（`#[derive(...)]`）
- ✅ impl ブロック

### 非同期処理
- ✅ `async fn` / `.await`
- ✅ `#[tokio::main]`

### エラーハンドリング
- ✅ `Result<T, E>` 型
- ✅ `?` オペレータ
- ✅ `anyhow::Context`
- ✅ `bail!` マクロ

### モジュール
- ✅ `mod` / `use`
- ✅ `pub` による可視性制御

### クレート（ライブラリ）
- ✅ `clap` - CLI引数パース
- ✅ `tokio` - 非同期ランタイム
- ✅ `reqwest` - HTTPクライアント
- ✅ `serde` - シリアライゼーション
- ✅ `anyhow` - エラーハンドリング
- ✅ `tracing` - ロギング

---

## 💡 参考リソース

### 公式ドキュメント
- [The Rust Programming Language（日本語版）](https://doc.rust-jp.rs/book-ja/)
- [Rust by Example（日本語版）](https://doc.rust-jp.rs/rust-by-example-ja/)
- [Anthropic API Documentation](https://docs.anthropic.com/)

### プロジェクト内ドキュメント
- `codex.md` - Codexの技術スタック詳細
- `docs/ch1_cli.md` - CLI実装ガイド

---

質問があれば、いつでもお気軽にどうぞ！🦀
