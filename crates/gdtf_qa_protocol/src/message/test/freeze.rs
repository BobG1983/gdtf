//! The freeze: the wire's version number and the exact shape of its three enums
//! (GTW-943).
//!
//! **If you are reading this test because you had to edit it, you have left the command
//! layer and are changing the wire.** Adding a command to a host — the game's or the
//! editor's — must never bring you here: a command is a row in a
//! [`CommandCatalogue`](crate::command::CommandCatalogue) and a name inside a
//! [`Run`](QaRequest::Run), so a host can grow from one command to sixty without moving
//! [`ProtocolVersion::CURRENT`](crate::message::ProtocolVersion::CURRENT) or any enum
//! below. If this file fails, stop and ask whether the change really belongs on the wire;
//! if it does, bump the version in the same edit, because both hosts and the courier
//! negotiate on exact equality and a stale peer must be refused rather than left to
//! mis-decode every later reply.
//!
//! Each test below is an exhaustive `match` with no wildcard arm, so a new variant is a
//! COMPILE error here rather than a silently-accepted wire change.

use crate::{
    command::{CommandArgsJson, CommandCatalogue, CommandName, CommandOutcome},
    message::{
        HelloFacts, ProtocolVersion, QaError, QaRequest, QaResponse, RunCommand, ServerNameNet,
    },
};

/// The version this build of the wire speaks.
///
/// It reached `14` when [`RunCommand`] gained `options` and
/// [`CommandEntry`](crate::command::CommandEntry) gained `timing` (GTW-942) — a field added
/// to a shipped shape moves the number, exactly as the 1 → 2 `available` bump did. GTW-943
/// then DELETED the whole pre-command vocabulary, which is as breaking as an addition, and
/// the number stays at `14`: nothing negotiated `14` before this landing, so no peer can be
/// holding the shape that was cut.
#[test]
fn the_wire_stands_at_protocol_version_14() {
    assert_eq!(
        *ProtocolVersion::CURRENT,
        14,
        "the command-layer wire is protocol version 14",
    );
}

/// [`QaRequest`] has exactly THREE variants.
///
/// The `match` is the pin: a fourth arm fails to compile here. Adding a COMMAND must never
/// require one.
#[test]
fn the_request_enum_has_exactly_three_variants() {
    let every = [
        QaRequest::Hello(ProtocolVersion::CURRENT),
        QaRequest::Catalogue,
        QaRequest::Run(RunCommand::new(
            CommandName::from_static("app.phase"),
            CommandArgsJson::new("{}".to_owned()),
        )),
    ];
    let mut seen = 0_usize;
    for request in &every {
        match request {
            QaRequest::Hello(_) | QaRequest::Catalogue | QaRequest::Run(_) => seen += 1,
        }
    }
    assert_eq!(seen, 3, "the request wire is Hello, Catalogue and Run");
}

/// [`QaResponse`] has exactly FOUR variants.
///
/// What a command replies with is JSON inside
/// [`Outcome`](QaResponse::Outcome), never a variant here.
#[test]
fn the_response_enum_has_exactly_four_variants() {
    let every = [
        QaResponse::HelloOk(HelloFacts::new(
            ProtocolVersion::CURRENT,
            ServerNameNet::new("gdtf-net-qa".to_owned()),
        )),
        QaResponse::Catalogue(CommandCatalogue::new(
            ServerNameNet::new("gdtf-net-qa".to_owned()),
            Vec::new(),
        )),
        QaResponse::Outcome(CommandOutcome::Unknown { known: Vec::new() }),
        QaResponse::Error(QaError::Malformed),
    ];
    let mut seen = 0_usize;
    for response in &every {
        match response {
            QaResponse::HelloOk(_)
            | QaResponse::Catalogue(_)
            | QaResponse::Outcome(_)
            | QaResponse::Error(_) => seen += 1,
        }
    }
    assert_eq!(
        seen, 4,
        "the response wire is HelloOk, Catalogue, Outcome and Error",
    );
}

/// [`QaError`] has exactly FIVE variants, and every one round-trips.
#[test]
fn the_error_enum_has_exactly_five_variants() {
    let every = [
        QaError::Malformed,
        QaError::NotNegotiated,
        QaError::VersionMismatch,
        QaError::Busy,
        QaError::Timeout,
    ];
    for error in every {
        match error {
            QaError::Malformed
            | QaError::NotNegotiated
            | QaError::VersionMismatch
            | QaError::Busy
            | QaError::Timeout => {}
        }
        crate::test_support::assert_ron_round_trip(&error);
    }
    assert_eq!(every.len(), 5, "the error wire is five values");
}
