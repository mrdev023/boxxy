use log::{debug, warn};
use std::path::PathBuf;

pub fn load_prompt_fallback(_resource_path: &str, filename: &str) -> String {
    // Since the engine may run in the host daemon, we cannot depend on GResource/GTK.
    // We prioritize local files in the filesystem.

    // 1. Try standard installation path
    if let Some(home) = home::home_dir() {
        let home: PathBuf = home;
        let install_path = home.join(".local/share/boxxy-terminal/prompts").join(filename);
        if let Ok(content) = std::fs::read_to_string(&install_path) {
            debug!("Loaded prompt from install path: {:?}", install_path);
            return content;
        }
    }

    // 2. Try Local File Fallback (Development/Testing path)
    let possible_roots = vec![
        PathBuf::from("."),
        PathBuf::from("resources/prompts"),
        PathBuf::from("../../resources/prompts"),
        std::env::current_dir().unwrap_or_default().join("resources/prompts"),
    ];

    for root in possible_roots {
        let fallback_path = if root.is_dir() && !root.to_string_lossy().contains("prompts") {
             root.join("resources").join("prompts").join(filename)
        } else {
             root.join(filename)
        };
        
        if let Ok(content) = std::fs::read_to_string(&fallback_path) {
            debug!("Loaded prompt from fallback file: {:?}", fallback_path);
            return content;
        }
    }

    // 3. Last resort: Hardcoded minimal fallback to prevent crash
    warn!("CRITICAL: Failed to find prompt file {}. Using minimal built-in fallback.", filename);
    
    if filename == "claw.md" {
        return "You are Boxxy, a helpful AI assistant. Available skills: {{available_skills}}".to_string();
    }

    format!("Fallback content for {}", filename)
}
