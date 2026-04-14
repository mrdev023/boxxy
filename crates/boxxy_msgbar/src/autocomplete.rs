use boxxy_core_widgets::autocomplete::{CompletionItem, CompletionProvider};

pub struct AgentCompletionProvider;

impl CompletionProvider for AgentCompletionProvider {
    fn trigger(&self) -> String {
        "@".to_string()
    }

    fn get_completions(&self, query: &str) -> Vec<CompletionItem> {
        let query_lower = query.to_lowercase();
        let mut items = Vec::new();

        let runtime = boxxy_ai_core::utils::runtime();
        let agents = runtime.block_on(async {
            boxxy_claw::registry::workspace::global_workspace()
                .await
                .get_all_agents()
                .await
        });

        for agent in agents {
            if agent.name.to_lowercase().contains(&query_lower) {
                items.push(CompletionItem {
                    display_name: agent.name.clone(),
                    replacement_text: format!("@{}", agent.name),
                    icon_name: Some("boxxy-boxxyclaw-symbolic".to_string()),
                    secondary_text: Some(agent.status),
                    badge_text: None,
                    badge_color: None,
                });
            }
        }

        items
    }
}

pub struct CommandCompletionProvider;

impl CompletionProvider for CommandCompletionProvider {
    fn trigger(&self) -> String {
        "/".to_string()
    }

    fn get_completions(&self, query: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        let query_lower = query.to_lowercase();

        if "resume".contains(&query_lower) {
            items.push(CompletionItem {
                display_name: "Resume past session".to_string(),
                replacement_text: "/resume".to_string(),
                icon_name: None,
                secondary_text: None,
                badge_text: None,
                badge_color: None,
            });
        }

        items
    }
}

fn generate_color(name: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    let hash = hasher.finish();

    let r = (hash & 0xFF) as u8 % 150 + 50;
    let g = ((hash >> 8) & 0xFF) as u8 % 150 + 50;
    let b = ((hash >> 16) & 0xFF) as u8 % 150 + 50;

    format!("rgb({}, {}, {})", r, g, b)
}

fn get_relative_age(updated_at: &str) -> String {
    use chrono::{NaiveDateTime, Utc};
    if let Ok(ndt) = NaiveDateTime::parse_from_str(updated_at, "%Y-%m-%d %H:%M:%S") {
        let now = Utc::now().naive_utc();
        let duration = now.signed_duration_since(ndt);

        if duration.num_days() > 0 {
            format!("{}d", duration.num_days())
        } else if duration.num_hours() > 0 {
            format!("{}h", duration.num_hours())
        } else if duration.num_minutes() > 0 {
            format!("{}m", duration.num_minutes())
        } else {
            "now".to_string()
        }
    } else {
        "unknown".to_string()
    }
}

pub struct ResumeCompletionProvider;

impl CompletionProvider for ResumeCompletionProvider {
    fn trigger(&self) -> String {
        "/resume ".to_string()
    }

    fn get_completions(&self, query: &str) -> Vec<CompletionItem> {
        // Query starts after "/resume". If user types "/resume ", query is " ".
        // We trim it to handle both cases.
        let query_lower = query.trim().to_lowercase();
        let mut items = Vec::new();

        let runtime = boxxy_ai_core::utils::runtime();
        let sessions = runtime.block_on(async {
            if let Ok(db) = boxxy_db::Db::new().await {
                let store = boxxy_db::store::Store::new(db.pool());
                let res = store
                    .get_recent_active_sessions(10)
                    .await
                    .unwrap_or_default();
                log::debug!("ResumeCompletionProvider fetched {} sessions from DB.", res.len());
                for s in &res {
                    log::debug!("  - Session: {} (Title: {:?})", s.id, s.title);
                }
                res
            } else {
                Vec::new()
            }
        });

        for session in sessions {
            let mut title = session
                .title
                .unwrap_or_else(|| "Untitled Session".to_string());
            if title.trim().is_empty() {
                title = "Untitled Session".to_string();
            }
            let agent_name = session.agent_name.unwrap_or_else(|| "Unknown".to_string());
            let msg_count = session.message_count;

            // Format age (very basic implementation)
            let age = if let Some(updated_at) = session.updated_at {
                get_relative_age(&updated_at)
            } else {
                "unknown".to_string()
            };

            if query_lower.is_empty()
                || title.to_lowercase().contains(&query_lower)
                || agent_name.to_lowercase().contains(&query_lower)
            {
                let icon_name = if session.pinned {
                    "boxxy-view-pin-symbolic".to_string()
                } else {
                    "boxxy-chat-symbolic".to_string()
                };

                let badge_color = generate_color(&agent_name);

                items.push(CompletionItem {
                    display_name: format!("{title} [{msg_count} msgs]"),
                    replacement_text: format!("/resume {}", session.id),
                    icon_name: Some(icon_name),
                    secondary_text: Some(age),
                    badge_text: Some(agent_name),
                    badge_color: Some(badge_color),
                });
            }
        }

        items
    }
}
