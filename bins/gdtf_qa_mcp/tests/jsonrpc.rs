//! JSON-RPC fixture tests — literal request strings through the REAL dispatch (GTW-741).
//!
//! Each test feeds a literal JSON-RPC 2.0 request line to [`gdtf_qa_mcp::dispatch`] and
//! asserts on the response, exercising `initialize`, `tools/list`, and `tools/call`
//! without any real game (a canned [`GameLink`]).

use gdtf_qa_mcp::{
    ChildPid, GameLifecycle, GameLink, GamePort, LaunchOutcome, McpError, StopOutcome, dispatch,
};
use gdtf_qa_protocol::{
    envelope::{InjectReceipt, QaError, QaRequest, QaResponse},
    view::{AppFlowView, AppStateNet, BattleActiveNet, CaughtUpNet},
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
                Vec::new(),
                CaughtUpNet::new(true),
            ))),
            QaRequest::Inject(_) => Ok(QaResponse::Injected(InjectReceipt::Queued)),
            _ => Ok(QaResponse::Error(QaError::BadRequest)),
        }
    }
}

/// A lifecycle the forwarding-tool fixtures never invoke — only present so `dispatch` has
/// its argument.
struct NoLifecycle;

impl GameLifecycle for NoLifecycle {
    fn launch(&mut self, _port: GamePort) -> LaunchOutcome {
        unreachable!("the forwarding-tool fixtures never launch");
    }

    fn stop(&mut self) -> StopOutcome {
        StopOutcome::NotRunning
    }
}

/// A lifecycle with canned launch / stop outcomes, so the two host-local tools can be
/// exercised through the real dispatch without spawning a process.
struct CannedLifecycle;

impl GameLifecycle for CannedLifecycle {
    fn launch(&mut self, _port: GamePort) -> LaunchOutcome {
        LaunchOutcome::Launched {
            port: GamePort::new(7616),
            pid:  ChildPid::new(4242),
        }
    }

    fn stop(&mut self) -> StopOutcome {
        StopOutcome::Stopped {
            pid: ChildPid::new(4242),
        }
    }
}

/// Dispatch a literal line and parse the response line to a JSON value.
fn dispatch_json(line: &str) -> Value {
    let Some(response) = dispatch(line, &mut CannedGame, &mut NoLifecycle) else {
        unreachable!("a request with an id yields a response line");
    };
    serde_json::from_str(&response).unwrap_or(Value::Null)
}

/// Dispatch a literal line against the canned lifecycle and parse the response.
fn dispatch_lifecycle_json(line: &str) -> Value {
    let Some(response) = dispatch(line, &mut CannedGame, &mut CannedLifecycle) else {
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

/// `tools/list` advertises every implemented tool — the seven forwarding tools plus the
/// two lifecycle tools, `start_battle` among them.
///
/// `start_battle` is listed over the real JSON-RPC surface, which is what an MCP client
/// actually reads: the game has serviced `QaRequest::StartBattle` since T9 (GTW-742), but
/// no client tool sent it, so an agent could never reach a battle at all (GTW-760).
#[test]
fn tools_list_returns_every_tool() {
    let response = dispatch_json(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
    let Some(tools) = response["result"]["tools"].as_array() else {
        unreachable!("tools/list carries a tools array");
    };
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert_eq!(names.len(), 9);
    for expected in [
        "send_input",
        "query_state",
        "get_output",
        "take_screenshot",
        "screenshot_after",
        "app_flow",
        "start_battle",
        "launch_game",
        "stop_game",
    ] {
        assert!(names.contains(&expected), "missing tool {expected}");
    }
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

/// `tools/call launch_game` routes to the lifecycle and renders the launched result as
/// text content — it does NOT forward a request to the game link.
#[test]
fn tools_call_launch_game_renders_launched() {
    let response = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"launch_game","arguments":{"port":7616}}}"#,
    );
    assert_eq!(response["id"], json!(10));
    assert_eq!(response["result"]["isError"], json!(false));
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("launch_game renders text content");
    };
    assert!(text.contains("launched"), "rendered: {text}");
    assert!(text.contains("4242"), "rendered: {text}");
}

/// `tools/call stop_game` routes to the lifecycle and renders the stopped result.
#[test]
fn tools_call_stop_game_renders_stopped() {
    let response = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"stop_game","arguments":{}}}"#,
    );
    assert_eq!(response["id"], json!(11));
    assert_eq!(response["result"]["isError"], json!(false));
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("stop_game renders text content");
    };
    assert!(text.contains("stopped"), "rendered: {text}");
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
    let response = dispatch(
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        &mut CannedGame,
        &mut NoLifecycle,
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
        &mut NoLifecycle,
    ) else {
        unreachable!("ping yields a response");
    };
    let expected =
        serde_json::to_string(&json!({"jsonrpc":"2.0","id":9,"result":{}})).unwrap_or_default();
    assert_eq!(response, expected);
}
