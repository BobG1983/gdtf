use serde_json::json;

use crate::jsonrpc::support::{
    BRAMBLE_PID, BRAMBLE_PORT, FIRST_BRAMBLE_INSTANCE, SECOND_BRAMBLE_INSTANCE, THISTLE_INSTANCE,
    THISTLE_LOG, THISTLE_PID, THISTLE_PORT, dispatch_lifecycle_json, dispatch_seeded, payload,
};

fn expected_lines(log: &str) -> serde_json::Value {
    json!(log.lines().collect::<Vec<&str>>())
}

fn only(replies: Vec<serde_json::Value>) -> serde_json::Value {
    let [reply] = replies.as_slice() else {
        unreachable!("one dispatched line yields one reply: {replies:?}");
    };
    reply.clone()
}

#[test]
fn launch_is_aimed_by_its_host_argument() {
    let thistle = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"launch","arguments":{}}}"#,
    );
    assert_eq!(payload(&thistle)["status"], json!("launched"), "{thistle}");
    assert_eq!(payload(&thistle)["port"], json!(THISTLE_PORT), "{thistle}");
    assert_eq!(payload(&thistle)["pid"], json!(THISTLE_PID), "{thistle}");

    let bramble = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"launch","arguments":{"host":"bramble"}}}"#,
    );
    assert_eq!(payload(&bramble)["port"], json!(BRAMBLE_PORT), "{bramble}");
    assert_eq!(payload(&bramble)["pid"], json!(BRAMBLE_PID), "{bramble}");
}

#[test]
fn every_bramble_launch_names_its_own_instance() {
    let replies = dispatch_seeded(
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"launch","arguments":{"host":"bramble"}}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"launch","arguments":{"host":"bramble"}}}"#,
        ],
        &[],
        &[],
    );
    let [first, second] = replies.as_slice() else {
        unreachable!("two dispatched lines yield two replies: {replies:?}");
    };

    let (first_id, second_id) = (
        payload(first)["instance"].clone(),
        payload(second)["instance"].clone(),
    );
    for (reply, id) in [(first, &first_id), (second, &second_id)] {
        let named = id.as_str().unwrap_or_default();
        assert!(
            !named.is_empty(),
            "a launched reply names the instance it recorded: {reply}"
        );
    }
    assert_ne!(
        first_id, second_id,
        "each launch names its own instance, first: {first}, second: {second}"
    );
}

#[test]
fn stop_is_aimed_by_its_host_argument() {
    let replies = dispatch_seeded(
        &[
            &format!(
                r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"stop","arguments":{{"instance":"{}"}}}}}}"#,
                THISTLE_INSTANCE.id()
            ),
            &format!(
                r#"{{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{{"name":"stop","arguments":{{"host":"bramble","instance":"{}"}}}}}}"#,
                FIRST_BRAMBLE_INSTANCE.id()
            ),
        ],
        &[FIRST_BRAMBLE_INSTANCE],
        &[THISTLE_INSTANCE],
    );
    let [thistle, bramble] = replies.as_slice() else {
        unreachable!("two dispatched lines yield two replies: {replies:?}");
    };

    assert_eq!(payload(thistle)["status"], json!("stopped"), "{thistle}");
    assert_eq!(
        payload(thistle)["pid"],
        json!(THISTLE_INSTANCE.pid()),
        "{thistle}"
    );
    assert_eq!(
        payload(bramble)["pid"],
        json!(FIRST_BRAMBLE_INSTANCE.pid()),
        "a stop against the bramble stops the BRAMBLE's instance: {bramble}"
    );
    assert_ne!(
        payload(bramble)["pid"],
        json!(THISTLE_INSTANCE.pid()),
        "and never the thistle's: {bramble}"
    );
}

#[test]
fn stop_names_the_bramble_instance_it_stops() {
    let reply = only(dispatch_seeded(
        &[&format!(
            r#"{{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{{"name":"stop","arguments":{{"host":"bramble","instance":"{}"}}}}}}"#,
            SECOND_BRAMBLE_INSTANCE.id()
        )],
        &[FIRST_BRAMBLE_INSTANCE, SECOND_BRAMBLE_INSTANCE],
        &[],
    ));

    assert_eq!(
        payload(&reply)["pid"],
        json!(SECOND_BRAMBLE_INSTANCE.pid()),
        "the stop reports the pid of the instance it was told to stop: {reply}"
    );
    assert_ne!(
        payload(&reply)["pid"],
        json!(FIRST_BRAMBLE_INSTANCE.pid()),
        "and never another recorded instance's: {reply}"
    );
}

#[test]
fn logs_returns_the_childs_output_tail() {
    let response = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"logs","arguments":{}}}"#,
    );
    let body = payload(&response);
    assert_eq!(body["status"], json!("running"), "{response}");
    assert_eq!(body["host"], json!("thistle"), "{response}");
    assert_eq!(
        body["lines"],
        expected_lines(THISTLE_LOG),
        "logs must return every captured line, in order: {response}",
    );
}

#[test]
fn logs_is_aimed_by_its_host_argument() {
    let bramble = only(dispatch_seeded(
        &[&format!(
            r#"{{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{{"name":"logs","arguments":{{"host":"bramble","instance":"{}"}}}}}}"#,
            FIRST_BRAMBLE_INSTANCE.id()
        )],
        &[FIRST_BRAMBLE_INSTANCE],
        &[THISTLE_INSTANCE],
    ));
    let body = payload(&bramble);

    assert_eq!(body["status"], json!("running"), "{bramble}");
    assert_eq!(body["host"], json!("bramble"), "{bramble}");
    assert_eq!(
        body["lines"],
        expected_lines(FIRST_BRAMBLE_INSTANCE.log()),
        "logs against the bramble must read the BRAMBLE's lifecycle: {bramble}",
    );
    assert_ne!(
        body["lines"],
        expected_lines(THISTLE_LOG),
        "and never the thistle's: {bramble}",
    );
}

#[test]
fn logs_reads_the_instance_it_names() {
    let reply = only(dispatch_seeded(
        &[&format!(
            r#"{{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{{"name":"logs","arguments":{{"host":"bramble","instance":"{}"}}}}}}"#,
            SECOND_BRAMBLE_INSTANCE.id()
        )],
        &[FIRST_BRAMBLE_INSTANCE, SECOND_BRAMBLE_INSTANCE],
        &[],
    ));
    let body = payload(&reply);

    assert_eq!(
        body["lines"],
        expected_lines(SECOND_BRAMBLE_INSTANCE.log()),
        "logs returns the named instance's own lines: {reply}",
    );
    assert_ne!(
        body["lines"],
        expected_lines(FIRST_BRAMBLE_INSTANCE.log()),
        "and never another recorded instance's: {reply}",
    );
}

#[test]
fn logs_carries_the_line_cap_a_call_asked_for() {
    let response = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"logs","arguments":{"max_lines":7}}}"#,
    );
    assert_eq!(payload(&response)["max_lines"], json!(7), "{response}");
}

#[test]
fn a_bad_max_lines_is_rejected() {
    let response = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"logs","arguments":{"max_lines":"lots"}}}"#,
    );
    assert!(
        response["error"].is_object(),
        "a non-integer max_lines must be a JSON-RPC error: {response}",
    );
}

#[test]
fn logs_against_no_child_reports_not_running() {
    let response = crate::jsonrpc::support::dispatch_json(
        r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"logs","arguments":{}}}"#,
    );
    assert_eq!(
        payload(&response)["status"],
        json!("not_running"),
        "{response}"
    );
}

#[test]
fn a_thistle_stop_needs_no_instance_while_the_thistle_records_one() {
    let reply = only(dispatch_seeded(
        &[
            r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"stop","arguments":{}}}"#,
        ],
        &[],
        &[THISTLE_INSTANCE],
    ));

    assert_eq!(
        reply["result"]["isError"],
        json!(false),
        "the thistle takes a stop that names no instance: {reply}"
    );
    assert_eq!(payload(&reply)["status"], json!("stopped"), "{reply}");
}

#[test]
fn a_thistle_logs_needs_no_instance_while_the_thistle_records_one() {
    let reply = only(dispatch_seeded(
        &[
            r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"logs","arguments":{}}}"#,
        ],
        &[],
        &[THISTLE_INSTANCE],
    ));

    assert_eq!(
        reply["result"]["isError"],
        json!(false),
        "the thistle takes a logs call that names no instance: {reply}"
    );
    assert_eq!(
        payload(&reply)["lines"],
        expected_lines(THISTLE_LOG),
        "{reply}"
    );
}
