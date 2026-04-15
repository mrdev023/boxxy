use crate::core::state::AgentState;
use zbus::fdo;
use zbus::interface;

pub struct MaintenanceSubsystem {
    state: AgentState,
}

impl MaintenanceSubsystem {
    pub fn new(state: AgentState) -> Self {
        Self { state }
    }
}

#[interface(name = "dev.boxxy.BoxxyTerminal.Agent.Maintenance")]
impl MaintenanceSubsystem {
    async fn get_maintenance_status(&self) -> fdo::Result<String> {
        Ok("idle".to_string())
    }

    async fn trigger_maintenance_now(&self) -> fdo::Result<()> {
        log::info!("Manual maintenance triggered via D-Bus");
        boxxy_telemetry::flush_journal().await;
        Ok(())
    }
}
