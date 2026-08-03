use gdtf_battle_sim::{falls::StoreysFallen, prelude::Cell};

use crate::overlays::cross_level_signals::{
    aggregate::{aggregate_threats, build_signals, cap_badges},
    types::{CrossLevelBadgeKind, LevelDelta, ThreatCount},
};

#[test]
fn same_delta_enemies_collapse_a_different_delta_stays_separate() {
    let same = LevelDelta::new(2);
    let different = LevelDelta::new(-1);

    let badges = aggregate_threats([same, same, different]);

    assert_eq!(
        badges,
        vec![
            CrossLevelBadgeKind::Threat {
                delta: different,
                count: ThreatCount::ONE,
            },
            CrossLevelBadgeKind::Threat {
                delta: same,
                count: ThreatCount::ONE.incremented(),
            },
        ],
        "two enemies at delta +2 collapse into one x2 badge; the -1 badge \
         (nearest, magnitude 1) sorts before the +2 badge (magnitude 2)",
    );
}

#[test]
fn more_than_three_candidates_keep_exactly_three_in_priority_order() {
    let near = CrossLevelBadgeKind::Threat {
        delta: LevelDelta::new(1),
        count: ThreatCount::ONE,
    };
    let far = CrossLevelBadgeKind::Threat {
        delta: LevelDelta::new(-4),
        count: ThreatCount::ONE,
    };
    let drop = CrossLevelBadgeKind::DropDepth {
        storeys: StoreysFallen::new(2),
    };
    let connector = CrossLevelBadgeKind::ConnectorDelta {
        delta: LevelDelta::new(1),
    };

    let capped = cap_badges(vec![connector, drop, far, near]);

    assert_eq!(
        capped,
        vec![near, far, drop],
        "top 3 by priority: nearest-delta Threat, farther Threat, then \
         DropDepth — the ConnectorDelta candidate is silently dropped",
    );
}

#[test]
fn build_signals_caps_independently_per_cell() {
    let busy = Cell::new(1, 1);
    let quiet = Cell::new(9, 9);

    let threats = vec![
        (busy, LevelDelta::new(1)),
        (busy, LevelDelta::new(2)),
        (busy, LevelDelta::new(3)),
        (busy, LevelDelta::new(4)),
        (quiet, LevelDelta::new(1)),
    ];

    let signals = build_signals(threats, Vec::new(), Vec::new());

    assert_eq!(
        signals.badges_at(busy).len(),
        3,
        "the busy cell's 4 distinct-delta threats cap at 3 (nearest-delta-first)",
    );
    assert_eq!(
        signals.badges_at(quiet).len(),
        1,
        "the quiet cell keeps its one badge, unaffected by the busy cell's overflow",
    );
}
