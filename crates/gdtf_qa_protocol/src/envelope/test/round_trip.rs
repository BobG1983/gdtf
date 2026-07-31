//! Round-trip pins for the COMMAND layer's envelope variants (GTW-939).
//!
//! The sibling `request` / `response` / `small` suites cover the variants the envelope
//! carried before the command layer. This one covers what GTW-939 added beside them —
//! [`QaRequest::Catalogue`] / [`QaRequest::Run`], [`QaResponse::Catalogue`] /
//! [`QaResponse::Outcome`], and the two new [`QaError`] variants — plus the frozen version
//! those additions moved to.

use crate::{
    command::{
        ArgSchemaJson, ArgumentFault, ArtifactPath, AttachmentKind, CommandArgsJson,
        CommandAvailability, CommandCatalogue, CommandEntry, CommandName, CommandOutcome,
        CommandReplyJson, CommandSummary, RefusalNote, ReplyAttachment, ReplySchemaJson,
        UnavailableCode,
    },
    envelope::{ProtocolVersion, QaError, QaRequest, QaResponse, RunCommand, ServerNameNet},
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
                ArgSchemaJson::new(
                    r#"{"type":"object","properties":{},"additionalProperties":false}"#.to_owned(),
                ),
                ReplySchemaJson::new(r#"{"type":"object","required":["app"]}"#.to_owned()),
                CommandAvailability::Available,
            ),
            CommandEntry::new(
                CommandName::from_static("act.fire"),
                CommandSummary::from_static("Fire the selected ganger's weapon at a cell."),
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

/// Both new [`QaRequest`] variants round-trip, `Run` carrying a real command name and JSON
/// arguments.
#[test]
fn the_command_requests_round_trip() {
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

/// Both new [`QaResponse`] variants round-trip — a populated catalogue and every outcome
/// shape a `Run` can answer with.
#[test]
fn the_command_responses_round_trip() {
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
}

/// Both new [`QaError`] variants round-trip, on their own and inside a
/// [`QaResponse::Error`].
#[test]
fn the_command_layer_errors_round_trip() {
    for error in [QaError::Malformed, QaError::NotNegotiated] {
        assert_ron_round_trip(&error);
        assert_ron_round_trip(&QaResponse::Error(error));
    }
}

/// The version the command layer's envelope landed at.
///
/// It moved because the request, response and error enums each grew closed-enum variants a
/// version-12 client cannot decode. It must NOT move again for a command: which commands a
/// host offers is data inside [`QaResponse::Catalogue`], not a wire shape.
#[test]
fn the_command_layer_landed_at_protocol_version_13() {
    assert_eq!(
        *ProtocolVersion::CURRENT,
        13,
        "the command-layer envelope is protocol version 13",
    );
}
