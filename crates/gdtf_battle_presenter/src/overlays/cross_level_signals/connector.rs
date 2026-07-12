//! Connector-delta gathering (GTW-596): one badge per authored vertical-link
//! endpoint on the active storey, showing the signed level-delta to its OTHER
//! endpoint.

use gdtf_battle_sim::{
    prelude::{Cell, Level},
    vertical::VerticalLinkGraph,
    visibility::SquadVisibility,
};

use super::types::LevelDelta;

/// Gather one `(cell, level-delta)` entry per fog-EXPLORED vertical-link endpoint
/// on `active_level` — the signed delta to the link's OTHER endpoint (positive =
/// ascend, negative = descend).
pub(super) fn gather_connectors(
    active_level: Level,
    graph: &VerticalLinkGraph,
    squad: &SquadVisibility,
) -> Vec<(Cell, LevelDelta)> {
    let active_z = i32::from(*active_level);
    let mut out = Vec::new();
    for link in graph.links() {
        for (endpoint, other) in [(link.from, link.to), (link.to, link.from)] {
            if endpoint.z != active_z {
                continue;
            }
            if !*squad.is_cell_explored(&endpoint) {
                continue;
            }
            // Endpoints are both bounded 0..MAX_LEVELS (8), so the difference
            // always fits an i8 (-7..=7); `unwrap_or` is a defensive no-panic
            // fallback for an unreachable overflow.
            let delta = i8::try_from(other.z - endpoint.z).unwrap_or(0);
            out.push((endpoint.cell(), LevelDelta::new(delta)));
        }
    }
    out
}
