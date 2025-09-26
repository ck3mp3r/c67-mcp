mod handler;

use anyhow::Result;
use clap::Parser;
use handler::run_server;

#[derive(Parser)]
#[command(name = "c67-mcp")]
#[command(about = "A Rust alternative to the Context7 MCP server")]
#[command(version)]
struct Cli {
    /// Log level (logs to stderr)
    #[arg(long, default_value = "warn")]
    log_level: String,

    /// API key for Context7 authentication
    #[arg(long)]
    api_key: Option<String>,

    /// Enable debug logging to stderr
    #[arg(short, long)]
    debug: bool,

    /// Enable verbose output to stderr
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Disable TLS certificate verification (insecure, for corporate MITM)
    #[arg(long)]
    insecure: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = match cli.log_level.as_str() {
        "trace" => "trace",
        "debug" => "debug",
        "info" => "info",
        "warn" => "warn",
        "error" => "error",
        _ => "warn",
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    run_server(cli.api_key, cli.insecure).await
}
