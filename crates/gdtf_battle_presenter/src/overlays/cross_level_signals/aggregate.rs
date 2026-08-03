use bevy::platform::collections::HashMap;
use gdtf_battle_sim::{falls::StoreysFallen, prelude::Cell};

use super::types::{
    BADGE_CAP_PER_CELL, CrossLevelBadgeKind, CrossLevelSignals, LevelDelta, ThreatCount,
};

const fn priority_key(kind: CrossLevelBadgeKind) -> (u8, u8) {
    match kind {
        CrossLevelBadgeKind::Threat { delta, .. } => (0, delta.magnitude()),
        CrossLevelBadgeKind::DropDepth { .. } => (1, 0),
        CrossLevelBadgeKind::ConnectorDelta { .. } => (2, 0),
    }
}

pub(super) fn aggregate_threats(
    deltas: impl IntoIterator<Item = LevelDelta>,
) -> Vec<CrossLevelBadgeKind> {
    let mut counts: HashMap<LevelDelta, ThreatCount> = HashMap::default();
    for delta in deltas {
        counts
            .entry(delta)
            .and_modify(|count| *count = count.incremented())
            .or_insert(ThreatCount::ONE);
    }
    let mut badges: Vec<CrossLevelBadgeKind> = counts
        .into_iter()
        .map(|(delta, count)| CrossLevelBadgeKind::Threat { delta, count })
        .collect();
    badges.sort_by_key(|&kind| priority_key(kind));
    badges
}

pub(super) fn cap_badges(mut badges: Vec<CrossLevelBadgeKind>) -> Vec<CrossLevelBadgeKind> {
    badges.sort_by_key(|&kind| priority_key(kind));
    badges.truncate(BADGE_CAP_PER_CELL);
    badges
}

pub(super) fn build_signals(
    threats: Vec<(Cell, LevelDelta)>,
    drops: Vec<(Cell, StoreysFallen)>,
    connectors: Vec<(Cell, LevelDelta)>,
) -> CrossLevelSignals {
    let mut per_cell: HashMap<Cell, Vec<CrossLevelBadgeKind>> = HashMap::default();

    let mut threats_by_cell: HashMap<Cell, Vec<LevelDelta>> = HashMap::default();
    for (cell, delta) in threats {
        threats_by_cell.entry(cell).or_default().push(delta);
    }
    for (cell, deltas) in threats_by_cell {
        per_cell
            .entry(cell)
            .or_default()
            .extend(aggregate_threats(deltas));
    }
    for (cell, storeys) in drops {
        per_cell
            .entry(cell)
            .or_default()
            .push(CrossLevelBadgeKind::DropDepth { storeys });
    }
    for (cell, delta) in connectors {
        per_cell
            .entry(cell)
            .or_default()
            .push(CrossLevelBadgeKind::ConnectorDelta { delta });
    }

    for badges in per_cell.values_mut() {
        let capped = cap_badges(std::mem::take(badges));
        *badges = capped;
    }

    CrossLevelSignals::build(per_cell)
}
