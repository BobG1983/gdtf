//! The freeze: the wire's version number and the exact shape of its three enums
//! [`ProtocolVersion::CURRENT`](crate::message::ProtocolVersion::CURRENT) or any enum
use crate::{
    command::{CommandArgsRon, CommandCatalogue, CommandName, CommandOutcome},
    message::{
        HelloFacts, ProtocolVersion, QaError, QaRequest, QaResponse, RunCommand, ServerNameNet,
    },
};

#[test]
fn the_wire_stands_at_protocol_version_15() {
    assert_eq!(
        *ProtocolVersion::CURRENT,
        15,
        "the command-layer wire is protocol version 15",
    );
}

#[test]
fn the_request_enum_has_exactly_three_variants() {
    let every = [
        QaRequest::Hello(ProtocolVersion::CURRENT),
        QaRequest::Catalogue,
        QaRequest::Run(RunCommand::new(
            CommandName::from_static("app.phase"),
            CommandArgsRon::new("()".to_owned()),
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
