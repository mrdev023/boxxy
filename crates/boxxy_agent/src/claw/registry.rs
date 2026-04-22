//! Persistent pane-to-agent identity mapping.
//!
//! The daemon is the single source of truth for agent identity. Each
//! pane gets a stable `(agent_name, session_id)` that is saved to
//! disk so it survives UI restarts and daemon reloads.
//!
//! Storage: `{XDG_DATA_HOME}/boxxy-terminal/agent-registry.json`,
//! rewritten atomically on every mutation via a tempfile rename.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub agent_name: String,
    pub session_id: String,
}

/// Thread-safe, file-backed map from `pane_id` → `AgentIdentity`.
///
/// Reads hit the in-memory copy. Writes update the map and then schedule a
/// best-effort async flush to disk so the IPC caller doesn't block on I/O.
pub struct AgentRegistry {
    inner: RwLock<HashMap<String, AgentIdentity>>,
    path: PathBuf,
}

impl AgentRegistry {
    /// Loads the registry from disk, or returns an empty one if the file
    /// doesn't exist / can't be parsed. A corrupt file is not fatal — we log
    /// and start fresh so a schema change doesn't brick the daemon.
    pub fn load_or_default() -> Self {
        let path = registry_path();
        let map: HashMap<String, AgentIdentity> = match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
                log::warn!(
                    "agent-registry: failed to parse {}, resetting: {}",
                    path.display(),
                    e
                );
                HashMap::new()
            }),
            Err(_) => HashMap::new(),
        };

        Self {
            inner: RwLock::new(map),
            path,
        }
    }

    /// Returns the identity for a pane, or `None` if one has never been
    /// recorded.
    pub async fn get(&self, pane_id: &str) -> Option<AgentIdentity> {
        self.inner.read().await.get(pane_id).cloned()
    }

    /// Inserts or replaces the identity for a pane and triggers a flush.
    pub async fn set(&self, pane_id: String, identity: AgentIdentity) {
        let snapshot = {
            let mut guard = self.inner.write().await;
            guard.insert(pane_id, identity);
            guard.clone()
        };
        self.flush_background(snapshot);
    }

    /// Removes a pane's identity. Called when a pane is permanently closed
    /// (not merely when its UI detaches).
    pub async fn remove(&self, pane_id: &str) {
        let snapshot_opt = {
            let mut guard = self.inner.write().await;
            if guard.remove(pane_id).is_some() {
                Some(guard.clone())
            } else {
                None
            }
        };
        if let Some(snapshot) = snapshot_opt {
            self.flush_background(snapshot);
        }
    }

    /// Spawns a detached task that writes the given snapshot atomically.
    /// Write failures are logged but do not propagate — losing the registry
    /// degrades to "agents get fresh names next session", not a daemon crash.
    fn flush_background(&self, snapshot: HashMap<String, AgentIdentity>) {
        let path = self.path.clone();
        tokio::spawn(async move {
            if let Err(e) = write_atomically(&path, &snapshot).await {
                log::warn!("agent-registry: failed to persist: {}", e);
            }
        });
    }
}

fn registry_path() -> PathBuf {
    if let Some(dirs) = directories::ProjectDirs::from("org", "boxxy", "boxxy-terminal") {
        let dir = dirs.data_dir();
        let _ = std::fs::create_dir_all(dir);
        return dir.join("agent-registry.json");
    }
    // Fallback — unreachable on Linux, but keeps the code total.
    PathBuf::from("agent-registry.json")
}

async fn write_atomically(path: &PathBuf, map: &HashMap<String, AgentIdentity>) -> Result<()> {
    let json = serde_json::to_string_pretty(map).context("serialize registry")?;
    let tmp = path.with_extension("json.tmp");
    tokio::fs::write(&tmp, json)
        .await
        .with_context(|| format!("write {}", tmp.display()))?;
    tokio::fs::rename(&tmp, path)
        .await
        .with_context(|| format!("rename {}", tmp.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper: build a registry rooted at a given path, bypassing the
    /// XDG resolution so tests can use tempfiles.
    fn registry_at(path: PathBuf) -> AgentRegistry {
        AgentRegistry {
            inner: RwLock::new(HashMap::new()),
            path,
        }
    }

    #[tokio::test]
    async fn set_get_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let reg = registry_at(dir.path().join("reg.json"));

        reg.set(
            "pane-1".into(),
            AgentIdentity {
                agent_name: "red pony".into(),
                session_id: "abc".into(),
            },
        )
        .await;

        let got = reg.get("pane-1").await.expect("identity stored");
        assert_eq!(got.agent_name, "red pony");
        assert_eq!(got.session_id, "abc");

        assert!(reg.get("pane-unknown").await.is_none());
    }

    #[tokio::test]
    async fn persists_across_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("reg.json");

        {
            let reg = registry_at(path.clone());
            reg.set(
                "pane-1".into(),
                AgentIdentity {
                    agent_name: "red pony".into(),
                    session_id: "s1".into(),
                },
            )
            .await;
            // Let the background flush run.
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        // Re-load via the public API, pointing XDG at the temp dir would be
        // painful across platforms; instead verify the on-disk JSON parses
        // and contains the entry.
        let raw = tokio::fs::read_to_string(&path).await.unwrap();
        let parsed: HashMap<String, AgentIdentity> = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed.get("pane-1").unwrap().agent_name, "red pony");
    }

    #[tokio::test]
    async fn remove_forgets_identity() {
        let dir = tempfile::tempdir().unwrap();
        let reg = registry_at(dir.path().join("reg.json"));

        reg.set(
            "pane-1".into(),
            AgentIdentity {
                agent_name: "ghost".into(),
                session_id: "s1".into(),
            },
        )
        .await;
        reg.remove("pane-1").await;

        assert!(reg.get("pane-1").await.is_none());
    }
}
