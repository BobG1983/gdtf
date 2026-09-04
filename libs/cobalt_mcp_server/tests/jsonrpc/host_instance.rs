use serde_json::{Value, json};

use crate::support::{
    FIRST_BRAMBLE_INSTANCE, SECOND_BRAMBLE_INSTANCE, SeededInstance, dispatch_seeded,
};

fn bramble_call(tool: &str, instance: Option<&str>, seeded: &[SeededInstance]) -> Value {
    let arguments = instance.map_or_else(
        || r#"{"host":"bramble"}"#.to_owned(),
        |named| format!(r#"{{"host":"bramble","instance":"{named}"}}"#),
    );
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{tool}","arguments":{arguments}}}}}"#
    );
    let replies = dispatch_seeded(&[&line], seeded, &[]);
    let [reply] = replies.as_slice() else {
        unreachable!("one dispatched line yields one reply: {replies:?}");
    };
    reply.clone()
}

fn refusal_text(reply: &Value) -> String {
    let Some(text) = reply["result"]["content"][0]["text"].as_str() else {
        unreachable!("a refusal carries a text content block: {reply}");
    };
    text.to_owned()
}

fn assert_lists_both_ids(reply: &Value) {
    assert_eq!(
        reply["result"]["isError"],
        json!(true),
        "a bramble call that does not name a recorded instance is refused: {reply}"
    );
    let text = refusal_text(reply);
    for instance in [FIRST_BRAMBLE_INSTANCE, SECOND_BRAMBLE_INSTANCE] {
        assert!(
            text.contains(instance.id()),
            "the refusal names every recorded bramble instance, missing {}: {text}",
            instance.id()
        );
    }
}

#[test]
fn a_bramble_stop_naming_no_instance_is_refused_while_two_are_recorded() {
    let reply = bramble_call(
        "stop",
        None,
        &[FIRST_BRAMBLE_INSTANCE, SECOND_BRAMBLE_INSTANCE],
    );

    assert_lists_both_ids(&reply);
}

#[test]
fn a_bramble_logs_naming_no_instance_is_refused_while_two_are_recorded() {
    let reply = bramble_call(
        "logs",
        None,
        &[FIRST_BRAMBLE_INSTANCE, SECOND_BRAMBLE_INSTANCE],
    );

    assert_lists_both_ids(&reply);
}

#[test]
fn a_bramble_stop_naming_no_instance_is_refused_while_one_is_recorded() {
    let reply = bramble_call("stop", None, &[FIRST_BRAMBLE_INSTANCE]);

    assert_eq!(
        reply["result"]["isError"],
        json!(true),
        "one recorded instance is still named rather than assumed: {reply}"
    );
    assert!(
        refusal_text(&reply).contains(FIRST_BRAMBLE_INSTANCE.id()),
        "the refusal names the one recorded instance: {reply}"
    );
}

#[test]
fn a_bramble_logs_naming_no_instance_is_refused_while_one_is_recorded() {
    let reply = bramble_call("logs", None, &[FIRST_BRAMBLE_INSTANCE]);

    assert_eq!(
        reply["result"]["isError"],
        json!(true),
        "one recorded instance is still named rather than assumed: {reply}"
    );
    assert!(
        refusal_text(&reply).contains(FIRST_BRAMBLE_INSTANCE.id()),
        "the refusal names the one recorded instance: {reply}"
    );
}

#[test]
fn a_bramble_stop_naming_no_instance_answers_while_none_is_recorded() {
    let reply = bramble_call("stop", None, &[]);

    assert_eq!(
        reply["result"]["isError"],
        json!(false),
        "with no bramble record the stop falls through to the orphan check: {reply}"
    );
}

#[test]
fn a_bramble_logs_naming_no_instance_answers_while_none_is_recorded() {
    let reply = bramble_call("logs", None, &[]);

    assert_eq!(
        reply["result"]["isError"],
        json!(false),
        "with no bramble record the logs call answers rather than refusing: {reply}"
    );
}

#[test]
fn a_bramble_stop_naming_an_unrecorded_instance_is_refused() {
    let reply = bramble_call(
        "stop",
        Some("bramble-nine"),
        &[FIRST_BRAMBLE_INSTANCE, SECOND_BRAMBLE_INSTANCE],
    );

    assert_lists_both_ids(&reply);
}

#[test]
fn a_bramble_stop_naming_an_instance_is_refused_while_none_is_recorded() {
    let reply = bramble_call("stop", Some("bramble-nine"), &[]);

    assert_eq!(
        reply["result"]["isError"],
        json!(true),
        "an id no record holds never falls through to stopping something else: {reply}"
    );
}
