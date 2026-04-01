use crate::engine::ClawEngineEvent;
use crate::engine::session::SessionState;
use crate::engine::tools::skills::ActivateSkillTool;
use crate::engine::tools::tasks::{CancelTaskTool, ListTasksTool, ScheduleTaskTool};
use crate::engine::tools::terminal::TerminalCommandTool;
use crate::engine::tools::workspace::{
    CloseAgentTool, DelegateTaskTool, ListActiveAgentsTool, ReadPaneTool, SendKeystrokesTool,
    SetGlobalIntentTool, SpawnAgentTool,
};
use crate::engine::tools::{ClawApprovalHandler, SysShellTool};
use boxxy_agent::ipc::AgentClawProxy;
use boxxy_core_toolbox::{
    FileDeleteTool, FileReadTool, FileWriteTool, GetClipboardTool, GetSystemInfoTool,
    HttpFetchTool, KillProcessTool, ListDirectoryTool, ListProcessesTool, SetClipboardTool,
};
use boxxy_model_selection::ModelProvider;
use rig::agent::Agent;
use rig::client::CompletionClient;
use rig::message::Message;
use rig::providers::gemini;
use rig::providers::ollama;
use rig::providers::openai::responses_api::ResponsesCompletionModel;
use serde_json::json;

use boxxy_ai_core::AiCredentials;

pub enum ClawAgent {
    Gemini(Agent<gemini::CompletionModel>),
    Ollama(Agent<ollama::CompletionModel>),
    Anthropic(Agent<rig::providers::anthropic::completion::CompletionModel>),
    OpenAi(Agent<ResponsesCompletionModel>),
    Error(String),
}

impl ClawAgent {
    pub async fn chat(
        &self,
        prompt: &str,
        history: Vec<Message>,
    ) -> Result<(String, Option<rig::completion::Usage>), rig::completion::PromptError> {
        use rig::completion::Prompt;
        let start = std::time::Instant::now();

        let res_result = match self {
            Self::Gemini(agent) => agent.prompt(prompt).with_history(history.clone()).extended_details().await,
            Self::Ollama(agent) => agent.prompt(prompt).with_history(history.clone()).extended_details().await,
            Self::Anthropic(agent) => agent.prompt(prompt).with_history(history.clone()).extended_details().await,
            Self::OpenAi(agent) => agent.prompt(prompt).with_history(history.clone()).extended_details().await,
            Self::Error(e) => return Err(rig::completion::PromptError::CompletionError(
                rig::completion::CompletionError::ProviderError(e.clone()),
            )),
        };

        match res_result {
            Ok(res) => {
                let duration = start.elapsed();
                let model_name = match self {
                    Self::Gemini(_) => "gemini",
                    Self::Ollama(_) => "ollama",
                    Self::Anthropic(_) => "anthropic",
                    Self::OpenAi(_) => "openai",
                    _ => "unknown",
                };

                // Track Invocations
                boxxy_telemetry::track_ai_invocation(model_name, model_name).await;
                
                // Track Latency
                boxxy_telemetry::track_ai_latency(model_name, model_name, duration.as_millis() as u64).await;

                // Track Tokens
                boxxy_telemetry::track_ai_tokens(model_name, "input", res.usage.input_tokens as u64).await;
                boxxy_telemetry::track_ai_tokens(model_name, "output", res.usage.output_tokens as u64).await;

                Ok((res.output.clone(), Some(res.usage)))
            }
            Err(e) => Err(e),
        }
    }
}

#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn create_claw_agent(
    provider: &Option<ModelProvider>,
    creds: &AiCredentials,
    system_prompt: &str,
    claw_proxy: &AgentClawProxy<'static>,
    current_dir: &str,
    tx_ui: async_channel::Sender<ClawEngineEvent>,
    state: std::sync::Arc<tokio::sync::Mutex<SessionState>>,
    db: std::sync::Arc<tokio::sync::Mutex<Option<boxxy_db::Db>>>,
    settings: &boxxy_preferences::Settings,
    session_id: String,
    pane_id: String,
) -> ClawAgent {
    let provider = match provider {
        Some(p) => p,
        None => {
            return ClawAgent::Error(
                "No Claw model selected. Please configure your models in Settings -> APIs -> Models Selection."
                    .to_string(),
            )
        }
    };

    let approval_handler = std::sync::Arc::new(ClawApprovalHandler {
        tx_ui: tx_ui.clone(),
        state: state.clone(),
        db: db.clone(),
        session_id: session_id.clone(),
        pane_id: pane_id.clone(),
    });

    let mut tools: Vec<Box<dyn rig::tool::ToolDyn>> = vec![
        Box::new(SysShellTool {
            proxy: claw_proxy.clone(),
            current_dir: current_dir.to_string(),
        }),
        Box::new(crate::memories::MemoryStoreTool {
            db: db.clone(),
            current_dir: current_dir.to_string(),
        }),
        Box::new(crate::engine::tools::scrollback::ReadScrollbackTool {
            tx_ui: tx_ui.clone(),
            state: state.clone(),
        }),
        Box::new(ActivateSkillTool),
        Box::new(TerminalCommandTool {
            tx_ui: tx_ui.clone(),
            state: state.clone(),
            db: db.clone(),
            session_id: session_id.clone(),
            pane_id: pane_id.clone(),
        }),
        Box::new(ListActiveAgentsTool),
        Box::new(ReadPaneTool),
        Box::new(DelegateTaskTool {
            state: state.clone(),
        }),
        Box::new(SpawnAgentTool {
            tx_ui: tx_ui.clone(),
            state: state.clone(),
        }),
        Box::new(CloseAgentTool {
            tx_ui: tx_ui.clone(),
        }),
        Box::new(SendKeystrokesTool {
            tx_ui: tx_ui.clone(),
        }),
        Box::new(ListProcessesTool {
            proxy: claw_proxy.clone(),
            approval: approval_handler.clone(),
        }),
        Box::new(ScheduleTaskTool {
            state: state.clone(),
            tx_ui: tx_ui.clone(),
        }),
        Box::new(ListTasksTool {
            state: state.clone(),
        }),
        Box::new(CancelTaskTool {
            state: state.clone(),
            tx_ui: tx_ui.clone(),
        }),
        Box::new(SetGlobalIntentTool),
    ];

    // Conditional Core Toolbox tools
    if settings.enable_file_tools {
        tools.push(Box::new(FileReadTool {
            proxy: claw_proxy.clone(),
            current_dir: current_dir.to_string(),
        }));
        tools.push(Box::new(FileWriteTool {
            proxy: claw_proxy.clone(),
            current_dir: current_dir.to_string(),
            approval: approval_handler.clone(),
        }));
        tools.push(Box::new(ListDirectoryTool {
            proxy: claw_proxy.clone(),
            current_dir: current_dir.to_string(),
        }));
        tools.push(Box::new(FileDeleteTool {
            proxy: claw_proxy.clone(),
            current_dir: current_dir.to_string(),
            approval: approval_handler.clone(),
        }));
    }

    if settings.enable_system_tools {
        tools.push(Box::new(GetSystemInfoTool {
            proxy: claw_proxy.clone(),
            approval: approval_handler.clone(),
        }));
    }

    if settings.enable_dangerous_tools {
        tools.push(Box::new(KillProcessTool {
            proxy: claw_proxy.clone(),
            approval: approval_handler.clone(),
        }));
    }

    if settings.enable_web_tools {
        tools.push(Box::new(HttpFetchTool));
    }

    if settings.enable_clipboard_tools {
        tools.push(Box::new(GetClipboardTool {
            proxy: claw_proxy.clone(),
            approval: approval_handler.clone(),
        }));
        tools.push(Box::new(SetClipboardTool {
            proxy: claw_proxy.clone(),
            approval: approval_handler.clone(),
        }));
    }

    match provider {
        ModelProvider::Gemini(model, _thinking) => {
            let key = creds.api_keys.get("Gemini").cloned().unwrap_or_default();
            let client = gemini::Client::new(key.trim()).unwrap();
            let gemini_model = client.completion_model(model.api_name());

            let builder = rig::agent::AgentBuilder::new(gemini_model)
                .preamble(system_prompt)
                .default_max_turns(100)
                .tools(tools);

            ClawAgent::Gemini(builder.build())
        }
        ModelProvider::Ollama(model_name) => {
            let client: ollama::Client = ollama::Client::builder()
                .api_key(rig::client::Nothing)
                .base_url(creds.ollama_url.as_str())
                .build()
                .unwrap();
            let ollama_model = client.completion_model(model_name.as_str());

            let builder = rig::agent::AgentBuilder::new(ollama_model)
                .preamble(system_prompt)
                .default_max_turns(100)
                .tools(tools);

            ClawAgent::Ollama(builder.build())
        }
        ModelProvider::Anthropic(model) => {
            let key = creds.api_keys.get("Anthropic").cloned().unwrap_or_default();
            let client = rig::providers::anthropic::Client::new(key.trim()).unwrap();
            let anthropic_model = client.completion_model(model.api_name());

            let builder = rig::agent::AgentBuilder::new(anthropic_model)
                .preamble(system_prompt)
                .default_max_turns(100)
                .tools(tools);

            ClawAgent::Anthropic(builder.build())
        }
        ModelProvider::OpenAi(model, thinking) => {
            let key = creds.api_keys.get("OpenAI").cloned().unwrap_or_default();
            let client = rig::providers::openai::Client::new(key.trim()).unwrap();
            let openai_model = client.completion_model(model.api_name());

            let mut builder = rig::agent::AgentBuilder::new(openai_model)
                .preamble(system_prompt)
                .default_max_turns(100)
                .tools(tools);

            if let Some(level) = thinking {
                builder = builder.additional_params(json!({
                    "reasoning": { "effort": level.api_name() }
                }));
            }

            ClawAgent::OpenAi(builder.build())
        }
    }
}
