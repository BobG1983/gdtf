use serde_json::{Value, json};

use crate::{
    hosts::QaHost,
    mcp::{
        courier::commands::CatalogueDetail,
        tools::name::{ALL, ToolName},
    },
};

fn host_property() -> Value {
    json!({
        "type": "string",
        "enum": [QaHost::Game.label(), QaHost::Editor.label()],
        "description": "Which child to act on; omit for the game.",
    })
}

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

fn stop_schema() -> Value {
    json!({
        "type": "object",
        "properties": { "host": host_property() }
    })
}

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
        fn input_schema(self) -> Value {
        match self {
            Self::Launch => launch_schema(),
            Self::Stop => stop_schema(),
            Self::Logs => logs_schema(),
            Self::Commands => commands_schema(),
            Self::Run => run_schema(),
        }
    }

        fn descriptor(self) -> Value {
        json!({
            "name": self.wire_name(),
            "description": self.description(),
            "inputSchema": self.input_schema(),
        })
    }
}

#[must_use]
pub fn tools_list_result() -> Value {
    let tools: Vec<Value> = ALL.iter().map(|tool| tool.descriptor()).collect();
    json!({ "tools": tools })
}
