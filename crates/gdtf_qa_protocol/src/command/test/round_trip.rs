use crate::{
    command::{
        ArgSchemaJson, ArgumentFault, ArtifactPath, AttachmentKind, AwaitBudget, CaptureRider,
        CommandArgsJson, CommandAvailability, CommandCatalogue, CommandEntry, CommandName,
        CommandOutcome, CommandReplyJson, CommandSummary, CommandTiming, RefusalNote,
        ReplyAttachment, ReplySchemaJson, RunOptions, UnavailableCode,
    },
    ids::ShotName,
    message::ServerNameNet,
    test_support::assert_ron_round_trip,
};

const DERIVED_ARG_SCHEMA: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"ShotName","description":"A requested capture's **file stem** — the caller-chosen name a\n[`CaptureRider`](crate::command::CaptureRider) writes under.\n\nThe host constrains the actual path under its own capture directory; this is\nonly the stem the client asks for. A name newtype over `String` (no-bare-types),\nserde-transparent. `Clone`-not-`Copy` (holds a `String`).","type":"string"}"#;

const DERIVED_REPLY_SCHEMA: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"LevelNet","description":"A 0-based **storey** index — which floor of the coarse grid, valid `0..MAX_LEVELS`\n— the wire mirror of the sim `Level`.\n\nA private-inner newtype (no-bare-types), serde-transparent so it rides the wire as\nits bare `u8`. The contract does not re-encode the `MAX_LEVELS` bound here; the game\nside validates against the live grid extent.","type":"integer","format":"uint8","maximum":255,"minimum":0}"#;

#[test]
fn names_and_summaries_round_trip() {
    assert_ron_round_trip(&CommandName::from_static("app.phase"));
    assert_ron_round_trip(&CommandName::from_owned("act.fire".to_owned()));
    assert_ron_round_trip(&CommandSummary::from_static("Read the whole state tuple."));
    assert_ron_round_trip(&CommandSummary::from_owned("Fire the weapon.".to_owned()));
}

#[test]
fn a_static_and_an_owned_name_are_interchangeable() {
    let from_const = CommandName::from_static("app.phase");
    let decoded = CommandName::from_owned("app.phase".to_owned());
    assert_eq!(from_const, decoded);
    let (Ok(left), Ok(right)) = (
        ron::ser::to_string(&from_const),
        ron::ser::to_string(&decoded),
    ) else {
        unreachable!("a command name serializes to compact RON");
    };
    assert_eq!(left, right, "a name rides the wire as a plain string");
    assert_eq!(left, "\"app.phase\"");
}

#[test]
fn json_payload_newtypes_round_trip() {
    assert_ron_round_trip(&CommandArgsJson::new(r#"{"mode":1}"#.to_owned()));
    assert_ron_round_trip(&CommandReplyJson::new(
        r#"{"Accepted":{"seq":412}}"#.to_owned(),
    ));
    assert_ron_round_trip(&ArgumentFault::new(
        "missing field `mode` at line 1 column 41".to_owned(),
    ));
    assert_ron_round_trip(&ArgSchemaJson::new(DERIVED_ARG_SCHEMA.to_owned()));
    assert_ron_round_trip(&ReplySchemaJson::new(DERIVED_REPLY_SCHEMA.to_owned()));
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
            note: RefusalNote::from_static("needs a live battle"),
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
            ArtifactPath::new("target/qa_screenshots/after_fire.png".to_owned()),
        ));
    }
}

#[test]
fn command_outcome_round_trips_every_variant() {
    let cases = vec![
        CommandOutcome::Ran {
            reply:       CommandReplyJson::new(r#"{"Accepted":{"seq":412}}"#.to_owned()),
            attachments: Vec::new(),
        },
        CommandOutcome::Ran {
            reply:       CommandReplyJson::new(
                r#"{"saved":"target/qa_screenshots/after_fire.png"}"#.to_owned(),
            ),
            attachments: vec![ReplyAttachment::new(
                AttachmentKind::Png,
                ArtifactPath::new("target/qa_screenshots/after_fire.png".to_owned()),
            )],
        },
        CommandOutcome::Unavailable {
            code: UnavailableCode::WrongState,
            note: RefusalNote::from_static("needs BattleRunning; the battle is generating"),
        },
        CommandOutcome::BadArguments {
            detail: ArgumentFault::new("missing field `mode` at line 1 column 41".to_owned()),
            schema: ArgSchemaJson::new(DERIVED_ARG_SCHEMA.to_owned()),
        },
        CommandOutcome::Unknown {
            known: vec![
                CommandName::from_static("app.phase"),
                CommandName::from_static("act.fire"),
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
            "after_fire".to_owned(),
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
fn derived_schema_text_survives_the_round_trip_unchanged() {
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
        DERIVED_ARG_SCHEMA,
        "the derived argument schema text is unchanged",
    );
    assert_eq!(
        first.reply.as_str(),
        DERIVED_REPLY_SCHEMA,
        "the derived reply schema text is unchanged",
    );
    assert_eq!(decoded, catalogue);
}

fn populated_catalogue() -> CommandCatalogue {
    CommandCatalogue::new(
        ServerNameNet::new("gdtf-net-qa".to_owned()),
        vec![
            CommandEntry::new(
                CommandName::from_static("app.phase"),
                CommandSummary::from_static("Read the whole state tuple plus readiness."),
                CommandTiming::Immediate,
                ArgSchemaJson::new(DERIVED_ARG_SCHEMA.to_owned()),
                ReplySchemaJson::new(DERIVED_REPLY_SCHEMA.to_owned()),
                CommandAvailability::Available,
            ),
            CommandEntry::new(
                CommandName::from_static("act.fire"),
                CommandSummary::from_static("Fire the selected ganger's weapon at a cell."),
                CommandTiming::Deferred,
                ArgSchemaJson::new(DERIVED_ARG_SCHEMA.to_owned()),
                ReplySchemaJson::new(DERIVED_REPLY_SCHEMA.to_owned()),
                CommandAvailability::Unavailable {
                    code: UnavailableCode::WrongState,
                    note: RefusalNote::from_static("the battle is still generating"),
                },
            ),
        ],
    )
}
