pub mod claw;
pub mod maintenance;
pub mod pty;

use crate::claw::{AgentIdentity, notifier};
use crate::daemon::DaemonCore;
use anyhow::Result;
use boxxy_claw_protocol::{ClawEngineEvent, ClawMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, watch};
use zbus::{Connection, connection::Builder, object_server::SignalEmitter};

pub async fn start_services(
    core: Arc<DaemonCore>,
    client_count_tx: watch::Sender<usize>,
) -> Result<Connection> {
    let claw_subsystem = Arc::new(crate::claw::ClawSubsystem::new(core.state.clone()));
    let session_manager = Arc::new(Mutex::new(ClawSessionManager::new(claw_subsystem.clone())));

    let conn = Builder::session()?
        .name("dev.boxxy.BoxxyAgent")?
        .serve_at(
            "/dev/boxxy/Agent",
            AgentInterface {
                core: core.clone(),
                client_count_tx: client_count_tx.clone(),
                session_manager,
            },
        )?
        .serve_at(
            "/dev/boxxy/BoxxyTerminal/Agent/Pty",
            crate::pty::PtySubsystem::new(core.state.clone()),
        )?
        .serve_at(
            "/dev/boxxy/BoxxyTerminal/Agent/Claw",
            crate::claw::ClawSubsystem::new(core.state.clone()),
        )?
        .serve_at(
            "/dev/boxxy/BoxxyTerminal/Agent/Maintenance",
            crate::maintenance::MaintenanceSubsystem::new(
                core.state.clone(),
                core.dream_status.clone(),
                core.power.clone(),
            ),
        )?
        .build()
        .await?;

    Ok(conn)
}

/// Bookkeeping for one hosted `ClawSession`. We keep the pane id and agent
/// name so the UI can reattach by querying `list_sessions()`.
struct HostedSession {
    pane_id: String,
    agent_name: String,
    tx: async_channel::Sender<ClawMessage>,
}

struct ClawSessionManager {
    sessions: HashMap<String, HostedSession>,
    env: Arc<crate::claw::ClawSubsystem>,
}

impl ClawSessionManager {
    fn new(env: Arc<crate::claw::ClawSubsystem>) -> Self {
        Self {
            sessions: HashMap::new(),
            env,
        }
    }

    /// Drops any sessions whose sender is closed — the actor has exited on
    /// its own, or the channel was dropped. Keeps the map from growing
    /// unboundedly over the daemon's lifetime.
    fn prune_dead(&mut self) {
        self.sessions.retain(|_, s| !s.tx.is_closed());
    }
}

#[zbus::proxy(
    interface = "dev.boxxy.BoxxyTerminal.Agent",
    default_service = "dev.boxxy.BoxxyAgent",
    default_path = "/dev/boxxy/Agent"
)]
pub trait Agent {
    async fn get_version(&self) -> zbus::Result<String>;
    async fn update_credentials(
        &self,
        api_keys: std::collections::HashMap<String, String>,
        ollama_url: String,
    ) -> zbus::Result<()>;
    async fn notify_client_connected(&self) -> zbus::Result<()>;
    async fn notify_client_disconnected(&self) -> zbus::Result<()>;
    async fn request_reload(&self) -> zbus::Result<()>;
    async fn request_stop(&self) -> zbus::Result<()>;

    // Claw Session Management (Using JSON for complex types)
    async fn create_claw_session(&self, pane_id: String) -> zbus::Result<String>;
    async fn post_claw_message(&self, session_id: String, message_json: String)
    -> zbus::Result<()>;
    /// Returns `[(session_id, pane_id, agent_name), ...]` for every session
    /// the daemon is currently hosting. Used by a freshly launched UI to
    /// discover sessions it can reattach to.
    async fn list_claw_sessions(&self) -> zbus::Result<Vec<(String, String, String)>>;
    /// Tears down a session actor and forgets its identity in the registry.
    /// Called when a pane is permanently closed (not just detached).
    async fn end_claw_session(&self, session_id: String) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn claw_event(&self, session_id: String, event_json: String) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn desktop_notification(&self, title: String, message: String) -> zbus::Result<()>;
}

struct AgentInterface {
    pub core: Arc<DaemonCore>,
    pub client_count_tx: watch::Sender<usize>,
    session_manager: Arc<Mutex<ClawSessionManager>>,
}

#[zbus::interface(name = "dev.boxxy.BoxxyTerminal.Agent")]
impl AgentInterface {
    async fn get_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    async fn update_credentials(
        &self,
        api_keys: std::collections::HashMap<String, String>,
        ollama_url: String,
    ) {
        log::info!("Updating credentials for {} providers", api_keys.len());
        let mut keys_guard = self.core.state.api_keys.write().await;
        *keys_guard = api_keys;
        let mut url_guard = self.core.state.ollama_url.write().await;
        *url_guard = ollama_url;
    }

    #[zbus(signal)]
    async fn desktop_notification(
        emitter: &SignalEmitter<'_>,
        title: String,
        message: String,
    ) -> zbus::Result<()>;

    async fn notify_client_connected(&self) {
        let current = *self.client_count_tx.borrow();
        let _ = self.client_count_tx.send(current + 1);
        log::info!("Client connected. Total clients: {}", current + 1);
    }

    async fn notify_client_disconnected(&self) {
        let current = *self.client_count_tx.borrow();
        let new_count = current.saturating_sub(1);
        let _ = self.client_count_tx.send(new_count);
        log::info!("Client disconnected. Total clients: {}", new_count);
    }

    async fn request_reload(&self) {
        log::info!("Self-reload requested. Executing new binary...");

        let exe = std::env::current_exe().unwrap_or_default();
        let args: Vec<String> = std::env::args().collect();

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let c_exe = std::ffi::CString::new(exe.to_string_lossy().to_string()).unwrap();
            let c_args: Vec<std::ffi::CString> = args
                .iter()
                .map(|s| std::ffi::CString::new(s.as_str()).unwrap())
                .collect();
            let _ = nix::unistd::execvp(&c_exe, &c_args);
        });
    }

    async fn request_stop(&self) {
        log::info!("Stop requested. Exiting...");
        std::process::exit(0);
    }

    async fn create_claw_session(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        #[zbus(connection)] conn: &Connection,
        pane_id: String,
    ) -> String {
        // Resolve identity via the persistent registry. New panes get a
        // freshly minted petname; reconnecting UIs see the same name as last
        // time. Session UUID is re-minted per actor run — it identifies the
        // in-memory actor, not the durable agent.
        let persisted_name = self.core.registry.get(&pane_id).await;
        let agent_name = persisted_name
            .as_ref()
            .map(|id| id.agent_name.clone())
            .unwrap_or_else(|| {
                petname::petname(2, " ").unwrap_or_else(|| "Unknown Agent".to_string())
            });

        log::info!(
            "Creating Claw session for pane {} as '{}'",
            pane_id,
            agent_name
        );

        let (session, tx, rx_ui) = boxxy_claw::engine::ClawSession::new(pane_id.clone());
        let fresh_session_id = session.session_id.clone();
        let session = session.with_identity(agent_name.clone(), fresh_session_id.clone());
        let session_id = fresh_session_id;

        // Persist the identity so the next `create_claw_session(pane_id)`
        // call resolves to the same agent.
        self.core
            .registry
            .set(
                pane_id.clone(),
                AgentIdentity {
                    agent_name: agent_name.clone(),
                    session_id: session_id.clone(),
                },
            )
            .await;

        {
            // Initialize session with core state
            let mut state_lock = session.state.lock().await;
            state_lock.tracked_pids = self.core.state.tracked_pids.clone();
            state_lock.api_keys = self.core.state.api_keys.clone();
            state_lock.ollama_url = self.core.state.ollama_url.clone();
            state_lock.tx_self = tx.clone();
        }

        {
            let mut manager = self.session_manager.lock().await;
            manager.prune_dead();
            manager.sessions.insert(
                session_id.clone(),
                HostedSession {
                    pane_id: pane_id.clone(),
                    agent_name: agent_name.clone(),
                    tx,
                },
            );
        }

        // Spawn the session worker with the agent's subsystem (which implements ClawEnvironment)
        let env = {
            let manager = self.session_manager.lock().await;
            manager.env.clone()
        };
        session.start(env);

        // Forward UI events from the session to the D-Bus signal. Certain
        // events (task completion, global notifications) also trigger a
        // host-level desktop toast via the freedesktop notifications portal
        // so the user hears about work that finishes while the UI is
        // detached.
        let session_id_clone = session_id.clone();
        let emitter = emitter.to_owned();
        let conn_clone = conn.clone();
        tokio::spawn(async move {
            while let Ok(event) = rx_ui.recv().await {
                maybe_send_desktop_notification(&conn_clone, &agent_name, &event).await;

                if let Ok(event_json) = serde_json::to_string(&event) {
                    let _ = Self::claw_event(&emitter, session_id_clone.clone(), event_json).await;
                }
            }
        });

        session_id
    }

    async fn post_claw_message(&self, session_id: String, message_json: String) {
        if let Ok(message) = serde_json::from_str::<ClawMessage>(&message_json) {
            let manager = self.session_manager.lock().await;
            if let Some(session) = manager.sessions.get(&session_id) {
                let _ = session.tx.send(message).await;
            }
        }
    }

    async fn list_claw_sessions(&self) -> Vec<(String, String, String)> {
        let mut manager = self.session_manager.lock().await;
        manager.prune_dead();
        manager
            .sessions
            .iter()
            .map(|(sid, s)| (sid.clone(), s.pane_id.clone(), s.agent_name.clone()))
            .collect()
    }

    async fn end_claw_session(&self, session_id: String) {
        let pane_id = {
            let mut manager = self.session_manager.lock().await;
            manager.sessions.remove(&session_id).map(|s| s.pane_id)
        };
        if let Some(pane_id) = pane_id {
            // Forget the agent's identity too — this pane is being disposed.
            self.core.registry.remove(&pane_id).await;
            log::info!(
                "Ended Claw session {} (pane {} removed from registry)",
                session_id,
                pane_id
            );
        }
    }

    #[zbus(signal)]
    async fn claw_event(
        emitter: &SignalEmitter<'_>,
        session_id: String,
        event_json: String,
    ) -> zbus::Result<()>;
}

/// Converts select `ClawEngineEvent`s into host desktop notifications.
///
/// We intentionally do NOT gate on "UI is detached". The notifications daemon
/// on most desktop environments suppresses toasts when the emitting app is
/// focused, so this is already the right behaviour; and when a background
/// task completes while the user is *looking* at another window, the toast
/// is still the signal they need.
async fn maybe_send_desktop_notification(
    conn: &Connection,
    agent_name: &str,
    event: &ClawEngineEvent,
) {
    match event {
        ClawEngineEvent::PushGlobalNotification { title, message } => {
            notifier::send(conn, title, message).await;
        }
        ClawEngineEvent::TaskCompleted {
            agent_name: task_agent,
            ..
        } => {
            let title = format!("{} finished a task", task_agent);
            notifier::send(
                conn,
                &title,
                &format!("Agent {} completed a background task.", agent_name),
            )
            .await;
        }
        _ => {}
    }
}
