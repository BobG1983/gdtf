//! Bare-scalar RON leaf parses + the serde-through-private-inner round-trips
//! (arbitrary literals, never shipped magnitudes).

use super::super::*;
use crate::occupancy::TerrainKind;

/// C5/C3 — the E2.1 extension parses from a hand-written RON fragment with
/// every numeric leaf a **bare scalar** (`#[serde(transparent)]`) and the
/// stability curves authored as bare point lists. Value-agnostic: asserts only
/// structural success (arbitrary literals, never shipped magnitudes).
#[test]
fn cone_stability_parses_from_ron_with_bare_scalar_leaves() {
    let ron = r"(
        stance_stability: ( prone: 40.0, kneel: 25.0, stand: 10.0 ),
        brace_contribution: 30.0,
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
