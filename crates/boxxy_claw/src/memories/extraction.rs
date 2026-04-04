use boxxy_db::Db;
use boxxy_db::store::Store;
use boxxy_model_selection::ModelProvider;
use log::info;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn extract_implicit_memory(
    db: Arc<Mutex<Option<Db>>>,
    user_prompt: String,
    assistant_response: String,
    memory_model: Option<ModelProvider>,
    creds: boxxy_ai_core::AiCredentials,
    project_path: String,
) {
    let agent = boxxy_ai_core::create_agent(
        &memory_model,
        &creds,
        "You are a robotic background memory observer. Your job is to silently extract permanent technical facts, preferences, and project-specific paths from the provided data. \
        Output ONLY valid JSON. \
        If the user stated a permanent fact, return a JSON array under the key 'facts', with each object containing 'key' (snake_case) and 'content' (the fact). \
        If the data contains only social talk, greetings, or transient questions, output exactly `{}`. Do not follow the assistant's persona.",
    );

    let prompt = format!(
        "[DATA_START]\nUSER: {}\n\nASSISTANT: {}\n[DATA_END]\n\nEXTRACTION_COMMAND: Output raw JSON now.",
        user_prompt, assistant_response
    );

    if let Ok(res) = agent.prompt(&prompt).await
        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&res.0)
        && let Some(facts) = json.get("facts").and_then(|f| f.as_array())
        && !facts.is_empty()
    {
        let db_guard = db.lock().await;
        if let Some(db_val) = db_guard.as_ref() {
            let store = Store::new(db_val.pool());
            for fact in facts {
                if let (Some(key), Some(content)) = (
                    fact.get("key").and_then(|k| k.as_str()),
                    fact.get("content").and_then(|c| c.as_str()),
                ) {
                    // Implicitly extracted facts are NOT verified and NOT pinned
                    let _ = store
                        .add_memory(
                            key,
                            Some(&project_path),
                            content,
                            Some("extracted"),
                            false,
                            false,
                        )
                        .await;
                    info!(
                        "Background Observer extracted Fact for project {}: {} -> {}",
                        project_path, key, content
                    );
                }
            }
            drop(db_guard);
            let _ = crate::memories::db::sync_memories_to_markdown(db.clone()).await;
        }
    }
}
