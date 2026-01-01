use anyhow::{Context, Result};
use clap::Parser;
use dotenvy::dotenv;
mod anthropic;
use anthropic::AnthropicClient;

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

    let response = client
        .create_message(&args.model, args.max_tokens, &args.message)
        .await
        .context("Failed to communicate with Claude API")?;

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
