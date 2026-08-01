//! The three host-local tools — `launch`, `stop`, `logs` — over the real JSON-RPC surface
//! (GTW-745, GTW-808, GTW-943).
//!
//! They never reach a wire: each drives that host's lifecycle directly, so these fixtures
//! run against a canned lifecycle rather than a canned link. What they pin is that the
//! `host` argument selects WHICH lifecycle, and what each outcome renders.

use serde_json::json;

use crate::support::{
    EDITOR_LOG, EDITOR_PID, EDITOR_PORT, GAME_LOG, GAME_PID, GAME_PORT, dispatch_lifecycle_json,
};

/// The lines of a canned log, in the order the child "printed" them.
fn expected_lines(log: &str) -> serde_json::Value {
    json!(log.lines().collect::<Vec<&str>>())
}

/// The text block of a tool reply, parsed back into JSON.
fn payload(response: &serde_json::Value) -> serde_json::Value {
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("a tool reply carries a text content block: {response}");
    };
    serde_json::from_str(text).unwrap_or(serde_json::Value::Null)
}

/// `launch` with no `host` reaches the GAME's lifecycle; `host: "editor"` reaches the
/// editor's.
///
/// The two canned lifecycles report different ports and pids, so a swapped lookup is
/// visible in the reply rather than silent.
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

/// `stop` is aimed the same way, and reports the pid its own lifecycle stopped.
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

/// **Clause 5.** `logs` returns the child's stdout/stderr tail, one line per entry.
///
/// The canned lifecycle's child "printed" two lines; the reply carries both, in order, under
/// `lines`. A tool that reported the tail as one opaque blob, or dropped a line, fails here.
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

/// **Clause 4/5.** `logs` is aimed by its `host` argument, exactly as `launch` and `stop` are.
///
/// The two canned lifecycles report DIFFERENT log text, so a `logs` that read the game's
/// lifecycle while labelling the reply `editor` — or read the game's and labelled it `game`
/// while the caller asked for the editor — fails here. Without this case the host argument
/// could be dropped on the floor for this one tool and nothing in the suite would see it.
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

/// `logs` takes `max_lines`, and reports the cap it applied.
///
/// The RING applies the cap (`ProcessChild::output_tail`), so what this pins on the courier
/// side is that the argument is read, typed and carried rather than dropped — a `max_lines`
/// silently ignored would leave a caller unable to bound a chatty child at all.
#[test]
fn logs_carries_the_line_cap_a_call_asked_for() {
    let response = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"logs","arguments":{"max_lines":7}}}"#,
    );
    assert_eq!(payload(&response)["max_lines"], json!(7), "{response}");
}

/// A `max_lines` that is not a non-negative integer is invalid params, not a silent default.
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

/// `logs` against a host owning no child says so, rather than answering an empty log.
///
/// "This process printed nothing" and "there is no process" are different facts a caller
/// acts on differently — the no-lifecycle fixture owns no child, and the reply says exactly
/// that.
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
