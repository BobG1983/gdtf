//! The JSON-RPC surface itself — the handshake, the advertised tool list, the error
//! shapes, and `ping`.

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

/// `tools/list` advertises every implemented tool — the twelve forwarding tools plus the
/// four lifecycle tools, `start_battle` and `stepper_control` among them.
///
/// `start_battle` is listed over the real JSON-RPC surface, which is what an MCP client
/// actually reads: the game has serviced `QaRequest::StartBattle` since T9 (GTW-742), but
/// no client tool sent it, so an agent could never reach a battle at all (GTW-760).
/// `stepper_control` is listed for the SAME reason (GTW-766): the game services
/// `QaRequest::StepperControl`, so a missing client tool would be an unreachable half.
/// `focus_control` is listed for the SAME reason (GTW-802): without it no off-battle
/// screen (the Options screen among them) is drivable over the wire at all. The four
/// editor tools are listed for the SAME reason (GTW-808): the editor has answered
/// `GetEditorQueryOptions` / `QueryEditor` since GTW-805 and no client tool sent either,
/// so no agent could reach a running editor at all.
#[test]
fn tools_list_returns_every_tool() {
    let response = dispatch_json(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
    let Some(tools) = response["result"]["tools"].as_array() else {
        unreachable!("tools/list carries a tools array");
    };
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert_eq!(names.len(), 16);
    for expected in [
        "send_input",
        "query_state",
        "get_output",
        "take_screenshot",
        "screenshot_after",
        "app_flow",
        "start_battle",
        "stepper_control",
        "activate_menu_item",
        "focus_control",
        "launch_game",
        "stop_game",
        "get_editor_query_options",
        "query_editor",
        "launch_editor",
        "stop_editor",
    ] {
        assert!(names.contains(&expected), "missing tool {expected}");
    }
}

/// `tools/call` with an unknown tool name is a JSON-RPC invalid-params error naming the
/// tool that could not be resolved.
///
/// The probe name must be one no tool will ever claim: this test used to probe with
/// `start_battle` back when that tool did not exist, which quietly stopped testing
/// name resolution the moment it did (GTW-760 added it, and the call then failed one step
/// later on the missing `situation` argument — the same `-32602` code for a different
/// reason). Asserting the message, not just the code, keeps the two apart.
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

/// An unknown method is answered `MethodNotFound`.
#[test]
fn unknown_method_is_method_not_found() {
    let response = dispatch_json(r#"{"jsonrpc":"2.0","id":5,"method":"does/not/exist"}"#);
    assert_eq!(response["error"]["code"], json!(-32601));
}

/// A notification (no id) is answered with silence.
#[test]
fn notification_yields_no_response() {
    let response = dispatch_line(
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        false,
    );
    assert!(response.is_none());
}

/// `ping` produces exactly the empty-result response line (an exact-string check, robust
/// to key ordering by serializing the expectation through the same serializer).
#[test]
fn ping_returns_exact_empty_result() {
    let Some(response) = dispatch_line(r#"{"jsonrpc":"2.0","id":9,"method":"ping"}"#, false) else {
        unreachable!("ping yields a response");
    };
    let expected =
        serde_json::to_string(&json!({"jsonrpc":"2.0","id":9,"result":{}})).unwrap_or_default();
    assert_eq!(response, expected);
}
