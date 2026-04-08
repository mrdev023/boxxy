use crate::config::McpTransport;
use anyhow::{Context, Result};
use rmcp::service::RunningService;
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::{RoleClient, serve_client};
use std::collections::HashMap;
use tokio::process::Command;

pub async fn build_stdio_client(
    command_name: &str,
    args: &[String],
    env: &HashMap<String, String>,
) -> Result<RunningService<RoleClient, ()>> {
    let mut cmd = Command::new(command_name);
    cmd.args(args);
    cmd.kill_on_drop(true);

    for (k, v) in env {
        // TODO: Resolve "$KEYCHAIN:" values here
        cmd.env(k, v);
    }

    // `process_wrap::tokio::CommandWrap` implements `From<tokio::process::Command>`
    let transport =
        TokioChildProcess::new(cmd).context("Failed to initialize Stdio MCP transport")?;

    let client = serve_client((), transport)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to serve client: {:?}", e))?;

    Ok(client)
}
