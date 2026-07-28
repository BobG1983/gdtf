//! The per-tool JSON-Schema for its `arguments` object, and the `tools/list` descriptor
//! assembly.

use serde_json::{Value, json};

use crate::mcp::{
    editor_topic::topic_wire_names,
    tools::name::{ALL, ToolName},
};

/// The `launch_game` argument schema — its five optional arguments (GTW-875).
///
/// Pulled out of the [`ToolName::input_schema`] `match` so no single arm pushes that
/// function past the clippy `too_many_lines` ceiling.
fn launch_game_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "port": { "type": "integer", "minimum": 0, "maximum": 65535,
                      "description": "Loopback port for the game's net_qa \
                       listener; omit for the default." },
            "package": { "type": "string",
                         "description": "Cargo package to build and run; omit \
                          for grimdark_turfwar." },
            "features": { "type": ["array", "string"],
                          "items": { "type": "string" },
                          "description": "Cargo features to enable, as an array \
                           or a comma-separated string; omit for \
                           dynamic_linking,net_qa. Add dev_tools to QA a dev \
                           build." },
            "working_dir": { "type": "string",
                             "description": "Directory to run the build in — the \
                              checkout under test. Omit to use the MCP host's \
                              own; pass a git worktree path to QA that tree." },
            "env": { "type": "object",
                     "additionalProperties": { "type": "string" },
                     "description": "Extra environment variables for the child, \
                      e.g. {\"GDTF_BATTLE_SEED\": \"42\"}." }
        }
    })
}

/// The `launch_editor` argument schema — the same five optional arguments `launch_game`
/// takes, with the editor's own defaults spelled out (GTW-808).
///
/// Pulled out of the [`ToolName::input_schema`] `match` for the same reason
/// [`launch_game_schema`] is.
fn launch_editor_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "port": { "type": "integer", "minimum": 0, "maximum": 65535,
                      "description": "Loopback port for the editor's net_qa \
                       listener; omit for 7617." },
            "package": { "type": "string",
                         "description": "Cargo package to build and run; omit \
                          for gdtf_content_editor_bin." },
            "features": { "type": ["array", "string"],
                          "items": { "type": "string" },
                          "description": "Cargo features to enable, as an array \
                           or a comma-separated string; omit for \
                           dynamic_linking,net_qa." },
            "working_dir": { "type": "string",
                             "description": "Directory to run the build in — the \
                              checkout under test. Omit to use the MCP host's \
                              own; pass a git worktree path to QA that tree." },
            "env": { "type": "object",
                     "additionalProperties": { "type": "string" },
                     "description": "Extra environment variables for the child." }
        }
    })
}

/// The `query_editor` argument schema — its one required `topic`, enumerated from the
/// protocol's own topic list so the schema can never advertise a topic the parser rejects
/// (GTW-808).
fn query_editor_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "topic": { "type": "string",
                       "enum": topic_wire_names(),
                       "description": "The editor topic to ask about. Read the \
                        topics that are live right now from \
                        get_editor_query_options." }
        },
        "required": ["topic"]
    })
}

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
            Self::QueryState
            | Self::AppFlow
            | Self::StopGame
            | Self::GetEditorQueryOptions
            | Self::StopEditor => json!({
                "type": "object",
                "properties": {}
            }),
            Self::LaunchGame => launch_game_schema(),
            Self::LaunchEditor => launch_editor_schema(),
            Self::QueryEditor => query_editor_schema(),
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
