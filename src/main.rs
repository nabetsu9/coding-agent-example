use anyhow::Result;
use clap::Parser;
use dotenvy::dotenv;
mod anthropic;
mod tools;
use anthropic::{AnthropicClient, ContentBlock, ToolRegistry};
use tools::{ListFilesTool, ReadFileTool, SearchInDirectoryTool, WriteFileTool};

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
    #[arg(long, short = 'm', default_value = "claude-sonnet-4-5")]
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

    // load environment variables from .env file
    dotenv().ok();

    // CLI引数のパース
    let args = Args::parse();

    // APIキーの検証
    if args.api_key.is_empty() {
        anyhow::bail!(
            "ANTHROPIC_API_KEY is required. Set via environment variable or --api-key flag."
        );
    }

    tracing::info!("Sending message to Claude API");

    let client = AnthropicClient::new(args.api_key);

    // ToolRegistry の作成
    let mut tool_registry = ToolRegistry::new();
    tool_registry.register(ReadFileTool::schema(), ReadFileTool::new());
    tool_registry.register(ListFilesTool::schema(), ListFilesTool::new());
    tool_registry.register(
        SearchInDirectoryTool::schema(),
        SearchInDirectoryTool::new(),
    );
    tool_registry.register(WriteFileTool::schema(), WriteFileTool::new());

    tracing::info!(
        "Registered tools: readFile, listFiles, searchInDirectory, writeFile" // 🆕 更新
    );

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
