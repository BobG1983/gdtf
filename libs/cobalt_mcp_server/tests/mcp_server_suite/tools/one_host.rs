//! With one host registered, a call that omits `host` lands on it.

use cobalt_mcp_server::{dispatch, tools_list_result};
use serde_json::Value;

use crate::tools::support::{
    CANNED_COMMAND, CannedLifecycle, CannedLink, one_host_set, registered, test_identity,
};

const SOLO: &str = "thistle";

fn call(tool: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    )
}

fn answer(request: &str, lifecycle: &mut CannedLifecycle) -> Value {
    let mut link = CannedLink;
    let mut hosts = one_host_set(SOLO, &mut link, lifecycle);
    let Some(response) = dispatch(request, &test_identity(), &mut hosts) else {
        unreachable!("a request with an id yields a response line");
    };
    serde_json::from_str(&response).unwrap_or(Value::Null)
}

#[test]
fn one_registered_host_is_the_only_value_every_tool_offers() {
    let registry = cobalt_mcp_server::HostRegistry::new(vec![registered(
        SOLO,
        4100,
        cobalt_mcp_server::LaunchPolicy::Reuse,
    )]);

    let result = tools_list_result(&registry);

    let tools = result["tools"].as_array().cloned().unwrap_or_default();
    assert_eq!(tools.len(), 5, "got: {result}");
    for tool in tools {
        assert_eq!(
            tool["inputSchema"]["properties"]["host"]["enum"],
            serde_json::json!([SOLO]),
            "{} offers the one registered host: {tool}",
            tool["name"]
        );
    }
}

#[test]
fn every_tool_called_without_a_host_key_resolves_to_the_one_registered_host() {
    let calls = [
        call("launch", "{}"),
        call("stop", "{}"),
        call("logs", "{}"),
        call("commands", "{}"),
        call("run", &format!(r#"{{"command":"{CANNED_COMMAND}"}}"#)),
    ];

    for line in calls {
        let mut lifecycle = CannedLifecycle::new();
        let response = answer(&line, &mut lifecycle);
        assert_eq!(
            response["error"],
            Value::Null,
            "a call that omits `host` is served by the only registered host: {response}"
        );
        assert_eq!(
            response["result"]["isError"],
            serde_json::json!(false),
            "the tool answered rather than refusing: {response}"
        );
    }
}

#[test]
fn an_omitted_host_reaches_that_hosts_own_lifecycle() {
    let mut lifecycle = CannedLifecycle::new();

    let response = answer(&call("launch", "{}"), &mut lifecycle);

    assert_eq!(
        lifecycle.launches(),
        1,
        "the launch reached the registered host's lifecycle: {response}"
    );
}

#[test]
fn a_name_that_is_not_registered_is_refused_rather_than_defaulted() {
    let mut lifecycle = CannedLifecycle::new();

    let response = answer(&call("logs", r#"{"host":"nowhere"}"#), &mut lifecycle);

    assert_ne!(
        response["error"],
        Value::Null,
        "an unregistered name is refused: {response}"
    );
}
