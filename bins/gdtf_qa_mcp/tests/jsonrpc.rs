//! JSON-RPC fixture tests — literal request strings through the REAL dispatch (GTW-741).
//!
//! Each test feeds a literal JSON-RPC 2.0 request line to [`gdtf_qa_mcp::dispatch`] and
//! asserts on the response, exercising `initialize`, `tools/list`, and `tools/call`
//! without any real game (a canned [`GameLink`]).

use gdtf_qa_mcp::{GameLink, McpError, dispatch};
use gdtf_qa_protocol::{
    envelope::{InjectReceipt, QaError, QaRequest, QaResponse},
    view::{AppFlowView, AppStateNet, BattleActiveNet},
};
use serde_json::{Value, json};

/// A canned game link: answers `app_flow` with a Running snapshot and `send_input` with a
/// queued receipt, so `tools/call` can be exercised with no socket.
struct CannedGame;

impl GameLink for CannedGame {
    fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError> {
        match request {
            QaRequest::GetAppFlow => Ok(QaResponse::AppFlow(AppFlowView::new(
                AppStateNet::Running,
                BattleActiveNet::new(true),
            ))),
            QaRequest::Inject(_) => Ok(QaResponse::Injected(InjectReceipt::Queued)),
            _ => Ok(QaResponse::Error(QaError::BadRequest)),
        }
    }
}

/// Dispatch a literal line and parse the response line to a JSON value.
fn dispatch_json(line: &str) -> Value {
    let Some(response) = dispatch(line, &mut CannedGame) else {
        unreachable!("a request with an id yields a response line");
    };
    serde_json::from_str(&response).unwrap_or(Value::Null)
}

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

/// `tools/list` advertises the five implemented tools and NOT `start_battle`.
#[test]
fn tools_list_returns_the_five_tools() {
    let response = dispatch_json(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
    let Some(tools) = response["result"]["tools"].as_array() else {
        unreachable!("tools/list carries a tools array");
    };
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert_eq!(names.len(), 5);
    for expected in [
        "send_input",
        "query_state",
        "get_output",
        "take_screenshot",
        "app_flow",
    ] {
        assert!(names.contains(&expected), "missing tool {expected}");
    }
    assert!(!names.contains(&"start_battle"));
}

/// `tools/call app_flow` forwards to the game and renders the reply as text content.
#[test]
fn tools_call_app_flow_renders_text_content() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"app_flow","arguments":{}}}"#,
    );
    assert_eq!(response["id"], json!(3));
    assert_eq!(response["result"]["isError"], json!(false));
    assert_eq!(response["result"]["content"][0]["type"], json!("text"));
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("text content carries a text field");
    };
    assert!(text.contains("Running"), "rendered: {text}");
}

/// `tools/call` with an unknown tool name is a JSON-RPC invalid-params error.
#[test]
fn tools_call_unknown_tool_is_invalid_params() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"start_battle","arguments":{}}}"#,
    );
    assert_eq!(response["error"]["code"], json!(-32602));
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
    let response = dispatch(
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        &mut CannedGame,
    );
    assert!(response.is_none());
}

/// `ping` produces exactly the empty-result response line (an exact-string check, robust
/// to key ordering by serializing the expectation through the same serializer).
#[test]
fn ping_returns_exact_empty_result() {
    let Some(response) = dispatch(
        r#"{"jsonrpc":"2.0","id":9,"method":"ping"}"#,
        &mut CannedGame,
    ) else {
        unreachable!("ping yields a response");
    };
    let expected =
        serde_json::to_string(&json!({"jsonrpc":"2.0","id":9,"result":{}})).unwrap_or_default();
    assert_eq!(response, expected);
}
