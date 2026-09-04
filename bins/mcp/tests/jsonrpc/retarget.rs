use cobalt_mcp_protocol::{
    command::{CommandCatalogue, CommandOutcome},
    message::{QaError, QaRequest, QaResponse, ServerNameNet},
};
use mcp::{HostPair, HostSet, McpError, QaLink, QaPort, dispatch};
use serde_json::Value;

use crate::support::{
    CannedGame, CannedLifecycle, EDITOR_HOST_NAME, EDITOR_LOG, EDITOR_PID, EDITOR_PORT, GAME_LOG,
    GAME_PID, GAME_PORT, SeededInstance,
};

/// Editor link that answers like the real one and records every port it is pointed at.
struct RecordingLink {
    ports: Vec<QaPort>,
}

impl RecordingLink {
    const fn new() -> Self {
        Self { ports: Vec::new() }
    }
}

impl QaLink for RecordingLink {
    fn request(&mut self, request: QaRequest) -> Result<QaResponse, McpError> {
        match request {
            QaRequest::Catalogue => Ok(QaResponse::Catalogue(CommandCatalogue::new(
                ServerNameNet::new(EDITOR_HOST_NAME.to_owned()),
                Vec::new(),
            ))),
            QaRequest::Run(_) => Ok(QaResponse::Outcome(CommandOutcome::Unknown {
                known: Vec::new(),
            })),
            QaRequest::Hello(_) => Ok(QaResponse::Error(QaError::Malformed)),
        }
    }

    fn retarget(&mut self, port: QaPort) {
        self.ports.push(port);
    }
}

/// Dispatch one line against an editor host seeded with these instances.
///
/// Returns the reply and every port the editor's link was pointed at while it was answered.
pub(crate) fn dispatch_recording(
    line: &str,
    editor_instances: &[SeededInstance],
) -> (Value, Vec<QaPort>) {
    let mut game_link = CannedGame;
    let mut editor_link = RecordingLink::new();
    let mut game_life = CannedLifecycle::new(GAME_PORT, GAME_PID, GAME_LOG, &[]);
    let mut editor_life =
        CannedLifecycle::new(EDITOR_PORT, EDITOR_PID, EDITOR_LOG, editor_instances);
    let reply = {
        let mut hosts = HostSet::new(
            HostPair::new(&mut game_link, &mut game_life),
            HostPair::new(&mut editor_link, &mut editor_life),
        );
        let Some(response) = dispatch(line, &mut hosts) else {
            unreachable!("a request with an id yields a response line");
        };
        serde_json::from_str(&response).unwrap_or(Value::Null)
    };
    (reply, editor_link.ports)
}
