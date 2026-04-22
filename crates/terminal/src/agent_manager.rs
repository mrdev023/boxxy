use anyhow::{Context, Result};
use boxxy_agent::ipc::claw::AgentClawProxy;
use boxxy_agent::ipc::pty::{AgentPtyProxy, SpawnOptions};
use boxxy_agent::ipc::AgentProxy;
use boxxy_claw_protocol::{ClawMessage, ClawEngineEvent, ClawEnvironment};
use zbus::Connection;
use zbus::zvariant::OwnedFd;
use futures_util::StreamExt;
use serde_json;

#[derive(Clone)]
pub struct AgentManager {
    agent_proxy: AgentProxy<'static>,
    proxy: AgentPtyProxy<'static>,
    claw_proxy: AgentClawProxy<'static>,
}

impl AgentManager {
    pub async fn new() -> Result<Self> {
        let conn = Connection::session().await.context("Failed to connect to Session Bus")?;

        let agent_proxy = AgentProxy::builder(&conn)
            .destination("dev.boxxy.BoxxyAgent")?
            .build()
            .await
            .context("Failed to create AgentProxy on Session Bus")?;

        let proxy = AgentPtyProxy::builder(&conn)
            .destination("dev.boxxy.BoxxyAgent")?
            .build()
            .await
            .context("Failed to create AgentPtyProxy on Session Bus")?;

        let claw_proxy = AgentClawProxy::builder(&conn)
            .destination("dev.boxxy.BoxxyAgent")?
            .build()
            .await
            .context("Failed to create AgentClawProxy on Session Bus")?;

        // Notify daemon that a client has connected
        let _ = agent_proxy.notify_client_connected().await;

        Ok(Self { agent_proxy, proxy, claw_proxy })
    }

    pub async fn disconnect(&self) {
        let _ = self.agent_proxy.notify_client_disconnected().await;
    }

    pub async fn update_credentials(&self, api_keys: std::collections::HashMap<String, String>, ollama_url: String) -> Result<()> {
        let _ : () = self.agent_proxy.update_credentials(api_keys, ollama_url).await
            .context("Failed to update agent credentials")?;
        Ok(())
    }

    pub async fn create_claw_session(&self, pane_id: String) -> Result<(String, async_channel::Receiver<ClawEngineEvent>)> {
        let session_id = self.agent_proxy.create_claw_session(pane_id).await
            .context("Failed to create Claw session via Agent")?;
            
        let (tx, rx) = async_channel::unbounded();
        
        // Subscribe to events for this session
        let mut stream = self.agent_proxy.receive_claw_event().await
            .context("Failed to subscribe to Claw events")?;
            
        let session_id_filter = session_id.clone();
        tokio::spawn(async move {
            while let Some(msg) = stream.next().await {
                if let Ok(args) = msg.args() {
                    if args.session_id == session_id_filter {
                        if let Ok(event) = serde_json::from_str::<ClawEngineEvent>(&args.event_json) {
                            let _ = tx.send(event).await;
                        }
                    }
                }
            }
        });
        
        Ok((session_id, rx))
    }

    pub async fn post_claw_message(&self, session_id: String, message: ClawMessage) -> Result<()> {
        let message_json = serde_json::to_string(&message)
            .context("Failed to serialize ClawMessage")?;
        self.agent_proxy.post_claw_message(session_id, message_json).await
            .context("Failed to post Claw message via Agent")
    }

    pub fn proxy(&self) -> &AgentPtyProxy<'static> {
        &self.proxy
    }

    pub fn claw_proxy(&self) -> &AgentClawProxy<'static> {
        &self.claw_proxy
    }

    pub async fn get_preferred_shell(&self) -> Result<String> {
        self.proxy
            .get_preferred_shell()
            .await
            .context("Agent get_preferred_shell failed")
    }

    pub async fn create_pty(&self) -> Result<OwnedFd> {
        self.proxy
            .create_pty()
            .await
            .context("Agent create_pty failed")
    }

    pub async fn spawn_process(&self, pty_master: OwnedFd, options: SpawnOptions) -> Result<u32> {
        self.proxy
            .spawn(pty_master, options)
            .await
            .context("Agent spawn failed")
    }

    pub async fn get_cwd(&self, pid: u32) -> Result<String> {
        self.proxy
            .get_cwd(pid)
            .await
            .context("Agent get_cwd failed")
    }

    pub async fn get_foreground_process(&self, pid: u32) -> Result<String> {
        self.proxy
            .get_foreground_process(pid)
            .await
            .context("Agent get_foreground_process failed")
    }

    pub async fn get_running_processes(&self, pid: u32) -> Result<Vec<(u32, String)>> {
        self.proxy
            .get_running_processes(pid)
            .await
            .context("Agent get_running_processes failed")
    }

    pub async fn set_foreground_tracking(&self, pid: u32, enabled: bool) -> Result<()> {
        self.proxy
            .set_foreground_tracking(pid, enabled)
            .await
            .context("Agent set_foreground_tracking failed")
    }

    pub async fn signal_process_group(&self, pid: u32, signal: i32) -> Result<()> {
        self.proxy
            .signal_process_group(pid, signal)
            .await
            .context("Agent signal_process_group failed")
    }

    pub async fn set_persistence(&self, pid: u32, enabled: bool) -> Result<()> {
        self.proxy
            .set_persistence(pid, enabled)
            .await
            .context("Agent set_persistence failed")
    }

    /// Returns the raw `DetachOutcome` code: 0=Terminated, 1=Detached,
    /// 2=StillViewed, 3=DetachedUnbuffered, u32::MAX=Unknown.
    pub async fn detach(&self, pid: u32) -> Result<u32> {
        self.proxy
            .detach(pid)
            .await
            .context("Agent detach failed")
    }

    pub async fn list_detached_sessions(&self) -> Result<Vec<(u32, String, u64)>> {
        self.proxy
            .list_detached_sessions()
            .await
            .context("Agent list_detached_sessions failed")
    }

    pub async fn reattach(&self, pid: u32) -> Result<(Vec<u8>, OwnedFd)> {
        self.proxy
            .reattach(pid)
            .await
            .context("Agent reattach failed")
    }
}

/// Implementation of ClawEnvironment that forwards calls over D-Bus.
/// Used by scenarios or UI-side code that still needs to run Claw sessions locally.
pub struct DbusClawEnvironment {
    proxy: AgentClawProxy<'static>,
}

impl DbusClawEnvironment {
    pub fn new(proxy: AgentClawProxy<'static>) -> Self {
        Self { proxy }
    }
}

#[async_trait::async_trait]
impl ClawEnvironment for DbusClawEnvironment {
    async fn exec_shell(&self, command: String) -> Result<(i32, String, String)> {
        self.proxy.exec_shell(command).await.context("D-Bus exec_shell failed")
    }

    async fn read_file(&self, path: String, start_line: u32, end_line: u32) -> Result<String> {
        self.proxy.read_file(path, start_line, end_line).await.context("D-Bus read_file failed")
    }

    async fn write_file(&self, path: String, content: String) -> Result<()> {
        self.proxy.write_file(path, content).await.context("D-Bus write_file failed")
    }

    async fn list_directory(&self, path: String) -> Result<Vec<(String, bool, u64)>> {
        self.proxy.list_directory(path).await.context("D-Bus list_directory failed")
    }

    async fn delete_file(&self, path: String) -> Result<()> {
        self.proxy.delete_file(path).await.context("D-Bus delete_file failed")
    }

    async fn get_system_info(&self) -> Result<String> {
        self.proxy.get_system_info().await.context("D-Bus get_system_info failed")
    }

    async fn list_processes(&self) -> Result<Vec<(u32, String, f64, u64, u64, u64)>> {
        self.proxy.list_processes().await.context("D-Bus list_processes failed")
    }

    async fn kill_process(&self, pid: u32, signal: i32) -> Result<()> {
        self.proxy.kill_process(pid, signal).await.context("D-Bus kill_process failed")
    }

    async fn get_clipboard(&self) -> Result<String> {
        self.proxy.get_clipboard().await.context("D-Bus get_clipboard failed")
    }

    async fn set_clipboard(&self, text: String) -> Result<()> {
        self.proxy.set_clipboard(text).await.context("D-Bus set_clipboard failed")
    }
}
