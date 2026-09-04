//! Pick the port a new instance listens on.

use crate::{
    lifecycle::{
        orphan::{OrphanWatch, PortHold},
        values::ProbeTimeout,
    },
    link::QaPort,
};

// How far above the requested port a new instance may be placed.
const PORT_SEARCH_WINDOW: u16 = 16;

// The requested port when no record holds it, else the first free port above it.
pub(super) fn pick_port(
    requested: QaPort,
    taken: &[QaPort],
    orphans: &dyn OrphanWatch,
    probe: ProbeTimeout,
) -> Option<QaPort> {
    if !taken.contains(&requested) {
        return Some(requested);
    }
    for step in 1..=PORT_SEARCH_WINDOW {
        let candidate = QaPort::new(requested.checked_add(step)?);
        if taken.contains(&candidate) {
            continue;
        }
        if matches!(orphans.inspect(candidate, probe), PortHold::Free) {
            return Some(candidate);
        }
    }
    None
}
