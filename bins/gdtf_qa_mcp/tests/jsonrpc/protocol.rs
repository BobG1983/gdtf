use serde_json::json;

use crate::support::{dispatch_json, dispatch_line};

/// `initialize` echoes the requested protocol version and reports the server identity.
#[test]
fn initialize_returns_capabilities_and_echoes_version() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
    );
    assert_eq!(response["jsonrpc"], json!("2.0"));
    assert_eq!(response["id"], json!(1));
    assert_eq!(response["result"]["protocolVersion"], json!("2025-06-18"));
    assert!(response["result"]["capabilities"]["tools"].is_object());
    assert!(response["result"]["serverInfo"]["name"].is_string());
}

#[test]
fn tools_list_returns_every_tool() {
    let response = dispatch_json(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
    let Some(tools) = response["result"]["tools"].as_array() else {
        unreachable!("tools/list carries a tools array");
    };
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert_eq!(names.len(), 5);
    for expected in ["launch", "stop", "logs", "commands", "run"] {
        assert!(names.contains(&expected), "missing tool {expected}");
    }
}

#[test]
fn every_tool_taking_an_instance_names_it_in_its_description() {
    let response = dispatch_json(r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#);
    let Some(tools) = response["result"]["tools"].as_array() else {
        unreachable!("tools/list carries a tools array");
    };

    for tool in tools {
        let name = tool["name"].as_str().unwrap_or_default();
        let described = tool["description"].as_str().unwrap_or_default();
        if tool["inputSchema"]["properties"]["instance"].is_object() {
            assert!(
                described.contains("instance"),
                "tool {name} takes an `instance` argument and its description never says so: \
                 {described}"
            );
        }
        if name == "launch" {
            assert!(
                described.contains("instance"),
                "the launch reply names the instance it recorded, and the launch description \
                 never says so: {described}"
            );
        }
    }
}

#[test]
fn tools_call_unknown_tool_is_invalid_params() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"no_such_tool","arguments":{}}}"#,
    );
    assert_eq!(response["error"]["code"], json!(-32602));
    assert_eq!(
        response["error"]["message"],
        json!("unknown tool: no_such_tool")
    );
}

#[test]
fn unknown_method_is_method_not_found() {
    let response = dispatch_json(r#"{"jsonrpc":"2.0","id":5,"method":"does/not/exist"}"#);
    assert_eq!(response["error"]["code"], json!(-32601));
}

#[test]
fn notification_yields_no_response() {
    let response = dispatch_line(
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        false,
    );
    assert!(response.is_none());
}

#[test]
fn ping_returns_exact_empty_result() {
    let Some(response) = dispatch_line(r#"{"jsonrpc":"2.0","id":9,"method":"ping"}"#, false) else {
        unreachable!("ping yields a response");
    };
    let expected =
        serde_json::to_string(&json!({"jsonrpc":"2.0","id":9,"result":{}})).unwrap_or_default();
    assert_eq!(response, expected);
}
