use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashSet;

#[derive(Clone)]
pub struct AgentState {
    pub tracked_pids: Arc<RwLock<HashSet<u32>>>,
    // Add other persistent state (e.g., config, power status) here later
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            tracked_pids: Arc::new(RwLock::new(HashSet::new())),
        }
    }
}

impl AgentState {
    pub fn new() -> Self {
        Self::default()
    }
}
