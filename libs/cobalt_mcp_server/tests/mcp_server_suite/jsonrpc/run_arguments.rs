use serde_json::json;

use crate::jsonrpc::support::dispatch_json;

fn body(response: &serde_json::Value) -> serde_json::Value {
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("a courier reply carries a text content block: {response}");
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) else {
        unreachable!("a courier reply's text block is JSON: {text}");
    };
    parsed
}

#[test]
fn a_ron_argument_string_is_forwarded_to_the_host_unchanged() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":20,"method":"tools/call","params":{"name":"run","arguments":{"host":"thistle","command":"sample.status","arguments":"(mode:1)"}}}"#,
    );
    assert_eq!(response["result"]["isError"], json!(true));
    let body = body(&response);
    assert_eq!(body["outcome"], json!("BadArguments"));
    assert!(
        body["detail"]
            .as_str()
            .is_some_and(|detail| detail.contains("(mode:1)")),
        "the host saw the caller's RON text byte for byte: {body}",
    );
}

#[test]
fn a_missing_argument_string_defaults_to_the_empty_ron_value() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"run","arguments":{"host":"thistle","command":"sample.status"}}}"#,
    );
    assert_eq!(
        response["result"]["isError"],
        json!(false),
        "an omitted `arguments` must reach the host as `()`, which the canned host accepts: \
         {response}",
    );
}

#[test]
fn a_non_string_argument_is_refused_naming_the_field() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":22,"method":"tools/call","params":{"name":"run","arguments":{"host":"thistle","command":"sample.status","arguments":{"mode":1}}}}"#,
    );
    assert!(
        response["error"].is_object(),
        "an `arguments` that is not a string must be a JSON-RPC error: {response}",
    );
    let Some(message) = response["error"]["message"].as_str() else {
        unreachable!("a JSON-RPC error carries a message: {response}");
    };
    assert!(
        message.contains("arguments"),
        "the refusal names the field it rejected: {message}",
    );
}
