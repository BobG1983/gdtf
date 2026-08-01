//! The per-tool JSON-Schema for its `arguments` object, and the `tools/list` descriptor
//! assembly.

use serde_json::{Value, json};

use crate::{
    hosts::QaHost,
    mcp::{
        courier::commands::CatalogueDetail,
        tools::name::{ALL, ToolName},
    },
};

/// The `host` property every tool carries — which child the call acts on.
///
/// One builder rather than five copies: the enumeration comes from [`QaHost::label`], so a
/// schema can never advertise a word [`QaHost::from_label`] rejects.
fn host_property() -> Value {
    json!({
        "type": "string",
        "enum": [QaHost::Game.label(), QaHost::Editor.label()],
        "description": "Which child to act on; omit for the game.",
    })
}

/// The `launch` argument schema — the host plus the five optional recipe arguments.
fn launch_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "host": host_property(),
            "port": { "type": "integer", "minimum": 0, "maximum": 65535,
                      "description": "Loopback port for the child's net_qa listener; \
                       omit for that host's default (game 7616, editor 7617)." },
            "package": { "type": "string",
                         "description": "Cargo package to build and run; omit for that \
                          host's own (grimdark_turfwar, gdtf_content_editor_bin)." },
            "features": { "type": ["array", "string"],
                          "items": { "type": "string" },
                          "description": "Cargo features to enable, as an array or a \
                           comma-separated string; omit for dynamic_linking,net_qa. Add \
                           dev_tools to QA a dev build of the game." },
            "working_dir": { "type": "string",
                             "description": "Directory to run the build in — the \
                              checkout under test. Omit to use the MCP host's own; \
                              pass a git worktree path to QA that tree." },
            "env": { "type": "object",
                     "additionalProperties": { "type": "string" },
                     "description": "Extra environment variables for the child, \
                      e.g. {\"GDTF_BATTLE_SEED\": \"42\"}." }
        }
    })
}

/// The `stop` argument schema — the host, and nothing else.
fn stop_schema() -> Value {
    json!({
        "type": "object",
        "properties": { "host": host_property() }
    })
}

/// The `logs` argument schema — the host and an optional line cap.
fn logs_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "host": host_property(),
            "max_lines": { "type": "integer", "minimum": 0,
                           "description": "How many trailing lines to return; omit for \
                            the default tail." }
        }
    })
}

/// The `commands` argument schema — the host, an optional one-command filter, and the detail
/// level, enumerated from [`CatalogueDetail`] so the schema can never advertise a word the
/// parser rejects.
///
/// The `command` property is a FREE STRING with no `enum`: which commands exist is read from
/// the running host, and enumerating them here would be a second copy of that truth, stale
/// the moment either host gains one. `test/courier.rs` pins the absence.
fn commands_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "host": host_property(),
            "command": { "type": "string",
                         "description": "Narrow the reply to one command by name; omit \
                          for the whole catalogue. Names come from this tool." },
            "detail": { "type": "string",
                        "enum": CatalogueDetail::labels(),
                        "description": "How much of each row to return; omit for \
                         Summary. Full also carries each command's derived argument \
                         and reply schemas." }
        }
    })
}

/// The `run` argument schema — the command name, its opaque argument object, the host, and
/// the two per-call riders.
///
/// `command` carries no `enum` and `arguments` no `properties`, for the same reason: both are
/// the HOST's to define, per command, and only the host that owns the command can say what it
/// accepts. A shape written here could only ever drift from the one the host derives.
fn run_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "command": { "type": "string",
                         "description": "The command to run, as `commands` named it." },
            "arguments": { "type": "object",
                           "description": "The command's arguments, shaped by its own \
                            `schemas.arguments` from `commands`; omit for a command \
                            that takes none." },
            "host": host_property(),
            "await_ready": { "type": "integer", "minimum": 0,
                             "description": "Whole seconds to keep re-testing admission \
                              before giving up; omit to decide once." },
            "capture": { "type": ["boolean", "string"],
                         "description": "Capture the screen once the command has run: \
                          true for a host-chosen file stem, or a string to pick one." }
        },
        "required": ["command"]
    })
}

impl ToolName {
    /// The JSON-Schema for this tool's `arguments` object.
    fn input_schema(self) -> Value {
        match self {
            Self::Launch => launch_schema(),
            Self::Stop => stop_schema(),
            Self::Logs => logs_schema(),
            Self::Commands => commands_schema(),
            Self::Run => run_schema(),
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
