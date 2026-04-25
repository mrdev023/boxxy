use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU64;

lazy_static::lazy_static! {
    /// A globally accessible, lock-free reactive cache of the character registry.
    /// This is updated via D-Bus and read synchronously by the UI (e.g. MsgBar autocomplete).
    pub static ref CHARACTER_CACHE: ArcSwap<Vec<CharacterInfo>> = ArcSwap::from_pointee(Vec::new());
}

/// Bumped every time CHARACTER_CACHE is replaced. UI components poll this to
/// detect registry changes without subscribing to D-Bus signals directly.
pub static CHARACTER_CACHE_VERSION: AtomicU64 = AtomicU64::new(0);

/// Represents the static configuration of a Character, loaded from a CHARACTER.toml file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterConfig {
    /// Stable UUID — persists even when the character is renamed or its directory moved.
    /// Written to CHARACTER.toml on first load if absent.
    pub id: String,
    /// Directory slug (derived from the subdirectory name, not read from TOML).
    pub name: String,
    /// Human-readable name displayed in the UI.
    pub display_name: String,
    /// CSS hex color used for UI branding (e.g., "#7B61FF").
    pub color: String,
    /// High-level description of the character's role, injected into the system prompt.
    pub duties: String,
    /// Behavioral guidelines for the character, injected into the system prompt.
    pub personality: String,
}

/// A group of sessions whose recorded `character_id` no longer matches any character
/// in the loaded catalog — i.e., the character was deleted or its UUID changed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrphanGroup {
    /// The UUID that was recorded when these sessions were created.
    pub character_id: String,
    /// The display name as recorded at session-creation time (used for UI labelling).
    pub character_display_name: String,
    /// IDs of sessions in this group.
    pub session_ids: Vec<String>,
}

/// Represents the current runtime status of a Character.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CharacterStatus {
    /// The character is not currently active in any pane.
    Available,
    /// The character is currently active in a specific terminal pane.
    Active {
        /// The unique ID of the pane where this character is active.
        pane_id: String,
    },
}

/// A consolidated view of a Character's configuration and its current status.
/// This is the primary DTO sent across D-Bus for UI synchronization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterInfo {
    /// The static configuration loaded from disk.
    pub config: CharacterConfig,
    /// The dynamic status managed by the daemon.
    pub status: CharacterStatus,
    /// Indicates whether an AVATAR.png was found in the character's directory.
    pub has_avatar: bool,
}

impl CharacterInfo {
    /// Creates a new CharacterInfo instance.
    pub fn new(config: CharacterConfig, has_avatar: bool) -> Self {
        Self {
            config,
            status: CharacterStatus::Available,
            has_avatar,
        }
    }
}
