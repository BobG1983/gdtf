//! The two per-call RIDERS a `run` may carry, through the REAL dispatch (GTW-942).
//!
//! Split from [`courier_tools`](crate::courier_tools) because they answer a different
//! question. Those cases ask what the courier RENDERS for each catalogue and outcome shape;
//! these ask whether a rider written by a client reaches the host at all, and with the value
//! it was written with. Nothing else in the suite can see a dropped rider: every rider is
//! refused `NotBuilt` today, so a courier that never sent one would look identical from the
//! outside — which is why the canned host reports each rider's VALUE back in its refusal.

use serde_json::{Value, json};

use crate::support::dispatch_json;

/// The parsed JSON body of a rendered tool result's first text block.
fn body(response: &Value) -> Value {
    let Some(text) = response["result"]["content"][0]["text"].as_str() else {
        unreachable!("a courier reply carries a text content block: {response}");
    };
    let Ok(parsed) = serde_json::from_str::<Value>(text) else {
        unreachable!("a courier reply's text block is JSON: {text}");
    };
    parsed
}

/// **Bullet 6.** `run(..., await_ready=5)` returns `Unavailable(NotBuilt)` — the rider stub,
/// refusing rather than running the command with the rider silently dropped.
///
/// It also proves the courier CARRIES the rider: the canned host answers `Ran` for a plain
/// call with these exact arguments, so this reply can only come from `await_ready` having
/// reached it (GTW-942 added the `options` field to `RunCommand` for exactly this — the type
/// existed and the host-side refusal existed, but no wire field carried the value).
#[test]
fn an_unbuilt_rider_is_refused_not_silently_dropped() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"run","arguments":{"host":"game","command":"app.phase","arguments":{},"await_ready":5}}}"#,
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

/// The `capture` rider travels the same way — and carries its file stem when it names one.
///
/// The second half is what a bare "is the rider present" assertion cannot see: `parse_capture`
/// has three accepting shapes (`true`, a stem string, and `false`/absent meaning no rider), and
/// dropping the stem — `Value::String` returning `CaptureRider::new(None)` — would silently
/// turn "capture this as `mid_turn`" into "capture this as whatever you like".
#[test]
fn the_capture_rider_travels_with_its_file_stem() {
    for (line, expected) in [
        (
            r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"run","arguments":{"host":"game","command":"app.phase","arguments":{},"capture":true}}}"#,
            "capture=<host-chosen>",
        ),
        (
            r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"run","arguments":{"host":"game","command":"app.phase","arguments":{},"capture":"mid_turn"}}}"#,
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

/// `capture: false` is NOT a rider — it is the plain call said explicitly, so it RUNS.
///
/// Without this, `parse_capture` could map every `capture` value to a rider and the two cases
/// above would still pass, leaving a caller who wrote `false` refused for asking for nothing.
#[test]
fn capture_false_is_a_plain_call_and_runs() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"run","arguments":{"host":"game","command":"app.phase","arguments":{},"capture":false}}}"#,
    );
    assert_eq!(
        response["result"]["isError"],
        json!(false),
        "`capture: false` asks for no rider, so the command runs: {response}",
    );
    assert_eq!(body(&response)["outcome"], json!("Ran"));
}

/// A `capture` that is neither a boolean nor a string is INVALID PARAMS, not a silent no-rider.
#[test]
fn a_capture_of_the_wrong_shape_is_invalid_params() {
    let response = dispatch_json(
        r#"{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"run","arguments":{"host":"game","command":"app.phase","capture":7}}}"#,
    );
    assert!(
        response["error"].is_object(),
        "a `capture` the courier cannot read must be an error, not a dropped rider: {response}",
    );
}
