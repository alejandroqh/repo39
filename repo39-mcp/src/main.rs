mod changes;
mod config;
mod deps;
mod git;
mod glob;
mod identify;
mod map;
mod mcp;
mod util;
mod walk;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mcp::run_mcp_stdio().await
}
