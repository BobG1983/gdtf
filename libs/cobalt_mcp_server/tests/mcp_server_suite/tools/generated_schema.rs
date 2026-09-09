//! Every tool's `host` enum and prose comes from the registered hosts, not a literal.

use cobalt_mcp_server::tools_list_result;
use serde_json::Value;

use crate::tools::support::two_registered_hosts;

const TOOLS: [&str; 5] = ["launch", "stop", "logs", "commands", "run"];

// Names this crate must never write into a schema on its own.
const NEVER_LITERAL: [&str; 4] = ["game", "editor", "7616", "7617"];

fn listed(result: &Value) -> Vec<Value> {
    result["tools"].as_array().cloned().unwrap_or_default()
}

fn host_enum(tool: &Value) -> Vec<String> {
    tool["inputSchema"]["properties"]["host"]["enum"]
        .as_array()
        .map(|names| {
            names
                .iter()
                .filter_map(|name| name.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn every_tools_host_enum_is_exactly_the_registered_names() {
    let result = tools_list_result(&two_registered_hosts());

    let tools = listed(&result);
    assert_eq!(tools.len(), TOOLS.len(), "got: {result}");
    for tool in &tools {
        let Some(name) = tool["name"].as_str() else {
            unreachable!("every tool descriptor carries a name: {tool}");
        };
        assert!(TOOLS.contains(&name), "unexpected tool {name}");
        assert_eq!(
            host_enum(tool),
            vec!["thistle".to_owned(), "bramble".to_owned()],
            "{name}'s host enum is written from the registry, in registration order"
        );
    }
}

#[test]
fn no_tool_text_names_a_host_this_registry_never_registered() {
    let result = tools_list_result(&two_registered_hosts());

    for tool in listed(&result) {
        let rendered = tool.to_string();
        for literal in NEVER_LITERAL {
            assert!(
                !rendered.contains(literal),
                "tool {} carries the hardcoded {literal:?}: {rendered}",
                tool["name"]
            );
        }
    }
}

#[test]
fn the_registered_names_are_what_the_descriptions_offer() {
    let result = tools_list_result(&two_registered_hosts());

    for tool in listed(&result) {
        let Some(description) = tool["description"].as_str() else {
            unreachable!("every tool descriptor carries a description: {tool}");
        };
        assert!(
            description.contains("thistle") && description.contains("bramble"),
            "{} offers both registered hosts by name: {description}",
            tool["name"]
        );
    }
}

#[test]
fn a_single_registered_host_leaves_one_legal_value_the_call_may_omit() {
    let result = tools_list_result(&cobalt_mcp_server::HostRegistry::new(vec![
        crate::tools::support::registered("thistle", 4100, cobalt_mcp_server::LaunchPolicy::Reuse),
    ]));

    for tool in listed(&result) {
        assert_eq!(
            host_enum(&tool),
            vec!["thistle".to_owned()],
            "{} offers the one registered host and nothing else",
            tool["name"]
        );
        let Some(host_text) = tool["inputSchema"]["properties"]["host"]["description"].as_str()
        else {
            unreachable!("the host property carries a description: {tool}");
        };
        assert!(
            host_text.contains("omitted"),
            "one-host mode says the argument may be left out: {host_text}"
        );
    }
}
