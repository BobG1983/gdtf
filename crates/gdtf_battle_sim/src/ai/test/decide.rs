//! Pure-function unit tests for the AI decision core (GTW-70 §C / §D.2 / leaf 2) — the
//! engagement gate ([`can_engage`]), the target pick ([`pick_nearest`]), and the reposition
//! step ([`plan_advance`]). No `App`, no ECS — `World::new()` only mints distinct
//! [`Entity`] handles (the `bevy-traps.md` #7 carve-out (b) for a pure-sim unit test).

use bevy::prelude::World;

use crate::{
    acts::can_engage,
    ai::{AiTarget, pick_nearest, plan_advance},
    ganger::{Direction, Tu},
    metric::{Cell, CellLevel, Level},
    tuning::CombatTuning,
};

/// A ground cell `(x, y, 0)`.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// [`pick_nearest`] returns the minimum-Chebyshev candidate, independent of the slice order
/// (§C) — the far candidate listed first does not win.
#[test]
fn pick_nearest_returns_the_chebyshev_closest() {
    let mut world = World::new();
    let near = world.spawn_empty().id();
    let far = world.spawn_empty().id();
    // `far` listed FIRST, to prove the pick does not depend on slice/Entity order.
    let candidates = [
        AiTarget::new(far, Cell::new(15, 5), Level::new(0)),
        AiTarget::new(near, Cell::new(8, 5), Level::new(0)),
    ];
    let picked = pick_nearest(Cell::new(5, 5), Level::new(0), &candidates);
    assert_eq!(
        picked.map(|t| t.entity),
        Some(near),
        "pick_nearest must return the 2D-Chebyshev closest candidate (8,5 over 15,5), not the \
         first in the slice",
    );
}

/// [`pick_nearest`] breaks an equal-Chebyshev tie by the smaller `|Δlevel|` (§C tie-break 1)
/// — a same-storey target beats a one-storey-up target at the same planar distance.
#[test]
fn pick_nearest_tie_breaks_on_level_gap() {
    let mut world = World::new();
    let same_storey = world.spawn_empty().id();
    let one_up = world.spawn_empty().id();
    // Both at planar Chebyshev 3 from (5,5); the one-storey-up listed first.
    let candidates = [
        AiTarget::new(one_up, Cell::new(8, 5), Level::new(1)),
        AiTarget::new(same_storey, Cell::new(8, 5), Level::new(0)),
    ];
    let picked = pick_nearest(Cell::new(5, 5), Level::new(0), &candidates);
    assert_eq!(
        picked.map(|t| t.entity),
        Some(same_storey),
        "an equal-Chebyshev tie breaks on the smaller |Δlevel| (same storey over one up)",
    );
}

/// [`pick_nearest`] over an empty candidate list is [`None`] (no opposing ganger to engage).
#[test]
fn pick_nearest_empty_is_none() {
    assert_eq!(
        pick_nearest(Cell::new(0, 0), Level::new(0), &[]),
        None,
        "no candidates → no pick",
    );
}

/// [`plan_advance`] returns the reachable cell that most reduces the Chebyshev distance to
/// the goal (§D.2) — the cell closest to the goal among the reachable set.
#[test]
fn plan_advance_steps_toward_the_goal() {
    let start = ground(2, 5);
    let goal = Cell::new(40, 5);
    // Reachable cells east + west of the start; east reduces distance to the (east) goal.
    let reachable = [
        (ground(2, 5), Tu::new(0)),
        (ground(1, 5), Tu::new(4)),
        (ground(3, 5), Tu::new(4)),
        (ground(4, 5), Tu::new(8)),
    ];
    assert_eq!(
        plan_advance(start, goal, &reachable),
        Some(ground(4, 5)),
        "plan_advance must pick the reachable cell closest to the goal (4,5 — furthest east)",
    );
}

/// [`plan_advance`] HOLDs ([`None`]) when no reachable cell strictly reduces the distance —
/// here the start itself is already the closest reachable cell to the goal (§B clause 2 /
/// §D.2 "strictly reduces distance").
#[test]
fn plan_advance_holds_when_no_cell_gets_closer() {
    let start = ground(10, 5);
    let goal = Cell::new(0, 5);
    // Every reachable cell is at the start or FURTHER from the (west) goal.
    let reachable = [
        (ground(10, 5), Tu::new(0)),
        (ground(11, 5), Tu::new(4)),
        (ground(12, 5), Tu::new(8)),
    ];
    assert_eq!(
        plan_advance(start, goal, &reachable),
        None,
        "no reachable cell strictly reduces the distance → HOLD (None)",
    );
}

/// [`can_engage`] — an IN-ARC target is engageable regardless of the turn cost (the shot
/// fires directly), so the boolean arc verdict is `true` (GTW-70 leaf 2 / §B).
#[test]
fn can_engage_in_arc_is_true() {
    let tuning = CombatTuning::default();
    // Facing East, target due East (straight ahead) → in arc.
    assert!(
        can_engage(
            Direction::East,
            Cell::new(0, 0),
            Cell::new(5, 0),
            Tu::new(100),
            Tu::new(10),
            &tuning,
        ),
        "an in-arc target is engageable (the shot fires directly, no turn needed)",
    );
}

/// [`can_engage`] — an OUT-OF-ARC target the shooter CANNOT afford to turn-and-fire is NOT
/// engageable (the `¬Reject` verdict is `false`): the load-bearing termination guard, since
/// the dispatcher would spend no TU on such a shot (GTW-70 leaf 2 self-critique B).
#[test]
fn can_engage_out_of_arc_unaffordable_is_false() {
    let tuning = CombatTuning::default();
    // Facing East, target due WEST (behind) → out of arc; with zero TU the turn-into-arc is
    // unaffordable → Reject → not engageable.
    assert!(
        !can_engage(
            Direction::East,
            Cell::new(0, 0),
            Cell::new(-5, 0),
            Tu::new(0),
            Tu::new(0),
            &tuning,
        ),
        "an out-of-arc target the shooter cannot afford to turn-and-fire is NOT engageable",
    );
}

/// [`can_engage`] — an OUT-OF-ARC target the shooter CAN afford to turn-and-fire IS
/// engageable (the verdict is the affordable [`TurnThenFire`], i.e. `¬Reject`).
#[test]
fn can_engage_out_of_arc_affordable_is_true() {
    let tuning = CombatTuning::default();
    // Same behind-target, but a full TU pool affords the turn-into-arc + the shot.
    assert!(
        can_engage(
            Direction::East,
            Cell::new(0, 0),
            Cell::new(-5, 0),
            Tu::new(255),
            Tu::new(0),
            &tuning,
        ),
        "an out-of-arc target the shooter CAN afford to turn-and-fire is engageable",
    );
}
