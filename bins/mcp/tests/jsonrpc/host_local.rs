use serde_json::json;

use crate::support::{
    EDITOR_PID, EDITOR_PORT, FIRST_EDITOR_INSTANCE, GAME_INSTANCE, GAME_LOG, GAME_PID, GAME_PORT,
    SECOND_EDITOR_INSTANCE, dispatch_lifecycle_json, dispatch_seeded, payload,
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
    let game = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"launch","arguments":{}}}"#,
    );
    assert_eq!(payload(&game)["status"], json!("launched"), "{game}");
    assert_eq!(payload(&game)["port"], json!(GAME_PORT), "{game}");
    assert_eq!(payload(&game)["pid"], json!(GAME_PID), "{game}");

    let editor = dispatch_lifecycle_json(
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"launch","arguments":{"host":"editor"}}}"#,
    );
    assert_eq!(payload(&editor)["port"], json!(EDITOR_PORT), "{editor}");
    assert_eq!(payload(&editor)["pid"], json!(EDITOR_PID), "{editor}");
}

#[test]
fn every_editor_launch_names_its_own_instance() {
    let replies = dispatch_seeded(
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"launch","arguments":{"host":"editor"}}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"launch","arguments":{"host":"editor"}}}"#,
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
                GAME_INSTANCE.id()
            ),
            &format!(
                r#"{{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{{"name":"stop","arguments":{{"host":"editor","instance":"{}"}}}}}}"#,
                FIRST_EDITOR_INSTANCE.id()
            ),
        ],
        &[FIRST_EDITOR_INSTANCE],
        &[GAME_INSTANCE],
    );
    let [game, editor] = replies.as_slice() else {
        unreachable!("two dispatched lines yield two replies: {replies:?}");
    };

    assert_eq!(payload(game)["status"], json!("stopped"), "{game}");
    assert_eq!(payload(game)["pid"], json!(GAME_INSTANCE.pid()), "{game}");
    assert_eq!(
        payload(editor)["pid"],
        json!(FIRST_EDITOR_INSTANCE.pid()),
        "a stop against the editor stops the EDITOR's instance: {editor}"
    );
    assert_ne!(
        payload(editor)["pid"],
        json!(GAME_INSTANCE.pid()),
        "and never the game's: {editor}"
    );
}

#[test]
fn stop_names_the_editor_instance_it_stops() {
    let reply = only(dispatch_seeded(
        &[&format!(
            r#"{{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{{"name":"stop","arguments":{{"host":"editor","instance":"{}"}}}}}}"#,
            SECOND_EDITOR_INSTANCE.id()
        )],
        &[FIRST_EDITOR_INSTANCE, SECOND_EDITOR_INSTANCE],
        &[],
    ));

    assert_eq!(
        payload(&reply)["pid"],
        json!(SECOND_EDITOR_INSTANCE.pid()),
        "the stop reports the pid of the instance it was told to stop: {reply}"
    );
    assert_ne!(
        payload(&reply)["pid"],
        json!(FIRST_EDITOR_INSTANCE.pid()),
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
    assert_eq!(body["host"], json!("game"), "{response}");
    assert_eq!(
        body["lines"],
        expected_lines(GAME_LOG),
        "logs must return every captured line, in order: {response}",
    );
}

#[test]
fn logs_is_aimed_by_its_host_argument() {
    let editor = only(dispatch_seeded(
        &[&format!(
            r#"{{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{{"name":"logs","arguments":{{"host":"editor","instance":"{}"}}}}}}"#,
            FIRST_EDITOR_INSTANCE.id()
        )],
        &[FIRST_EDITOR_INSTANCE],
        &[GAME_INSTANCE],
    ));
    let body = payload(&editor);

    assert_eq!(body["status"], json!("running"), "{editor}");
    assert_eq!(body["host"], json!("editor"), "{editor}");
    assert_eq!(
        body["lines"],
        expected_lines(FIRST_EDITOR_INSTANCE.log()),
        "logs against the editor must read the EDITOR's lifecycle: {editor}",
    );
    assert_ne!(
        body["lines"],
        expected_lines(GAME_LOG),
        "and never the game's: {editor}",
    );
}

#[test]
fn logs_reads_the_instance_it_names() {
    let reply = only(dispatch_seeded(
        &[&format!(
            r#"{{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{{"name":"logs","arguments":{{"host":"editor","instance":"{}"}}}}}}"#,
            SECOND_EDITOR_INSTANCE.id()
        )],
        &[FIRST_EDITOR_INSTANCE, SECOND_EDITOR_INSTANCE],
        &[],
    ));
    let body = payload(&reply);

    assert_eq!(
        body["lines"],
        expected_lines(SECOND_EDITOR_INSTANCE.log()),
        "logs returns the named instance's own lines: {reply}",
    );
    assert_ne!(
        body["lines"],
        expected_lines(FIRST_EDITOR_INSTANCE.log()),
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
    let response = crate::support::dispatch_json(
        r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"logs","arguments":{}}}"#,
    );
    assert_eq!(
        payload(&response)["status"],
        json!("not_running"),
        "{response}"
    );
}

#[test]
fn a_game_stop_needs_no_instance_while_the_game_records_one() {
    let reply = only(dispatch_seeded(
        &[
            r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"stop","arguments":{}}}"#,
        ],
        &[],
        &[GAME_INSTANCE],
    ));

    assert_eq!(
        reply["result"]["isError"],
        json!(false),
        "the game takes a stop that names no instance: {reply}"
    );
    assert_eq!(payload(&reply)["status"], json!("stopped"), "{reply}");
}

#[test]
fn a_game_logs_needs_no_instance_while_the_game_records_one() {
    let reply = only(dispatch_seeded(
        &[
            r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"logs","arguments":{}}}"#,
        ],
        &[],
        &[GAME_INSTANCE],
    ));

    assert_eq!(
        reply["result"]["isError"],
        json!(false),
        "the game takes a logs call that names no instance: {reply}"
    );
    assert_eq!(
        payload(&reply)["lines"],
        expected_lines(GAME_LOG),
        "{reply}"
    );
}
