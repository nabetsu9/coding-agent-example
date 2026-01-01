use clap::Parser;
use dotenvy::dotenv;

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

fn main() {
    // load environment variables from .env file
    dotenv().ok();
    let args = Args::parse();

    // 引数の表示（動作確認用）
    println!("Message: {}", args.message);
    println!("Model: {}", args.model);
    println!("Max tokens: {}", args.max_tokens);
}
