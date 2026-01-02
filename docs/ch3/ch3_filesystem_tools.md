# Chapter 3: ファイルシステムツールでエージェントの能力を拡張する

このドキュメントでは、Anthropic Claude APIのTool Use機能を使用して、ファイルシステムを操作する3つの重要なツールを追加し、エージェントの能力を大幅に拡張する方法について説明します。

## 1. Chapter 3 の概要と目的

### このChapterで実装すること

前章では、`readFile` ツールを実装し、LLMがファイルを読み取る能力を獲得しました。
この章では、以下の3つのツールを追加して、エージェントがファイルシステムを本格的に扱えるようにします。

1. **listFiles** - ディレクトリ内のファイル・ディレクトリ一覧の取得
2. **searchInDirectory** - ファイル内容のキーワード検索
3. **writeFile** - 新規ファイルの作成（ユーザー確認付き）

### なぜこれらのツールが重要か

コーディングエージェントの基本的な能力は、以下の3つに集約されます：

- **読む** (readFile) - ファイルの内容を理解する
- **探す** (listFiles, searchInDirectory) - プロジェクト構造を把握し、必要な情報を見つける
- **書く** (writeFile) - 新しいコードやファイルを生成する

Chapter 2で「読む」能力を実装したので、Chapter 3では「探す」と「書く」を追加します。
これにより、エージェントは以下のような実用的なタスクを実行できるようになります：

- プロジェクト全体の構造を把握
- 特定の関数や変数が使われている場所を検索
- 新しいコードファイルやドキュメントを生成
- ユーザーの指示に基づいて複数のファイルを操作

### 学べること

この章を通じて、以下のRustプログラミングスキルを習得できます：

- **再帰的なファイル探索** - `walkdir` クレートの使い方
- **ユーザー入力の取り扱い** - `std::io` を使った対話的なCLI
- **セキュリティ考慮事項** - パス検証や破壊的操作の安全な実装
- **非同期ファイルI/O** - `tokio::fs` と `std::fs` の使い分け
- **より複雑なツールパラメータの設計** - オプション引数やデフォルト値の扱い

## 2. 各ツールの設計

### 2.1 listFiles ツール

#### 機能概要

`listFiles` ツールは、指定されたディレクトリ内のファイルとディレクトリの一覧を取得します。
以下の2つのモードをサポートします：

- **非再帰モード** - 指定ディレクトリの直下のみ
- **再帰モード** - サブディレクトリを含むすべてのファイル

**使用例：**
```json
{
  "path": "src",
  "recursive": false
}
```

**レスポンス例：**
```json
{
  "files": [
    {"path": "src/main.rs", "is_dir": false, "size": 1234},
    {"path": "src/anthropic.rs", "is_dir": false, "size": 5678},
    {"path": "src/tools", "is_dir": true, "size": 0}
  ]
}
```

#### JSON Schema設計

```json
{
  "type": "object",
  "properties": {
    "path": {
      "type": "string",
      "description": "一覧を取得するディレクトリパス"
    },
    "recursive": {
      "type": "boolean",
      "description": "サブディレクトリも含めるか（デフォルト: false）"
    }
  },
  "required": ["path"]
}
```

#### 実装アプローチ

**非再帰モード（`recursive: false`）:**
```rust
use std::fs;

// 1階層のみのシンプルな実装
let entries = fs::read_dir(path)?;
for entry in entries {
    let entry = entry?;
    // エントリを処理
}
```

**再帰モード（`recursive: true`）:**
```rust
use walkdir::WalkDir;

// すべてのサブディレクトリを探索
for entry in WalkDir::new(path) {
    let entry = entry?;
    // エントリを処理
}
```

**選択の理由：**
- `std::fs::read_dir` - 標準ライブラリ、軽量、1階層のみ
- `walkdir` - 再帰的探索に特化、エラーハンドリングが柔軟、イテレータパターン

#### Go実装との違い

| Go (filepath.Walk) | Rust (walkdir) |
|-------------------|----------------|
| コールバック関数を渡す | イテレータで順次処理 |
| エラーハンドリングがコールバック内 | Result型で型安全に処理 |
| 1つの関数で再帰/非再帰を切り替え | 異なるAPIを使い分け |

**Go（参考元記事）:**
```go
err := filepath.Walk(path, func(path string, info os.FileInfo, err error) error {
    // コールバック内で処理
    return nil
})
```

**Rust（本プロジェクト）:**
```rust
for entry in WalkDir::new(path) {
    let entry = entry?;
    // イテレータで処理
}
```

### 2.2 searchInDirectory ツール

#### 機能概要

`searchInDirectory` ツールは、指定されたディレクトリ配下のファイルをキーワード検索し、マッチしたファイルのパスを返します。

**特徴：**
- 大文字小文字を区別しない検索
- バイナリファイルを自動的にスキップ
- 行単位での検索（マッチした行番号も返す）

**使用例：**
```json
{
  "path": "src",
  "keyword": "ToolHandler"
}
```

**レスポンス例：**
```json
{
  "matches": [
    {
      "path": "src/anthropic.rs",
      "line_number": 42,
      "line": "pub trait ToolHandler: Send + Sync {"
    },
    {
      "path": "src/tools/read_file.rs",
      "line_number": 18,
      "line": "impl ToolHandler for ReadFileTool {"
    }
  ]
}
```

#### JSON Schema設計

```json
{
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
}
```

#### 実装アプローチ

```rust
use walkdir::WalkDir;
use tokio::fs;

// 1. ディレクトリを再帰的に探索
for entry in WalkDir::new(path) {
    let entry = entry?;

    // 2. ディレクトリはスキップ
    if entry.file_type().is_dir() {
        continue;
    }

    // 3. ファイルを読み込み
    let content = fs::read_to_string(entry.path()).await?;

    // 4. 行単位で検索
    for (line_num, line) in content.lines().enumerate() {
        if line.to_lowercase().contains(&keyword.to_lowercase()) {
            // マッチした行を記録
        }
    }
}
```

**バイナリファイル対策：**
- `fs::read_to_string` がエラーを返した場合（非UTF-8ファイル）は静かにスキップ
- `.gitignore` のパターンは考慮しない（シンプルさ重視）

### 2.3 writeFile ツール

#### 機能概要

`writeFile` ツールは、指定されたパスに新しいファイルを作成し、内容を書き込みます。

**重要な特徴：**
- **既存ファイルの保護** - 既存ファイルが存在する場合はエラーを返す
- **ユーザー確認** - 実行前に必ずユーザーの明示的な許可を得る
- **ディレクトリ自動作成** - 親ディレクトリが存在しない場合は自動的に作成

**使用例：**
```json
{
  "path": "test/example.rs",
  "content": "fn main() {\n    println!(\"Hello, World!\");\n}"
}
```

#### JSON Schema設計

```json
{
  "type": "object",
  "properties": {
    "path": {
      "type": "string",
      "description": "作成するファイルの完全なパス"
    },
    "content": {
      "type": "string",
      "description": "ファイルに書き込む内容"
    }
  },
  "required": ["path", "content"]
}
```

#### セキュリティ重要事項

**破壊的操作の前に必ずユーザー確認を取る**

```rust
use std::io::{self, Write};
use std::path::Path;

// 1. ファイルが既に存在するか確認
if Path::new(&path).exists() {
    // 2. ユーザーに確認
    print!("ファイル {} は既に存在します。上書きしますか？ [y/N]: ", path);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    // 3. 'y' または 'Y' 以外はキャンセル
    if input.trim().to_lowercase() != "y" {
        return Ok(ToolResult {
            content: String::new(),
            error: Some("ユーザーによりキャンセルされました".to_string()),
        });
    }
}
```

**この確認機能が重要な理由：**
- LLMは予期しない動作をする可能性がある
- 重要なファイルの誤った上書きを防ぐ
- ユーザーが常にコントロールを保持

## 3. Rust固有の実装パターン

### 3.1 非同期ファイルI/O: `std::fs` vs `tokio::fs`

Rustでは、ファイル操作に2つのアプローチがあります：

#### `std::fs` (同期I/O)

```rust
use std::fs;

// ブロッキング操作
let entries = fs::read_dir(path)?;
let metadata = fs::metadata(path)?;
```

**使用すべき場合：**
- メタデータの取得（`metadata()`, `exists()`）
- ディレクトリの存在チェック
- 即座に完了する軽量な操作

#### `tokio::fs` (非同期I/O)

```rust
use tokio::fs;

// 非同期操作
let content = fs::read_to_string(path).await?;
let metadata = fs::metadata(path).await?;
```

**使用すべき場合：**
- ファイルの読み書き（I/O待ち時間が長い）
- 複数のファイルを同時に処理
- 他の非同期タスクと並行実行

#### `walkdir` (同期イテレータ)

```rust
use walkdir::WalkDir;

// 同期イテレータだが、非同期タスク内で使用可能
for entry in WalkDir::new(path) {
    let entry = entry?;
    // tokio::fs と組み合わせる
    let content = tokio::fs::read_to_string(entry.path()).await?;
}
```

**注意点：**
- `walkdir` は同期イテレータ
- ディレクトリ探索自体は高速なのでブロッキングでも問題ない
- ファイル読み込みは `tokio::fs` を使う

**使い分けの方針：**

| 操作 | 使用するAPI | 理由 |
|------|-----------|------|
| ディレクトリ一覧取得 | `std::fs::read_dir` | 軽量、即座に完了 |
| 再帰的探索 | `walkdir::WalkDir` | ディレクトリ探索に特化 |
| ファイル読み込み | `tokio::fs::read_to_string` | I/O待ち時間が長い |
| ファイル書き込み | `tokio::fs::write` | I/O待ち時間が長い |
| メタデータ取得 | `std::fs::metadata` | 即座に完了 |
| 存在チェック | `Path::exists()` | 即座に完了 |

### 3.2 PathBuf と Path の使い分け

Rustのパス処理は2つの型で行います：

#### `PathBuf` - 所有権を持つパス（Stringに相当）

```rust
use std::path::PathBuf;

// 所有権を持つパスを作成
let mut path_buf = PathBuf::from("/home/user");
path_buf.push("src");        // 変更可能
path_buf.push("main.rs");
// → /home/user/src/main.rs
```

**使用すべき場合：**
- パスを構築・変更する場合
- パスを所有する必要がある場合
- 関数の戻り値として返す場合

#### `Path` - パスへの参照（&strに相当）

```rust
use std::path::Path;

// 参照として扱う
let path: &Path = Path::new("/home/user/file.txt");
let parent = path.parent();       // Option<&Path>
let file_name = path.file_name(); // Option<&OsStr>
let extension = path.extension(); // Option<&OsStr>
```

**使用すべき場合：**
- パスを読み取るだけの場合
- 関数の引数として受け取る場合
- パスの一部を抽出する場合

#### 相互変換

```rust
use std::path::{Path, PathBuf};

// PathBuf から &Path
let path_buf = PathBuf::from("src/main.rs");
let path: &Path = &path_buf;  // 自動的に参照に変換

// &Path から PathBuf
let path = Path::new("src/main.rs");
let path_buf: PathBuf = path.to_path_buf();  // クローンを作成
```

**よく使うメソッド：**

```rust
use std::path::Path;

let path = Path::new("src/tools/read_file.rs");

// パス分解
path.parent()       // Some("src/tools")
path.file_name()    // Some("read_file.rs")
path.file_stem()    // Some("read_file")
path.extension()    // Some("rs")

// パス結合
let mut new_path = path.parent().unwrap().to_path_buf();
new_path.push("write_file.rs");  // src/tools/write_file.rs

// パス検証
path.exists()       // ファイル/ディレクトリが存在するか
path.is_file()      // ファイルか
path.is_dir()       // ディレクトリか
path.is_absolute()  // 絶対パスか
```

### 3.3 ユーザー入力の取得

#### std::io（基本実装）

このプロジェクトでは、依存関係を最小限にするため `std::io` を使用します：

```rust
use std::io::{self, Write};

fn prompt_user(message: &str) -> Result<bool> {
    // 1. プロンプトを表示
    print!("{} [y/N]: ", message);

    // 2. バッファをフラッシュ（即座に表示）
    io::stdout().flush()?;

    // 3. ユーザー入力を読み取り
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    // 4. 入力を検証
    Ok(input.trim().to_lowercase() == "y")
}
```

**重要なポイント：**

1. **`stdout().flush()`** - `print!` マクロはバッファリングされるため、`flush()` で即座に表示
2. **`trim()`** - 改行文字やスペースを除去
3. **`to_lowercase()`** - 大文字小文字を区別しない比較
4. **デフォルトは "No"** - セキュリティのため、明示的な "y" のみを許可

#### dialoguer（将来的な改善案）

Chapter 4 では、より良いUXのために `dialoguer` クレートへの移行を検討します：

```rust
use dialoguer::Confirm;

// より洗練されたUI
let confirmed = Confirm::new()
    .with_prompt("ファイルを上書きしますか？")
    .default(false)
    .interact()?;
```

**dialoguer の利点：**
- 色付きのプロンプト
- デフォルト値の視覚的な表示
- より多彩な入力タイプ（Select, Input, Passwordなど）
- クロスプラットフォームで一貫したUX

**codexのアプローチ（参考）：**

codexプロジェクトでは、さらに高度なTUI実装を採用しています：
- `ratatui` + `crossterm` による本格的なターミナルUI
- イベント駆動アーキテクチャ（`Op::ExecApproval` イベント）
- リアルタイムフィードバックと対話的なワークフロー

**本プロジェクトでの段階的なアプローチ：**
1. **Chapter 3** - `std::io` で基本実装（学習重視）
2. **Chapter 4** - `dialoguer` への移行（UX改善）
3. **将来的** - codex風のTUI（advanced topic）

### 3.4 エラーハンドリング戦略（継続）

Chapter 2 で確立したエラーハンドリングの原則を継続します：

**原則: ツールエラーは `ToolResult` として返す**

```rust
// ✅ GOOD: エラーをToolResultとして返す
match fs::create_dir_all(&parent_dir).await {
    Ok(_) => { /* 続行 */ }
    Err(e) => return Ok(ToolResult {
        content: String::new(),
        error: Some(format!("ディレクトリ作成失敗: {}", e)),
    }),
}

// ❌ BAD: エラーを伝播させる（agentic loopが停止）
fs::create_dir_all(&parent_dir).await?;
```

**理由：**
- LLMはエラーメッセージを読んで適切に対応できる
- Agentic loop が途中で停止しない
- ユーザーにとって分かりやすいエラーメッセージ

**新しいツールでの応用：**

```rust
// listFiles の例
if !path.exists() {
    return Ok(ToolResult {
        content: String::new(),
        error: Some(format!("ディレクトリが見つかりません: {}", path.display())),
    });
}

// searchInDirectory の例
match fs::read_to_string(&file_path).await {
    Ok(content) => { /* 検索処理 */ }
    Err(_) => {
        // バイナリファイルや権限エラーは静かにスキップ
        debug!("Skipping file: {:?}", file_path);
        continue;
    }
}

// writeFile の例
if path.exists() && !user_confirmed {
    return Ok(ToolResult {
        content: String::new(),
        error: Some("ユーザーによりキャンセルされました".to_string()),
    });
}
```

## 4. セキュリティ考慮事項

### 4.1 パス検証の強化

ファイルシステム操作には、セキュリティリスクが伴います。特に以下の攻撃を防ぐ必要があります：

#### ディレクトリトラバーサル攻撃

**攻撃例：**
```
path: "../../etc/passwd"
```

**対策：**
```rust
use std::path::Path;
use std::env;

fn validate_path(path: &Path) -> Result<()> {
    // 1. カノニカライズ（シンボリックリンク解決、絶対パス化）
    let canonical = path.canonicalize()
        .or_else(|_| {
            // ファイルが存在しない場合は親ディレクトリで検証
            path.parent()
                .ok_or_else(|| anyhow::anyhow!("Invalid path"))?
                .canonicalize()
        })?;

    // 2. 現在のディレクトリ外へのアクセスを拒否
    let cwd = env::current_dir()?;
    if !canonical.starts_with(&cwd) {
        bail!("Access denied: path outside working directory");
    }

    Ok(())
}
```

**実装の段階的アプローチ：**
- **Chapter 3** - 基本的なパス検証（`exists()`, `is_file()`）
- **Chapter 4** - `canonicalize()` による高度な検証
- **将来的** - サンドボックス環境での実行（codexのアプローチ）

### 4.2 破壊的操作の確認フロー

`writeFile` ツールの安全な実装フロー：

```
1. パス検証
   ↓
2. ファイル存在チェック
   ↓
3. 存在する場合 → ユーザー確認
   ├─ Yes → 4へ
   └─ No  → エラーメッセージを返す
   ↓
4. 親ディレクトリの存在確認
   ↓
5. 存在しない場合 → 自動作成
   ↓
6. ファイル書き込み
   ↓
7. 成功メッセージを返す
```

**ユーザー確認の実装：**
```rust
// デフォルトは "No"（安全側）
if !prompt_user(&format!("ファイル {} を作成しますか？", path))? {
    return Ok(ToolResult {
        content: String::new(),
        error: Some("ユーザーによりキャンセルされました".to_string()),
    });
}
```

### 4.3 ファイルサイズ制限

大ファイルによるメモリ枯渇を防ぐ：

```rust
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;  // 10MB

let metadata = fs::metadata(path).await?;
if metadata.len() > MAX_FILE_SIZE {
    return Ok(ToolResult {
        content: String::new(),
        error: Some("ファイルが大きすぎます（10MB以上）".to_string()),
    });
}
```

**実装の段階的アプローチ：**
- **Chapter 3** - 基本機能の実装（サイズ制限なし）
- **Chapter 4** - ファイルサイズ制限の追加
- **将来的** - ストリーミング読み込み

## 5. Chapter 4 への橋渡し

### 次のステップ

Chapter 4 では、さらに高度なツールを実装します：

#### 5.1 editFile ツール（最重要）

既存ファイルの部分編集機能：
- 特定の行範囲を置換
- 正規表現による検索・置換
- 差分の表示とユーザー確認

**課題：**
- 安全な編集方法の設計
- バックアップ機能
- ロールバック機能

#### 5.2 executeCommand ツール（慎重に）

シェルコマンドの実行：
- テストの実行（`cargo test`）
- ビルドの実行（`cargo build`）
- Linterの実行（`cargo clippy`）

**セキュリティ上の重要な考慮事項：**
- ホワイトリスト方式（許可されたコマンドのみ）
- サンドボックス環境での実行
- ユーザー確認の徹底

#### 5.3 UX改善

- `dialoguer` クレートの導入
- カラフルなターミナル出力（`colored` クレート）
- プログレスバーの表示（`indicatif` クレート）

#### 5.4 テストの充実化

- 各ツールのユニットテスト
- 統合テスト
- エッジケースのテスト

### アーキテクチャの進化

**現在（Chapter 3 完了時）:**
```
User → main.rs → AnthropicClient → LLM
                       ↓
                 ToolRegistry
                       ↓
           readFile, listFiles, searchInDirectory, writeFile
```

**将来（Chapter 4+）:**
```
User → main.rs → AnthropicClient → LLM
                       ↓
                 ToolRegistry
                       ↓
           複数のツール + ツール間連携
                       ↓
              Agentic Workflow
              （検索 → 読む → 編集 → テスト）
```

## 6. 参考リソース

### Rustクレート

- **[walkdir](https://docs.rs/walkdir/latest/walkdir/)** - 再帰的ディレクトリ探索
  - 柔軟なエラーハンドリング
  - イテレータパターン
  - フィルタリング機能

- **[tokio::fs](https://docs.rs/tokio/latest/tokio/fs/)** - 非同期ファイルI/O
  - `read_to_string`, `write`, `create_dir_all`
  - 他の非同期タスクと並行実行可能

- **[dialoguer](https://docs.rs/dialoguer/latest/dialoguer/)** - 対話的なCLI（将来的な改善）
  - Confirm, Input, Select などの入力タイプ
  - カラフルなUI

### Anthropic API

- **[Tool Use Best Practices](https://docs.anthropic.com/en/docs/build-with-claude/tool-use#best-practices-for-tool-definitions)**
  - ツール定義のベストプラクティス
  - パフォーマンス最適化

- **[Tool Use Overview](https://docs.anthropic.com/en/docs/build-with-claude/tool-use)**
  - Tool Useの基本概念
  - API仕様

### プロジェクト内

- **`docs/ch2/ch2_tool_use.md`** - Tool Useの基礎
  - ToolHandler traitの設計
  - ToolRegistryパターン

- **`src/tools/read_file.rs`** - 参考実装
  - ツール実装のパターン
  - エラーハンドリングの例

- **`codex.md`** - 本格的な実装の参考
  - ratatui + crosstermによるTUI
  - イベント駆動アーキテクチャ
  - サンドボックス実装

### Rustプログラミング

- **[The Rust Programming Language](https://doc.rust-lang.org/book/)** - 公式ガイド
- **[Rust By Example](https://doc.rust-lang.org/rust-by-example/)** - 実例で学ぶ
- **[Async Book](https://rust-lang.github.io/async-book/)** - 非同期プログラミング

## まとめ

この Chapter 3 では、エージェントに「探す」「書く」能力を追加します。

**達成目標:**
- ✅ 3つの新しいツールの実装（listFiles, searchInDirectory, writeFile）
- ✅ 再帰的ファイル探索の習得（walkdir）
- ✅ ユーザー入力の適切な処理（std::io）
- ✅ セキュリティベストプラクティスの適用
- ✅ より実用的なコーディングエージェントへの進化

**学習ポイント:**
- Rustのファイルシステム操作（std::fs vs tokio::fs）
- PathBuf と Path の使い分け
- イテレータパターンの活用
- セキュリティを考慮した設計

次の `implementation_guide.md` では、これらの概念を実際のRustコードとして10タスクに分けて段階的に実装していきます。
