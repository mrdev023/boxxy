use anyhow::Result;
use log::{info, warn};
use zbus::{Connection, proxy};

pub const WELL_KNOWN_NAME: &str = "dev.boxxy.BoxxyAgent";
pub const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
pub enum ClaimResult {
    /// We are the first — proceed to start the daemon loop.
    Claimed,
    /// An existing daemon with the same or newer version is running.
    HandedOff,
    /// We triggered the existing daemon to update itself; it will restart.
    Upgraded,
}

/// Proxy pointing at the running Ghost's version interface.
#[proxy(
    interface = "dev.boxxy.BoxxyTerminal.Agent",
    default_service = "dev.boxxy.BoxxyAgent",
    default_path = "/dev/boxxy/Agent"
)]
trait AgentVersion {
    async fn get_version(&self) -> zbus::Result<String>;
    async fn request_reload(&self) -> zbus::Result<()>;
}

pub async fn try_claim_or_handoff() -> Result<ClaimResult> {
    let conn = Connection::session().await?;

    // Try to claim the well-known name with "do not queue" semantics.
    use zbus::fdo::DBusProxy;
    let dbus = DBusProxy::new(&conn).await?;

    let flags = zbus::fdo::RequestNameFlags::DoNotQueue.into();
    let name: zbus::names::WellKnownName = WELL_KNOWN_NAME.try_into()?;
    let reply = dbus.request_name(name, flags).await?;

    use zbus::fdo::RequestNameReply::*;
    match reply {
        PrimaryOwner => Ok(ClaimResult::Claimed),

        Exists | AlreadyOwner | InQueue => {
            // Ghost is running — do a version check.
            match AgentVersionProxy::new(&conn).await {
                Ok(proxy) => {
                    let running_ver = proxy.get_version().await.unwrap_or_default();
                    if running_ver == AGENT_VERSION {
                        info!("Ghost version matches ({running_ver}) — standing down");
                        Ok(ClaimResult::HandedOff)
                    } else {
                        warn!("Ghost version mismatch: running={running_ver} new={AGENT_VERSION}");
                        // Ask the Ghost to reload itself from the newly-deployed binary.
                        let _ = proxy.request_reload().await;
                        Ok(ClaimResult::Upgraded)
                    }
                }
                Err(e) => {
                    warn!("Could not reach Ghost for version check: {e}");
                    Ok(ClaimResult::HandedOff)
                }
            }
        }
    }
}
