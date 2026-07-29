//! `take_screenshot`'s per-call host routing over the real JSON-RPC surface (GTW-880) —
//! the one tool both children can serve, so the `host` argument decides which link the
//! call travels over.
//!
//! These go through `dispatch` → `handle_tool_call` → `HostSet::pair`, not through the
//! host-resolution helper on its own: the point is that the resolved host is what the
//! request is actually carried over. The two canned links report DIFFERENT saved paths,
//! and neither path exists on disk, so the reply names the link that answered — a call
//! that reached the wrong child names the wrong path and fails here.

use serde_json::json;

use crate::support::{EDITOR_SHOT_PATH, GAME_SHOT_PATH, dispatch_json};

/// Read the rendered text of a `tools/call` reply, asserting it is a tool error (the
/// canned saved path does not exist, so rendering reports it unreadable — and that report
/// names the path, which is the host evidence).
fn rendered_shot_text(line: &str) -> String {
    let response = dispatch_json(line);
    assert_eq!(
        response["result"]["isError"],
        json!(true),
        "a canned, unreadable saved path renders as a tool error: {response}",
    );
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("a tool error carries text content");
    };
    text.to_owned()
}

/// A `take_screenshot` naming no host travels over the GAME's link — the default the
/// existing single-host tools already had.
#[test]
fn take_screenshot_without_a_host_reaches_the_game() {
    let text = rendered_shot_text(
        r#"{"jsonrpc":"2.0","id":30,"method":"tools/call","params":{"name":"take_screenshot","arguments":{}}}"#,
    );
    assert!(text.contains(GAME_SHOT_PATH), "rendered: {text}");
    assert!(!text.contains(EDITOR_SHOT_PATH), "rendered: {text}");
}

/// `take_screenshot` with `host: "editor"` travels over the EDITOR's link. Reverting the
/// per-call host lookup in `handle_tool_call` to the tool's fixed host fails this: the
/// reply would name the game's path.
#[test]
fn take_screenshot_with_host_editor_reaches_the_editor() {
    let text = rendered_shot_text(
        r#"{"jsonrpc":"2.0","id":31,"method":"tools/call","params":{"name":"take_screenshot","arguments":{"host":"editor"}}}"#,
    );
    assert!(text.contains(EDITOR_SHOT_PATH), "rendered: {text}");
    assert!(!text.contains(GAME_SHOT_PATH), "rendered: {text}");
}

/// `host: "game"` names the game explicitly and reaches the same link the default does.
#[test]
fn take_screenshot_with_host_game_reaches_the_game() {
    let text = rendered_shot_text(
        r#"{"jsonrpc":"2.0","id":32,"method":"tools/call","params":{"name":"take_screenshot","arguments":{"host":"game","name":"shell"}}}"#,
    );
    assert!(text.contains(GAME_SHOT_PATH), "rendered: {text}");
    assert!(!text.contains(EDITOR_SHOT_PATH), "rendered: {text}");
}

/// A misspelled host is an invalid-params error out of the real `tools/call` path — never
/// a capture of the default child. The rejection reaches the caller as JSON-RPC `-32602`,
/// so a typo is visible instead of silently screenshotting the wrong process.
#[test]
fn take_screenshot_rejects_an_unknown_host_word() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":33,"method":"tools/call","params":{"name":"take_screenshot","arguments":{"host":"edtior"}}}"#,
    );
    assert_eq!(response["error"]["code"], json!(-32602));
    assert_eq!(
        response["result"],
        json!(null),
        "a rejected host must not also render a capture: {response}",
    );
    let Some(message) = response["error"]["message"].as_str() else {
        unreachable!("an invalid-params error carries a message");
    };
    assert!(message.contains("edtior"), "message: {message}");
    assert!(message.contains("editor"), "message: {message}");
    assert!(message.contains("game"), "message: {message}");
}

/// A non-string `host` is rejected the same way, out of the same real path.
#[test]
fn take_screenshot_rejects_a_non_string_host() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":34,"method":"tools/call","params":{"name":"take_screenshot","arguments":{"host":7}}}"#,
    );
    assert_eq!(response["error"]["code"], json!(-32602));
    assert_eq!(response["result"], json!(null));
}

/// The `host` argument is `take_screenshot`'s alone: a single-host tool keeps its own
/// link whatever a call asks for, over the real dispatch.
#[test]
fn a_single_host_tool_ignores_a_host_argument_over_the_wire() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":35,"method":"tools/call","params":{"name":"get_editor_query_options","arguments":{"host":"game"}}}"#,
    );
    assert_eq!(
        response["result"]["isError"],
        json!(false),
        "the editor tool must still reach the editor: {response}",
    );
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("the options reply renders as text content");
    };
    assert!(text.contains("Validation"), "rendered: {text}");
}
