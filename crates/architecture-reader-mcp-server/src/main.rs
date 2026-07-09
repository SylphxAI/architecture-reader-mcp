use architecture_reader_mcp_server::{cli_bridge, ArchitectureReaderMcp, SERVER_VERSION};
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("doctor") {
        eprintln!(
            "architecture-reader-mcp Rust MCP server {SERVER_VERSION} ({})",
            architecture_reader_core::ENGINE_NAME
        );
        if let Some(cli) = cli_bridge::resolve_cli_binary() {
            eprintln!("engine cli: {}", cli.display());
        } else {
            eprintln!("engine cli: unavailable (run `bun run build:rust`)");
        }
        return Ok(());
    }

    let service = ArchitectureReaderMcp::new()
        .serve(rmcp::transport::stdio())
        .await?;
    service.waiting().await?;
    Ok(())
}