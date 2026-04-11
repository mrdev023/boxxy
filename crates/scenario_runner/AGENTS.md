# Scenario Runner — Headless E2E Agent Testing

## Overview

The `scenario-runner` crate provides an automated End-to-End (E2E) testing framework for Boxxy-Claw. It allows developers to define complex multi-agent "scenarios" in YAML and execute them headlessly (without GTK) to verify agent behavior, tool usage, and swarm orchestration.

## Responsibilities

- **Headless Runtime:** Simulates the Boxxy environment by spawning `WorkspaceRegistry` and `ClawSession` actors directly.
- **Terminal Mocking:** Maintains in-memory string buffers for each pane, allowing agents to use `read_pane_buffer` and receive simulated output.
- **P2P Host Bridge:** Establishes a private zbus P2P connection to an internal `BoxxyAgent`. This allows agents to execute real shell commands and file operations in a safe, isolated `/tmp` directory.
- **Event Interception:** Listens to the `tx_ui` channel to verify that agents reach specific states (e.g., `Suspended`, `Thinking`) or call specific tools.

## How to Create a Scenario

Scenarios are defined in `crates/scenario_runner/scenarios/*.yml`. 

### YAML Schema

```yaml
name: "My Scenario Name"
timeout_sec: 60      # Global timeout
panes:
  - id: "server"     # Unique ID for the test
    name: "Server"   # Mnemonic name used by agents
steps:
  - action: "prompt"
    pane: "server"
    prompt: "Monitor logs for error..."
  - action: "inject_terminal_output"
    pane: "server"
    output: "ERROR: DB Down\n"
  - action: "wait_for_status"
    pane: "server"
    status: "Suspended"
assertions:
  - type: "file_contains"
    path: "result.txt"
    content: "Success"
```

## How to Run Scenarios

You can run a specific scenario using the `scenario-runner` binary:

```bash
# Run a specific scenario
cargo run -p scenario-runner -- crates/scenario_runner/scenarios/01_pubsub_watchdog.yml

# Run with full logging
RUST_LOG=info cargo run -p scenario-runner -- crates/scenario_runner/scenarios/01_pubsub_watchdog.yml
```

## Safety Constraints

1. **Isolation:** The runner automatically creates a temporary directory (e.g., `/tmp/boxxy_test_...`) and sets it as the `CWD` for all agents. 
2. **Real Execution:** Shell commands run via `sys_shell_exec` are **real**. Do not include destructive commands (like `rm -rf /`) in your YAML files.
3. **Mocking:** Terminal output is mocked. Use `inject_terminal_output` in your YAML to simulate what an agent "sees" in the terminal.
