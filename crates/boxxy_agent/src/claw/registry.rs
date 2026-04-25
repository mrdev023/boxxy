//! Persistent pane-to-character assignment mapping and character catalog.
//!
//! The daemon is the single source of truth for character assignments. Each
//! pane gets a stable `(character_name, session_id)` that is saved to
//! disk so it survives UI restarts and daemon reloads.
//!
//! Storage: `{XDG_DATA_HOME}/boxxy-terminal/character-assignments.json`,
//! rewritten atomically on every mutation via a tempfile rename.

use anyhow::Result;
use boxxy_claw_protocol::characters::{CharacterInfo, CharacterStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterAssignment {
    pub character_id: String,
    pub session_id: String,
}

pub struct CharacterRegistry {
    assignments: RwLock<HashMap<String, CharacterAssignment>>,
    catalog: RwLock<Vec<CharacterInfo>>,
}

impl CharacterRegistry {
    pub fn load_or_default() -> Self {
        // Assignments will be loaded lazily or on demand if we shift fully to DB.
        // For now, we'll keep the in-memory map backed by the database.
        let catalog = boxxy_claw_protocol::character_loader::load_characters().unwrap_or_default();
        Self {
            assignments: RwLock::new(HashMap::new()),
            catalog: RwLock::new(catalog),
        }
    }

    pub async fn load_assignments_from_db(&self, db: &boxxy_db::Db) -> Result<()> {
        let records =
            sqlx::query("SELECT pane_id, character_id, session_id FROM active_pane_assignments")
                .fetch_all(db.pool())
                .await?;

        let mut guard = self.assignments.write().await;
        guard.clear();
        for r in records {
            use sqlx::Row;
            guard.insert(
                r.get("pane_id"),
                CharacterAssignment {
                    character_id: r.get("character_id"),
                    session_id: r.get("session_id"),
                },
            );
        }
        Ok(())
    }

    pub async fn get_assignment(&self, pane_id: &str) -> Option<CharacterAssignment> {
        self.assignments.read().await.get(pane_id).cloned()
    }

    pub async fn get_active_pane_for_character(
        &self,
        character_id: &str,
        exclude_pane_id: &str,
    ) -> Option<String> {
        let guard = self.assignments.read().await;
        for (pane_id, assignment) in guard.iter() {
            if pane_id != exclude_pane_id && assignment.character_id == character_id {
                return Some(pane_id.clone());
            }
        }
        None
    }

    pub async fn set_assignment(
        &self,
        db: &boxxy_db::Db,
        pane_id: String,
        assignment: CharacterAssignment,
    ) {
        {
            let mut guard = self.assignments.write().await;
            guard.insert(pane_id.clone(), assignment.clone());
        }

        let pool = db.pool().clone();
        tokio::spawn(async move {
            let res = sqlx::query(
                "INSERT INTO active_pane_assignments (pane_id, character_id, session_id, updated_at)
                 VALUES (?, ?, ?, CURRENT_TIMESTAMP)
                 ON CONFLICT(pane_id) DO UPDATE SET
                 character_id=excluded.character_id,
                 session_id=excluded.session_id,
                 updated_at=CURRENT_TIMESTAMP"
            )
            .bind(pane_id)
            .bind(assignment.character_id)
            .bind(assignment.session_id)
            .execute(&pool).await;

            if let Err(e) = res {
                log::error!("Failed to persist character assignment to db: {}", e);
            }
        });
    }

    pub async fn remove_assignment(&self, db: &boxxy_db::Db, pane_id: &str) {
        {
            let mut guard = self.assignments.write().await;
            guard.remove(pane_id);
        }

        let pool = db.pool().clone();
        let pane_id = pane_id.to_string();
        tokio::spawn(async move {
            let _ = sqlx::query("DELETE FROM active_pane_assignments WHERE pane_id = ?")
                .bind(pane_id)
                .execute(&pool)
                .await;
        });
    }

    pub async fn reload_catalog(&self) {
        let catalog = boxxy_claw_protocol::character_loader::load_characters().unwrap_or_default();
        let mut guard = self.catalog.write().await;
        *guard = catalog;
    }

    pub async fn get_full_registry(&self) -> Vec<CharacterInfo> {
        let catalog_guard = self.catalog.read().await;
        let assignments_guard = self.assignments.read().await;

        let mut result = catalog_guard.clone();
        for info in &mut result {
            let mut active_pane = None;
            for (pane_id, assignment) in assignments_guard.iter() {
                if assignment.character_id == info.config.id {
                    active_pane = Some(pane_id.clone());
                    break;
                }
            }

            if let Some(pane_id) = active_pane {
                info.status = CharacterStatus::Active { pane_id };
            } else {
                info.status = CharacterStatus::Available;
            }
        }
        result
    }
}
