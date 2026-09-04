//! The freeze: the wire's version number and the exact shape of its three enums
//! [`ProtocolVersion::CURRENT`](crate::message::ProtocolVersion::CURRENT) or any enum
use crate::{
    command::{CommandArgsRon, CommandCatalogue, CommandName, CommandOutcome},
    message::{
        HelloFacts, McpRequest, McpResponse, McpSessionError, ProtocolVersion, RunCommand,
        ServerNameNet,
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
        McpRequest::Hello(ProtocolVersion::CURRENT),
        McpRequest::Catalogue,
        McpRequest::Run(RunCommand::new(
            CommandName::from_static("app.phase"),
            CommandArgsRon::new("()".to_owned()),
        )),
    ];
    let mut seen = 0_usize;
    for request in &every {
        match request {
            McpRequest::Hello(_) | McpRequest::Catalogue | McpRequest::Run(_) => seen += 1,
        }
    }
    assert_eq!(seen, 3, "the request wire is Hello, Catalogue and Run");
}

#[test]
fn the_response_enum_has_exactly_four_variants() {
    let every = [
        McpResponse::HelloOk(HelloFacts::new(
            ProtocolVersion::CURRENT,
            ServerNameNet::new("host-under-test".to_owned()),
        )),
        McpResponse::Catalogue(CommandCatalogue::new(
            ServerNameNet::new("host-under-test".to_owned()),
            Vec::new(),
        )),
        McpResponse::Outcome(CommandOutcome::Unknown { known: Vec::new() }),
        McpResponse::Error(McpSessionError::Malformed),
    ];
    let mut seen = 0_usize;
    for response in &every {
        match response {
            McpResponse::HelloOk(_)
            | McpResponse::Catalogue(_)
            | McpResponse::Outcome(_)
            | McpResponse::Error(_) => seen += 1,
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
        McpSessionError::Malformed,
        McpSessionError::NotNegotiated,
        McpSessionError::VersionMismatch,
        McpSessionError::Busy,
        McpSessionError::Timeout,
    ];
    for error in every {
        match error {
            McpSessionError::Malformed
            | McpSessionError::NotNegotiated
            | McpSessionError::VersionMismatch
            | McpSessionError::Busy
            | McpSessionError::Timeout => {}
        }
        crate::test_support::assert_ron_round_trip(&error);
    }
    assert_eq!(every.len(), 5, "the error wire is five values");
}
