//! The MCP tool registry — the five tools this bridge exposes (GTW-741).
//!
//! Each tool maps 1:1 onto a [`QaRequest`](gdtf_qa_protocol::envelope::QaRequest). The
//! [`ToolName`] enum is the single place the tool set is enumerated: `tools/list` walks
//! it, and [`ToolName::from_wire`] resolves a `tools/call` name. Adding the future
//! `start_battle` tool (T9) is a one-variant addition here plus one arm in the request
//! builder — not a redesign.

use serde_json::{Value, json};

/// One MCP tool the bridge exposes.
///
/// Deliberately does NOT include `start_battle`: the wire request
/// [`QaRequest::StartBattle`](gdtf_qa_protocol::envelope::QaRequest::StartBattle) exists
/// but the game-side consumer for it lands in T9, so exposing it now would hang against
/// nothing. When T9 lands, add a `StartBattle` variant here (plus its `from_wire` name,
/// `descriptor` entry, and a request-builder arm) and nothing else changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolName {
    /// Inject one battle intent — maps to `QaRequest::Inject`.
    SendInput,
    /// Read the whole battle snapshot — maps to `QaRequest::GetBattleState`.
    QueryState,
    /// Drain buffered combat events — maps to `QaRequest::GetOutput`.
    GetOutput,
    /// Capture a screenshot — maps to `QaRequest::TakeScreenshot`.
    TakeScreenshot,
    /// Read the app-lifecycle snapshot — maps to `QaRequest::GetAppFlow`.
    AppFlow,
}

/// Every tool, in listing order — the one enumeration `tools/list` and any future
/// registry walk read.
const ALL: &[ToolName] = &[
    ToolName::SendInput,
    ToolName::QueryState,
    ToolName::GetOutput,
    ToolName::TakeScreenshot,
    ToolName::AppFlow,
];

impl ToolName {
    /// The MCP wire name a client calls this tool by.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::SendInput => "send_input",
            Self::QueryState => "query_state",
            Self::GetOutput => "get_output",
            Self::TakeScreenshot => "take_screenshot",
            Self::AppFlow => "app_flow",
        }
    }

    /// Resolve a `tools/call` name to its [`ToolName`], or `None` for an unknown tool.
    #[must_use]
    pub fn from_wire(name: &str) -> Option<Self> {
        ALL.iter().copied().find(|tool| tool.wire_name() == name)
    }

    /// A one-line human description of what the tool does.
    const fn description(self) -> &'static str {
        match self {
            Self::SendInput => {
                "Inject one battle intent (a NetIntent) into the running game as the \
                 selected ganger. Argument `intent` is a NetIntent as a JSON object or a \
                 compact-RON string."
            }
            Self::QueryState => {
                "Read the whole battle snapshot (gangers, terrain, fog, selection, turn) \
                 as JSON. No arguments."
            }
            Self::GetOutput => {
                "Drain buffered combat events as JSON. Optional argument `max` caps how \
                 many events are drained."
            }
            Self::TakeScreenshot => {
                "Capture a screenshot of the running game and return it as an image. \
                 Optional argument `name` picks the file stem."
            }
            Self::AppFlow => {
                "Read the app-lifecycle snapshot (which AppState, whether a battle is \
                 running) as JSON. No arguments."
            }
        }
    }

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
            Self::QueryState | Self::AppFlow => json!({
                "type": "object",
                "properties": {}
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

#[cfg(test)]
mod tests {
    use super::{ToolName, tools_list_result};

    /// `tools/list` advertises exactly the five implemented tools, and NOT `start_battle`.
    #[test]
    fn lists_the_five_tools_without_start_battle() {
        let result = tools_list_result();
        let Some(tools) = result["tools"].as_array() else {
            unreachable!("tools/list result carries a `tools` array");
        };
        assert_eq!(tools.len(), 5);
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect();
        assert!(names.contains(&"send_input"));
        assert!(names.contains(&"query_state"));
        assert!(names.contains(&"get_output"));
        assert!(names.contains(&"take_screenshot"));
        assert!(names.contains(&"app_flow"));
        assert!(!names.contains(&"start_battle"));
    }

    /// Every wire name round-trips through `from_wire`, and an unknown name resolves to
    /// `None`.
    #[test]
    fn wire_names_round_trip() {
        for tool in [
            ToolName::SendInput,
            ToolName::QueryState,
            ToolName::GetOutput,
            ToolName::TakeScreenshot,
            ToolName::AppFlow,
        ] {
            assert_eq!(ToolName::from_wire(tool.wire_name()), Some(tool));
        }
        assert_eq!(ToolName::from_wire("start_battle"), None);
        assert_eq!(ToolName::from_wire("nope"), None);
    }
}
