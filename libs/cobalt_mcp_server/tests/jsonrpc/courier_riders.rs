use serde_json::{Value, json};

use crate::support::dispatch_json;

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
fn an_unbuilt_rider_is_refused_not_silently_dropped() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"run","arguments":{"host":"thistle","command":"app.phase","arguments":"()","await_ready":5}}}"#,
    );
    assert_eq!(response["result"]["isError"], json!(true));
    let body = body(&response);
    assert_eq!(body["outcome"], json!("Unavailable"));
    assert_eq!(body["code"], json!("NotBuilt"));
    assert!(
        body["note"]
            .as_str()
            .is_some_and(|n| n.contains("await_ready=5")),
        "the refusal names the rider that is missing AND the budget it carried, so a courier \
         that sent the rider with the wrong value is visible too: {body}",
    );
}

#[test]
fn the_capture_rider_travels_with_its_file_stem() {
    for (line, expected) in [
        (
            r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"run","arguments":{"host":"thistle","command":"app.phase","arguments":"()","capture":true}}}"#,
            "capture=<host-chosen>",
        ),
        (
            r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"run","arguments":{"host":"thistle","command":"app.phase","arguments":"()","capture":"mid_turn"}}}"#,
            "capture=mid_turn",
        ),
    ] {
        let response = dispatch_json(line);
        assert_eq!(
            response["result"]["isError"],
            json!(true),
            "an unbuilt capture rider must be refused, not run without it: {response}",
        );
        let body = body(&response);
        assert_eq!(body["outcome"], json!("Unavailable"));
        assert_eq!(body["code"], json!("NotBuilt"));
        assert!(
            body["note"].as_str().is_some_and(|n| n.contains(expected)),
            "the host must have received `{expected}`: {body}",
        );
    }
}

#[test]
fn capture_false_is_a_plain_call_and_runs() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"run","arguments":{"host":"thistle","command":"app.phase","arguments":"()","capture":false}}}"#,
    );
    assert_eq!(
        response["result"]["isError"],
        json!(false),
        "`capture: false` asks for no rider, so the command runs: {response}",
    );
    assert_eq!(body(&response)["outcome"], json!("Ran"));
}

#[test]
fn a_capture_of_the_wrong_shape_is_invalid_params() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"run","arguments":{"host":"thistle","command":"app.phase","capture":7}}}"#,
    );
    assert!(
        response["error"].is_object(),
        "a `capture` the courier cannot read must be an error, not a dropped rider: {response}",
    );
}
