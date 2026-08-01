//! What a `logs` call reads and hands the lifecycle, and what each answer renders
//! (GTW-943).
//!
//! The tool is three steps: parse `max_lines`, ask the lifecycle for that many, render the
//! answer. The middle step is the one nothing else can see — the ring applies the cap, so a
//! handler that asked for its own default instead of the caller's would still render the cap
//! it echoes. These pin the value the lifecycle is actually handed.

use serde_json::json;

use super::support::StubLifecycle;
use crate::{
    hosts::QaHost,
    lifecycle::{OutputTail, TailLines},
    mcp::{ToolCallOutcome, control::handle::handle_logs},
};

/// The JSON payload a rendered `logs` reply carries.
fn payload(rendered: &serde_json::Value) -> serde_json::Value {
    let Some(text) = rendered["content"][0]["text"].as_str() else {
        unreachable!("a logs reply is text content: {rendered}");
    };
    let Ok(parsed) = serde_json::from_str(text) else {
        unreachable!("a logs reply's text is JSON: {text}");
    };
    parsed
}

/// Render a `logs` call for `host` against `lifecycle`, failing loudly on anything but a
/// result.
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

/// Render a `logs` call against the GAME's lifecycle.
fn logs(args: &serde_json::Value, lifecycle: &mut StubLifecycle) -> serde_json::Value {
    logs_for(QaHost::Game, args, lifecycle)
}

/// A `max_lines` the caller named reaches the LIFECYCLE, not just the reply.
///
/// The discriminating fact: the reply echoes the cap it parsed, so an echo assertion alone
/// passes even if the handler asked the ring for [`TailLines::DEFAULT`]. This reads what the
/// lifecycle was handed.
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

/// A call that names no cap asks for the default — never zero, and never "everything".
#[test]
fn a_call_with_no_cap_asks_for_the_default() {
    let mut lifecycle = StubLifecycle::printing(Some(OutputTail::new("a line".to_owned())));
    drop(logs(&json!({}), &mut lifecycle));

    assert_eq!(lifecycle.asked_for.get(), Some(TailLines::DEFAULT));
}

/// A running child's tail renders as one entry per LINE, in order.
///
/// One blob would make a caller split the text itself, and the split is the host's to do:
/// it is the side that knows the tail is line-oriented at all.
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

/// The reply names the host the CALL was aimed at, on both answers.
///
/// The label is rendered, not echoed from the request, so a `logs` that read one host's
/// lifecycle under the other's name is only visible if something reads it per host. The
/// routing half — that `host: "editor"` reaches the EDITOR's lifecycle — is pinned over the
/// real dispatch in `tests/jsonrpc/host_local.rs`.
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

/// A host owning no child answers `not_running` — not an empty log, which reads as a silent
/// process. The two are different facts a caller acts on differently.
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

/// A child that has printed NOTHING answers `running` with an empty list — the other half
/// of the same distinction.
#[test]
fn a_silent_child_renders_an_empty_line_list() {
    let mut lifecycle = StubLifecycle::printing(Some(OutputTail::default()));
    let rendered = logs(&json!({}), &mut lifecycle);

    let body = payload(&rendered);
    assert_eq!(body["status"], json!("running"), "{rendered}");
    assert_eq!(body["lines"], json!([]), "{rendered}");
}
