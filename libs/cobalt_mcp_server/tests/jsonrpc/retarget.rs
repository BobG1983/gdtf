use cobalt_mcp_protocol::{
    command::{CommandCatalogue, CommandOutcome},
    message::{McpRequest, McpResponse, McpSessionError, ServerNameNet},
};
use cobalt_mcp_server::{McpError, McpLink, McpPort, dispatch};
use serde_json::Value;

use crate::{
    hosts::{test_identity, two_host_set},
    support::{
        BRAMBLE_HOST_NAME, BRAMBLE_LOG, BRAMBLE_PID, BRAMBLE_PORT, CannedLifecycle, CannedThistle,
        SeededInstance, THISTLE_LOG, THISTLE_PID, THISTLE_PORT,
    },
};

/// Bramble link that answers like the real one and records every port it is pointed at.
struct RecordingLink {
    ports: Vec<McpPort>,
}

impl RecordingLink {
    const fn new() -> Self {
        Self { ports: Vec::new() }
    }
}

impl McpLink for RecordingLink {
    fn request(&mut self, request: McpRequest) -> Result<McpResponse, McpError> {
        match request {
            McpRequest::Catalogue => Ok(McpResponse::Catalogue(CommandCatalogue::new(
                ServerNameNet::new(BRAMBLE_HOST_NAME.to_owned()),
                Vec::new(),
            ))),
            McpRequest::Run(_) => Ok(McpResponse::Outcome(CommandOutcome::Unknown {
                known: Vec::new(),
            })),
            McpRequest::Hello(_) => Ok(McpResponse::Error(McpSessionError::Malformed)),
        }
    }

    fn retarget(&mut self, port: McpPort) {
        self.ports.push(port);
    }
}

/// Dispatch one line against a bramble host seeded with these instances.
///
/// Returns the reply and every port the bramble's link was pointed at while it was answered.
pub(crate) fn dispatch_recording(
    line: &str,
    bramble_instances: &[SeededInstance],
) -> (Value, Vec<McpPort>) {
    let mut thistle_link = CannedThistle;
    let mut bramble_link = RecordingLink::new();
    let mut thistle_life = CannedLifecycle::new(THISTLE_PORT, THISTLE_PID, THISTLE_LOG, &[]);
    let mut bramble_life =
        CannedLifecycle::new(BRAMBLE_PORT, BRAMBLE_PID, BRAMBLE_LOG, bramble_instances);
    let reply = {
        let mut hosts = two_host_set(
            &mut thistle_link,
            &mut thistle_life,
            &mut bramble_link,
            &mut bramble_life,
        );
        let Some(response) = dispatch(line, &test_identity(), &mut hosts) else {
            unreachable!("a request with an id yields a response line");
        };
        serde_json::from_str(&response).unwrap_or(Value::Null)
    };
    (reply, bramble_link.ports)
}
