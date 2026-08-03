use super::super::*;
use crate::occupancy::TerrainKind;


#[test]
fn shipped_reaction_leaves_parse_and_satisfy_ordering_invariant() {
    const SHIPPED_TUNING_RON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/core_tuning/combat.tuning.ron"
    ));

    let Ok(tuning) = ron::from_str::<CombatTuning>(SHIPPED_TUNING_RON) else {
        return;
    };
    let Ok(reparsed) = ron::from_str::<CombatTuning>(SHIPPED_TUNING_RON) else {
        return;
    };

    assert_eq!(
        tuning.reaction.cap_base, reparsed.reaction.cap_base,
        "shipped reaction.cap_base must be present and parse deterministically",
    );
    assert_eq!(
        tuning.reaction.cap_per_reactions, reparsed.reaction.cap_per_reactions,
        "shipped reaction.cap_per_reactions must be present and parse deterministically",
    );
    assert_eq!(
        tuning.reaction.p_min, reparsed.reaction.p_min,
        "shipped reaction.p_min must be present and parse deterministically",
    );
    assert_eq!(
        tuning.reaction.p_max, reparsed.reaction.p_max,
        "shipped reaction.p_max must be present and parse deterministically",
    );

    assert!(
        *tuning.reaction.p_min < *tuning.reaction.p_max,
        "shipped reaction.p_min must be strictly less than reaction.p_max so the \
         probability clamp is well-defined",
    );
}

#[test]
fn shipped_melee_leaves_parse_and_satisfy_structural_invariants() {
    const SHIPPED_TUNING_RON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/core_tuning/combat.tuning.ron"
    ));

    let Ok(tuning) = ron::from_str::<CombatTuning>(SHIPPED_TUNING_RON) else {
        return;
    };
    let Ok(reparsed) = ron::from_str::<CombatTuning>(SHIPPED_TUNING_RON) else {
        return;
    };

    assert_eq!(
        tuning.melee.k_margin, reparsed.melee.k_margin,
        "shipped melee.k_margin must be present and parse deterministically",
    );
    assert_eq!(
        tuning.melee.mult_min, reparsed.melee.mult_min,
        "shipped melee.mult_min must be present and parse deterministically",
    );
    assert_eq!(
        tuning.melee.mult_max, reparsed.melee.mult_max,
        "shipped melee.mult_max must be present and parse deterministically",
    );
    assert_eq!(
        tuning.melee.variance, reparsed.melee.variance,
        "shipped melee.variance must be present and parse deterministically",
    );

    assert!(
        *tuning.melee.mult_min <= *tuning.melee.mult_max,
        "shipped melee.mult_min must be <= melee.mult_max so the damage-mult clamp \
         is well-defined",
    );
    assert!(
        *tuning.melee.variance >= 0.0 && *tuning.melee.variance < 1.0,
        "shipped melee.variance must satisfy 0 <= v < 1 so the per-side roll never \
         reaches 0 (got {})",
        *tuning.melee.variance,
    );
}

#[test]
fn shipped_tuning_ron_deserializes() {
    const SHIPPED_TUNING_RON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/core_tuning/combat.tuning.ron"
    ));

    let parsed = ron::from_str::<CombatTuning>(SHIPPED_TUNING_RON);
    assert!(
        parsed.is_ok(),
        "shipped assets/core_tuning/combat.tuning.ron must deserialize into CombatTuning: {parsed:?}",
    );

    let Ok(tuning) = parsed else {
        return;
    };
    let multipliers = tuning.matchup_multipliers;
    assert!(
        *multipliers.resisted < *multipliers.neutral,
        "shipped resisted multiplier must be < neutral",
    );
    assert!(
        *multipliers.neutral < *multipliers.favorable,
        "shipped neutral multiplier must be < favorable",
    );

    let edges = tuning.severity_scaling.edges;
    assert!(
        *edges.e0 < *edges.e1,
        "shipped severity edge e0 must be < e1"
    );
    assert!(
        *edges.e1 < *edges.e2,
        "shipped severity edge e1 must be < e2"
    );
    assert!(
        *edges.e2 < *edges.e3,
        "shipped severity edge e2 must be < e3"
    );

    assert!(
        *tuning.bleed_rate > 0,
        "shipped bleed_rate must be > 0 so the bleed-out clock actually drains",
    );

    // `#[serde(default)]`, so a missing leaf would fail the parse above; this
    let Ok(reparsed) = ron::from_str::<CombatTuning>(SHIPPED_TUNING_RON) else {
        return;
    };
    assert_eq!(
        tuning.stabilize_tu, reparsed.stabilize_tu,
        "shipped stabilize_tu must be present and parse deterministically",
    );
    assert_eq!(
        tuning.execute_tu, reparsed.execute_tu,
        "shipped execute_tu must be present and parse deterministically",
    );

    // `#[serde(default)]`, so a missing leaf would fail the parse above; this
    assert_eq!(
        tuning.stance_change_tu, reparsed.stance_change_tu,
        "shipped stance_change_tu must be present and parse deterministically",
    );
    assert_eq!(
        tuning.turn_tu, reparsed.turn_tu,
        "shipped turn_tu must be present and parse deterministically",
    );

    // shipped file. `CombatTuning` has no `#[serde(default)]`, so a missing leaf would
    assert_eq!(
        tuning.move_costs, reparsed.move_costs,
        "shipped move_costs must be present and parse deterministically",
    );
    assert_eq!(
        tuning.move_costs.cost(TerrainKind::Open),
        tuning.move_costs.open,
        "cost(Open) must map to the open field",
    );
    assert_eq!(
        tuning.move_costs.cost(TerrainKind::Cover),
        tuning.move_costs.cover,
        "cost(Cover) must map to the cover field",
    );
    assert_eq!(
        tuning.move_costs.cost(TerrainKind::Wall),
        tuning.move_costs.wall,
        "cost(Wall) must map to the wall field",
    );

    // file. `CombatTuning` has no `#[serde(default)]`, so a missing leaf would fail the
    assert_eq!(
        tuning.firing_arc, reparsed.firing_arc,
        "shipped firing_arc must be present and parse deterministically",
    );
    assert!(
        *tuning.firing_arc > 0.0,
        "shipped firing_arc must be a positive cone width (a real facing arc)",
    );

    assert_visibility_and_link_leaves_resolve(&tuning, &reparsed);
}

/// `CombatTuning` has no `#[serde(default)]`, so a missing leaf would fail the parse in
fn assert_visibility_and_link_leaves_resolve(tuning: &CombatTuning, reparsed: &CombatTuning) {
    assert_eq!(
        tuning.view_range, reparsed.view_range,
        "shipped view_range must be present and parse deterministically",
    );
    assert!(
        *tuning.view_range > 0,
        "shipped view_range must be a positive sight disc (the squad can see at all)",
    );
    assert_eq!(
        tuning.explored_dim, reparsed.explored_dim,
        "shipped explored_dim must be present and parse deterministically",
    );
    assert!(
        (0.0..=1.0).contains(&*tuning.explored_dim),
        "shipped explored_dim must be an RGB modulate in 0..=1 (dimmer, never brightened)",
    );
    assert_eq!(
        tuning.link_tu, reparsed.link_tu,
        "shipped link_tu must be present and parse deterministically",
    );
}
