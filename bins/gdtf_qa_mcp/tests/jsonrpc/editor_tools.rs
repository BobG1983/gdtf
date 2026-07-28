//! The four EDITOR tools over the real JSON-RPC surface (GTW-808) — the topic list, one
//! topic query, the unknown-topic rejection, and the editor's own launch / stop.

use serde_json::json;

use crate::support::{
    EDITOR_PID, EDITOR_PORT, GAME_PID, GAME_PORT, dispatch_json, dispatch_lifecycle_json,
};

/// `tools/call get_editor_query_options` forwards to the EDITOR's link and renders the
/// options reply — the topic list plus the readiness a client polls (GTW-808).
///
/// The fixture's game link rejects the editor's requests, so this reply is only renderable
/// if the call actually reached the EDITOR's link: a swapped host lookup renders an error
/// here instead.
#[test]
fn tools_call_editor_query_options_renders_the_topic_list() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":20,"method":"tools/call","params":{"name":"get_editor_query_options","arguments":{}}}"#,
    );
    assert_eq!(response["result"]["isError"], json!(false));
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("the options reply renders as text content");
    };
    assert!(text.contains("Validation"), "rendered: {text}");
    assert!(text.contains("Load"), "rendered: {text}");
}

/// `tools/call query_editor` carries the named topic and renders the answer.
#[test]
fn tools_call_query_editor_carries_the_topic() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"query_editor","arguments":{"topic":"Readiness"}}}"#,
    );
    assert_eq!(response["result"]["isError"], json!(false));
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("the query reply renders as text content");
    };
    assert!(text.contains("Editing"), "rendered: {text}");
}

/// `query_editor` with a topic no editor claims is an invalid-params rejection listing the
/// legal topics — never a silent fallback to some default topic.
#[test]
fn tools_call_query_editor_rejects_an_unknown_topic() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":22,"method":"tools/call","params":{"name":"query_editor","arguments":{"topic":"Everything"}}}"#,
    );
    assert_eq!(response["error"]["code"], json!(-32602));
    let Some(message) = response["error"]["message"].as_str() else {
        unreachable!("an invalid-params error carries a message");
    };
    assert!(message.contains("Everything"), "message: {message}");
    assert!(message.contains("Readiness"), "message: {message}");
}

/// `tools/call launch_editor` routes to the EDITOR's lifecycle and renders the launched
/// result — it does NOT forward a request, and it does not touch the game's manager.
///
/// The two hosts' canned lifecycles report DIFFERENT ports and pids, so the reply naming
/// the editor's pair is what proves the editor's manager ran; the game's absence from the
/// reply is what proves its manager did not (GTW-808 clause 6).
#[test]
fn tools_call_launch_editor_renders_launched() {
    let response = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":23,"method":"tools/call","params":{"name":"launch_editor","arguments":{"port":7617}}}"#,
    );
    assert_eq!(response["result"]["isError"], json!(false));
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("launch_editor renders text content");
    };
    assert!(text.contains("launched"), "rendered: {text}");
    assert!(text.contains("gdtf_content_editor_bin"), "rendered: {text}");
    assert!(text.contains(&EDITOR_PID.to_string()), "rendered: {text}");
    assert!(text.contains(&EDITOR_PORT.to_string()), "rendered: {text}");
    assert!(!text.contains(&GAME_PID.to_string()), "rendered: {text}");
    assert!(!text.contains(&GAME_PORT.to_string()), "rendered: {text}");
}

/// `tools/call stop_editor` routes to the EDITOR's lifecycle and renders the stopped
/// result — the editor's pid, never the game's.
#[test]
fn tools_call_stop_editor_renders_stopped() {
    let response = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":24,"method":"tools/call","params":{"name":"stop_editor","arguments":{}}}"#,
    );
    assert_eq!(response["result"]["isError"], json!(false));
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("stop_editor renders text content");
    };
    assert!(text.contains("stopped"), "rendered: {text}");
    assert!(text.contains(&EDITOR_PID.to_string()), "rendered: {text}");
    assert!(!text.contains(&GAME_PID.to_string()), "rendered: {text}");
}
