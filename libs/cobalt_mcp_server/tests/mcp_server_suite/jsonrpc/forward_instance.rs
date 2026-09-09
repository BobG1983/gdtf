use serde_json::{Value, json};

use crate::jsonrpc::{
    retarget::dispatch_recording,
    support::{
        FIRST_BRAMBLE_INSTANCE, SECOND_BRAMBLE_INSTANCE, SeededInstance, dispatch_lifecycle_json,
        payload,
    },
};

const LAUNCH_CALL: &str = "launch(host=\"bramble\")";

fn bramble_forward(tool: &str, seeded: &[SeededInstance]) -> Value {
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{tool}","arguments":{{"host":"bramble","command":"sample.status"}}}}}}"#
    );
    let (reply, _) = dispatch_recording(&line, seeded);
    reply
}

fn refusal_text(reply: &Value) -> String {
    let Some(text) = reply["result"]["content"][0]["text"].as_str() else {
        unreachable!("a refusal carries a text content block: {reply}");
    };
    text.to_owned()
}

fn assert_refused_naming(reply: &Value, wanted: &[&str]) {
    assert_eq!(
        reply["result"]["isError"],
        json!(true),
        "a bramble call that names no instance is refused: {reply}"
    );
    let text = refusal_text(reply);
    for named in wanted {
        assert!(
            text.contains(named),
            "the refusal names {named}, and said instead: {text}"
        );
    }
}

#[test]
fn a_bramble_run_naming_no_instance_is_refused_while_two_are_recorded() {
    let reply = bramble_forward("run", &[FIRST_BRAMBLE_INSTANCE, SECOND_BRAMBLE_INSTANCE]);

    assert_refused_naming(
        &reply,
        &[FIRST_BRAMBLE_INSTANCE.id(), SECOND_BRAMBLE_INSTANCE.id()],
    );
}

#[test]
fn a_bramble_commands_naming_no_instance_is_refused_while_two_are_recorded() {
    let reply = bramble_forward(
        "commands",
        &[FIRST_BRAMBLE_INSTANCE, SECOND_BRAMBLE_INSTANCE],
    );

    assert_refused_naming(
        &reply,
        &[FIRST_BRAMBLE_INSTANCE.id(), SECOND_BRAMBLE_INSTANCE.id()],
    );
}

#[test]
fn a_bramble_run_naming_no_instance_is_refused_while_one_is_recorded() {
    let reply = bramble_forward("run", &[FIRST_BRAMBLE_INSTANCE]);

    assert_refused_naming(&reply, &[FIRST_BRAMBLE_INSTANCE.id()]);
}

#[test]
fn a_bramble_run_naming_no_instance_is_refused_while_none_is_recorded() {
    let reply = bramble_forward("run", &[]);

    assert_refused_naming(&reply, &[LAUNCH_CALL]);
}

#[test]
fn a_thistle_run_needs_no_instance_named() {
    let reply = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"run","arguments":{"command":"sample.status","arguments":"()"}}}"#,
    );

    assert_eq!(
        reply["result"]["isError"],
        json!(false),
        "the thistle takes a run that names no instance: {reply}"
    );
    assert_eq!(payload(&reply)["outcome"], json!("Ran"), "{reply}");
}
