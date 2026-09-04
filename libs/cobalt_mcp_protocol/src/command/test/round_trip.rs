use crate::{
    command::{
        ArgSchemaRon, ArgumentFault, ArtifactPath, AttachmentKind, AwaitBudget, CaptureRider,
        CommandArgsRon, CommandAvailability, CommandCatalogue, CommandEntry, CommandName,
        CommandOutcome, CommandReplyRon, CommandSummary, CommandTiming, RefusalNote,
        ReplyAttachment, ReplySchemaRon, RunOptions, UnavailableCode,
    },
    ids::ShotName,
    message::ServerNameNet,
    test_support::assert_ron_round_trip,
};

const TRACED_ARG_SHAPE: &str = r#"(root:Named("TraceArgs"),defs:[("PairNet",Record([("first",Int),("second",Int)])),("TraceArgs",Record([("at",Named("PairNet")),("mode",Int)]))])"#;

const TRACED_REPLY_SHAPE: &str =
    r#"(root:Named("TraceReply"),defs:[("TraceReply",Record([("seq",Int)]))])"#;

#[test]
fn names_and_summaries_round_trip() {
    assert_ron_round_trip(&CommandName::from_static("sample.status"));
    assert_ron_round_trip(&CommandName::from_owned("probe.trace".to_owned()));
    assert_ron_round_trip(&CommandSummary::from_static("Read the whole state tuple."));
    assert_ron_round_trip(&CommandSummary::from_owned(
        "Run the deferred command.".to_owned(),
    ));
}

#[test]
fn a_static_and_an_owned_name_are_interchangeable() {
    let from_const = CommandName::from_static("sample.status");
    let decoded = CommandName::from_owned("sample.status".to_owned());
    assert_eq!(from_const, decoded);
    let (Ok(left), Ok(right)) = (
        ron::ser::to_string(&from_const),
        ron::ser::to_string(&decoded),
    ) else {
        unreachable!("a command name serializes to compact RON");
    };
    assert_eq!(left, right, "a name rides the wire as a plain string");
    assert_eq!(left, "\"sample.status\"");
}

#[test]
fn ron_payload_newtypes_round_trip() {
    assert_ron_round_trip(&CommandArgsRon::new("(mode:1)".to_owned()));
    assert_ron_round_trip(&CommandReplyRon::new("Accepted((seq:412))".to_owned()));
    assert_ron_round_trip(&ArgumentFault::new(
        "missing field `mode` at line 1 column 41".to_owned(),
    ));
    assert_ron_round_trip(&ArgSchemaRon::new(TRACED_ARG_SHAPE.to_owned()));
    assert_ron_round_trip(&ReplySchemaRon::new(TRACED_REPLY_SHAPE.to_owned()));
}

#[test]
fn unavailable_code_round_trips_every_variant() {
    assert_eq!(
        UnavailableCode::ALL.len(),
        4,
        "UnavailableCode::ALL lists every refusal class",
    );
    for code in UnavailableCode::ALL {
        match code {
            UnavailableCode::WrongState
            | UnavailableCode::Replaying
            | UnavailableCode::MissingModel
            | UnavailableCode::NotBuilt => {}
        }
        assert_ron_round_trip(&code);
    }
}

#[test]
fn availability_round_trips_every_variant() {
    let mut cases = vec![CommandAvailability::Available];
    for code in UnavailableCode::ALL {
        cases.push(CommandAvailability::Unavailable {
            code,
            note: RefusalNote::from_static("needs a live session"),
        });
    }
    cases.push(CommandAvailability::Unavailable {
        code: UnavailableCode::Replaying,
        note: RefusalNote::from_owned("the screen is still replaying".to_owned()),
    });
    for case in &cases {
        match case {
            CommandAvailability::Available | CommandAvailability::Unavailable { .. } => {}
        }
        assert_ron_round_trip(case);
    }
}

#[test]
fn attachments_round_trip_every_kind() {
    assert_eq!(
        AttachmentKind::ALL.len(),
        1,
        "AttachmentKind::ALL lists every attachment kind",
    );
    for kind in AttachmentKind::ALL {
        match kind {
            AttachmentKind::Png => {}
        }
        assert_ron_round_trip(&kind);
        assert_ron_round_trip(&ReplyAttachment::new(
            kind,
            ArtifactPath::new("target/qa_screenshots/probe_shot.png".to_owned()),
        ));
    }
}

#[test]
fn command_outcome_round_trips_every_variant() {
    let cases = vec![
        CommandOutcome::Ran {
            reply:       CommandReplyRon::new("Accepted((seq:412))".to_owned()),
            attachments: Vec::new(),
        },
        CommandOutcome::Ran {
            reply:       CommandReplyRon::new(
                r#"(saved:"target/qa_screenshots/probe_shot.png")"#.to_owned(),
            ),
            attachments: vec![ReplyAttachment::new(
                AttachmentKind::Png,
                ArtifactPath::new("target/qa_screenshots/probe_shot.png".to_owned()),
            )],
        },
        CommandOutcome::Unavailable {
            code: UnavailableCode::WrongState,
            note: RefusalNote::from_static("needs the running state; the host is still starting"),
        },
        CommandOutcome::BadArguments {
            detail: ArgumentFault::new("missing field `mode` at line 1 column 41".to_owned()),
            schema: ArgSchemaRon::new(TRACED_ARG_SHAPE.to_owned()),
        },
        CommandOutcome::Unknown {
            known: vec![
                CommandName::from_static("sample.status"),
                CommandName::from_static("probe.trace"),
            ],
        },
    ];
    for case in &cases {
        match case {
            CommandOutcome::Ran { .. }
            | CommandOutcome::Unavailable { .. }
            | CommandOutcome::BadArguments { .. }
            | CommandOutcome::Unknown { .. } => {}
        }
        assert_ron_round_trip(case);
    }
}

#[test]
fn run_options_round_trip() {
    let plain = RunOptions::default();
    assert!(plain.is_plain(), "the default rider set asks for nothing");
    assert_ron_round_trip(&plain);

    let loaded = RunOptions::new(
        Some(AwaitBudget::new(5)),
        Some(CaptureRider::new(Some(ShotName::new(
            "probe_shot".to_owned(),
        )))),
    );
    assert!(!loaded.is_plain(), "a populated rider set is not plain");
    assert_ron_round_trip(&loaded);

    let await_only = RunOptions::new(Some(AwaitBudget::new(0)), None);
    assert!(
        !await_only.is_plain(),
        "an await budget alone is not a plain call"
    );
    assert_ron_round_trip(&await_only);

    let capture_only = RunOptions::new(None, Some(CaptureRider::new(None)));
    assert!(
        !capture_only.is_plain(),
        "a capture rider alone is not a plain call"
    );
    assert_ron_round_trip(&capture_only);
}

#[test]
fn a_populated_catalogue_round_trips() {
    assert_ron_round_trip(&populated_catalogue());
}

#[test]
fn traced_shape_text_survives_the_round_trip_unchanged() {
    let catalogue = populated_catalogue();
    let Ok(encoded) = ron::ser::to_string(&catalogue) else {
        unreachable!("a catalogue serializes to compact RON");
    };
    let Ok(decoded) = ron::de::from_str::<CommandCatalogue>(&encoded) else {
        unreachable!("the compact RON `{encoded}` parses back to a CommandCatalogue");
    };
    let Some(first) = decoded.entries.first() else {
        unreachable!("the decoded catalogue keeps its rows");
    };
    assert_eq!(
        first.arguments.as_str(),
        TRACED_ARG_SHAPE,
        "the traced argument shape text is unchanged",
    );
    assert_eq!(
        first.reply.as_str(),
        TRACED_REPLY_SHAPE,
        "the traced reply shape text is unchanged",
    );
    assert_eq!(decoded, catalogue);
}

fn populated_catalogue() -> CommandCatalogue {
    CommandCatalogue::new(
        ServerNameNet::new("host-under-test".to_owned()),
        vec![
            CommandEntry::new(
                CommandName::from_static("sample.status"),
                CommandSummary::from_static("Read the whole state tuple plus readiness."),
                CommandTiming::Immediate,
                ArgSchemaRon::new(TRACED_ARG_SHAPE.to_owned()),
                ReplySchemaRon::new(TRACED_REPLY_SHAPE.to_owned()),
                CommandAvailability::Available,
            ),
            CommandEntry::new(
                CommandName::from_static("probe.trace"),
                CommandSummary::from_static("Run the deferred command against the host."),
                CommandTiming::Deferred,
                ArgSchemaRon::new(TRACED_ARG_SHAPE.to_owned()),
                ReplySchemaRon::new(TRACED_REPLY_SHAPE.to_owned()),
                CommandAvailability::Unavailable {
                    code: UnavailableCode::WrongState,
                    note: RefusalNote::from_static("the host is still starting"),
                },
            ),
        ],
    )
}
