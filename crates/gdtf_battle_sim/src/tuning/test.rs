//! Relocated unit tests for the `tuning` data dir-module (GTW-201 code-health
//! wave) — the inline `#[cfg(test)] mod tests` moved VERBATIM. Behavior-preserving
//! move: same assertions, same count (6 tests).

use super::*;
use crate::{cover::HeightBand, occupancy::TerrainKind};

/// Each tuning newtype wraps the right inner type and its derived [`Deref`]
/// reaches that inner value (C9/C10/C11 mandate a derived `Deref` on every
/// tuning newtype; this exercises that surface so dropping the derive would
/// fail a test).
///
/// Built from **arbitrary** literals, never the shipped/default magnitudes:
/// this pins the Deref *target type and mechanism*, not a balance value, so
/// it stays non-brittle against a tuning edit. The `f32` newtypes are
/// compared by bit pattern (the literals are exactly representable, so this
/// is an exact integer equality — no `float_cmp` lint, no epsilon).
#[test]
fn tuning_newtypes_wrap_inner_and_deref() {
    // Each f32 newtype: deref reaches the inner f32 (bit-exact arbitrary
    // value, not the default).
    assert_eq!((*BandEdge(5.0)).to_bits(), 5.0_f32.to_bits());
    assert_eq!((*PenDamageScale(2.5)).to_bits(), 2.5_f32.to_bits());
    assert_eq!((*ToughnessMitigation(3.5)).to_bits(), 3.5_f32.to_bits());
    assert_eq!((*ShooterLuckScale(4.5)).to_bits(), 4.5_f32.to_bits());
    assert_eq!((*DefenderLuckScale(6.5)).to_bits(), 6.5_f32.to_bits());
    assert_eq!((*RandomSpread(7.5)).to_bits(), 7.5_f32.to_bits());
    assert_eq!((*SeverityEdge(8.5)).to_bits(), 8.5_f32.to_bits());
    // The u16 newtype: deref reaches the inner u16 (arbitrary value).
    assert_eq!(*BodyPartWeight(3), 3u16);
    // The u8 bleed-out rate: deref reaches the inner u8 (arbitrary value, the
    // mechanism not the shipped magnitude).
    assert_eq!(*BleedRate(4), 4u8);
    // GTW-242 — the firing-arc f32 newtype: deref reaches the inner degrees (bit-exact
    // arbitrary value, never the 120° default — the Deref mechanism, not a magnitude).
    assert_eq!((*FiringArc::new(75.0)).to_bits(), 75.0_f32.to_bits());
}

/// C3 — every E2.1 cone/stability/recoil/aim extension leaf wraps the right
/// inner type and its derived [`Deref`] reaches it. Built from **arbitrary**
/// literals (never the shipped/default magnitudes), so this pins the Deref
/// mechanism + target type, not a balance value (bit-exact f32 equality on
/// exactly-representable literals — no `float_cmp` lint).
#[test]
fn cone_stability_newtypes_wrap_inner_and_deref() {
    assert_eq!((*StanceContribution(11.0)).to_bits(), 11.0_f32.to_bits());
    assert_eq!((*BraceContribution(22.0)).to_bits(), 22.0_f32.to_bits());
    assert_eq!((*StabilityCurveCoord(33.0)).to_bits(), 33.0_f32.to_bits());
    assert_eq!((*AimConeMult(0.25)).to_bits(), 0.25_f32.to_bits());
    assert_eq!((*AimTuPremium(1.25)).to_bits(), 1.25_f32.to_bits());
    assert_eq!((*RecoilClimb(0.5)).to_bits(), 0.5_f32.to_bits());
    assert_eq!((*ConcentrationCoeff(2.5)).to_bits(), 2.5_f32.to_bits());
    assert_eq!((*AimHeightFrac(0.75)).to_bits(), 0.75_f32.to_bits());
    assert_eq!((*MuzzleForwardOffset(0.125)).to_bits(), 0.125_f32.to_bits());
    assert_eq!((*MuzzleHeight(0.625)).to_bits(), 0.625_f32.to_bits());
    assert_eq!((*SilhouetteTop(0.875)).to_bits(), 0.875_f32.to_bits());

    // The curve newtype derefs to its inner Vec (arbitrary one-point shape).
    let curve = StabilityCurve::new(vec![StabilityCurvePoint {
        score:  StabilityCurveCoord(50.0),
        output: StabilityCurveCoord(0.7),
    }]);
    assert_eq!(curve.len(), 1);
    assert_eq!((*curve[0].output).to_bits(), 0.7_f32.to_bits());

    // The brace gate carries a HeightBand per stance (not a magnitude).
    let gate = BraceMinHeight {
        prone: HeightBand::Low,
        kneel: HeightBand::Mid,
        stand: HeightBand::High,
    };
    assert_eq!(gate.stand, HeightBand::High);
}

/// C5/C3 — the E2.1 extension parses from a hand-written RON fragment with
/// every numeric leaf a **bare scalar** (`#[serde(transparent)]`) and the
/// stability curves authored as bare point lists. Value-agnostic: asserts only
/// structural success (arbitrary literals, never shipped magnitudes).
#[test]
fn cone_stability_parses_from_ron_with_bare_scalar_leaves() {
    let ron = r"(
        stance_stability: ( prone: 40.0, kneel: 25.0, stand: 10.0 ),
        brace_contribution: 30.0,
        brace_min_height: ( prone: Low, kneel: Mid, stand: High ),
        stability_curves: (
            cone_mult:     [ ( score: 0.0, output: 1.0 ), ( score: 100.0, output: 0.5 ) ],
            recoil_growth: [ ( score: 0.0, output: 1.0 ), ( score: 100.0, output: 0.25 ) ],
        ),
        aim_mode: ( cone_mult: 0.6, tu_premium: 1.5 ),
        recoil_climb: 0.01,
        concentration: ( base: 1.0, scale: 1.0 ),
        aim_height_frac: 1.0,
        muzzle_forward_offset: 0.3,
        muzzle_heights: ( prone: 0.15, kneel: 0.45, stand: 0.8 ),
        silhouette_tops: ( prone: 0.3, kneel: 0.6, stand: 0.95 ),
    )";
    let parsed = ron::from_str::<ConeStabilityTuning>(ron);
    assert!(
        parsed.is_ok(),
        "the cone/stability tuning extension must deserialize from bare-scalar RON: {parsed:?}",
    );
}

/// The shipped `assets/combat/tuning.ron` deserializes into a
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
        "/../../assets/combat/tuning.ron"
    ));

    let parsed = ron::from_str::<CombatTuning>(SHIPPED_TUNING_RON);
    assert!(
        parsed.is_ok(),
        "shipped assets/combat/tuning.ron must deserialize into CombatTuning: {parsed:?}",
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
}

/// GTW-234 AC8 — the `MoveCosts` table round-trips through serde (the `tuning.ron`
/// shape: one bare `#[serde(transparent)]` scalar per terrain) and its
/// `cost(terrain)` accessor maps each [`TerrainKind`] variant to its corresponding
/// authored field.
///
/// Value-agnostic on the magnitudes (the per-terrain move costs are tunable balance
/// data): it uses arbitrary, mutually-distinct literals — never the shipped/default
/// values — so the assertion proves only that the leaf round-trips and the accessor
/// dispatches each variant to the right field (distinct values prove no field
/// aliasing), not any balance number.
#[test]
fn move_costs_round_trips_and_cost_accessor_maps_each_variant() {
    // Arbitrary, mutually-distinct literals (never the defaults) — distinctness is
    // what proves the accessor routes each variant to a DIFFERENT field.
    let ron = "( open: 3, cover: 7, wall: 9 )";
    let parsed = ron::from_str::<MoveCosts>(ron);
    assert!(
        parsed.is_ok(),
        "MoveCosts must deserialize from the bare-scalar RON shape: {parsed:?}",
    );
    let Ok(costs) = parsed else {
        return;
    };

    // The accessor maps each variant to its authored field (value-agnostic — it
    // compares the accessor's result to the parsed field, not to a literal).
    assert_eq!(
        costs.cost(TerrainKind::Open),
        costs.open,
        "cost(Open) must return the open field",
    );
    assert_eq!(
        costs.cost(TerrainKind::Cover),
        costs.cover,
        "cost(Cover) must return the cover field",
    );
    assert_eq!(
        costs.cost(TerrainKind::Wall),
        costs.wall,
        "cost(Wall) must return the wall field",
    );

    // The fields parsed to the distinct authored values (proving the round-trip read
    // each leaf into the right slot — the inner u8 reachable via the derived Deref).
    assert_eq!(*costs.open, 3u8, "open parsed to its authored leaf");
    assert_eq!(*costs.cover, 7u8, "cover parsed to its authored leaf");
    assert_eq!(*costs.wall, 9u8, "wall parsed to its authored leaf");
}

/// GTW-194 AC6 — `StanceChangeTu` and `TurnTu` each deserialize from a
/// hand-written bare-scalar RON fragment (`#[serde(transparent)]`) and read back
/// through their derived [`Deref`]. Value-agnostic: arbitrary literals (never the
/// shipped/default magnitudes) prove only that the leaves parse into their newtypes
/// and the inner `u8` is reachable — the costs themselves are tunable balance data.
#[test]
fn posture_tu_leaves_parse_from_bare_scalar_ron() {
    let stance = ron::from_str::<StanceChangeTu>("7");
    assert_eq!(
        stance,
        Ok(StanceChangeTu(7)),
        "StanceChangeTu must parse from a bare RON scalar: {stance:?}",
    );
    // Read the parsed value back through the derived Deref (arbitrary magnitude).
    if let Ok(parsed) = stance {
        assert_eq!(*parsed, 7u8, "StanceChangeTu derefs to its inner u8");
    }

    let turn = ron::from_str::<TurnTu>("3");
    assert_eq!(
        turn,
        Ok(TurnTu(3)),
        "TurnTu must parse from a bare RON scalar: {turn:?}",
    );
    if let Ok(parsed) = turn {
        assert_eq!(*parsed, 3u8, "TurnTu derefs to its inner u8");
    }
}
