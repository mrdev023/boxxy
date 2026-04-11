use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Scenario {
    pub name: String,
    #[serde(default = "default_timeout")]
    pub timeout_sec: u64,
    pub panes: Vec<PaneSetup>,
    pub steps: Vec<Step>,
    #[serde(default)]
    pub assertions: Vec<Assertion>,
}

fn default_timeout() -> u64 {
    60
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PaneSetup {
    pub id: String,
    pub name: String,
    pub initial_cwd: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Step {
    Prompt {
        pane: String,
        prompt: String,
    },
    InjectTerminalOutput {
        pane: String,
        output: String,
    },
    WaitForStatus {
        pane: String,
        status: String, // "Active", "Suspended", "Thinking", etc.
        #[serde(default = "default_step_timeout")]
        timeout_sec: u64,
    },
    WaitForToolCall {
        pane: String,
        tool_name: String,
        #[serde(default = "default_step_timeout")]
        timeout_sec: u64,
    },
    Sleep {
        seconds: u64,
    },
}

fn default_step_timeout() -> u64 {
    30
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Assertion {
    FileContains {
        path: String,
        content: String,
    },
    CommandExitCode {
        command: String,
        expected_code: i32,
    },
    AgentMemoryContains {
        pane: String,
        key: String,
        content: String,
    },
}
