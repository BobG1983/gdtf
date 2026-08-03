use serde_json::Value;

use crate::mcp::{
    courier::commands::CatalogueDetail,
    tools::{ToolName, name::ALL, tools_list_result},
};

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

#[test]
fn no_tool_names_a_command() {
    let listed = tools_list_result().to_string();
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
