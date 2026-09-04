use crate::{
    command::{
        ArgSchemaRon, ArgumentFault, ArtifactPath, AttachmentKind, AwaitBudget, CommandArgsRon,
        CommandAvailability, CommandCatalogue, CommandEntry, CommandName, CommandOutcome,
        CommandReplyRon, CommandSummary, CommandTiming, RefusalNote, ReplyAttachment,
        ReplySchemaRon, RunOptions, UnavailableCode,
    },
    message::{
        HelloFacts, ProtocolVersion, QaError, QaRequest, QaResponse, RunCommand, ServerNameNet,
    },
    test_support::assert_ron_round_trip,
};

fn catalogue() -> CommandCatalogue {
    CommandCatalogue::new(
        ServerNameNet::new("host-under-test".to_owned()),
        vec![
            CommandEntry::new(
                CommandName::from_static("app.phase"),
                CommandSummary::from_static("Read the whole state tuple plus readiness."),
                CommandTiming::Immediate,
                ArgSchemaRon::new(
                    r#"(root:Named("AppPhaseArgs"),defs:[("AppPhaseArgs",Record([]))])"#.to_owned(),
                ),
                ReplySchemaRon::new(
                    r#"(root:Named("AppPhaseReply"),defs:[("AppPhaseReply",Record([("app",Text)]))])"#
                        .to_owned(),
                ),
                CommandAvailability::Available,
            ),
            CommandEntry::new(
                CommandName::from_static("probe.trace"),
                CommandSummary::from_static("Run the deferred command against the host."),
                CommandTiming::Deferred,
                ArgSchemaRon::new(
                    r#"(root:Named("TraceArgs"),defs:[("TraceArgs",Record([("at",Int)]))])"#
                        .to_owned(),
                ),
                ReplySchemaRon::new(
                    r#"(root:Named("TraceReply"),defs:[("TraceReply",Choice([("Accepted",Unit)]))])"#
                        .to_owned(),
                ),
                CommandAvailability::Unavailable {
                    code: UnavailableCode::WrongState,
                    note: RefusalNote::from_static("the host is still starting"),
                },
            ),
        ],
    )
}

#[test]
fn the_requests_round_trip() {
    assert_ron_round_trip(&QaRequest::Hello(ProtocolVersion::CURRENT));
    assert_ron_round_trip(&QaRequest::Catalogue);
    assert_ron_round_trip(&QaRequest::Run(RunCommand::new(
        CommandName::from_static("probe.trace"),
        CommandArgsRon::new("(at:(first:7,second:3),mode:1)".to_owned()),
    )));
    assert_ron_round_trip(&QaRequest::Run(RunCommand::new(
        CommandName::from_owned("app.phase".to_owned()),
        CommandArgsRon::new("()".to_owned()),
    )));
}

#[test]
fn a_run_carries_its_riders_over_the_wire() {
    let plain = RunCommand::new(
        CommandName::from_static("app.phase"),
        CommandArgsRon::new("()".to_owned()),
    );
    assert!(
        plain.options.is_plain(),
        "the two-argument constructor asks for no machinery",
    );
    assert_ron_round_trip(&QaRequest::Run(plain));

    let with_budget = RunCommand::with_options(
        CommandName::from_static("app.phase"),
        CommandArgsRon::new("()".to_owned()),
        RunOptions::new(Some(AwaitBudget::new(5)), None),
    );
    assert_eq!(
        with_budget.options.await_ready,
        Some(AwaitBudget::new(5)),
        "the await budget a caller sent must survive to the host",
    );
    assert_ron_round_trip(&QaRequest::Run(with_budget));
}

/// The compatibility claim the `#[serde(default)]` on that field makes: this is the exact
#[test]
fn a_run_encoded_without_options_decodes_as_a_plain_call() {
    let legacy = r#"Run((command:"app.phase",arguments:"()"))"#;
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

#[test]
fn the_responses_round_trip() {
    assert_ron_round_trip(&QaResponse::HelloOk(HelloFacts::new(
        ProtocolVersion::CURRENT,
        ServerNameNet::new("host-under-test".to_owned()),
    )));
    assert_ron_round_trip(&QaResponse::Catalogue(catalogue()));
    for outcome in [
        CommandOutcome::Ran {
            reply:       CommandReplyRon::new("Accepted((seq:412))".to_owned()),
            attachments: vec![ReplyAttachment::new(
                AttachmentKind::Png,
                ArtifactPath::new("target/qa_screenshots/probe_shot.png".to_owned()),
            )],
        },
        CommandOutcome::Unavailable {
            code: UnavailableCode::NotBuilt,
            note: RefusalNote::from_static("the await_ready rider is not built yet"),
        },
        CommandOutcome::BadArguments {
            detail: ArgumentFault::new("missing field `mode` at line 1 column 41".to_owned()),
            schema: ArgSchemaRon::new(
                r#"(root:Named("TraceArgs"),defs:[("TraceArgs",Record([("mode",Int)]))])"#
                    .to_owned(),
            ),
        },
        CommandOutcome::Unknown {
            known: vec![CommandName::from_static("app.phase")],
        },
    ] {
        assert_ron_round_trip(&QaResponse::Outcome(outcome));
    }
    assert_ron_round_trip(&QaResponse::Error(QaError::Timeout));
}
