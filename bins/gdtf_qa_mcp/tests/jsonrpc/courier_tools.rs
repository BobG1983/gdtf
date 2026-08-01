//! The two COURIER tools through the REAL dispatch — what a client actually receives for each
//! catalogue and outcome shape (GTW-942).
//!
//! The per-call riders are next door in [`courier_riders`](crate::courier_riders) and the
//! attachment blocks in [`courier_attach`](crate::courier_attach): both are about what the
//! courier CARRIES rather than what it renders.
//!
//! These are the courier half of the ticket's eight evidence bullets. The GAME half — that a
//! real running host produces these replies from a real command set, over a real socket —
//! is `crates/gdtf_app/tests/net_qa/commands.rs`. The split is deliberate: this crate carries
//! schema documents as opaque text and links no `schemars`, so it can prove what the courier
//! DOES with a reply but never that the reply was derived.
//!
//! Each case is a literal JSON-RPC line through `gdtf_qa_mcp::dispatch` against the canned
//! game link in [`support`](crate::support), whose `Run` arm decides from the frame the
//! COURIER built — so a courier that dropped an argument, a rider, or a name is visible here
//! rather than only against a live game.

use serde_json::{Value, json};

use crate::support::{CANNED_ARG_SCHEMA, CANNED_COMMAND, CANNED_REPLY_SCHEMA, dispatch_json};

/// The parsed JSON body of a rendered tool result's first text block.
///
/// Every courier reply — success and refusal alike — carries its payload as JSON text, so a
/// caller can act on it rather than scrape prose. Failing to parse is a real failure of that
/// contract, not a test convenience.
fn body(response: &Value) -> Value {
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("a courier reply carries a text content block: {response}");
    };
    let Ok(parsed) = serde_json::from_str::<Value>(text) else {
        unreachable!("a courier reply's text block is JSON: {text}");
    };
    parsed
}

/// **Bullet 1.** `commands(host="game")` returns the catalogue with exactly one entry: name
/// `app.phase`, `timing: Immediate`, `availability: Available`, `schemas: None`.
///
/// `schemas` is `null` rather than absent, so a client reads ONE path at either detail level
/// and can tell "you did not ask for these" from "this command has none".
#[test]
fn commands_returns_the_summary_catalogue() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"commands","arguments":{"host":"game"}}}"#,
    );
    assert_eq!(response["result"]["isError"], json!(false));
    let body = body(&response);
    assert_eq!(body["detail"], json!("Summary"));
    let Some(rows) = body["commands"].as_array() else {
        unreachable!("the catalogue renders a `commands` array: {body}");
    };
    assert_eq!(rows.len(), 1, "the canned host offers one command: {body}");
    assert_eq!(rows[0]["command"], json!(CANNED_COMMAND));
    assert_eq!(rows[0]["timing"], json!("Immediate"));
    assert_eq!(rows[0]["availability"], json!("Available"));
    assert_eq!(
        rows[0]["schemas"],
        Value::Null,
        "Summary carries no schemas: {body}"
    );
}

/// **Bullet 2.** `commands(host="game", command="app.phase", detail="Full")` returns that one
/// entry with `arguments` and `reply` as the host's derived schemas, and `arguments` carries
/// `"additionalProperties": false`.
///
/// The schemas arrive as TEXT on the wire and are handed back as real JSON objects, so a
/// client reads `schemas.arguments.additionalProperties` rather than parsing a string itself.
#[test]
fn commands_full_detail_carries_the_derived_schemas() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"commands","arguments":{"host":"game","command":"app.phase","detail":"Full"}}}"#,
    );
    assert_eq!(response["result"]["isError"], json!(false));
    let body = body(&response);
    assert_eq!(body["detail"], json!("Full"));
    let Some(rows) = body["commands"].as_array() else {
        unreachable!("the catalogue renders a `commands` array: {body}");
    };
    assert_eq!(rows.len(), 1, "the filter narrows to one row: {body}");

    let Ok(arguments) = serde_json::from_str::<Value>(CANNED_ARG_SCHEMA) else {
        unreachable!("the fixture's argument schema is JSON");
    };
    let Ok(reply) = serde_json::from_str::<Value>(CANNED_REPLY_SCHEMA) else {
        unreachable!("the fixture's reply schema is JSON");
    };
    assert_eq!(rows[0]["schemas"]["arguments"], arguments);
    assert_eq!(rows[0]["schemas"]["reply"], reply);
    assert_eq!(
        rows[0]["schemas"]["arguments"]["additionalProperties"],
        json!(false),
        "the argument schema must publish its strictness: {body}",
    );
}

/// A `command` filter naming nothing is a TOOL ERROR listing every known name, not an empty
/// list.
///
/// An empty list reads as "this host offers nothing", which is a different and wrong answer
/// to a typo — and the caller would have no way to tell the two apart.
#[test]
fn an_unknown_command_filter_is_an_error_naming_the_known_ones() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"commands","arguments":{"host":"game","command":"app.phasee"}}}"#,
    );
    assert_eq!(response["result"]["isError"], json!(true));
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("the refusal carries text: {response}");
    };
    assert!(text.contains("app.phasee"), "{text}");
    assert!(text.contains(CANNED_COMMAND), "{text}");
}

/// **Bullet 3.** `run(host="game", command="app.phase", arguments={})` returns the five-level
/// state tuple.
///
/// The reply body comes back as real JSON, not an escaped string, and every one of the five
/// levels is present — the four nested ones as explicit `null` where they are not live, which
/// is what makes "not in a battle" distinguishable from "could not read the battle phase".
#[test]
fn run_returns_the_five_level_state_tuple() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"run","arguments":{"host":"game","command":"app.phase","arguments":{}}}}"#,
    );
    assert_eq!(response["result"]["isError"], json!(false));
    let body = body(&response);
    assert_eq!(body["outcome"], json!("Ran"));
    let phase = &body["reply"]["phase"];
    assert_eq!(phase["app"], json!("Running"));
    assert_eq!(phase["running"], json!("Menu"));
    for level in ["game", "battlescape", "aftermath"] {
        assert_eq!(
            phase[level],
            Value::Null,
            "the inactive level `{level}` is present and null: {body}",
        );
    }
}

/// **Bullet 4.** `run(command="app.phasee")` returns `Unknown` listing the one known name.
#[test]
fn an_unknown_command_run_lists_the_known_names() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"run","arguments":{"host":"game","command":"app.phasee","arguments":{}}}}"#,
    );
    assert_eq!(
        response["result"]["isError"],
        json!(true),
        "a call that ran nothing is not a success: {response}",
    );
    let body = body(&response);
    assert_eq!(body["outcome"], json!("Unknown"));
    assert_eq!(body["known"], json!([CANNED_COMMAND]));
}

/// **Bullet 5.** `run(command="app.phase", arguments={"nope":1})` returns `BadArguments` with
/// the schema attached.
///
/// The schema rides along so the call is fixable in ONE round trip — and it is the same
/// document `commands(detail="Full")` publishes, rendered the same way, so a caller never has
/// to reconcile two accounts of one shape.
#[test]
fn bad_arguments_come_back_with_the_schema_attached() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"run","arguments":{"host":"game","command":"app.phase","arguments":{"nope":1}}}}"#,
    );
    assert_eq!(response["result"]["isError"], json!(true));
    let body = body(&response);
    assert_eq!(body["outcome"], json!("BadArguments"));
    assert!(
        body["detail"].as_str().is_some_and(|d| d.contains("nope")),
        "the fault names the field that was wrong: {body}",
    );
    let Ok(schema) = serde_json::from_str::<Value>(CANNED_ARG_SCHEMA) else {
        unreachable!("the fixture's argument schema is JSON");
    };
    assert_eq!(body["schema"], schema);
    assert_eq!(body["schema"]["additionalProperties"], json!(false));
}

/// A `run` with no `command` argument is INVALID PARAMS, not a call to some default.
#[test]
fn run_without_a_command_is_invalid_params() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"run","arguments":{"host":"game"}}}"#,
    );
    assert!(
        response["error"].is_object(),
        "a `run` naming no command must be a JSON-RPC error: {response}",
    );
}

/// Both courier tools are AIMED by their `host` argument: pointed at the editor, they reach
/// the editor's link, which offers no command layer and answers `BadRequest`.
///
/// Without the aim they would silently reach the game and report the GAME's catalogue as the
/// editor's — the exact failure `accepts_host_argument` exists to prevent (GTW-880).
#[test]
fn the_courier_tools_are_aimed_by_their_host_argument() {
    for line in [
        r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"commands","arguments":{"host":"editor"}}}"#,
        r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"run","arguments":{"host":"editor","command":"app.phase"}}}"#,
    ] {
        let response = dispatch_json(line);
        assert_eq!(
            response["result"]["isError"],
            json!(true),
            "the editor link answers the command layer BadRequest: {response}",
        );
        let Some(text) = response["result"]["content"][0]["text"].as_str() else {
            unreachable!("the refusal carries text: {response}");
        };
        assert!(
            text.contains("BadRequest"),
            "the call must have reached the EDITOR's link, got: {text}",
        );
    }
}
