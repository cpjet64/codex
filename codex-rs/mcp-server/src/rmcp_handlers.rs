//! rmcp-based server handler stubs (feature: `rmcp_sdk`).
//!
//! Define tool handlers that mirror legacy `codex` and `codex-reply` tools.
//! These are not yet hooked into `run_main`.

#![allow(dead_code)]

use serde_json::Value;

pub struct CodexTools;

impl CodexTools {
    pub async fn list_tools() -> serde_json::Value {
        // Placeholder: return a minimal schema-like value
        serde_json::json!({ "tools": [ { "name": "codex" }, { "name": "codex-reply" } ] })
    }

    pub async fn call_codex(_args: Option<Value>) -> serde_json::Value {
        serde_json::json!({ "content": [{"type":"text","text":"not yet implemented (rmcp path)"}], "isError": true })
    }
}

