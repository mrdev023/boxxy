# Boxxy Agent (`boxxy-agent`)

## Role

Host-level daemon that sits between the Boxxy UI and the operating system. It owns every privileged operation (PTY spawning, shell-side tools, clipboard, memory consolidation) and exposes them to the UI process over D-Bus on the user's session bus.

A single instance serves all open UI windows. It is launched by the UI on demand via `ensure_agent_running` and claims `dev.boxxy.BoxxyAgent` on the session bus; a second launch detects the existing instance and exits.

## Binary Entry Points

The `boxxy-agent` binary accepts subcommands in addition to starting the daemon:

- `boxxy-agent` / `boxxy-agent start`: claims the session-bus name and runs the daemon loop.
- `boxxy-agent stop`: talks to a running daemon over D-Bus and requests a clean exit.
- `boxxy-agent restart`: tells a running daemon to `exec()` its own binary, picking up a newer version on disk without downtime.
- `boxxy-agent list-sessions`: prints every detached PTY session the daemon is hosting (`PID`, idle seconds, pane UUID). Useful while the "Detached Sessions" UI view isn't built yet.
- `--background`: daemonizes (double-fork + setsid) before starting the runtime. Called by the UI-side `agent_deployer` when it needs a host-detached process.

The binary also extracts itself from the Flatpak GResource bundle on first launch and writes the host-side copy into `~/.local/boxxy-terminal/boxxy-agent` so it can run outside the sandbox.

## Subsystems

Each D-Bus interface lives in its own module under `src/`. All of them share an `AgentState` (cheap-clone via `Arc`) that carries the process-wide PTY registry, API keys, ollama URL, and power/ghost-mode watchers.

### `pty/` — `dev.boxxy.BoxxyTerminal.Agent.Pty`

Latency-critical terminal I/O. Exposes:

- `create_pty`, `spawn`: open a master PTY, set window size, then `fork/exec` the shell. The daemon `dup()`s the master FD at spawn and keeps it in the session record — idle while a UI is attached, activated when `detach()` is called.
- `signal_process_group`, `get_foreground_process`, `get_cwd`, `get_running_processes`, `get_environment_variable`, `set_foreground_tracking`: host-side process introspection used by the UI + Claw.
- `set_persistence(pid, bool)`, `detach(pid)`, `reattach(pid)`, `list_detached_sessions()`: persistent-shells surface (experimental, see below).
- Emits `Exited`, `ForegroundProcessChanged` signals that the UI subscribes to.

**`pty::registry::PtyRegistry`** holds one `PtySession` per spawned shell, keyed by PID. Each session tracks `viewer_count`, `persistence_enabled`, the captured master FD, a reader task (only while detached), a 4 MB byte ring buffer (only while detached), and `last_activity`. A 60-second zombie-sweeper task running at `niceness 19` SIGTERMs any detached session idle longer than 4 hours and only runs while in ghost mode.

### `claw/` — `dev.boxxy.BoxxyTerminal.Agent.Claw`

Host-side implementation of `ClawEnvironment`: shell execution, file I/O with path blacklisting, list/kill processes, clipboard access, sysinfo. Path blacklist seeds from `~/.config/boxxy-terminal/boxxyclaw/BLACKLIST.md` (falling back to a built-in list that blocks `/etc/shadow`, `.ssh/id_rsa`, etc.).

**`claw::registry::AgentRegistry`** is the persistent `pane_id → AgentIdentity { agent_name, session_id }` map saved to `$XDG_DATA_HOME/boxxy-terminal/agent-registry.json` via atomic tempfile rename. It's what makes a pane's agent keep the same petname across UI restarts. `AgentInterface::create_claw_session` reuses a stored identity if one exists for the pane, otherwise mints a fresh petname and persists it.

**`claw::notifier`** wraps `org.freedesktop.Notifications` with a tiny zbus proxy so `ClawEngineEvent::PushGlobalNotification` and `TaskCompleted` turn into real desktop toasts — so background AI work that finishes while the UI is detached still reaches the user.

### `ipc/` — `dev.boxxy.BoxxyTerminal.Agent`

Top-level orchestration interface. Hosts `ClawSession` actors: `create_claw_session(pane_id)` spawns the reasoning engine with the agent's own `ClawSubsystem` as its `ClawEnvironment`, forwards session events as `claw_event(session_id, event_json)` D-Bus signals, and prunes dead sessions on every map touch. Additional methods: `list_claw_sessions`, `end_claw_session`, `update_credentials` (UI → daemon API-key sync), `notify_client_connected` / `notify_client_disconnected` (drives ghost mode), `request_reload` / `request_stop`.

### `maintenance/` — `dev.boxxy.BoxxyTerminal.Agent.Maintenance`

Background-work status surface. `get_maintenance_status()` returns one of `idle`, `disabled`, `paused_on_battery`, `paused_ui_attached`, `running`. `is_on_battery()` exposes the current UPower reading. `trigger_maintenance_now()` flushes the telemetry journal. The actual maintenance work is driven by modules under `daemon/` (below).

### `daemon/` — lifecycle + scheduling primitives

- **`daemon::power::PowerMonitor`**: subscribes to `org.freedesktop.UPower.OnBattery` and streams `PropertiesChanged` events into a `watch::Receiver<bool>`. Graceful fallback to permanent-AC when UPower isn't reachable (VMs, minimal desktops) so unrelated work isn't accidentally blocked.
- **`daemon::lifecycle::GhostMode`**: derives a `client_count == 0` bool from the tracker that `notify_client_{connected,disconnected}` drives. Subsystems clone it cheaply to gate their own work.
- **`daemon::priority`**: `set_current_thread_nice(19)` wrapper around `setpriority(PRIO_PROCESS, 0, ...)`. Called at the entry of the dreaming task and the zombie sweeper.
- **`daemon::dreaming`**: daemon-owned driver for the three-phase Dream Cycle. After a 10 s warm-up it sets `niceness 19`, then polls every minute: runs a cycle only when `enable_auto_dreaming` AND `!on_battery` AND `ghost_mode`. 15-minute cooldown between cycles. Publishes its current state into a shared `DreamStatusCell` read by `MaintenanceSubsystem`.
- **`daemon::singleton`**: implements the name-claim handshake so two agent processes never coexist. The second process either hands off (same version) or upgrades (newer binary).

## Security Posture

- **Sandbox escape**: on Flatpak, the UI spawns `boxxy-agent` via `flatpak-spawn --host`. The binary lives at `~/.local/boxxy-terminal/boxxy-agent` and runs as the host user. The session bus is shared between sandbox and host, so D-Bus is the entire IPC surface.
- **Settings hydration**: the daemon calls `boxxy_preferences::Settings::init()` before starting the runtime so the in-memory cache reads `settings.json` from disk. A `SettingsInvalidated` ClawMessage from the UI triggers `Settings::reload()` so the daemon picks up changes the UI just saved.
- **Credential sync**: `update_credentials(api_keys, ollama_url)` lets the UI push effective credentials to the daemon, and the claw engine also falls back to `Settings::load().get_effective_api_keys()` when the IPC-pushed keys are empty.
- **File I/O blacklist**: every `ClawSubsystem::read_file` / `write_file` / `delete_file` / `list_directory` call checks the path against `load_blacklist()` and rejects matches.

## Public Surface (UI-facing)

- `AgentProxy`: top-level interface — `Agent` (`dev.boxxy.BoxxyTerminal.Agent`).
- `AgentPtyProxy` + `SpawnOptions { cwd, argv, env, cols, rows, pane_id }`: PTY interface.
- `AgentClawProxy`: ClawEnvironment over D-Bus, used by `DbusClawEnvironment` in `boxxy-terminal`.
- `AgentMaintenanceProxy`: maintenance status queries.

## Testing

The crate has unit tests for the pieces that can be exercised without the full daemon loop: the agent registry round-trips + on-disk persistence, the PTY registry's ring buffer + detach/reattach semantics (including a real-PTY end-to-end test), the priority wrapper, power-monitor fallback, and ghost-mode transitions driven by the client-count watch channel. Run with `cargo test -p boxxy-agent --lib`.
