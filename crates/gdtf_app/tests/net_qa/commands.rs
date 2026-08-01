//! GTW-942 bullets 1–6: the GAME's command layer, over its REAL loopback listener, its REAL
//! router, and its REAL `GAME_COMMANDS` set.
//!
//! Every case here drives the whole path a live QA client drives — a real `TcpStream`, the
//! real framing codec, the real listener, the real `route_requests` drain, the real `admit`
//! scan over the host's own slice, the real decode step and the real `app.phase` handler.
//! Nothing is stubbed; the fixtures in [`socket_support`](super::socket_support) and
//! [`command_exchange`](super::command_exchange) own only the client half.
//!
//! This is the GAME half of the ticket's evidence. The COURIER half — what a client receives
//! for each of these outcomes — is `bins/gdtf_qa_mcp/tests/jsonrpc/courier_tools.rs`, which
//! cannot prove derivation because that crate links no `schemars`. Between them the six
//! bullets are covered end to end without a live MCP call, which could not exist until this
//! ticket lands anyway.

use gdtf_app::test_support::NET_QA_SERVER_NAME;
use gdtf_qa_protocol::{
    command::{
        AwaitBudget, CommandAvailability, CommandName, CommandOutcome, CommandTiming, RunOptions,
    },
    envelope::{QaRequest, QaResponse},
};
use serde_json::Value;

use super::{
    command_exchange::{APP_PHASE, exchange, exchange_all, run},
    socket_support::TestResult,
};

/// **Bullet 1.** `Catalogue` returns exactly one entry: name `app.phase`, `timing: Immediate`,
/// `availability: Available` — under the game's own host name.
///
/// Availability is `Available` because the app is resting in `Running` with no battle — and
/// it would be `Available` anywhere, which is the point of `app.phase` being the first
/// command: a client can call it the instant the process boots, to find out where it is.
///
/// The host name is checked against the same `SERVER_NAME` the handshake reports (exported as
/// `NET_QA_SERVER_NAME`), so `game_host_name` restating a name — or restating the editor's —
/// fails here instead of shipping a catalogue a client cannot match to the host it reached.
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

/// **Bullet 2.** The catalogue row carries the DERIVED schemas of `AppPhaseArgs` /
/// `AppPhaseReply`, and the argument schema carries `"additionalProperties": false`.
///
/// The strictness is what makes bullet 5 possible at all: without `deny_unknown_fields` an
/// unexpected key would be silently ignored rather than answered `BadArguments`, and the
/// published schema would not say so. Both are checked here, on the same document, so the
/// advertisement and the behaviour cannot drift.
///
/// The reply schema is checked for the nested `phase` record rather than compared to a
/// literal: pinning derived schema TEXT would break on any `schemars` release without a
/// single behaviour changing.
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

/// **Bullet 3.** `Run(app.phase, {})` returns the five-level state tuple.
///
/// The app rests in `Running` at its menu, so the top two levels are live and the lower three
/// are absent — and absent means an explicit `null`, not a missing key, so a client can tell
/// "no battle is running" from "I could not read the battle phase".
///
/// Both live levels are asserted by VALUE, against what the harness really enters (entering
/// `AppState::Running` enters `RunningState::Menu`, pinned in
/// `crates/gdtf_test_utils/src/minimal_harness/builder/test.rs`). Asserting only that the key
/// is present would pass with `read.rs` handing back `None` for every sub-state, which is the
/// gap `app.phase` exists to close. The three deeper levels are asserted live in
/// [`app_phase_depth`](super::app_phase_depth), where the app is in a battle.
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

/// **Bullet 4.** `Run(app.phasee, {})` returns `Unknown` listing the one known name.
///
/// The known names ride along so a typo self-corrects in ONE round trip, and they come from
/// the host's own slice rather than a list written beside the router — so a command added to
/// `GAME_COMMANDS` appears here with no other edit.
#[test]
fn a_misspelled_name_is_unknown_and_lists_what_exists() -> TestResult {
    let reply = exchange(run("app.phasee", "{}", RunOptions::default()))?;
    let QaResponse::Outcome(CommandOutcome::Unknown { known }) = reply else {
        unreachable!("a name this host does not offer must be Unknown, got {reply:?}");
    };
    assert_eq!(known, vec![CommandName::from_static(APP_PHASE)]);
    Ok(())
}

/// **Bullet 5.** `Run(app.phase, {"nope":1})` returns `BadArguments` with the schema attached.
///
/// The attached schema is the command's OWN derived document — the same one the catalogue
/// publishes — so the app cannot describe a shape it does not accept, and the caller needs no
/// second round trip to find out what it should have sent.
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

/// **Bullet 6.** `Run(app.phase, {}, await_ready=5)` returns `Unavailable(NotBuilt)` — the
/// rider stub, refusing rather than running the command with the rider dropped.
///
/// It proves two things at once. The plumbing REFUSES an unbuilt rider (`gdtf_qa_command`'s
/// `rider_refusal`), and the rider actually TRAVELS: the identical call without it runs, so
/// this outcome can only come from `await_ready` reaching the host. Before GTW-942 added the
/// `options` field to `RunCommand`, it could not — the type and the refusal both existed and
/// no wire field carried the value between them.
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
