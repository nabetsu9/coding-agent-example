# Chapter 5: システムプロンプトと設定管理

このドキュメントでは、コーディングエージェントの「頭脳」となるシステムプロンプトと、ユーザー設定を永続化する設定管理システムの実装について説明します。

## 1. この章で解決する課題

### Chapter 4までの限界

Chapter 4までのエージェントは基本的なファイル操作ができるようになりましたが、複雑なタスクでは以下の問題に直面します：

```
問題1: 推測による実装
→ 参照ファイルを読まずに「知っているつもり」で実装開始
→ 既存のコードスタイルと異なる実装になる

問題2: 断片的な作業
→ 複数ファイルの変更が必要なタスクで一部を忘れる
→ registry.goへの登録を忘れるなど

問題3: 途中停止
→ 「実装してもよろしいですか？」と途中で確認
→ ユーザーが「はい」と答えるまで作業停止
```

### 根本原因

これらの問題は、LLMに「思考のフレームワーク」を与えていないことが原因です：

- どのように調査すべきかがわからない
- 何を禁止すべきかがわからない
- どの順序で作業すべきかがわからない

### 解決策：システムプロンプト

システムプロンプトを追加することで、以下を実現します：

| 問題 | 解決策 |
|------|--------|
| 推測による実装 | 事実に基づく情報収集を強制 |
| 断片的な作業 | プロジェクト全体を意識した連携実装 |
| 途中停止 | 自動実行による連続的な作業フロー |

## 2. システムプロンプト設計原則

### 効果的なプロンプトの5つのテクニック

#### テクニック1: 強制的な表現を使う

**避けるべき表現:**
```
「できれば」「可能であれば」「必要に応じて」
```

**使うべき表現:**
```
「NEVER」「MUST」「FORBIDDEN」「MANDATORY」「REQUIRED」
```

#### テクニック2: 具体的な禁止事項を明示

```
FORBIDDEN: Guessing file names (e.g., assuming "todo.ts" exists without checking)
FORBIDDEN: Guessing file extensions (e.g., assuming .js when it might be .ts)
FORBIDDEN: Guessing directory structure (e.g., assuming files are in "src/" without checking)
```

#### テクニック3: 段階的な実行プロトコル

```
## Step 1: Information Gathering (Required)
- Use 'listFiles' to understand project structure
- Use 'readFile' to read ALL reference files
- Use 'searchInDirectory' to find related files

## Step 2: Implementation (Proceed automatically after Step 1)
- Use 'writeFile' for new file creation
- Use 'editFile' for existing file modification
```

#### テクニック4: 自動実行の強制

```
IMPORTANT: Proceed from Step 1 to Step 2 automatically
without asking for permission or confirmation.
```

#### テクニック5: 実例による説明（Chain-of-Thought）

```
## Example: Reference File Reading
Request: "Create tools/copyFile.rs based on tools/writeFile.rs"

**Correct sequence:**
1. readFile("tools/writeFile.rs") ← MANDATORY FIRST STEP
2. Analyze the content and structure (silently)
3. writeFile("tools/copyFile.rs", <implementation>) ← PROCEED AUTOMATICALLY

**Incorrect sequence:**
1. writeFile("tools/copyFile.rs", ...) ← FORBIDDEN: Implemented without reading reference
```

### 失敗パターンと成功パターンの比較

| 要素 | 失敗パターン | 成功パターン |
|------|-------------|-------------|
| 表現の強さ | 「できれば」「可能であれば」 | 「NEVER」「MUST」「FORBIDDEN」 |
| 手順の明確さ | 「情報収集を行う」（抽象的） | 「Use readFile ALL reference files」（具体的） |
| 自動実行 | 「実装してもよろしいですか？」 | 「proceed automatically without asking」 |
| 禁止事項 | 一般的な注意事項 | 具体例付きFORBIDDEN項目 |
| 言語 | 日本語での長文指示 | 英語での構造化指示 |

## 3. システムプロンプトの構成

### 3.1 Role（役割定義）

```
You are a Rust coding assistant with access to file system tools.
```

エージェントの身分と能力を明確に定義します。

### 3.2 Critical Rules（非交渉的ルール）

```
## Critical Rules (Non-Negotiable)
1. NEVER assume or guess file contents, names, or locations
2. Information gathering is MANDATORY before implementation
3. Before using writeFile or editFile, you MUST have used readFile on reference files
4. NEVER ask for permission between steps
5. Complete the entire task in one continuous flow
```

### 3.3 Execution Protocol（実行プロトコル）

```
## Execution Protocol

### Step 1: Information Gathering
- Discover project structure: Use 'listFiles'
- Use 'readFile': Read ALL reference files
- Use 'searchInDirectory': Find related files

**Internal Verification:**
□ Have I discovered the project structure?
□ Have I read the reference file contents?
□ Do I understand the existing code structure?

### Step 2: Implementation
- Use 'writeFile' for new files
- Use 'editFile' for existing files
```

### 3.4 Forbidden Patterns（禁止パターン）

```
## Common Mistakes to Avoid
FORBIDDEN: Guessing file names
FORBIDDEN: Guessing file extensions
FORBIDDEN: Guessing directory structure
FORBIDDEN: Implementing without reading reference files
FORBIDDEN: Asking "Should I proceed?" after information gathering
```

## 4. 設定管理アーキテクチャ

### 4.1 設計方針

| 項目 | 選択 | 理由 |
|------|------|------|
| 形式 | TOML | Rustエコシステム標準、コメント可能 |
| 場所 | `~/.codex/config.toml` | Codexパターンに準拠 |
| APIキー | 環境変数のみ | セキュリティのため設定ファイルに保存しない |

### 4.2 設定ファイル構造

```toml
# ~/.codex/config.toml

[model]
default = "claude-sonnet-4-5"  # デフォルトモデル

[agent]
max_iterations = 10            # エージェントループの最大回数
```

### 4.3 設定の優先順位（Codexパターン）

```
1. CLI引数           (最優先)
2. 環境変数
3. 設定ファイル
4. デフォルト値      (最低優先)
```

### 4.4 Config構造体

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub model: ModelConfig,
    #[serde(default)]
    pub agent: AgentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    #[serde(default = "default_model")]
    pub default: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
}
```

## 5. モデル選択機能

### 5.1 対話的モデル切り替え

実行中に `/model` コマンドでモデルを切り替え可能：

```
You: /model claude-haiku-3-5-20241022
Model switched to: claude-haiku-3-5-20241022

You: /model claude-sonnet-4-5
Model switched to: claude-sonnet-4-5
```

### 5.2 モデル選択の指針

| モデル | 特徴 | 使用場面 |
|--------|------|---------|
| claude-haiku | 高速・軽量 | 単一ファイル編集、基本的な操作 |
| claude-sonnet | 複雑タスク対応 | 複数ファイル編集、アーキテクチャ理解 |

## 6. JSON安全性（制御文字処理）

### 6.1 問題

LLMの出力に稀にUnicode制御文字（`\u0006`など）が含まれ、ファイル書き込み時に問題を引き起こすことがあります。

### 6.2 解決策

```rust
/// Remove control characters from string (except \n, \r, \t)
pub fn sanitize_content(content: &str) -> String {
    content
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
        .collect()
}
```

この関数を `writeFile` と `editFile` の書き込み前に適用します。

## 7. アーキテクチャ変更

### 7.1 新規ファイル

```
src/
├── config.rs          # 設定管理モジュール
│   ├── Config構造体
│   ├── load() / save()
│   └── config_path()
└── system_prompt.rs   # システムプロンプトビルダー
    └── build_system_prompt()
```

### 7.2 変更ファイル

| ファイル | 変更内容 |
|----------|----------|
| `Cargo.toml` | `toml`, `dirs` 依存関係追加 |
| `src/anthropic.rs` | `MessageRequest` に `system` フィールド追加 |
| `src/main.rs` | Config読み込み、システムプロンプト統合 |
| `src/tools/write_file.rs` | コンテンツサニタイズ追加 |
| `src/tools/edit_file.rs` | コンテンツサニタイズ追加 |

### 7.3 到達目標の構造

```
codex/
├── src/
│   ├── main.rs              # システムプロンプト統合
│   ├── anthropic.rs         # system フィールド追加
│   ├── config.rs            # 新規: 設定管理
│   ├── system_prompt.rs     # 新規: プロンプトビルダー
│   └── tools/
│       ├── mod.rs           # sanitize_content 追加
│       ├── read_file.rs
│       ├── list_files.rs
│       ├── search_in_directory.rs
│       ├── write_file.rs    # サニタイズ適用
│       └── edit_file.rs     # サニタイズ適用
├── Cargo.toml               # toml, dirs 追加
└── Cargo.lock

~/.codex/
└── config.toml              # ユーザー設定ファイル
```

## 8. 次のステップ

### Phase 1: 基本実装
1. 依存関係追加（`toml`, `dirs`）
2. Config構造体の実装
3. システムプロンプトビルダーの実装

### Phase 2: API統合
4. MessageRequestへのsystemフィールド追加
5. main.rsへの統合

### Phase 3: 安全性と利便性
6. 制御文字サニタイズの実装
7. モデル切り替え機能の実装

## 9. 参考リソース

### プロンプトエンジニアリング
- [Prompt Engineering Guide](https://www.promptingguide.ai/)
- [Anthropic Prompt Engineering](https://docs.anthropic.com/en/docs/build-with-claude/prompt-engineering)

### Rust設定管理
- [toml crate documentation](https://docs.rs/toml/latest/toml/)
- [dirs crate documentation](https://docs.rs/dirs/latest/dirs/)

### プロジェクト内ドキュメント
- `docs/codex.md` - Codexの設計パターン
- `docs/ch1/ch1_cli.md` - CLI実装ガイド
