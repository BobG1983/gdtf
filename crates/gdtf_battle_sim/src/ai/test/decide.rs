use bevy::prelude::World;

use crate::{
    acts::can_engage,
    ai::{AiTarget, pick_nearest, plan_advance},
    ganger::{Direction, Tu},
    metric::{Cell, CellLevel, Level},
    tuning::CombatTuning,
};

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

#[test]
fn pick_nearest_returns_the_chebyshev_closest() {
    let mut world = World::new();
    let near = world.spawn_empty().id();
    let far = world.spawn_empty().id();
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

#[test]
fn pick_nearest_tie_breaks_on_level_gap() {
    let mut world = World::new();
    let same_storey = world.spawn_empty().id();
    let one_up = world.spawn_empty().id();
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

#[test]
fn pick_nearest_empty_is_none() {
    assert_eq!(
        pick_nearest(Cell::new(0, 0), Level::new(0), &[]),
        None,
        "no candidates → no pick",
    );
}

#[test]
fn plan_advance_steps_toward_the_goal() {
    let start = ground(2, 5);
    let goal = Cell::new(40, 5);
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

#[test]
fn plan_advance_holds_when_no_cell_gets_closer() {
    let start = ground(10, 5);
    let goal = Cell::new(0, 5);
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

#[test]
fn can_engage_in_arc_is_true() {
    let tuning = CombatTuning::default();
    assert!(
        *can_engage(
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

#[test]
fn can_engage_out_of_arc_unaffordable_is_false() {
    let tuning = CombatTuning::default();
    assert!(
        !*can_engage(
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

#[test]
fn can_engage_out_of_arc_affordable_is_true() {
    let tuning = CombatTuning::default();
    assert!(
        *can_engage(
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
