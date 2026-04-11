pub mod schema;
pub mod runner;

use std::path::Path;
use anyhow::Result;

pub async fn run_scenario_from_file(path: impl AsRef<Path>) -> Result<()> {
    let content = tokio::fs::read_to_string(path).await?;
    let scenario: schema::Scenario = serde_yml::from_str(&content)?;
    
    let mut runner = runner::ScenarioRunner::new(scenario).await?;
    runner.run().await
}
