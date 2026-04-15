use zbus::proxy;

#[proxy(
    interface = "dev.boxxy.BoxxyTerminal.Agent.Maintenance",
    default_path = "/dev/boxxy/BoxxyTerminal/Agent/Maintenance",
    gen_blocking = false
)]
pub trait AgentMaintenance {
    async fn get_maintenance_status(&self) -> zbus::Result<String>;
    async fn trigger_maintenance_now(&self) -> zbus::Result<()>;
}
