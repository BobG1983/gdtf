//! Pure `march_arc` geometry: open-sky landing, roof blocking, roof holes,
//! same-level lobs, purity, and `TrajectoryStyle` serde parsing.

use gdtf_battle_sim::{
    march::{MarchKind, march_arc},
    surface::{SlabState, SurfaceGrid},
    tuning::CombatTuning,
    weapon::{TrajectoryStyle, WeaponSpec},
};

use super::harness::*;

// === Pure `march_arc` geometry unit tests (deterministic, no app). ===

/// A tuning with the default projectile band edges — the arc march reads only the band
/// classification (for the reported band) + the slab surface; the default tuning suffices.
fn arc_tuning() -> CombatTuning {
    CombatTuning::default()
}

#[test]
fn arc_lands_on_the_target_through_open_sky() {
    // Thrower above (level 2) lobbing down onto a level-0 target, NO roof anywhere.
    let surface = SurfaceGrid::new();
    let landing = march_arc(at_level(5, 5, 2), ground(9, 5), &surface, &arc_tuning());
    // With no roof to intercept, the lob lands AT the target cell (cell + storey).
    assert_eq!(
        (landing.at.x, landing.at.y, landing.at.z),
        (9, 5, 0),
        "an unobstructed lob lands at the target cell: {landing:?}",
    );
}

#[test]
fn arc_is_blocked_by_an_intact_roof_between_the_thrower_and_a_lower_target() {
    // Thrower on level 1 lobbing DOWN onto a level-0 target under an INTACT roof at level 1
    // over the target column — the arc crosses the z=1 boundary and is stopped there.
    let mut surface = SurfaceGrid::new();
    surface.set_slab(at_level(6, 5, 1), SlabState::Present);
    let landing = march_arc(at_level(6, 5, 1), ground(6, 5), &surface, &arc_tuning());
    // The lob is stopped at the roof (level 1), NOT at the level-0 target — the target is
    // shielded (AC: arc blocked by intact roofs).
    assert_ne!(
        landing.at.z, 0,
        "an intact roof stops the lob above the target (not on the level-0 target): {landing:?}",
    );
    assert_eq!(
        landing.at.z, 1,
        "the lob lands on the intact roof at level 1: {landing:?}",
    );
}

#[test]
fn arc_passes_through_a_roof_hole_and_lands_on_the_lower_target() {
    // The SAME geometry as the blocked case, but the roof slab is DESTROYED (a hole) — the
    // lob drops through and lands on the level-0 target (AC: passes through holes / windows).
    let mut surface = SurfaceGrid::new();
    surface.set_slab(at_level(6, 5, 1), SlabState::Destroyed);
    let landing = march_arc(at_level(6, 5, 1), ground(6, 5), &surface, &arc_tuning());
    assert_eq!(
        (landing.at.x, landing.at.y, landing.at.z),
        (6, 5, 0),
        "a lob through a roof hole lands on the lower target: {landing:?}",
    );
}

#[test]
fn a_same_level_lob_is_not_self_blocked_by_a_same_level_roof() {
    // A short same-level throw under an intact roof at the storey ABOVE — the lob's apex stays
    // sub-storey, so it never crosses into the roofed level and lands on the target.
    let mut surface = SurfaceGrid::new();
    // A roof over the whole path at level 1.
    for x in 5..=9 {
        surface.set_slab(at_level(x, 5, 1), SlabState::Present);
    }
    let landing = march_arc(ground(5, 5), ground(9, 5), &surface, &arc_tuning());
    assert_eq!(
        (landing.at.x, landing.at.y, landing.at.z),
        (9, 5, 0),
        "a same-level lob clears cover but stays under the same-level roof, landing on target: {landing:?}",
    );
}

// === GTW-645: steep lobs crossing SEVERAL storey boundaries in one sample segment. ===

#[test]
fn a_steep_lob_is_blocked_by_an_intermediate_roof_it_crosses_mid_segment() {
    // GTW-645: a STEEP lob (5 storeys up over 1 cell of horizontal travel) samples at the
    // fixed 0.25-unit horizontal cadence — 4 segments — so its second segment climbs from
    // z≈1.64 to z≈3.02, crossing TWO storey planes (z=2 and z=3) in ONE sample step. The
    // z=2 crossing happens at x≈5.82 (still column 5): an intact roof there must stop the
    // lob. Pre-fix only `max(prev_z, cur_z)` was tested (at the segment-END cell), so the
    // z=2 plane was never tested anywhere and the grenade sailed through the intact slab.
    let mut surface = SurfaceGrid::new();
    surface.set_slab(at_level(5, 5, 2), SlabState::Present);
    let landing = march_arc(ground(5, 5), at_level(6, 5, 5), &surface, &arc_tuning());
    assert_eq!(
        landing.kind,
        MarchKind::Slab,
        "the steep lob is BLOCKED by the intermediate roof, not landed: {landing:?}",
    );
    assert_eq!(
        (landing.at.x, landing.at.y, landing.at.z),
        (5, 5, 2),
        "the lob stops at the z=2 roof where its segment crosses that plane: {landing:?}",
    );
}

#[test]
fn a_vertical_lob_cannot_pass_its_own_intact_ceiling() {
    // GTW-645, the steepest constructible lob: a same-column throw (zero horizontal travel)
    // samples the WHOLE parabola as ONE segment (level 0 -> level 4), crossing four storey
    // planes in a single step. The intact roof DIRECTLY OVER THE THROWER (the z=1 plane,
    // the first crossed in flight order) must stop it. Pre-fix only the top plane (z=4)
    // was tested, so the grenade flew straight through its own intact ceiling.
    let mut surface = SurfaceGrid::new();
    surface.set_slab(at_level(5, 5, 1), SlabState::Present);
    let landing = march_arc(ground(5, 5), at_level(5, 5, 4), &surface, &arc_tuning());
    assert_eq!(
        landing.kind,
        MarchKind::Slab,
        "a vertical lob is BLOCKED by the thrower's own intact ceiling: {landing:?}",
    );
    assert_eq!(
        (landing.at.x, landing.at.y, landing.at.z),
        (5, 5, 1),
        "the lob stops at the FIRST crossed plane (z=1), not a higher one: {landing:?}",
    );
}

#[test]
fn a_steep_drop_stops_at_the_highest_roof_in_flight_order() {
    // GTW-645 flight-order pin (the covered case): a steep DESCENT (5 storeys down over 1
    // cell) crosses z=3 and z=2 in one sample segment. With intact roofs at BOTH planes
    // (authored in both columns, so the pin is float-robust at the column seam), the lob
    // must stop at the HIGHEST plane — the first crossed while falling — never the lower.
    let mut surface = SurfaceGrid::new();
    for x in [5, 6] {
        surface.set_slab(at_level(x, 5, 3), SlabState::Present);
        surface.set_slab(at_level(x, 5, 2), SlabState::Present);
    }
    let landing = march_arc(at_level(5, 5, 5), ground(6, 5), &surface, &arc_tuning());
    assert_eq!(
        landing.kind,
        MarchKind::Slab,
        "the steep drop is BLOCKED by a roof: {landing:?}",
    );
    assert_eq!(
        landing.at.z, 3,
        "the falling lob stops at the FIRST roof it meets (z=3), not the lower z=2: {landing:?}",
    );
}

#[test]
fn march_arc_is_a_pure_function() {
    let mut surface = SurfaceGrid::new();
    surface.set_slab(at_level(6, 5, 1), SlabState::Destroyed);
    let a = march_arc(at_level(6, 5, 2), ground(9, 6), &surface, &arc_tuning());
    let b = march_arc(at_level(6, 5, 2), ground(9, 6), &surface, &arc_tuning());
    assert_eq!(a, b, "march_arc is deterministic (no RNG): {a:?} vs {b:?}");
}

// === TrajectoryStyle serde: default Straight, authored Arc. ===

#[test]
fn trajectory_defaults_to_straight_when_omitted() {
    // An existing-style weapon `.ron` with NO `trajectory:` field parses as Straight (the
    // identity property — every existing weapon is untouched).
    let ron = r"(
        base_spread: 0.1, accuracy: 1.0, kickback: 0.0, fatal_bias: 0.0,
        damage: 5, punch: 1, shred: 0, damage_type: Kinetic,
        magazine: (size: 6, reload_tu: 10),
        fire_mode: [(kind: Single, cone_mult: 1.0, tu_percent: 0.2, shots: 1)],
        stable: false, handedness: OneHanded,
    )";
    let Ok(spec) = ron::from_str::<WeaponSpec>(ron) else {
        unreachable!("the trajectory-less spec parses");
    };
    assert_eq!(
        spec.trajectory,
        TrajectoryStyle::Straight,
        "an omitted trajectory field defaults to Straight",
    );
}

#[test]
fn authored_arc_trajectory_parses() {
    let ron = r"(
        base_spread: 0.2, accuracy: 0.8, kickback: 0.0, fatal_bias: 0.0,
        damage: 8, punch: 2, shred: 1, damage_type: Blast,
        magazine: (size: 2, reload_tu: 18),
        trajectory: Arc,
        fire_mode: [(kind: Single, cone_mult: 1.0, tu_percent: 0.35, shots: 1, hit_type: Blast(radius: 1))],
        stable: false, handedness: OneHanded,
    )";
    let Ok(spec) = ron::from_str::<WeaponSpec>(ron) else {
        unreachable!("the arc grenade spec parses");
    };
    assert_eq!(
        spec.trajectory,
        TrajectoryStyle::Arc,
        "an authored `trajectory: Arc` parses as Arc",
    );
    assert!(spec.trajectory.is_arc(), "the arc weapon reports is_arc()");
}
