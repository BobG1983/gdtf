//! Round-trip pins for every message variant (GTW-939, narrowed to the three-variant wire
//! by GTW-943).

use crate::{
    command::{
        ArgSchemaJson, ArgumentFault, ArtifactPath, AttachmentKind, AwaitBudget, CommandArgsJson,
        CommandAvailability, CommandCatalogue, CommandEntry, CommandName, CommandOutcome,
        CommandReplyJson, CommandSummary, CommandTiming, RefusalNote, ReplyAttachment,
        ReplySchemaJson, RunOptions, UnavailableCode,
    },
    message::{
        HelloFacts, ProtocolVersion, QaError, QaRequest, QaResponse, RunCommand, ServerNameNet,
    },
    test_support::assert_ron_round_trip,
};

/// A two-row catalogue whose schema fields hold JSON documents — the reply payload both
/// the response test and the outcome test lean on.
fn catalogue() -> CommandCatalogue {
    CommandCatalogue::new(
        ServerNameNet::new("gdtf-net-qa".to_owned()),
        vec![
            CommandEntry::new(
                CommandName::from_static("app.phase"),
                CommandSummary::from_static("Read the whole state tuple plus readiness."),
                CommandTiming::Immediate,
                ArgSchemaJson::new(
                    r#"{"type":"object","properties":{},"additionalProperties":false}"#.to_owned(),
                ),
                ReplySchemaJson::new(r#"{"type":"object","required":["app"]}"#.to_owned()),
                CommandAvailability::Available,
            ),
            CommandEntry::new(
                CommandName::from_static("act.fire"),
                CommandSummary::from_static("Fire the selected ganger's weapon at a cell."),
                CommandTiming::Deferred,
                ArgSchemaJson::new(r#"{"type":"object","required":["target"]}"#.to_owned()),
                ReplySchemaJson::new(r#"{"oneOf":[{"required":["Accepted"]}]}"#.to_owned()),
                CommandAvailability::Unavailable {
                    code: UnavailableCode::WrongState,
                    note: RefusalNote::from_static("the battle is still generating"),
                },
            ),
        ],
    )
}

/// Every [`QaRequest`] variant round-trips, `Run` carrying a real command name and JSON
/// arguments.
#[test]
fn the_requests_round_trip() {
    assert_ron_round_trip(&QaRequest::Hello(ProtocolVersion::CURRENT));
    assert_ron_round_trip(&QaRequest::Catalogue);
    assert_ron_round_trip(&QaRequest::Run(RunCommand::new(
        CommandName::from_static("act.fire"),
        CommandArgsJson::new(r#"{"target":{"cell":[7,3],"level":0},"mode":1}"#.to_owned()),
    )));
    assert_ron_round_trip(&QaRequest::Run(RunCommand::new(
        CommandName::from_owned("app.phase".to_owned()),
        CommandArgsJson::new("{}".to_owned()),
    )));
}

/// A `Run` CARRYING its riders round-trips, and a plain one leaves them absent.
///
/// The riders were typed by GTW-939 and the host-side refusal built by GTW-941, but until
/// GTW-942 no wire field carried the value from a client to a host, so a caller asking for
/// `await_ready` was answered as though it had asked for nothing.
#[test]
fn a_run_carries_its_riders_over_the_wire() {
    let plain = RunCommand::new(
        CommandName::from_static("app.phase"),
        CommandArgsJson::new("{}".to_owned()),
    );
    assert!(
        plain.options.is_plain(),
        "the two-argument constructor asks for no machinery",
    );
    assert_ron_round_trip(&QaRequest::Run(plain));

    let with_budget = RunCommand::with_options(
        CommandName::from_static("app.phase"),
        CommandArgsJson::new("{}".to_owned()),
        RunOptions::new(Some(AwaitBudget::new(5)), None),
    );
    assert_eq!(
        with_budget.options.await_ready,
        Some(AwaitBudget::new(5)),
        "the await budget a caller sent must survive to the host",
    );
    assert_ron_round_trip(&QaRequest::Run(with_budget));
}

/// A `Run` encoded WITHOUT an `options` field still decodes, with both riders absent.
///
/// The compatibility claim the `#[serde(default)]` on that field makes: this is the exact
/// text a pre-GTW-942 client put on the wire.
#[test]
fn a_run_encoded_without_options_decodes_as_a_plain_call() {
    let legacy = r#"Run((command:"app.phase",arguments:"{}"))"#;
    let Ok(decoded) = ron::de::from_str::<QaRequest>(legacy) else {
        unreachable!("a Run without `options` must still decode: {legacy}");
    };
    let QaRequest::Run(run) = decoded else {
        unreachable!("the decoded request is a Run");
    };
    assert!(
        run.options.is_plain(),
        "a Run with no options on the wire asks for no machinery",
    );
}

/// Every [`QaResponse`] variant round-trips — the handshake facts, a populated catalogue,
/// every outcome shape a `Run` can answer with, and an error.
#[test]
fn the_responses_round_trip() {
    assert_ron_round_trip(&QaResponse::HelloOk(HelloFacts::new(
        ProtocolVersion::CURRENT,
        ServerNameNet::new("gdtf-net-qa".to_owned()),
    )));
    assert_ron_round_trip(&QaResponse::Catalogue(catalogue()));
    for outcome in [
        CommandOutcome::Ran {
            reply:       CommandReplyJson::new(r#"{"Accepted":{"seq":412}}"#.to_owned()),
            attachments: vec![ReplyAttachment::new(
                AttachmentKind::Png,
                ArtifactPath::new("target/qa_screenshots/after_fire.png".to_owned()),
            )],
        },
        CommandOutcome::Unavailable {
            code: UnavailableCode::NotBuilt,
            note: RefusalNote::from_static("the await_ready rider is not built yet"),
        },
        CommandOutcome::BadArguments {
            detail: ArgumentFault::new("missing field `mode` at line 1 column 41".to_owned()),
            schema: ArgSchemaJson::new(r#"{"type":"object","required":["mode"]}"#.to_owned()),
        },
        CommandOutcome::Unknown {
            known: vec![CommandName::from_static("app.phase")],
        },
    ] {
        assert_ron_round_trip(&QaResponse::Outcome(outcome));
    }
    assert_ron_round_trip(&QaResponse::Error(QaError::Timeout));
}
