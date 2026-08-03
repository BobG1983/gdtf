use serde_json::json;

use crate::support::{
    EDITOR_LOG, EDITOR_PID, EDITOR_PORT, GAME_LOG, GAME_PID, GAME_PORT, dispatch_lifecycle_json,
};

fn expected_lines(log: &str) -> serde_json::Value {
    json!(log.lines().collect::<Vec<&str>>())
}

fn payload(response: &serde_json::Value) -> serde_json::Value {
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("a tool reply carries a text content block: {response}");
    };
    serde_json::from_str(text).unwrap_or(serde_json::Value::Null)
}

#[test]
fn launch_is_aimed_by_its_host_argument() {
    let game = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"launch","arguments":{}}}"#,
    );
    assert_eq!(payload(&game)["status"], json!("launched"), "{game}");
    assert_eq!(payload(&game)["port"], json!(GAME_PORT), "{game}");
    assert_eq!(payload(&game)["pid"], json!(GAME_PID), "{game}");

    let editor = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"launch","arguments":{"host":"editor"}}}"#,
    );
    assert_eq!(payload(&editor)["port"], json!(EDITOR_PORT), "{editor}");
    assert_eq!(payload(&editor)["pid"], json!(EDITOR_PID), "{editor}");
}

#[test]
fn stop_is_aimed_by_its_host_argument() {
    let game = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"stop","arguments":{}}}"#,
    );
    assert_eq!(payload(&game)["status"], json!("stopped"), "{game}");
    assert_eq!(payload(&game)["pid"], json!(GAME_PID), "{game}");

    let editor = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"stop","arguments":{"host":"editor"}}}"#,
    );
    assert_eq!(payload(&editor)["pid"], json!(EDITOR_PID), "{editor}");
}

#[test]
fn logs_returns_the_childs_output_tail() {
    let response = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"logs","arguments":{}}}"#,
    );
    let body = payload(&response);
    assert_eq!(body["status"], json!("running"), "{response}");
    assert_eq!(body["host"], json!("game"), "{response}");
    assert_eq!(
        body["lines"],
        expected_lines(GAME_LOG),
        "logs must return every captured line, in order: {response}",
    );
}

#[test]
fn logs_is_aimed_by_its_host_argument() {
    let editor = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"logs","arguments":{"host":"editor"}}}"#,
    );
    let body = payload(&editor);
    assert_eq!(body["status"], json!("running"), "{editor}");
    assert_eq!(body["host"], json!("editor"), "{editor}");
    assert_eq!(
        body["lines"],
        expected_lines(EDITOR_LOG),
        "logs against the editor must read the EDITOR's lifecycle: {editor}",
    );
    assert_ne!(
        body["lines"],
        expected_lines(GAME_LOG),
        "and never the game's: {editor}",
    );
}

#[test]
fn logs_carries_the_line_cap_a_call_asked_for() {
    let response = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"logs","arguments":{"max_lines":7}}}"#,
    );
    assert_eq!(payload(&response)["max_lines"], json!(7), "{response}");
}

#[test]
fn a_bad_max_lines_is_rejected() {
    let response = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"logs","arguments":{"max_lines":"lots"}}}"#,
    );
    assert!(
        response["error"].is_object(),
        "a non-integer max_lines must be a JSON-RPC error: {response}",
    );
}

#[test]
fn logs_against_no_child_reports_not_running() {
    let response = crate::support::dispatch_json(
        r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"logs","arguments":{}}}"#,
    );
    assert_eq!(
        payload(&response)["status"],
        json!("not_running"),
        "{response}"
    );
}
