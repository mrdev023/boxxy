# Claw Protocol Crate (`boxxy-claw-protocol`)

## Role

Shared, GTK-free, engine-free DTO layer used across the UI ↔ daemon D-Bus boundary. Anything the UI and the reasoning engine need to agree on lives here; anything heavier does not.

The crate is the only place where a definition appears in *both* the UI process and the daemon process, so its contents are kept deliberately small and serialisable.

## Contents

- **`ClawMessage`**: every request the UI can send down to an active `ClawSession`. Includes `Initialize`, `Deactivate`, `Reload`, `ClawQuery`, `UserMessage`, approval replies (`FileWriteReply`, `KillProcessReply`, …), sleep/pin/web-search toggles, delegated-task plumbing, correlation-ID replies (`ScrollbackReply`), and `SettingsInvalidated` for cross-process settings sync.
- **`ClawEngineEvent`**: every event the engine emits upward — `AgentThinking`, `DiagnosisComplete`, proposal events (`ProposeFileWrite`, `ProposeKillProcess`, …), `Identity`, `PinStatusChanged`, `RequestScrollback`, `RestoreHistory`, `PushGlobalNotification`, etc. `RequestScrollback` uses a `request_id` with a matching `ClawMessage::ScrollbackReply` so no `oneshot::Sender` ever crosses the IPC boundary.
- **`PersistentClawRow`**: the row variants the sidebar renders and the DB stores (`User`, `Diagnosis`, `Suggested`, `ToolCall`, `ProcessList`, `SystemMessage`, `Command`). Round-trips through JSON + SQLite losslessly.
- **`AgentStatus`**: `{Off, Sleep, Waiting, Working, Locking{resource}, Faulted{reason}}` — the live state of an agent, used by the msgbar indicator and the engine FSM.
- **`ScheduledTask` + `TaskType` + `TaskStatus`**: the scheduled-reminders payload, including the cross-agent task registry shape.
- **`UsageWrapper`**: serde-safe wrapper for `rig::completion::Usage`. `{input_tokens, output_tokens}` only — no `total_tokens` field, sum at the call site.
- **`ClawEnvironment` trait**: the async interface the reasoning engine calls for privileged operations (`exec_shell`, `read_file`, `list_processes`, `get_clipboard`, …). Implemented in-process on the daemon (`boxxy_agent::claw::ClawSubsystem`) and forwarded over D-Bus by `boxxy_terminal::agent_manager::DbusClawEnvironment`.

## Design Rules

- **No GTK types.** This crate compiles without `gtk4`/`libadwaita`. The UI-side wrappers live in `boxxy-claw-ui`.
- **No `rig` / reasoning internals.** The engine crate (`boxxy-claw`) depends on this one, not the other way around.
- **All public types implement `Serialize + Deserialize`**, even the large enums. IPC transport currently uses JSON for readability; the types are wire-format-agnostic.
- **Correlation IDs over channels.** When the engine needs a response from the UI (scrollback, delegated-task reply), the request carries a `uuid::Uuid` request ID and the reply is a distinct `ClawMessage` variant with the same ID. No `Sender<T>` ever crosses a process boundary.
