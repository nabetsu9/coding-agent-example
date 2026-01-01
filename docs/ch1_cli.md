# Chapter 1: OpenAIと会話する最初のCLI

このドキュメントでは、Rustを使用してOpenAI APIと対話するCLIツールを実装するための基本的な始め方と技術選定について説明します。

## 1. Rustプロジェクトの基本的な始め方

### プロジェクトの作成

```bash
# 新しいRustプロジェクトを作成
cargo new nebula --bin
cd nebula

# または、複数クレート構成にする場合（推奨）
mkdir nebula
cd nebula
cargo init
```

### 基本的なディレクトリ構造

```
nebula/
├── Cargo.toml          # プロジェクトメタデータと依存関係
├── Cargo.lock          # 依存関係のロックファイル
├── src/
│   └── main.rs         # メインエントリーポイント
└── README.md
```

## 2. CLIライブラリの技術選定（codex.md参照）

codex.mdの推奨事項を基に、以下のライブラリを選定します。

### 必須ライブラリ

#### CLI引数パース
- **`clap` v4.x** (Derive API)
  - 型安全な引数パース
  - 自動ヘルプ/バージョン生成
  - シェル補完サポート
  - ベストプラクティス: Derive APIによる宣言的な定義

#### 非同期ランタイム
- **`tokio` v1.x** (full features)
  - マルチスレッド非同期処理
  - HTTPクライアントと相性が良い
  - 高性能な非同期I/O

#### HTTPクライアント
- **`reqwest` v0.12.x**
  - OpenAI APIとの通信
  - SSEストリーミング対応
  - JSON自動シリアライゼーション

#### エラーハンドリング
- **`anyhow` v1.x**
  - 簡潔なエラー処理
  - コンテキスト情報の付与
  - `?` オペレータによるエラー伝播

#### シリアライゼーション
- **`serde` v1.x** + **`serde_json` v1.x**
  - JSON形式のAPI通信
  - 型安全なシリアライゼーション/デシリアライゼーション

#### ロギング
- **`tracing` v0.1.x** + **`tracing-subscriber` v0.3.x**
  - 構造化ロギング
  - レベル別のログ出力（error, warn, info, debug）

### Cargo.tomlの例

```toml
[package]
name = "nebula"
version = "0.1.0"
edition = "2021"
rust-version = "1.75.0"

[dependencies]
# CLI
clap = { version = "4.5", features = ["derive"] }

# 非同期処理
tokio = { version = "1.48", features = ["full"] }

# HTTP
reqwest = { version = "0.12", features = ["json", "stream"] }

# エラーハンドリング
anyhow = "1.0"

# シリアライゼーション
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# ロギング
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[profile.release]
lto = true
codegen-units = 1
strip = true
```

## 3. 基本的な実装の骨組み

### src/main.rs の例

```rust
use clap::Parser;
use anyhow::Result;

/// OpenAI CLI Agent
#[derive(Debug, Parser)]
#[clap(author, version, about = "OpenAI CLI Agent")]
struct Cli {
    /// OpenAI API key
    #[clap(long, env = "OPENAI_API_KEY")]
    api_key: String,

    /// Model to use (e.g., gpt-4o)
    #[clap(long, default_value = "gpt-4o")]
    model: String,

    /// User message
    message: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // ロギング初期化
    tracing_subscriber::fmt()
        .with_env_filter("nebula=debug")
        .init();

    // CLI引数のパース
    let cli = Cli::parse();

    tracing::info!("Starting nebula CLI");
    tracing::debug!(?cli, "Parsed CLI arguments");

    // TODO: OpenAI API呼び出し実装

    Ok(())
}
```

## 4. 開発ワークフロー（codex.md推奨）

### 品質管理ツールの設定

```bash
# Formatterの実行
cargo fmt

# Linterの実行（厳格なルール）
cargo clippy -- -D warnings

# テストの実行
cargo test

# ビルド
cargo build --release
```

### 推奨設定ファイル

#### `.clippy.toml` (unwrap/expect禁止)

codex.mdの推奨に従い、`unwrap()`と`expect()`を禁止します。

```toml
disallowed-methods = [
    { path = "std::option::Option::unwrap", reason = "use ? or proper error handling" },
    { path = "std::result::Result::unwrap", reason = "use ? or proper error handling" },
    { path = "std::result::Result::expect", reason = "use context() instead" },
]
```

#### `rustfmt.toml` (コードフォーマット設定)

```toml
edition = "2021"
max_width = 100
tab_spaces = 4
newline_style = "Unix"
```

## 5. 設計原則（codex.md参照）

### Rustベストプラクティス

#### 安全性
- `unwrap()` / `expect()` 禁止（Clippyルールで強制）
- エラーハンドリングは`anyhow::Result`で統一
- メモリ安全性（所有権、借用チェッカー）

#### 非同期処理
- Tokioマルチスレッドランタイム
- `async/await` による直感的なコード
- チャネル（mpsc）による非同期通信

#### エラーハンドリングパターン

```rust
use anyhow::{Result, Context, bail};

async fn example_function() -> Result<()> {
    // contextによる文脈情報の付与
    let config = load_config()
        .await
        .context("failed to load configuration")?;

    // bailによる早期リターン
    if config.api_key.is_empty() {
        bail!("API key is required");
    }

    Ok(())
}
```

### CLIベストプラクティス

#### ユーザビリティ
- 自動ヘルプ生成
- シェル補完サポート
- 明確なエラーメッセージ

#### 設定管理
- 環境変数のサポート（`#[clap(env = "...")]`）
- デフォルト値の設定
- 階層化された設定システム

#### プログレッシブディスクロージャー
- デフォルトでシンプルなインターフェース
- サブコマンドで詳細制御
- 段階的な機能公開

## 6. TDD原則に従った開発フロー

### 開発サイクル

1. **Red**: 失敗するテストを先に書く
2. **Green**: テストが通る最小限のコードを書く
3. **Refactor**: コードを改善し、ハードコードを排除

### テスト例

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        // Given
        let args = vec!["nebula", "--model", "gpt-4o", "Hello"];

        // When
        let cli = Cli::parse_from(args);

        // Then
        assert_eq!(cli.model, "gpt-4o");
        assert_eq!(cli.message, "Hello");
    }
}
```

## 7. 次のステップ

このプロジェクトを進めるにあたり、以下の実装を順次行います。

### Phase 1: 基本実装
1. **プロジェクトセットアップ**
   - Cargo.tomlの設定
   - 基本的なCLI構造の実装
   - 品質管理ツールの設定

2. **OpenAI APIクライアントの実装**
   - HTTPリクエストの送信
   - レスポンスのパース
   - エラーハンドリング

### Phase 2: 機能拡張
3. **ストリーミングレスポンスの処理**
   - SSEストリーミングの実装
   - リアルタイムレスポンス表示

4. **会話履歴の管理**
   - メッセージの保存と読み込み
   - セッション管理

### Phase 3: インタラクティブ化
5. **インタラクティブモードの追加**
   - REPLループの実装
   - 複数ターンの会話

6. **高度な機能**
   - ツール呼び出し対応
   - Model Context Protocol (MCP) 統合

## 8. 参考リソース

### Rust CLI開発
- [How to Build a High-Performance CLI Tool in Rust](https://codezup.com/building-a-high-performance-cli-tool-in-rust/)
- [Writing a CLI Tool in Rust with Clap - Shuttle](https://www.shuttle.dev/blog/2023/12/08/clap-rust)
- [Tokio Rust Guide 2025](https://generalistprogrammer.com/tutorials/tokio-rust-crate-guide)

### OpenAI API
- [OpenAI API Documentation](https://platform.openai.com/docs/api-reference)
- [OpenAI Chat Completions API](https://platform.openai.com/docs/api-reference/chat)

### Codex参考実装
- codex.mdに記載されているアーキテクチャパターン
- イベント駆動設計
- タスクベースワークフロー

## まとめ

このChapter 1では、Rustを使用してOpenAI APIと対話する基本的なCLIツールの基盤を構築しました。次のChapterでは、実際のOpenAI API呼び出しとストリーミングレスポンスの実装に進みます。
