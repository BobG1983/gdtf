//! Pick the port a new instance listens on.

use cobalt_mcp_protocol::ports::McpPort;

use crate::lifecycle::{
    orphan::{OrphanWatch, PortHold},
    values::ProbeTimeout,
};

// How far above the requested port a new instance may be placed.
const PORT_SEARCH_WINDOW: u16 = 16;

// The requested port when no record holds it, else the first free port above it.
pub(super) fn pick_port(
    requested: McpPort,
    taken: &[McpPort],
    orphans: &dyn OrphanWatch,
    probe: ProbeTimeout,
) -> Option<McpPort> {
    if !taken.contains(&requested) {
        return Some(requested);
    }
    for step in 1..=PORT_SEARCH_WINDOW {
        let candidate = McpPort::new(requested.checked_add(step)?);
        if taken.contains(&candidate) {
            continue;
        }
        if matches!(orphans.inspect(candidate, probe), PortHold::Free) {
            return Some(candidate);
        }
    }
    None
}
