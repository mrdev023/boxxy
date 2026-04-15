# Boxxy Agent

## Overview

The `boxxy-agent` is a privileged host-level daemon designed to bridge the gap between the sandboxed Boxxy UI (Flatpak) and the host system. It handles terminal PTY management, agentic host operations (Claw), and background maintenance tasks.

## Architecture: Modular Subsystems

The agent is built using a **Subsystem Architecture** to ensure strict separation of concerns and predictable performance. Each subsystem runs in its own isolated async context.

### 1. PTY Subsystem (Interactive / High Priority)
- **Focus:** Real-time terminal I/O, process spawning, and signal propagation.
- **Interface:** `dev.boxxy.BoxxyTerminal.Agent.Pty`
- **Isolation:** Operates independently of heavy database or network tasks to ensure zero latency during terminal interactions.

### 2. Claw Subsystem (Agent Capabilities)
- **Focus:** Executing host-level tools for the Boxxy-Claw reasoning engine.
- **Interface:** `dev.boxxy.BoxxyTerminal.Agent.Claw`
- **Responsibilities:** Shell execution, secure file I/O (with path blacklisting), and system telemetry gathering.

### 3. Maintenance Engine (Batch / Low Priority)
- **Focus:** Background tasks like telemetry journaling and memory consolidation.
- **Interface:** `dev.boxxy.BoxxyTerminal.Agent.Maintenance`
- **Optimization:** Runs with lower process priority (`niceness 19`) and respects system power states.

## Lifecycle Management

### AgentMode State Machine
The agent explicitly tracks its lifecycle state:
- **`AttachedPTY`**: UI is connected. All subsystems are active.
- **`GhostMaintenance`**: UI is disconnected. The agent "sheds" the interactive PTY subsystem to minimize its RAM footprint (~3MB) while keeping background maintenance loops alive.

### Graceful Shutdown
The agent utilizes a global `CancellationToken` for coordinated shutdowns. When a `SIGTERM` or socket disconnection occurs, all subsystems are signaled to finish critical operations (like flushing telemetry) before the process exits.

## Security & Sandbox Escape

-   **Host Escape:** Spawned via `flatpak-spawn --host` to operate outside the Flatpak sandbox.
-   **FD Forwarding:** Uses a private Unix socket pair. The socket is forwarded into the sandbox via FD 3 and reconstructed by `zbus`.
-   **Security Blacklist:** The Claw subsystem enforces a strict blacklist for file operations (e.g., `/etc/shadow`, `.ssh/id_rsa`) to prevent accidental or malicious data exfiltration.

## Implementation Details

1.  **D-Bus Multiplexing:** Leverages native D-Bus object paths (`/dev/boxxy/BoxxyTerminal/Agent/*`) to route messages directly to the appropriate subsystem without custom internal multiplexing.
2.  **Environment Seeding:** In Flatpak mode, the agent seeds a baseline host environment (`HOME`, `USER`, `SHELL`) from the host's passwd entry to ensure the user's shell behaves as expected.
3.  **Process Tracking:** Monitors Terminal Process Group IDs (`tpgid`) to report foreground process changes, enabling UI modality awareness.

## Public Traits & Structs

### `AgentPtyProxy` / `AgentClawProxy`
Specialized D-Bus proxies used by the UI to interact with specific subsystems.

### `SpawnOptions` (struct)
- `cwd`: The initial working directory.
- `argv`: The command and arguments.
- `env`: Environment variables to forward.
- `cols`/`rows`: Initial PTY terminal size.
