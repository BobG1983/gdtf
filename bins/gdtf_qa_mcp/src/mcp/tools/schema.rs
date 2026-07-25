//! The per-tool JSON-Schema for its `arguments` object, and the `tools/list` descriptor
//! assembly.

use serde_json::{Value, json};

use crate::mcp::tools::name::{ALL, ToolName};

impl ToolName {
    /// The JSON-Schema for this tool's `arguments` object.
    fn input_schema(self) -> Value {
        match self {
            Self::SendInput => json!({
                "type": "object",
                "properties": {
                    "intent": {
                        "description": "A NetIntent, as a JSON object (e.g. \
                         {\"Reload\": null}) or a compact-RON string (e.g. \"Reload\")."
                    }
                },
                "required": ["intent"]
            }),
            Self::QueryState | Self::AppFlow | Self::StopGame => json!({
                "type": "object",
                "properties": {}
            }),
            Self::LaunchGame => json!({
                "type": "object",
                "properties": {
                    "port": { "type": "integer", "minimum": 0, "maximum": 65535,
                              "description": "Loopback port for the game's net_qa \
                               listener; omit for the default." }
                }
            }),
            Self::GetOutput => json!({
                "type": "object",
                "properties": {
                    "max": { "type": "integer", "minimum": 0,
                             "description": "Cap on events drained; omit to drain all." }
                }
            }),
            Self::TakeScreenshot => json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string",
                              "description": "Screenshot file stem; omit for a chosen name." }
                }
            }),
            Self::ScreenshotAfter => json!({
                "type": "object",
                "properties": {
                    "intent": {
                        "description": "A NetIntent, as a JSON object (e.g. \
                         {\"Reload\": null}) or a compact-RON string (e.g. \"Reload\")."
                    },
                    "frame_delay": { "type": "integer", "minimum": 0,
                                     "description": "Frames to wait, after the intent \
                                      queues, before capturing (0 = the very next frame)." },
                    "name": { "type": "string",
                              "description": "Screenshot file stem; omit for a chosen name." }
                },
                "required": ["intent", "frame_delay"]
            }),
            Self::StartBattle => json!({
                "type": "object",
                "properties": {
                    "situation": { "type": "string",
                                   "description": "The situation to start. The game \
                                    currently ships one, \"skirmish\"." },
                    "seed": { "type": "integer", "minimum": 0,
                              "description": "Deterministic procgen seed; omit for a \
                               game-chosen seed." }
                },
                "required": ["situation"]
            }),
            Self::StepperControl => json!({
                "type": "object",
                "properties": {
                    "command": {
                        "description": "A stepper command: the string \"Next\" or \"Skip\", \
                         or an object {\"Auto\": {\"running\": true}} to start / stop Auto \
                         free-run. Accepts a JSON value or a compact-RON string."
                    }
                },
                "required": ["command"]
            }),
            Self::ActivateMenuItem => json!({
                "type": "object",
                "properties": {
                    "token": { "type": "integer", "minimum": 0,
                               "description": "The menu item's numeric token, read from \
                                `app_flow`'s `menu.items[].token`." }
                },
                "required": ["token"]
            }),
            Self::FocusControl => json!({
                "type": "object",
                "properties": {
                    "command": {
                        "description": "A focus command: {\"ActivateTarget\": <token>} to \
                         click a control, the string \"Activate\" to click the focused one, \
                         {\"Step\": \"Next\"} (or \"Prev\" / \"Left\" / \"Right\") to move \
                         focus, or {\"Focus\": <token>} to point focus without clicking. \
                         Tokens come from `app_flow`'s `focus.focusables[].token`. Accepts \
                         a JSON value or a compact-RON string."
                    }
                },
                "required": ["command"]
            }),
        }
    }

    /// This tool's `tools/list` descriptor: name, description, and input schema.
    fn descriptor(self) -> Value {
        json!({
            "name": self.wire_name(),
            "description": self.description(),
            "inputSchema": self.input_schema(),
        })
    }
}

/// Build the `tools/list` result — the descriptor for every registered tool.
#[must_use]
pub fn tools_list_result() -> Value {
    let tools: Vec<Value> = ALL.iter().map(|tool| tool.descriptor()).collect();
    json!({ "tools": tools })
}
