# Chapter 4: editFile - コーディングエージェントの最重要機能

このドキュメントでは、コーディングエージェントにおいて最も重要な機能である「既存ファイルを安全に編集する能力」を実装する方法について説明します。

## 1. editFile の重要性

### なぜ editFile が最重要機能なのか

Chapter 3までで、エージェントは以下の能力を獲得しました：

- **readFile** - ファイルを読む
- **listFiles** - ディレクトリ構造を把握する
- **searchInDirectory** - コードを検索する
- **writeFile** - 新しいファイルを作成する

しかし、実際のコーディング作業の大部分は「既存のコードの修正」です。新しいファイルを作るだけでは、本格的なコーディングエージェントとは言えません。

**editFile が可能にすること：**
- バグ修正
- 機能追加
- コードのリファクタリング
- 設定ファイルの更新
- ドキュメントの改善

### 破壊的操作としての危険性

editFileは、既存ファイルを**完全に上書き**する非常に破壊的な操作です。

**潜在的なリスク：**
- 重要なコードの誤った削除
- 構文エラーの導入
- 意図しない動作の変更
- データの損失

そのため、以下の安全機構が必須です：

1. **ユーザー確認** - 実行前に必ず許可を得る
2. **ファイル存在チェック** - 新規作成と編集を明確に分離
3. **Read-Modify-Write パターン** - LLMが内容を理解してから編集

## 2. editFile vs writeFile の違い

### 機能の比較

| 機能 | writeFile | editFile |
|------|-----------|----------|
| 用途 | **新規ファイル作成** | **既存ファイル編集** |
| 前提条件 | ファイルが存在しない | ファイルが存在する |
| ディレクトリ作成 | 親ディレクトリを自動作成 | 不要（ファイルが存在） |
| エラー処理 | 既存ファイルがあればエラー | ファイルがなければエラー |
| ユーザー確認 | 新規作成時のみ | 毎回必須 |
| 使用例 | 新しいモジュールの追加 | バグ修正、機能追加 |

### 使い分けの指針

**writeFile を使うべき場合：**
```rust
// 新しいテストファイルを作成
{
  "path": "tests/new_test.rs",
  "content": "#[test]\nfn test_example() { ... }"
}
```

**editFile を使うべき場合：**
```rust
// 既存のmain.rsにログ追加
{
  "path": "src/main.rs",
  "new_content": "use tracing::info;\n\nfn main() {\n    info!(\"Starting...\");\n    ...\n}"
}
```

### なぜ明確に分離するのか

```rust
// ❌ BAD: writeFileで既存ファイルも処理する
// → 誤って重要なファイルを上書きする危険性

// ✅ GOOD: 役割を明確に分離
// → ツール名から意図が明確、安全性が向上
```

## 3. アーキテクチャ設計の選択

### 部分編集 vs 全体置換

ファイル編集には、主に2つのアプローチがあります：

#### アプローチ1: 部分編集（今回は採用しない）

**実装例：**
```rust
// 行番号指定での置換
fn edit_lines(path: &str, start_line: u32, end_line: u32, new_content: &str)

// 正規表現による置換
fn replace_pattern(path: &str, pattern: &str, replacement: &str)

// 差分適用
fn apply_patch(path: &str, patch: &str)
```

**利点：**
- パフォーマンスが良い（小さな変更のみ送信）
- APIコストが低い（トークン使用量が少ない）
- ネットワーク帯域幅を節約

**欠点（非常に重要）：**
- **構文エラーを引き起こしやすい** - 不完全な編集によるコンパイルエラー
- **インデントの不整合** - コンテキストを理解しないと正しいインデントが困難
- **インポート文の重複** - 依存関係の追加が適切に処理されない
- **複雑な実装** - エラー修正ロジックが複雑になる
- **LLMの負担** - 正確な行番号や正規表現パターンの生成が難しい

**参考：高度な部分編集の実装**

Gemini CLIでは、「編集行の前後数行のコンテキストを要求する」という手法で部分編集を実現していますが、実装と理解が複雑です：

- [Gemini CLI - editFile 実装](https://github.com/yourusername/gemini-cli) （参考リンク）

#### アプローチ2: 全体置換（本プロジェクトで採用）

**実装：**
```rust
pub struct EditFileArgs {
    pub path: String,
    pub new_content: String,  // ファイル全体の新しい内容
}

// ファイル全体を完全に置き換える
async fn execute(&self, input: Value) -> Result<ToolResult> {
    // 1. ファイルの存在を確認
    // 2. ユーザーに許可を求める
    // 3. 完全に上書き
    fs::write(&path, &new_content).await?;
}
```

**利点：**
- **安全性** - 構文エラーのリスクが低い（LLMがファイル全体を理解）
- **シンプルな実装** - 複雑なエラー処理が不要
- **予測可能** - 結果が明確で理解しやすい
- **LLMに適している** - コンテキスト全体を見て判断できる
- **初心者向け** - 実装が理解しやすい

**欠点：**
- パフォーマンス（大きなファイルでトークン消費）
- APIコスト（毎回全体を送信）

### なぜ全体置換を選ぶのか

**本プロジェクトの方針：**
```
安全性 > パフォーマンス
理解しやすさ > 高度な機能
```

**理由：**

1. **学習教材としての役割**
   - Rust初心者が理解しやすい
   - 実装が直感的
   - デバッグが容易

2. **安全性の確保**
   - 構文エラーのリスクが低い
   - ファイル破壊の可能性を最小化
   - ユーザーが内容を確認できる

3. **現実的なトレードオフ**
   - 通常のファイルサイズ（< 1000行）では問題ない
   - Claude 3.5 Sonnetの大きなコンテキストウィンドウを活用
   - エラー回復がシンプル

4. **Read-Modify-Write パターンとの相性**
   - LLMが自然に実行できるワークフロー
   - 段階的な処理で理解しやすい

## 4. Read-Modify-Write パターン

### パターンの説明

Read-Modify-Write は、ファイル編集における最も安全で確実なパターンです。

**3つのステップ：**

```
Step 1: Read（読む）
  ↓
  readFile を使って現在のファイル内容を取得

Step 2: Modify（修正）
  ↓
  LLMがメモリ内で内容を変更
  （完全な新しいファイル内容を構築）

Step 3: Write（書く）
  ↓
  editFile を使って完全な新しい内容で上書き
```

### 実際の動作フロー

**ユーザー指示：**
```
"config.tomlのmax_tokensを1024から2048に変更してください"
```

**エージェントの動作：**

```
1. LLM判断: "まずファイルを読む必要がある"
   ↓
2. ツール呼び出し: readFile("config.toml")
   ↓
3. 結果取得:
   {
     "content": "model = \"claude-3-5-sonnet-20241022\"\nmax_tokens = 1024\n"
   }
   ↓
4. LLM判断: "max_tokensを2048に変更した新しい内容を作成"
   ↓
5. ツール呼び出し: editFile({
     "path": "config.toml",
     "new_content": "model = \"claude-3-5-sonnet-20241022\"\nmax_tokens = 2048\n"
   })
   ↓
6. ユーザー確認: "config.tomlを編集します。よろしいですか？ [y/N]"
   ↓
7. 実行完了: ファイル更新
```

### なぜこのパターンが重要か

**LLMの特性に適している：**
- LLMは「全体のコンテキスト」を理解するのが得意
- 部分的な編集指示（行番号など）よりも、完全な内容を生成する方が得意
- エラーが発生した場合も、全体を見て修正できる

**安全性の向上：**
- 既存の内容を必ず読むため、破壊的な変更を避けられる
- ユーザーは変更前に内容を確認できる
- ファイル構造を壊すリスクが低い

**自然な会話フロー：**
```
User: "ファイルXのYを変更して"
Agent: "ファイルXを読み込みます..."
Agent: "内容を確認しました。Yを変更します..."
User: "はい、実行してください"
Agent: "完了しました"
```

### 複数ターンでの自動実行

Chapter 3で実装した `max_iterations` による agentic loop により、Read-Modify-Write パターンは自動的に実行されます：

```rust
// main.rs の agentic loop（既に実装済み）
for iteration in 0..max_iterations {
    // 1回目: readFile が呼ばれる
    // 2回目: editFile が呼ばれる
    // ...
}
```

**重要なポイント：**
- ユーザーは「ファイルを編集して」と指示するだけ
- エージェントが自律的に read → edit の流れを実行
- ユーザーは editFile 実行時のみ確認を求められる

## 5. 安全機構の実装

### 5.1 ファイル存在チェック

```rust
// 1. ファイルが存在しない場合はエラー
if !Path::new(&path).exists() {
    return Ok(ToolResult {
        content: String::new(),
        error: Some(
            "ファイルが存在しません。新しいファイルの作成にはwriteFileを使用してください。"
                .to_string()
        ),
    });
}
```

**理由：**
- 新規作成と編集を明確に区別
- 誤ったパスの早期検出
- writeFile との役割分担

### 5.2 ユーザー確認の必須化

```rust
// 2. 必ずユーザーに確認
use std::io::{self, Write};

print!("既存ファイルを編集します: {}\n実行してもよろしいですか？ [y/N]: ", path);
io::stdout().flush()?;

let mut input = String::new();
io::stdin().read_line(&mut input)?;

if input.trim().to_lowercase() != "y" {
    return Ok(ToolResult {
        content: String::new(),
        error: Some("ユーザーによってキャンセルされました".to_string()),
    });
}
```

**重要なポイント：**
- デフォルトは "No"（安全側）
- 明示的な "y" のみを許可
- キャンセルはエラーではなく正常な結果として扱う

### 5.3 完全な上書き

```rust
// 3. ファイルを完全に上書き
use tokio::fs;

fs::write(&path, &new_content).await?;
```

**`fs::write` の動作：**
- ファイルを開いて完全に置き換える
- 元の内容は完全に削除される
- アトミックな操作（途中で失敗しても中途半端にならない）

## 6. ツール定義とスキーマ

### JSON Schema

```json
{
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
}
```

### Description フィールドの重要性

Tool の `description` フィールドは、LLMがツールを正しく使用するための重要な情報源です。

**良い description の例：**

```rust
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
    input_schema: /* ... */
}
```

**この description が伝えること：**

1. **What（何をする）** - 完全に上書きする
2. **How（どう使う）** - Read-Modify-Write パターン
3. **Why（なぜ）** - ファイルを破壊しないため
4. **Constraints（制約）** - 部分的な編集は不可
5. **Safety（安全性）** - ユーザー許可が必要

## 7. エラーハンドリング

### エラーケースの分類

**1. ファイルが存在しない**
```rust
error: "ファイルが存在しません。新しいファイルの作成にはwriteFileを使用してください。"
```

**2. ユーザーキャンセル**
```rust
error: "ユーザーによってキャンセルされました"
```

**3. 書き込み権限がない**
```rust
error: "ファイルの書き込みに失敗しました: Permission denied"
```

**4. ディスク容量不足**
```rust
error: "ファイルの書き込みに失敗しました: No space left on device"
```

### エラー戦略（継続）

Chapter 2-3 で確立したエラーハンドリングの原則を継続：

```rust
// ✅ GOOD: エラーを ToolResult として返す
match fs::write(&path, &new_content).await {
    Ok(_) => Ok(ToolResult {
        content: "ファイルを正常に更新しました".to_string(),
        error: None,
    }),
    Err(e) => Ok(ToolResult {
        content: String::new(),
        error: Some(format!("ファイルの書き込みに失敗しました: {}", e)),
    }),
}

// ❌ BAD: エラーを伝播させる
fs::write(&path, &new_content).await?;  // agentic loop が停止
```

## 8. テスト戦略

### 基本的なテストケース

**1. 単純な内容変更**
```bash
# テストファイルを作成
echo "Hello World" > test.txt

# エージェントに指示
"test.txtの内容をHello Nebulaに変更してください"

# 期待される動作
1. readFile("test.txt") → "Hello World"
2. editFile("test.txt", "Hello Nebula")
3. ユーザー確認 → y
4. 完了
```

**2. 複数行の編集**
```bash
# テストファイル
cat > config.toml << EOF
model = "claude-3-5-sonnet-20241022"
max_tokens = 1024
EOF

# エージェントに指示
"config.tomlのmax_tokensを2048に変更してください"

# 期待される動作
1. readFile("config.toml")
2. editFile with complete new content
3. 確認 → y
4. 完了
```

**3. Read-Modify-Write パターンの確認**
```bash
# エージェントに指示
"test.txtに新しい行を追加してください"

# 期待される動作
1. readFile("test.txt") → 現在の内容
2. LLMが既存内容 + 新しい行を構築
3. editFile で完全な内容を書き込み
```

### エラーケースのテスト

**1. 存在しないファイル**
```bash
"存在しないファイル.txtを編集してください"
→ error: "ファイルが存在しません..."
```

**2. ユーザーキャンセル**
```bash
"test.txtを編集してください"
→ 確認プロンプト: "よろしいですか？ [y/N]"
→ ユーザー入力: "n"
→ error: "ユーザーによってキャンセルされました"
```

## 9. 実装概要

### ファイル構成

```
src/tools/
├── mod.rs              # モジュール宣言に edit_file を追加
├── read_file.rs        # 既存
├── write_file.rs       # 既存（参考実装）
├── list_files.rs       # 既存
├── search_in_directory.rs  # 既存
└── edit_file.rs        # 新規作成
```

### 実装ステップ（概要）

```rust
// 1. EditFileArgs 構造体
#[derive(Debug, Deserialize)]
pub struct EditFileArgs {
    pub path: String,
    pub new_content: String,
}

// 2. EditFileTool 構造体
pub struct EditFileTool;

// 3. schema() メソッド
impl EditFileTool {
    pub fn schema() -> Tool { /* ... */ }
}

// 4. ToolHandler trait 実装
#[async_trait]
impl ToolHandler for EditFileTool {
    async fn execute(&self, input: Value) -> Result<ToolResult> {
        // a. 引数をパース
        // b. ファイル存在チェック
        // c. ユーザー確認
        // d. ファイル書き込み
        // e. 結果を返す
    }
}

// 5. main.rs で登録
tool_registry.register(EditFileTool::schema(), EditFileTool::new());
```

## 10. Chapter 5 への展望

### 現在の限界

Chapter 4 完了時点で、エージェントは以下のことができます：

- ✅ ファイルの読み書き
- ✅ ディレクトリ構造の把握
- ✅ コード検索
- ✅ 既存ファイルの編集

**しかし、まだ以下の限界があります：**

❌ 複雑なタスクの計画的な実行
❌ プロジェクト全体の理解
❌ 段階的な実装プロセス
❌ コードスタイルの一貫性

### 次のステップ：システムプロンプト

**なぜシステムプロンプトが必要か：**

現在、エージェントはツールを「使える」だけで、「どう使うべきか」を理解していません。

**システムプロンプトで実現できること：**

1. **思考プロセスの定義**
   ```
   1. プロジェクト構造を調査
   2. 関連ファイルを読み込み
   3. 変更計画を立案
   4. 実装を実行
   5. 検証
   ```

2. **役割と制約の明確化**
   ```
   - あなたはRustコーディングアシスタントです
   - 既存のコードスタイルに従ってください
   - 変更前に必ず既存のコードを読んでください
   ```

3. **品質基準の設定**
   ```
   - cargo fmtを実行してください
   - cargo clippyでエラーがないことを確認してください
   - テストを実行してください
   ```

**Chapter 5 の予告：**
- システムプロンプトの設計と実装
- より高度な思考プロセス
- プロジェクト理解の向上
- 本格的なコーディングエージェントへの進化

## 11. 参考リソース

### Anthropic API
- **[Tool Use Best Practices](https://docs.anthropic.com/en/docs/build-with-claude/tool-use#best-practices-for-tool-definitions)**
  - description フィールドの書き方
  - ワークフローの説明方法

### Rust ファイルI/O
- **[std::fs](https://doc.rust-lang.org/std/fs/)** - ファイル操作
- **[tokio::fs](https://docs.rs/tokio/latest/tokio/fs/)** - 非同期ファイルI/O
- **[std::io](https://doc.rust-lang.org/std/io/)** - 入出力処理

### プロジェクト内ドキュメント
- **`docs/ch3/ch3_filesystem_tools.md`** - writeFile の実装参考
- **`docs/ch3/implementation_guide.md`** - ユーザー確認の実装例
- **`codex.md`** - より高度な実装の参考

### 他の実装例
- **Gemini CLI** - 部分編集の実装例（高度）
- **Aider** - AI ペアプログラミングツール
- **Cursor** - エディタ統合型AIアシスタント

## まとめ

この Chapter 4 では、コーディングエージェントにとって最も重要な機能である `editFile` を実装します。

**重要なポイント：**

1. **全体置換アプローチ** - 安全性と理解しやすさを重視
2. **Read-Modify-Write パターン** - LLMの特性に適した自然なワークフロー
3. **安全機構** - ファイル存在チェックとユーザー確認の必須化
4. **明確な役割分担** - writeFile との使い分け

**達成目標：**
- ✅ editFile ツールの実装
- ✅ Read-Modify-Write パターンの理解
- ✅ 安全なファイル編集の実現
- ✅ 本格的なコーディングエージェントへの進化

次の `implementation_guide.md` では、これらの概念を実際のRustコードとして10タスクに分けて段階的に実装していきます。
