use cobalt_mcp_server::McpPort;
use serde_json::{Value, json};

use crate::{
    retarget::dispatch_recording,
    support::{
        BRAMBLE_HOST_NAME, CANNED_ARG_SCHEMA, CANNED_COMMAND, CANNED_REPLY, CANNED_REPLY_SCHEMA,
        FIRST_BRAMBLE_INSTANCE, SECOND_BRAMBLE_INSTANCE, dispatch_json,
    },
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
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"commands","arguments":{"host":"thistle"}}}"#,
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
fn commands_full_detail_carries_each_shape_as_an_unparsed_string() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"commands","arguments":{"host":"thistle","command":"sample.status","detail":"Full"}}}"#,
    );
    assert_eq!(response["result"]["isError"], json!(false));
    let body = body(&response);
    assert_eq!(body["detail"], json!("Full"));
    let Some(rows) = body["commands"].as_array() else {
        unreachable!("the catalogue renders a `commands` array: {body}");
    };
    assert_eq!(rows.len(), 1, "the filter narrows to one row: {body}");

    assert_eq!(
        rows[0]["schemas"]["arguments"],
        json!(CANNED_ARG_SCHEMA),
        "the published shape travels as opaque text, not as a parsed document: {body}",
    );
    assert_eq!(
        rows[0]["schemas"]["reply"],
        json!(CANNED_REPLY_SCHEMA),
        "the published reply shape travels as opaque text: {body}",
    );
}

#[test]
fn an_unknown_command_filter_is_an_error_naming_the_known_ones() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"commands","arguments":{"host":"thistle","command":"sample.statuss"}}}"#,
    );
    assert_eq!(response["result"]["isError"], json!(true));
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("the refusal carries text: {response}");
    };
    assert!(text.contains("sample.statuss"), "{text}");
    assert!(text.contains(CANNED_COMMAND), "{text}");
}

#[test]
fn run_returns_the_ron_reply_text_intact() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"run","arguments":{"host":"thistle","command":"sample.status","arguments":"()"}}}"#,
    );
    assert_eq!(response["result"]["isError"], json!(false));
    let body = body(&response);
    assert_eq!(body["outcome"], json!("Ran"));
    assert_eq!(
        body["reply"],
        json!(CANNED_REPLY),
        "the RON reply travels as opaque text, so no enum field is blanked: {body}",
    );
    for wanted in ["link:Ready", "stage:Some(Idle)", "detail:None"] {
        assert!(
            body["reply"].as_str().is_some_and(|r| r.contains(wanted)),
            "`{wanted}` must survive the render: {body}",
        );
    }
}

#[test]
fn an_unknown_command_run_lists_the_known_names() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"run","arguments":{"host":"thistle","command":"sample.statuss","arguments":"()"}}}"#,
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
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"run","arguments":{"host":"thistle","command":"sample.status","arguments":"(nope:1)"}}}"#,
    );
    assert_eq!(response["result"]["isError"], json!(true));
    let body = body(&response);
    assert_eq!(body["outcome"], json!("BadArguments"));
    assert!(
        body["detail"].as_str().is_some_and(|d| d.contains("nope")),
        "the fault names the field that was wrong: {body}",
    );
    assert_eq!(
        body["schema"],
        json!(CANNED_ARG_SCHEMA),
        "the attached shape travels as opaque text: {body}",
    );
}

#[test]
fn run_without_a_command_is_invalid_params() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"run","arguments":{"host":"thistle"}}}"#,
    );
    assert!(
        response["error"].is_object(),
        "a `run` naming no command must be a JSON-RPC error: {response}",
    );
}

fn last_retarget(ports: &[McpPort], tool: &str) -> McpPort {
    let Some(port) = ports.last() else {
        unreachable!("a bramble {tool} points the link at the instance it named, recorded: none");
    };
    *port
}

#[test]
fn a_run_reaches_the_bramble_instance_it_names() {
    let (_, ports) = dispatch_recording(
        &format!(
            r#"{{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{{"name":"run","arguments":{{"host":"bramble","instance":"{}","command":"sample.status"}}}}}}"#,
            SECOND_BRAMBLE_INSTANCE.id()
        ),
        &[FIRST_BRAMBLE_INSTANCE, SECOND_BRAMBLE_INSTANCE],
    );

    assert_eq!(
        last_retarget(&ports, "run"),
        SECOND_BRAMBLE_INSTANCE.port(),
        "the run goes to the port the named instance listens on, recorded: {ports:?}"
    );
    assert_ne!(
        last_retarget(&ports, "run"),
        FIRST_BRAMBLE_INSTANCE.port(),
        "and never another recorded instance's, recorded: {ports:?}"
    );
}

#[test]
fn a_commands_call_reaches_the_bramble_instance_it_names() {
    let (_, ports) = dispatch_recording(
        &format!(
            r#"{{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{{"name":"commands","arguments":{{"host":"bramble","instance":"{}"}}}}}}"#,
            SECOND_BRAMBLE_INSTANCE.id()
        ),
        &[FIRST_BRAMBLE_INSTANCE, SECOND_BRAMBLE_INSTANCE],
    );

    assert_eq!(
        last_retarget(&ports, "commands"),
        SECOND_BRAMBLE_INSTANCE.port(),
        "the catalogue is asked of the port the named instance listens on, recorded: {ports:?}"
    );
    assert_ne!(
        last_retarget(&ports, "commands"),
        FIRST_BRAMBLE_INSTANCE.port(),
        "and never another recorded instance's, recorded: {ports:?}"
    );
}

#[test]
fn the_courier_tools_are_aimed_by_their_host_argument() {
    let (catalogue, _) = dispatch_recording(
        &format!(
            r#"{{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{{"name":"commands","arguments":{{"host":"bramble","instance":"{}"}}}}}}"#,
            FIRST_BRAMBLE_INSTANCE.id()
        ),
        &[FIRST_BRAMBLE_INSTANCE],
    );
    let Some(text) = catalogue["result"]["content"][0]["text"].as_str() else {
        unreachable!("the catalogue carries text: {catalogue}");
    };
    assert!(
        text.contains(BRAMBLE_HOST_NAME),
        "the call must have reached the BRAMBLE's link, got: {text}",
    );
    assert!(
        !text.contains(CANNED_COMMAND),
        "the THISTLE's one command must not appear in the BRAMBLE's catalogue: {text}",
    );

    let (outcome, _) = dispatch_recording(
        &format!(
            r#"{{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{{"name":"run","arguments":{{"host":"bramble","instance":"{}","command":"sample.status"}}}}}}"#,
            FIRST_BRAMBLE_INSTANCE.id()
        ),
        &[FIRST_BRAMBLE_INSTANCE],
    );
    assert_eq!(
        outcome["result"]["isError"],
        json!(true),
        "a command the BRAMBLE does not offer is a tool error: {outcome}",
    );
    let Some(text) = outcome["result"]["content"][0]["text"].as_str() else {
        unreachable!("the refusal carries text: {outcome}");
    };
    assert!(
        text.contains("Unknown"),
        "the BRAMBLE knows no command by that name, so the outcome is Unknown: {text}",
    );
}
