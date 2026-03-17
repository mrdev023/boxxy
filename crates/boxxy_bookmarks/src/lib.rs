pub mod editor;
pub mod manager;
pub mod parser;
pub mod sidebar;
pub mod tab;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bookmark {
    pub id: Uuid,
    pub name: String,
    pub filename: String,
    #[serde(default)]
    pub script: String,
}

impl Bookmark {
    pub fn new(name: String, script: String, filename: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            filename,
            script,
        }
    }
}
