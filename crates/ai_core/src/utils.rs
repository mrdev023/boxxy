use std::sync::{Arc, OnceLock, RwLock};
use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static LOCATION_CACHE: OnceLock<Arc<RwLock<Option<LocationContext>>>> = OnceLock::new();

#[derive(serde::Deserialize, Clone, Debug)]
pub struct LocationContext {
    pub city: String,
    pub country: String,
    pub timezone: String,
}

/// Returns a reference to the global multi-threaded Tokio runtime.
/// This runtime is used for background tasks (I/O, CPU-heavy work)
/// to keep them off the GTK UI thread.
pub fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime")
    })
}

/// Returns true if the application is running inside a Flatpak sandbox.
pub fn is_flatpak() -> bool {
    ashpd::is_sandboxed()
}

/// Fetches the current location context in the background.
pub async fn fetch_location_context() {
    let cache = LOCATION_CACHE.get_or_init(|| Arc::new(RwLock::new(None)));

    // Don't re-fetch if we already have it
    if cache.read().unwrap().is_some() {
        return;
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();

    // Use http://ip-api.com/json/ (Free, no key required, returns city/country/timezone)
    match client.get("http://ip-api.com/json/").send().await {
        Ok(res) => {
            if let Ok(loc) = res.json::<LocationContext>().await {
                *cache.write().unwrap() = Some(loc);
            }
        }
        Err(e) => {
            log::warn!("Failed to fetch location context: {}", e);
        }
    }
}

/// Returns the current location context from cache.
pub fn get_location_context() -> Option<LocationContext> {
    LOCATION_CACHE.get()?.read().unwrap().clone()
}
