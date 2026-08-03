use serde_json::{Value, json};

use crate::support::{
    CANNED_ARG_SCHEMA, CANNED_COMMAND, CANNED_REPLY_SCHEMA, EDITOR_HOST_NAME, dispatch_json,
};

fn body(response: &Value) -> Value {
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("a courier reply carries a text content block: {response}");
    };
    let Ok(parsed) = serde_json::from_str::<Value>(text) else {
        unreachable!("a courier reply's text block is JSON: {text}");
    };
    parsed
}

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

#[test]
fn the_courier_tools_are_aimed_by_their_host_argument() {
    let catalogue = dispatch_json(
        r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"commands","arguments":{"host":"editor"}}}"#,
    );
    let Some(text) = catalogue["result"]["content"][0]["text"].as_str() else {
        unreachable!("the catalogue carries text: {catalogue}");
    };
    assert!(
        text.contains(EDITOR_HOST_NAME),
        "the call must have reached the EDITOR's link, got: {text}",
    );
    assert!(
        !text.contains(CANNED_COMMAND),
        "the GAME's one command must not appear in the EDITOR's catalogue: {text}",
    );

    let outcome = dispatch_json(
        r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"run","arguments":{"host":"editor","command":"app.phase"}}}"#,
    );
    assert_eq!(
        outcome["result"]["isError"],
        json!(true),
        "a command the EDITOR does not offer is a tool error: {outcome}",
    );
    let Some(text) = outcome["result"]["content"][0]["text"].as_str() else {
        unreachable!("the refusal carries text: {outcome}");
    };
    assert!(
        text.contains("Unknown"),
        "the EDITOR knows no command by that name, so the outcome is Unknown: {text}",
    );
}
