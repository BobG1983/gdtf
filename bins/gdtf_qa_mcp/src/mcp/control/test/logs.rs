use serde_json::json;

use super::support::StubLifecycle;
use crate::{
    hosts::QaHost,
    lifecycle::{OutputTail, TailLines},
    mcp::{ToolCallOutcome, control::handle::handle_logs},
};

fn payload(rendered: &serde_json::Value) -> serde_json::Value {
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("a logs reply is text content: {rendered}");
    };
    let Ok(parsed) = serde_json::from_str(text) else {
        unreachable!("a logs reply's text is JSON: {text}");
    };
    parsed
}

fn logs_for(
    host: QaHost,
    args: &serde_json::Value,
    lifecycle: &mut StubLifecycle,
) -> serde_json::Value {
    let ToolCallOutcome::Result(rendered) = handle_logs(host, args, lifecycle) else {
        unreachable!("a logs call with valid arguments renders a result block");
    };
    rendered
}

fn logs(args: &serde_json::Value, lifecycle: &mut StubLifecycle) -> serde_json::Value {
    logs_for(QaHost::Game, args, lifecycle)
}

#[test]
fn the_parsed_line_cap_is_what_the_lifecycle_is_asked_for() {
    let mut lifecycle = StubLifecycle::printing(Some(OutputTail::new("a line".to_owned())));
    let rendered = logs(&json!({ "max_lines": 3 }), &mut lifecycle);

    assert_eq!(
        lifecycle.asked_for.get(),
        Some(TailLines::new(3)),
        "the lifecycle is asked for the cap the CALL named",
    );
    assert_eq!(
        payload(&rendered)["max_lines"],
        json!(3),
        "and the reply reports the same cap: {rendered}",
    );
}

#[test]
fn a_call_with_no_cap_asks_for_the_default() {
    let mut lifecycle = StubLifecycle::printing(Some(OutputTail::new("a line".to_owned())));
    drop(logs(&json!({}), &mut lifecycle));

    assert_eq!(lifecycle.asked_for.get(), Some(TailLines::DEFAULT));
}

#[test]
fn a_childs_output_renders_one_entry_per_line() {
    let mut lifecycle =
        StubLifecycle::printing(Some(OutputTail::new("first\nsecond\nthird".to_owned())));
    let rendered = logs(&json!({}), &mut lifecycle);

    let body = payload(&rendered);
    assert_eq!(body["status"], json!("running"), "{rendered}");
    assert_eq!(body["host"], json!("game"), "{rendered}");
    assert_eq!(
        body["lines"],
        json!(["first", "second", "third"]),
        "{rendered}"
    );
}

#[test]
fn the_reply_names_the_host_the_call_was_aimed_at() {
    let mut running = StubLifecycle::printing(Some(OutputTail::new("a line".to_owned())));
    let rendered = logs_for(QaHost::Editor, &json!({}), &mut running);
    assert_eq!(payload(&rendered)["host"], json!("editor"), "{rendered}");

    let mut absent = StubLifecycle::printing(None);
    let rendered = logs_for(QaHost::Editor, &json!({}), &mut absent);
    let body = payload(&rendered);
    assert_eq!(body["status"], json!("not_running"), "{rendered}");
    assert_eq!(
        body["host"],
        json!("editor"),
        "a host with no child still says WHICH host: {rendered}",
    );
}

#[test]
fn a_host_owning_no_child_says_so() {
    let mut lifecycle = StubLifecycle::printing(None);
    let rendered = logs(&json!({}), &mut lifecycle);

    let body = payload(&rendered);
    assert_eq!(body["status"], json!("not_running"), "{rendered}");
    assert!(
        body["lines"].is_null(),
        "a host with no child reports no line list at all: {rendered}",
    );
}

#[test]
fn a_silent_child_renders_an_empty_line_list() {
    let mut lifecycle = StubLifecycle::printing(Some(OutputTail::default()));
    let rendered = logs(&json!({}), &mut lifecycle);

    let body = payload(&rendered);
    assert_eq!(body["status"], json!("running"), "{rendered}");
    assert_eq!(body["lines"], json!([]), "{rendered}");
}
