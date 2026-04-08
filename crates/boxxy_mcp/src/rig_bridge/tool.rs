use super::schema::translate_schema;
use rig::completion::ToolDefinition;
use rig::tool::ToolDyn;
use rmcp::model::{CallToolRequestParams, Tool as McpToolDefinition};
use rmcp::{RoleClient, service::RunningService};
use std::sync::Arc;

pub struct DynamicMcpTool {
    pub client: Arc<RunningService<RoleClient, ()>>,
    pub mcp_tool: McpToolDefinition,
    pub server_name: String,
}

impl ToolDyn for DynamicMcpTool {
    fn name(&self) -> String {
        // Strictly normalize the name to [a-zA-Z0-9_-]+ for LLM compatibility
        let mut name = format!("{}__{}", self.server_name, self.mcp_tool.name)
            .replace(' ', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect::<String>();

        // Ensure name doesn't start with a number (invalid for many LLM function schemas)
        if name.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            name = format!("_{}", name);
        }
        name
    }

    fn definition<'a>(
        &'a self,
        _prompt: String,
    ) -> rig::wasm_compat::WasmBoxedFuture<'a, ToolDefinition> {
        let name = self.name();
        let schema_map = (*self.mcp_tool.input_schema).clone();
        let schema = serde_json::Value::Object(schema_map);

        let def = ToolDefinition {
            name,
            description: self
                .mcp_tool
                .description
                .clone()
                .unwrap_or_default()
                .to_string(),
            parameters: translate_schema(schema),
        };
        Box::pin(async move { def })
    }

    fn call<'a>(
        &'a self,
        args: String,
    ) -> rig::wasm_compat::WasmBoxedFuture<'a, Result<String, rig::tool::ToolError>> {
        Box::pin(async move {
            let parsed_args: serde_json::Value = match serde_json::from_str(&args) {
                Ok(v) => v,
                Err(e) => return Err(rig::tool::ToolError::JsonError(e)),
            };

            let mut params = CallToolRequestParams::new(self.mcp_tool.name.clone());
            if let Some(obj) = parsed_args.as_object() {
                params.arguments = Some(obj.clone());
            }

            match self.client.call_tool(params).await {
                Ok(result) => {
                    if result.is_error.unwrap_or(false) {
                        let err_msg = format!("{:?}", result.content);
                        Err(rig::tool::ToolError::ToolCallError(Box::new(
                            std::io::Error::new(std::io::ErrorKind::Other, err_msg),
                        )))
                    } else {
                        let val = serde_json::to_value(&result.content)
                            .unwrap_or(serde_json::Value::Null);
                        Ok(val.to_string())
                    }
                }
                Err(e) => Err(rig::tool::ToolError::ToolCallError(Box::new(e))),
            }
        })
    }
}
