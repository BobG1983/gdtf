//! Real-path parse of the shipped `assets/core_tuning/combat.tuning.ron` + its
//! structural invariants (value-agnostic — never pinned magnitudes).

use super::super::*;
use crate::occupancy::TerrainKind;

// ── GTW-466 reaction-tuning leaf tests ───────────────────────────────────────

/// GTW-466 C2 — the §8 reaction-fire tuning leaves parse from the shipped
/// `assets/core_tuning/combat.tuning.ron` (real-path parse-OK test).
///
/// Value-agnostic: asserts only that the four reaction leaves are PRESENT and
/// parse deterministically across two parses (never the exact magnitudes — the
/// cap inputs and probability clamp are tunable balance data). The `p_min < p_max`
/// ordering invariant is the ONE structural relation that the shipped values
/// must satisfy (no ganger can be both floored and ceilinged; `p_min >= p_max`
/// would make `clamp_probability` pathological).
#[test]
fn shipped_reaction_leaves_parse_and_satisfy_ordering_invariant() {
    const SHIPPED_TUNING_RON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/core_tuning/combat.tuning.ron"
    ));

    let Ok(tuning) = ron::from_str::<CombatTuning>(SHIPPED_TUNING_RON) else {
        // The parent `shipped_tuning_ron_deserializes` test already guards the
        // parse-OK path; if we are here a prior test failure will catch it.
        return;
    };
    let Ok(reparsed) = ron::from_str::<CombatTuning>(SHIPPED_TUNING_RON) else {
        return;
    };

    // The four reaction leaves parse deterministically (present + stable).
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

    // The ONE structural ordering invariant: p_min < p_max (the clamp is
    // well-defined — value-agnostic, never the exact 0.05 / 0.95 magnitudes).
    assert!(
        *tuning.reaction.p_min < *tuning.reaction.p_max,
        "shipped reaction.p_min must be strictly less than reaction.p_max so the \
         probability clamp is well-defined",
    );
}

/// GTW-506 C3 — the §7 melee opposed-Fight tuning leaves parse from the shipped
/// `assets/core_tuning/combat.tuning.ron` (real-path parse-OK test).
///
/// Value-agnostic: asserts only that the four melee leaves are PRESENT and parse
/// deterministically across two parses (never the exact magnitudes — `k_margin` /
/// `mult_min` / `mult_max` / `variance` are tunable balance data). The structural
/// invariants the shipped values MUST satisfy: `mult_min <= mult_max` (the clamp is
/// well-defined) and `0 <= variance < 1` (a roll never reaches 0, which would
/// collapse the `atk/def` margin).
#[test]
fn shipped_melee_leaves_parse_and_satisfy_structural_invariants() {
    const SHIPPED_TUNING_RON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/core_tuning/combat.tuning.ron"
    ));

    let Ok(tuning) = ron::from_str::<CombatTuning>(SHIPPED_TUNING_RON) else {
        // The parent `shipped_tuning_ron_deserializes` test already guards parse-OK.
        return;
    };
    let Ok(reparsed) = ron::from_str::<CombatTuning>(SHIPPED_TUNING_RON) else {
        return;
    };

    // The four melee leaves parse deterministically (present + stable).
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

    // Structural invariant 1: mult_min <= mult_max (the damage-mult clamp is
    // well-defined — value-agnostic, never the exact magnitudes).
    assert!(
        *tuning.melee.mult_min <= *tuning.melee.mult_max,
        "shipped melee.mult_min must be <= melee.mult_max so the damage-mult clamp \
         is well-defined",
    );
    // Structural invariant 2: 0 <= variance < 1 (a roll [1-v, 1+v] never reaches 0,
    // which would collapse the atk/def margin to a div-by-zero).
    assert!(
        *tuning.melee.variance >= 0.0 && *tuning.melee.variance < 1.0,
        "shipped melee.variance must satisfy 0 <= v < 1 so the per-side roll never \
         reaches 0 (got {})",
        *tuning.melee.variance,
    );
}

/// The shipped `assets/core_tuning/combat.tuning.ron` deserializes into a
/// [`CombatTuning`] on the **real** path (the same file the data-driven
/// tuning store loads).
///
/// Deliberately value-agnostic: the tuning magnitudes are the **tunable**
/// balance data, so this pins only that the shipped file parses into the
/// type — never a specific band edge, severity scalar, or part weight
/// (asserting a magnitude would be brittle against a balance edit). The
/// metric-const pin lives in [`crate::metric`] (a coordinate-system
/// definition, not a tunable).
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

    // AC4 — the E3.2 matchup-multiplier leaves are PRESENT in the shipped file
    // (the three fields parsed into the struct). Value-agnostic: it asserts the
    // ordering relation the leaves must hold (resisted < neutral < favorable),
    // never a magnitude — the multipliers are tunable balance data.
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

    // GTW-186 AC7 — the RESHAPED (floor-extend) severity_scaling leaves
    // deserialize from the real shipped file: the floor-extend L
    // (defender_luck_scale) and the bucket edges are present, and there is NO
    // random_spread_min (it was removed). Value-agnostic: it asserts only the
    // structural invariant the edges must hold (ascending e0 < e1 < e2 < e3 so
    // the buckets are monotone), never a magnitude — the scalars are tunable.
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

    // GTW-189 AC7 — the E3.7 bleed-out leaf (`bleed_rate`) deserializes from the
    // real shipped file. Value-agnostic: it asserts only the structural invariant
    // a bleed clock must hold (a positive drain, so the clock actually ticks down
    // and a Downed ganger eventually dies), never a magnitude — the rate is
    // tunable balance data.
    assert!(
        *tuning.bleed_rate > 0,
        "shipped bleed_rate must be > 0 so the bleed-out clock actually drains",
    );

    // GTW-190 AC7 — the E3.8 from-Downed TU leaves (`stabilize_tu` / `execute_tu`)
    // deserialize from the real shipped file. `CombatTuning` has no
    // `#[serde(default)]`, so a missing leaf would fail the parse above; this
    // re-parse asserts the two leaves are PRESENT and parse deterministically to
    // the same value. Value-agnostic — never a magnitude, since these are tunable
    // balance data this slice only READS (the TU economy that debits them is E4).
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

    // GTW-194 AC6 — the E4.1 posture TU leaves (`stance_change_tu` / `turn_tu`)
    // deserialize from the real shipped file. `CombatTuning` has no
    // `#[serde(default)]`, so a missing leaf would fail the parse above; this
    // asserts the two leaves are PRESENT and parse deterministically to the same
    // value across two parses. Value-agnostic — never a magnitude, since these are
    // tunable balance data (the only relation that matters is that the leaves exist
    // and parse). The per-toggle vs per-fire distinction is in `posture.rs`.
    assert_eq!(
        tuning.stance_change_tu, reparsed.stance_change_tu,
        "shipped stance_change_tu must be present and parse deterministically",
    );
    assert_eq!(
        tuning.turn_tu, reparsed.turn_tu,
        "shipped turn_tu must be present and parse deterministically",
    );

    // GTW-234 AC8 — the per-terrain `move_costs` table deserializes from the real
    // shipped file. `CombatTuning` has no `#[serde(default)]`, so a missing leaf would
    // fail the parse above; this asserts the table is PRESENT and parses
    // deterministically across two parses, and that its `cost(terrain)` accessor maps
    // each variant to its authored field. Value-agnostic — never a magnitude, since
    // the per-terrain move costs are tunable balance data (the floor tile crossed
    // determines the cost; only the mechanism is pinned).
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

    // GTW-242 — the firing-arc leaf (`firing_arc`) deserializes from the real shipped
    // file. `CombatTuning` has no `#[serde(default)]`, so a missing leaf would fail the
    // parse above; this asserts it is PRESENT and parses deterministically across two
    // parses, and that it is a positive angle (so the arc is a real cone, not a
    // degenerate zero). Value-agnostic — never the exact magnitude, since the arc width
    // is tunable balance data (only the relation "a real positive cone" is pinned).
    assert_eq!(
        tuning.firing_arc, reparsed.firing_arc,
        "shipped firing_arc must be present and parse deterministically",
    );
    assert!(
        *tuning.firing_arc > 0.0,
        "shipped firing_arc must be a positive cone width (a real facing arc)",
    );

    // GTW-338 / GTW-349 — the squad-FOV visibility + per-link economy leaves resolve from
    // the shipped file (extracted to keep this monolith under the line cap as it accretes
    // one block per ticket).
    assert_visibility_and_link_leaves_resolve(&tuning, &reparsed);
}

/// GTW-338 / GTW-349 — the squad-FOV visibility leaves (`view_range` / `explored_dim`)
/// and the per-link economy leaf (`link_tu`) deserialize from the real shipped file.
///
/// `CombatTuning` has no `#[serde(default)]`, so a missing leaf would fail the parse in
/// the caller; this asserts each leaf is PRESENT and parses deterministically across the
/// two parses (the resolve / parse-OK check), plus the structural invariants the FOV
/// leaves must hold: the view range is a positive sight disc (the squad can see at all),
/// and the explored dim is a real RGB modulate in `0..=1`. Value-agnostic — never the
/// exact `14` / `0.55` magnitudes nor any `link_tu` number, since all three are tunable
/// balance data (`link_tu` is consumed later by GTW-351; this leaf only ADDS the tunable,
/// per the loader-tests-no-magnitude-pins convention).
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
