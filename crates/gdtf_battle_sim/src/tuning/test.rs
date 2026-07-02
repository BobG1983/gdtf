//! Relocated unit tests for the `tuning` data dir-module (GTW-201 code-health
//! wave) — the inline `#[cfg(test)] mod tests` moved VERBATIM. Behavior-preserving
//! move: same assertions, same count (6 tests).

use super::*;
use crate::{cover::HeightBand, occupancy::TerrainKind};

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
    assert_eq!((*BandEdge::new(5.0)).to_bits(), 5.0_f32.to_bits());
    assert_eq!((*PenDamageScale::new(2.5)).to_bits(), 2.5_f32.to_bits());
    assert_eq!(
        (*ToughnessMitigation::new(3.5)).to_bits(),
        3.5_f32.to_bits()
    );
    assert_eq!((*ShooterLuckScale::new(4.5)).to_bits(), 4.5_f32.to_bits());
    assert_eq!((*DefenderLuckScale::new(6.5)).to_bits(), 6.5_f32.to_bits());
    assert_eq!((*RandomSpread::new(7.5)).to_bits(), 7.5_f32.to_bits());
    assert_eq!((*SeverityEdge::new(8.5)).to_bits(), 8.5_f32.to_bits());
    // The u16 newtype: deref reaches the inner u16 (arbitrary value).
    assert_eq!(*BodyPartWeight::new(3), 3u16);
    // The u8 bleed-out rate: deref reaches the inner u8 (arbitrary value, the
    // mechanism not the shipped magnitude).
    assert_eq!(*BleedRate::new(4), 4u8);
    // GTW-242 — the firing-arc f32 newtype: deref reaches the inner degrees (bit-exact
    // arbitrary value, never the 120° default — the Deref mechanism, not a magnitude).
    assert_eq!((*FiringArc::new(75.0)).to_bits(), 75.0_f32.to_bits());
    // GTW-338 — the view-range u16 newtype: deref reaches the inner Chebyshev cell count
    // (arbitrary value, never the 14 default — the Deref mechanism, not a magnitude).
    assert_eq!(*ViewRange::new(9), 9u16);
    // GTW-338 — the explored-dim f32 newtype: deref reaches the inner modulate factor
    // (bit-exact arbitrary value, never the 0.55 default — the Deref mechanism only).
    assert_eq!((*ExploredDim::new(0.25)).to_bits(), 0.25_f32.to_bits());
    // GTW-349 — the per-link traversal u8 newtype: deref reaches the inner TU count
    // (arbitrary value, never the default — the Deref mechanism, not a magnitude).
    assert_eq!(*LinkTu::new(5), 5u8);
}

/// C3 — every E2.1 cone/stability/recoil/aim extension leaf wraps the right
/// inner type and its derived [`Deref`] reaches it. Built from **arbitrary**
/// literals (never the shipped/default magnitudes), so this pins the Deref
/// mechanism + target type, not a balance value (bit-exact f32 equality on
/// exactly-representable literals — no `float_cmp` lint).
#[test]
fn cone_stability_newtypes_wrap_inner_and_deref() {
    assert_eq!(
        (*StanceContribution::new(11.0)).to_bits(),
        11.0_f32.to_bits()
    );
    assert_eq!(
        (*BraceContribution::new(22.0)).to_bits(),
        22.0_f32.to_bits()
    );
    assert_eq!(
        (*StabilityCurveCoord::new(33.0)).to_bits(),
        33.0_f32.to_bits()
    );
    assert_eq!((*AimConeMult::new(0.25)).to_bits(), 0.25_f32.to_bits());
    assert_eq!((*AimTuPremium::new(1.25)).to_bits(), 1.25_f32.to_bits());
    assert_eq!((*RecoilClimb::new(0.5)).to_bits(), 0.5_f32.to_bits());
    assert_eq!((*ConcentrationCoeff::new(2.5)).to_bits(), 2.5_f32.to_bits());
    assert_eq!((*AimHeightFrac::new(0.75)).to_bits(), 0.75_f32.to_bits());
    assert_eq!(
        (*MuzzleForwardOffset::new(0.125)).to_bits(),
        0.125_f32.to_bits()
    );
    assert_eq!((*MuzzleHeight::new(0.625)).to_bits(), 0.625_f32.to_bits());
    assert_eq!((*SilhouetteTop::new(0.875)).to_bits(), 0.875_f32.to_bits());

    // The curve newtype derefs to its inner Vec (arbitrary one-point shape).
    let curve = StabilityCurve::new(vec![StabilityCurvePoint {
        score:  StabilityCurveCoord::new(50.0),
        output: StabilityCurveCoord::new(0.7),
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
        sight_stability_bonus: 15.0,
        emplacement_stability_bonus: 40.0,
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
        Ok(StanceChangeTu::new(7)),
        "StanceChangeTu must parse from a bare RON scalar: {stance:?}",
    );
    // Read the parsed value back through the derived Deref (arbitrary magnitude).
    if let Ok(parsed) = stance {
        assert_eq!(*parsed, 7u8, "StanceChangeTu derefs to its inner u8");
    }

    let turn = ron::from_str::<TurnTu>("3");
    assert_eq!(
        turn,
        Ok(TurnTu::new(3)),
        "TurnTu must parse from a bare RON scalar: {turn:?}",
    );
    if let Ok(parsed) = turn {
        assert_eq!(*parsed, 3u8, "TurnTu derefs to its inner u8");
    }
}

/// GTW-338 — the two squad-FOV visibility leaves resolve to their **documented
/// defaults** on the const-fallback `CombatTuning::default()` path, so a headless
/// fixture with NO loaded asset still gets a real view range and explored dim
/// (the same fallback the failed-load and asset-less harnesses rely on).
///
/// Unlike the real-asset loader tests (value-agnostic by the brittle-test rule), this
/// pins the `Default` impl this ticket authored — the defaults ARE the design contract
/// (visibility.md §"Tunables": `view_range` 14, `explored_dim` 0.55). The check is
/// exact: view range is an integer count; the dim's `f32` is an exactly-representable
/// literal compared by bit pattern (no `float_cmp` lint, no epsilon).
#[test]
fn visibility_leaves_resolve_to_documented_defaults() {
    let tuning = CombatTuning::default();
    assert_eq!(
        *tuning.view_range, 14u16,
        "CombatTuning::default view_range must be the documented 14 Chebyshev cells",
    );
    assert_eq!(
        (*tuning.explored_dim).to_bits(),
        0.55_f32.to_bits(),
        "CombatTuning::default explored_dim must be the documented 0.55 RGB modulate",
    );
}

/// GTW-301 — every touched tuning sub-file's leaf newtype still **deserializes
/// through its now-private inner**: each parses from a bare-scalar RON literal
/// (`#[serde(transparent)]`) and the value read back through the derived [`Deref`]
/// equals the parsed input.
///
/// This is the privatization regression guard: with the inner field private,
/// `#[derive(Deserialize)]` on a `#[serde(transparent)]` newtype must still build
/// the value (the derive constructs through the wrapper, not by writing the field),
/// so a leaf that failed to round-trip would mean the private inner broke serde. One
/// representative leaf per touched sub-file (`severity` / `cone` / `economy` /
/// `body_part` / `wounds` / `band`). Value-agnostic: arbitrary, mutually-distinct literals (never the
/// shipped/default magnitudes) — it pins the serde-through-private-inner MECHANISM,
/// not a balance number. The `f32` leaves compare by bit pattern (the literals are
/// exactly representable, so this is exact integer equality — no `float_cmp` lint).
#[test]
fn touched_leaves_round_trip_through_private_inner() {
    // severity.rs
    let pen = ron::from_str::<PenDamageScale>("2.5");
    assert_eq!(pen, Ok(PenDamageScale::new(2.5)), "PenDamageScale: {pen:?}");
    if let Ok(p) = pen {
        assert_eq!((*p).to_bits(), 2.5_f32.to_bits(), "PenDamageScale deref");
    }
    let edge = ron::from_str::<SeverityEdge>("8.5");
    assert_eq!(edge, Ok(SeverityEdge::new(8.5)), "SeverityEdge: {edge:?}");
    if let Ok(e) = edge {
        assert_eq!((*e).to_bits(), 8.5_f32.to_bits(), "SeverityEdge deref");
    }

    // cone.rs
    let aim = ron::from_str::<AimConeMult>("0.25");
    assert_eq!(aim, Ok(AimConeMult::new(0.25)), "AimConeMult: {aim:?}");
    if let Ok(a) = aim {
        assert_eq!((*a).to_bits(), 0.25_f32.to_bits(), "AimConeMult deref");
    }

    // economy.rs
    let stance = ron::from_str::<StanceChangeTu>("7");
    assert_eq!(
        stance,
        Ok(StanceChangeTu::new(7)),
        "StanceChangeTu: {stance:?}"
    );
    if let Ok(s) = stance {
        assert_eq!(*s, 7u8, "StanceChangeTu deref");
    }

    // economy.rs (GTW-349) — the u8 per-link traversal cost round-trips through its
    // private inner (a bare `#[serde(transparent)]` scalar; arbitrary value, never the
    // default — it pins the serde-through-private-inner mechanism, not a magnitude).
    let link = ron::from_str::<LinkTu>("5");
    assert_eq!(link, Ok(LinkTu::new(5)), "LinkTu: {link:?}");
    if let Ok(l) = link {
        assert_eq!(*l, 5u8, "LinkTu deref");
    }

    // body_part.rs
    let weight = ron::from_str::<BodyPartWeight>("3");
    assert_eq!(
        weight,
        Ok(BodyPartWeight::new(3)),
        "BodyPartWeight: {weight:?}"
    );
    if let Ok(w) = weight {
        assert_eq!(*w, 3u16, "BodyPartWeight deref");
    }

    // wounds.rs
    let bleed = ron::from_str::<BleedRate>("4");
    assert_eq!(bleed, Ok(BleedRate::new(4)), "BleedRate: {bleed:?}");
    if let Ok(b) = bleed {
        assert_eq!(*b, 4u8, "BleedRate deref");
    }

    // band.rs
    let band = ron::from_str::<BandEdge>("5.0");
    assert_eq!(band, Ok(BandEdge::new(5.0)), "BandEdge: {band:?}");
    if let Ok(be) = band {
        assert_eq!((*be).to_bits(), 5.0_f32.to_bits(), "BandEdge deref");
    }

    // visibility.rs (GTW-338) — the u16 view range round-trips through its private inner.
    let view = ron::from_str::<ViewRange>("9");
    assert_eq!(view, Ok(ViewRange::new(9)), "ViewRange: {view:?}");
    if let Ok(v) = view {
        assert_eq!(*v, 9u16, "ViewRange deref");
    }

    // visibility.rs (GTW-338) — the f32 explored dim round-trips through its private inner.
    let dim = ron::from_str::<ExploredDim>("0.25");
    assert_eq!(dim, Ok(ExploredDim::new(0.25)), "ExploredDim: {dim:?}");
    if let Ok(d) = dim {
        assert_eq!((*d).to_bits(), 0.25_f32.to_bits(), "ExploredDim deref");
    }
}
