use gdtf_app::test_support::NET_QA_SERVER_NAME;
use gdtf_qa_protocol::{
    command::{
        AwaitBudget, CommandAvailability, CommandName, CommandOutcome, CommandTiming, RunOptions,
    },
    message::{QaRequest, QaResponse},
};
use serde_json::Value;

use super::{
    command_exchange::{APP_PHASE, exchange, exchange_all, run},
    socket_support::TestResult,
};

#[test]
fn the_catalogue_lists_exactly_app_phase() -> TestResult {
    let reply = exchange(QaRequest::Catalogue)?;
    let QaResponse::Catalogue(catalogue) = reply else {
        unreachable!("a Catalogue request is answered with a catalogue, got {reply:?}");
    };
    assert_eq!(
        *catalogue.host, NET_QA_SERVER_NAME,
        "the catalogue must answer under the same host name the handshake reports: {catalogue:?}",
    );
    assert_eq!(
        catalogue.entries.len(),
        1,
        "the game publishes exactly one command today: {catalogue:?}",
    );
    let Some(entry) = catalogue.entries.first() else {
        unreachable!("a one-row catalogue has a first row");
    };
    assert_eq!(entry.command, CommandName::from_static(APP_PHASE));
    assert_eq!(entry.timing, CommandTiming::Immediate);
    assert_eq!(entry.availability, CommandAvailability::Available);
    assert!(
        !entry.summary.as_str().is_empty(),
        "every row carries the one line a client reads to learn what it does",
    );
    Ok(())
}

#[test]
fn the_catalogue_row_carries_the_derived_schemas() -> TestResult {
    let reply = exchange(QaRequest::Catalogue)?;
    let QaResponse::Catalogue(catalogue) = reply else {
        unreachable!("a Catalogue request is answered with a catalogue, got {reply:?}");
    };
    let Some(entry) = catalogue.entries.first() else {
        unreachable!("a one-row catalogue has a first row");
    };

    let Ok(arguments) = serde_json::from_str::<Value>(entry.arguments.as_str()) else {
        unreachable!(
            "the derived argument schema is JSON: {}",
            entry.arguments.as_str()
        );
    };
    assert_eq!(
        arguments["title"], "AppPhaseArgs",
        "the argument schema is derived from the command's own Args type: {arguments}",
    );
    assert_eq!(
        arguments["additionalProperties"], false,
        "`deny_unknown_fields` must reach the published schema: {arguments}",
    );

    let Ok(reply_schema) = serde_json::from_str::<Value>(entry.reply.as_str()) else {
        unreachable!("the derived reply schema is JSON: {}", entry.reply.as_str());
    };
    assert_eq!(
        reply_schema["title"], "AppPhaseReply",
        "the reply schema is derived from the command's own Reply type: {reply_schema}",
    );
    assert!(
        reply_schema["properties"]["phase"].is_object(),
        "the reply schema describes the nested phase record: {reply_schema}",
    );
    Ok(())
}

#[test]
fn running_app_phase_returns_the_five_level_tuple() -> TestResult {
    let reply = exchange(run(APP_PHASE, "{}", RunOptions::default()))?;
    let QaResponse::Outcome(CommandOutcome::Ran { reply, .. }) = reply else {
        unreachable!("a plain app.phase call must RUN, got {reply:?}");
    };
    let Ok(body) = serde_json::from_str::<Value>(reply.as_str()) else {
        unreachable!(
            "the reply body is the command's own JSON: {}",
            reply.as_str()
        );
    };
    let phase = &body["phase"];
    assert_eq!(
        phase["app"], "Running",
        "the harness rests the app in Running: {body}",
    );
    assert_eq!(
        phase["running"], "Menu",
        "entering Running enters the Menu screen, and the reply must report the LIVE \
         sub-state rather than an absent one: {body}",
    );
    for level in ["running", "game", "battlescape", "aftermath"] {
        assert!(
            phase.get(level).is_some(),
            "the nested level `{level}` is always PRESENT, null when not live: {body}",
        );
    }
    assert_eq!(
        phase["battlescape"],
        Value::Null,
        "no battle is running, so the battle phase is null: {body}",
    );
    Ok(())
}

#[test]
fn a_misspelled_name_is_unknown_and_lists_what_exists() -> TestResult {
    let reply = exchange(run("app.phasee", "{}", RunOptions::default()))?;
    let QaResponse::Outcome(CommandOutcome::Unknown { known }) = reply else {
        unreachable!("a name this host does not offer must be Unknown, got {reply:?}");
    };
    assert_eq!(known, vec![CommandName::from_static(APP_PHASE)]);
    Ok(())
}

#[test]
fn an_unknown_argument_is_bad_arguments_with_the_schema() -> TestResult {
    let mut replies = exchange_all(vec![
        QaRequest::Catalogue,
        run(APP_PHASE, r#"{"nope":1}"#, RunOptions::default()),
    ])?;
    let Some(outcome) = replies.pop() else {
        unreachable!("two requests yield two replies");
    };
    let Some(QaResponse::Catalogue(catalogue)) = replies.pop() else {
        unreachable!("the first reply is the catalogue");
    };
    let Some(entry) = catalogue.entries.first() else {
        unreachable!("a one-row catalogue has a first row");
    };

    let QaResponse::Outcome(CommandOutcome::BadArguments { detail, schema }) = outcome else {
        unreachable!("an argument the command does not declare must be refused, got {outcome:?}");
    };
    assert!(
        detail.as_str().contains("nope"),
        "the fault names the field that was wrong: {}",
        detail.as_str(),
    );
    assert_eq!(
        schema, entry.arguments,
        "the schema attached to a refusal is the SAME document the catalogue publishes",
    );
    Ok(())
}

#[test]
fn an_unbuilt_rider_is_refused_rather_than_dropped() -> TestResult {
    let mut replies = exchange_all(vec![
        run(APP_PHASE, "{}", RunOptions::default()),
        run(
            APP_PHASE,
            "{}",
            RunOptions::new(Some(AwaitBudget::new(5)), None),
        ),
    ])?;
    let Some(with_rider) = replies.pop() else {
        unreachable!("two requests yield two replies");
    };
    let Some(plain) = replies.pop() else {
        unreachable!("two requests yield two replies");
    };
    assert!(
        matches!(plain, QaResponse::Outcome(CommandOutcome::Ran { .. })),
        "the identical call WITHOUT the rider must run, or this case proves nothing about \
         the rider — got {plain:?}",
    );

    let QaResponse::Outcome(CommandOutcome::Unavailable { code, note }) = with_rider else {
        unreachable!("an await_ready rider must be refused, got {with_rider:?}");
    };
    assert_eq!(
        code,
        gdtf_qa_protocol::command::UnavailableCode::NotBuilt,
        "nothing about the app's state can make an unbuilt rider exist",
    );
    assert!(
        note.as_str().contains("await_ready"),
        "the refusal names the rider that is missing: {}",
        note.as_str(),
    );
    Ok(())
}
