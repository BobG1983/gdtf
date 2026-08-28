use serde_json::{Value, json};

use crate::support::{
    FIRST_EDITOR_INSTANCE, SECOND_EDITOR_INSTANCE, SeededInstance, dispatch_seeded,
};

fn editor_call(tool: &str, instance: Option<&str>, seeded: &[SeededInstance]) -> Value {
    let arguments = instance.map_or_else(
        || r#"{"host":"editor"}"#.to_owned(),
        |named| format!(r#"{{"host":"editor","instance":"{named}"}}"#),
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
        "an editor call that does not name a recorded instance is refused: {reply}"
    );
    let text = refusal_text(reply);
    for instance in [FIRST_EDITOR_INSTANCE, SECOND_EDITOR_INSTANCE] {
        assert!(
            text.contains(instance.id()),
            "the refusal names every recorded editor instance, missing {}: {text}",
            instance.id()
        );
    }
}

#[test]
fn an_editor_stop_naming_no_instance_is_refused_while_two_are_recorded() {
    let reply = editor_call(
        "stop",
        None,
        &[FIRST_EDITOR_INSTANCE, SECOND_EDITOR_INSTANCE],
    );

    assert_lists_both_ids(&reply);
}

#[test]
fn an_editor_logs_naming_no_instance_is_refused_while_two_are_recorded() {
    let reply = editor_call(
        "logs",
        None,
        &[FIRST_EDITOR_INSTANCE, SECOND_EDITOR_INSTANCE],
    );

    assert_lists_both_ids(&reply);
}

#[test]
fn an_editor_stop_naming_no_instance_is_refused_while_one_is_recorded() {
    let reply = editor_call("stop", None, &[FIRST_EDITOR_INSTANCE]);

    assert_eq!(
        reply["result"]["isError"],
        json!(true),
        "one recorded instance is still named rather than assumed: {reply}"
    );
    assert!(
        refusal_text(&reply).contains(FIRST_EDITOR_INSTANCE.id()),
        "the refusal names the one recorded instance: {reply}"
    );
}

#[test]
fn an_editor_logs_naming_no_instance_is_refused_while_one_is_recorded() {
    let reply = editor_call("logs", None, &[FIRST_EDITOR_INSTANCE]);

    assert_eq!(
        reply["result"]["isError"],
        json!(true),
        "one recorded instance is still named rather than assumed: {reply}"
    );
    assert!(
        refusal_text(&reply).contains(FIRST_EDITOR_INSTANCE.id()),
        "the refusal names the one recorded instance: {reply}"
    );
}

#[test]
fn an_editor_stop_naming_no_instance_answers_while_none_is_recorded() {
    let reply = editor_call("stop", None, &[]);

    assert_eq!(
        reply["result"]["isError"],
        json!(false),
        "with no editor record the stop falls through to the orphan check: {reply}"
    );
}

#[test]
fn an_editor_logs_naming_no_instance_answers_while_none_is_recorded() {
    let reply = editor_call("logs", None, &[]);

    assert_eq!(
        reply["result"]["isError"],
        json!(false),
        "with no editor record the logs call answers rather than refusing: {reply}"
    );
}

#[test]
fn an_editor_stop_naming_an_unrecorded_instance_is_refused() {
    let reply = editor_call(
        "stop",
        Some("editor-nine"),
        &[FIRST_EDITOR_INSTANCE, SECOND_EDITOR_INSTANCE],
    );

    assert_lists_both_ids(&reply);
}

#[test]
fn an_editor_stop_naming_an_instance_is_refused_while_none_is_recorded() {
    let reply = editor_call("stop", Some("editor-nine"), &[]);

    assert_eq!(
        reply["result"]["isError"],
        json!(true),
        "an id no record holds never falls through to stopping something else: {reply}"
    );
}
