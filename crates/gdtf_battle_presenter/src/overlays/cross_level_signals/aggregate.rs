//! Pure aggregation + cap logic (GTW-596 RESOLVED SPEC, user ruling 2026-07-10):
//! dedupe Threat badges by distinct level-delta, then cap each cell at
//! [`BADGE_CAP_PER_CELL`] total badges in Threat (nearest-delta-first) →
//! `DropDepth` → `ConnectorDelta` priority order, silently dropping the rest.

use bevy::platform::collections::HashMap;
use gdtf_battle_sim::{falls::StoreysFallen, prelude::Cell};

use super::types::{
    BADGE_CAP_PER_CELL, CrossLevelBadgeKind, CrossLevelSignals, LevelDelta, ThreatCount,
};

/// The cap's PRIORITY key for one badge — ascending, so [`Vec::sort_by_key`] with
/// this puts the RESOLVED SPEC's keep-order first: Threat (rank 0, nearest
/// [`LevelDelta::magnitude`] first), then `DropDepth` (rank 1), then
/// `ConnectorDelta` (rank 2). `sort_by_key` is STABLE, so same-rank badges keep
/// their gather order (the spec is silent on ties within `DropDepth` /
/// `ConnectorDelta`).
const fn priority_key(kind: CrossLevelBadgeKind) -> (u8, u8) {
    match kind {
        CrossLevelBadgeKind::Threat { delta, .. } => (0, delta.magnitude()),
        CrossLevelBadgeKind::DropDepth { .. } => (1, 0),
        CrossLevelBadgeKind::ConnectorDelta { .. } => (2, 0),
    }
}

/// Dedupe one cell's raw Threat deltas into aggregated
/// [`CrossLevelBadgeKind::Threat`] badges — the RESOLVED SPEC's "distinct
/// level-delta per cell" fold: every enemy sharing an EXACT delta collapses into
/// one badge with an incremented [`ThreatCount`]; a different delta on the same
/// cell stays a separate badge. Returned pre-sorted nearest-delta-first.
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

/// Cap `badges` at [`BADGE_CAP_PER_CELL`], keeping the RESOLVED SPEC's priority
/// order (Threat nearest-first, then `DropDepth`, then `ConnectorDelta`) and
/// silently dropping the rest — the ONE overflow rule, applied per cell.
pub(super) fn cap_badges(mut badges: Vec<CrossLevelBadgeKind>) -> Vec<CrossLevelBadgeKind> {
    badges.sort_by_key(|&kind| priority_key(kind));
    badges.truncate(BADGE_CAP_PER_CELL);
    badges
}

/// Combine the three per-source gathers into the final capped [`CrossLevelSignals`]
/// — the derive system's one aggregation entry point.
pub(super) fn build_signals(
    threats: Vec<(Cell, LevelDelta)>,
    drops: Vec<(Cell, StoreysFallen)>,
    connectors: Vec<(Cell, LevelDelta)>,
) -> CrossLevelSignals {
    let mut per_cell: HashMap<Cell, Vec<CrossLevelBadgeKind>> = HashMap::default();

    // Group raw Threat deltas per cell FIRST so `aggregate_threats` only ever sees
    // one cell's deltas at a time (the dedupe is per-cell, never cross-cell).
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
