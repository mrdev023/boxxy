pub mod config;
pub mod component;
pub mod shortcuts;
pub use config::{AppState, CursorShape, Settings, ImagePreviewTrigger, SETTINGS_EVENT_BUS};
pub use component::PreferencesComponent;
