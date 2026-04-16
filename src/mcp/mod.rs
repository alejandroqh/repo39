mod params;
mod server;

use anyhow::Result;
use rmcp::ServiceExt;

use server::McpServer;

pub async fn run_mcp_stdio() -> Result<()> {
    let server = McpServer::new();
    let transport = rmcp::transport::io::stdio();
    let handle = server.serve(transport).await.map_err(anyhow::Error::msg)?;
    handle.waiting().await?;
    Ok(())
}
