//! One forwarding call and the two lifecycle calls, against the GAME's link and manager.

use serde_json::json;

use crate::support::{EDITOR_PID, GAME_PID, dispatch_json, dispatch_lifecycle_json};

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
    assert!(text.contains(&GAME_PID.to_string()), "rendered: {text}");
    assert!(!text.contains(&EDITOR_PID.to_string()), "rendered: {text}");
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
    assert!(text.contains(&GAME_PID.to_string()), "rendered: {text}");
    assert!(!text.contains(&EDITOR_PID.to_string()), "rendered: {text}");
}
