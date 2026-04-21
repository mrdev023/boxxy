use serde::{Deserialize, Serialize};
use zbus::proxy;
use zbus::zvariant::{OwnedFd, Type};

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct SpawnOptions {
    pub cwd: String,
    pub argv: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cols: u16,
    pub rows: u16,
}

#[proxy(
    interface = "dev.boxxy.BoxxyTerminal.Agent.Pty",
    default_path = "/dev/boxxy/BoxxyTerminal/Agent/Pty",
    gen_blocking = false
)]
pub trait AgentPty {
    async fn get_preferred_shell(&self) -> zbus::Result<String>;
    async fn create_pty(&self) -> zbus::Result<OwnedFd>;
    async fn spawn(&self, pty_master: OwnedFd, options: SpawnOptions) -> zbus::Result<u32>;
    async fn get_cwd(&self, pid: u32) -> zbus::Result<String>;
    async fn get_foreground_process(&self, pid: u32) -> zbus::Result<String>;
    async fn get_running_processes(&self, pid: u32) -> zbus::Result<Vec<(u32, String)>>;
    async fn signal_process_group(&self, pid: u32, signal: i32) -> zbus::Result<()>;
    async fn set_foreground_tracking(&self, pid: u32, enabled: bool) -> zbus::Result<()>;
    async fn get_environment_variable(&self, name: String) -> zbus::Result<String>;

    #[zbus(signal)]
    async fn exited(&self, pid: u32, exit_code: i32) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn foreground_process_changed(&self, pid: u32, process_name: String) -> zbus::Result<()>;
}
