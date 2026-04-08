use serde_json::Value;

/// Translates an MCP JSON Schema (Draft 7) into a Rig-compatible JSON schema format.
/// In most cases, this is a direct passthrough, as Rig also utilizes JSON Schema formats.
/// However, this function exists to allow explicit normalizations if Rig or the underlying LLMs
/// (OpenAI, Anthropic, Gemini) have specific strict requirements (e.g., stripping '$schema').
pub fn translate_schema(schema: Value) -> Value {
    let mut clean_schema = schema;

    if let Some(obj) = clean_schema.as_object_mut() {
        // Many LLM function-calling APIs (especially Anthropic/OpenAI) can choke on the standard
        // JSON Schema "$schema" key, so we remove it if it exists.
        obj.remove("$schema");
    }

    clean_schema
}
