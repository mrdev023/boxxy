use log::debug;

pub fn load_prompt_fallback(resource_path: &str, filename: &str) -> String {
    // 1. Try GResource (Standard UI path)
    if let Ok(data) = gtk4::gio::resources_lookup_data(resource_path, gtk4::gio::ResourceLookupFlags::NONE) {
        if let Ok(content) = String::from_utf8(data.to_vec()) {
            return content;
        }
    }

    // 2. Try Local File Fallback (Headless/Testing path)
    // We assume we are running from the workspace root or can find resources/prompts/
    // We check several possible locations to be robust.
    let possible_roots = vec![
        std::path::PathBuf::from("."),
        std::path::PathBuf::from("..").join(".."),
        std::env::current_dir().unwrap_or_default(),
    ];

    for root in possible_roots {
        let fallback_path = root.join("resources").join("prompts").join(filename);
        if let Ok(content) = std::fs::read_to_string(&fallback_path) {
            debug!("Loaded prompt from fallback file: {:?}", fallback_path);
            return content;
        }
    }

    // 3. Last resort: built-in defaults or panic
    panic!("CRITICAL: Failed to load prompt resource {} or find fallback file {}", resource_path, filename);
}
