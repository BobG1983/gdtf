//! Connector-delta gathering: one badge per authored vertical-link endpoint on the active storey.
use gdtf_battle_sim::{
    prelude::{Cell, Level},
    vertical::VerticalLinkGraph,
    visibility::SquadVisibility,
};

use super::types::LevelDelta;

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
            let delta = i8::try_from(other.z - endpoint.z).unwrap_or(0);
            out.push((endpoint.cell(), LevelDelta::new(delta)));
        }
    }
    out
}
