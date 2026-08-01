//! The courier's tool set: exactly five, each taking `host`, and none naming a command
//! (GTW-943).
//!
//! It replaces the three suites the sixteen-tool registry needed (`advertised`,
//! `module_doc`, `routing`), because there is nothing left to partition: every tool takes
//! the same `host` argument, so which child a call reaches is a CALL fact rather than a
//! per-tool one, and the "which host owns this tool" map those suites checked no longer
//! exists.

use serde_json::Value;

use crate::mcp::{
    courier::commands::CatalogueDetail,
    tools::{ToolName, name::ALL, tools_list_result},
};

/// Every tool the registry advertises, by wire name.
fn advertised_names() -> Vec<String> {
    let listed = tools_list_result();
    let Some(tools) = listed.get("tools").and_then(Value::as_array) else {
        unreachable!("tools/list returns a `tools` array, got {listed}");
    };
    tools
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

/// One tool's advertised descriptor.
fn descriptor(name: &str) -> Value {
    let listed = tools_list_result();
    let Some(tools) = listed.get("tools").and_then(Value::as_array) else {
        unreachable!("tools/list returns a `tools` array, got {listed}");
    };
    let Some(found) = tools
        .iter()
        .find(|tool| tool.get("name").and_then(Value::as_str) == Some(name))
    else {
        unreachable!("the registry advertises `{name}`, got {listed}");
    };
    found.clone()
}

/// The tool set is EXACTLY these five, in this order.
///
/// The count and the set both, because either alone lets a mistake through: a set check
/// alone would pass with a sixth tool added, and a count alone would pass with one swapped
/// for another.
#[test]
fn the_courier_advertises_exactly_five_tools() {
    assert_eq!(
        advertised_names(),
        vec![
            "launch".to_owned(),
            "stop".to_owned(),
            "logs".to_owned(),
            "commands".to_owned(),
            "run".to_owned(),
        ],
        "the courier's tool set is launch / stop / logs / commands / run, in that order",
    );
    assert_eq!(
        ALL.len(),
        5,
        "the enumeration and the advertised list agree"
    );
}

/// EVERY tool takes a `host` argument, enumerated to the two children.
///
/// This is what replaced the per-tool host map: a call says which child it means, so no
/// tool is fixed to one of them and no tool is duplicated per host.
#[test]
fn every_tool_takes_a_host_argument() {
    for tool in ALL {
        let name = tool.wire_name();
        let descriptor = descriptor(name);
        let Some(host) = descriptor.pointer("/inputSchema/properties/host") else {
            unreachable!("`{name}` must take a `host` argument, got {descriptor}");
        };
        let Some(choices) = host.get("enum").and_then(Value::as_array) else {
            unreachable!("`{name}`'s `host` must enumerate the two children, got {host}");
        };
        assert_eq!(
            choices,
            &vec![Value::from("game"), Value::from("editor")],
            "`{name}`'s `host` enumerates exactly the two children",
        );
    }
}

/// Every advertised name resolves back through [`ToolName::from_wire`], and nothing else
/// does.
#[test]
fn every_advertised_name_resolves_and_no_other_does() {
    for name in advertised_names() {
        assert!(
            ToolName::from_wire(&name).is_some(),
            "`{name}` is advertised, so a call naming it must resolve",
        );
    }
    for stale in [
        "launch_game",
        "stop_editor",
        "send_input",
        "query_state",
        "take_screenshot",
    ] {
        assert!(
            ToolName::from_wire(stale).is_none(),
            "`{stale}` was removed, so a call naming it must be an unknown tool",
        );
    }
}

/// The registry's module doc names all five tools.
///
/// A doc nothing checks goes stale silently — this one claimed twelve forwarding tools and
/// one host for months. Reading the doc text at compile time is what keeps it honest.
#[test]
fn the_module_doc_names_every_tool() {
    let doc = include_str!("../mod.rs");
    for tool in ALL {
        let name = tool.wire_name();
        assert!(
            doc.contains(name),
            "the tools module doc must name `{name}`",
        );
    }
}

/// Every argument a handler READS is advertised, and the two enumerated ones are drawn from
/// the parser's own vocabulary.
///
/// A client only ever sends what `tools/list` advertises, so an argument the host parses
/// perfectly and never advertises is unreachable — the GTW-875 gap, where the host read five
/// launch arguments no client could send. `max_lines` is the same shape one ticket later:
/// `logs` parses and applies it, and deleting it from the schema would leave the suite green
/// while making the cap unusable from any client.
#[test]
fn every_tool_advertises_the_arguments_its_handler_reads() {
    let properties = |name: &str| descriptor(name)["inputSchema"]["properties"].clone();

    let launch = properties("launch");
    for argument in ["port", "package", "features", "working_dir", "env"] {
        assert!(
            launch[argument].is_object(),
            "`launch` advertises `{argument}`: {launch}",
        );
    }

    let logs = properties("logs");
    assert!(
        logs["max_lines"].is_object(),
        "`logs` advertises `max_lines`: {logs}",
    );

    let commands = properties("commands");
    assert_eq!(
        commands["detail"]["enum"],
        serde_json::json!(CatalogueDetail::labels()),
        "`commands` enumerates its detail levels from the parser's own labels: {commands}",
    );
    assert!(
        commands["command"]["enum"].is_null(),
        "`commands`' filter stays a free string — which commands exist is the HOST's to \
         publish: {commands}",
    );

    let run = properties("run");
    for argument in ["command", "arguments", "host", "await_ready", "capture"] {
        assert!(
            run[argument].is_object(),
            "`run` advertises `{argument}`: {run}",
        );
    }
    assert_eq!(
        descriptor("run")["inputSchema"]["required"],
        serde_json::json!(["command"]),
    );
    assert!(
        run["arguments"]["properties"].is_null(),
        "`run`'s `arguments` carries the COMMAND's shape, not one written into the tool: \
         {run}",
    );
}

/// No tool's schema or description names a COMMAND.
///
/// The property the whole command layer exists for: a host's vocabulary is data it publishes
/// at runtime, so adding `app.phase`'s successors must never require an edit here — and a
/// name written into a schema or a description would be exactly that edit, silently stale
/// the moment the host moved on.
#[test]
fn no_tool_names_a_command() {
    let listed = tools_list_result().to_string();
    // Dotted names only: a bare word like `wait` is also ordinary English, and matching one
    // would fail on a description that merely uses it.
    for command_ish in [
        "app.phase",
        "battle.roster",
        "capture.screenshot",
        "lifecycle.hold",
        "situations.list",
    ] {
        assert!(
            !listed.contains(command_ish),
            "no tool schema or description may name the command `{command_ish}`: {listed}",
        );
    }
}
