//! The move quote, the per-step charges, and the walk all come off one path.

use super::support::*;
use crate::{
    injuries::MovementCostFactor,
    pathfinder::{MoveGrids, Path, PlanningView, find_path},
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, SquadVisibility},
};

fn plan(app: &App, from: CellLevel, to: CellLevel) -> Option<Path> {
    let world = app.world();
    let occupancy = world.get_resource::<OccupancyGrid>()?;
    let links = world.get_resource::<VerticalLinkGraph>()?;
    let floor_costs = world.get_resource::<FloorCostGrid>()?;
    let tuning = world.get_resource::<CombatTuning>()?;
    let squad = world.get_resource::<SquadVisibility>()?;
    let view = PlanningView::new(squad, |_| FactionRelation::Other);
    find_path(
        from,
        to,
        MoveGrids {
            occupancy,
            links,
            floor_costs,
            tuning,
        },
        MovementCostFactor::IDENTITY,
        &view,
    )
    .ok()
}

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn tu_of(app: &App, actor: Entity) -> Option<u8> {
    app.world().get::<Tu>(actor).map(|tu| **tu)
}

#[test]
fn move_step_costs_sum_to_the_move_quote() {
    let app = headless_app();
    let routed = plan(&app, ground(10, 10), ground(13, 12));
    assert!(routed.is_some(), "the test map must route (10,10)->(13,12)");
    let Some(path) = routed else {
        return;
    };

    let stepped: u32 = move_step_tu_costs(&path)
        .iter()
        .map(|step| u32::from(**step))
        .sum();
    assert_eq!(
        stepped,
        u32::from(*move_tu_cost(&path)),
        "the per-step charges the walk pops must sum to the move_tu_cost quote",
    );
}

#[test]
fn a_completed_walk_charges_exactly_the_move_quote() {
    let mut app = headless_app();
    let dest = ground(13, 10);
    let actor = spawn_move_actor(app.world_mut(), 10, 10, 200);
    let routed = plan(&app, ground(10, 10), dest);
    assert!(routed.is_some(), "the test map must route (10,10)->(13,10)");
    let Some(path) = routed else {
        return;
    };
    let quoted = move_tu_cost(&path);
    let before = tu_of(&app, actor);

    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    for _ in 0..6 {
        app.update();
    }

    assert_eq!(
        app.world().get::<Position>(actor).copied(),
        Some(Position::new(dest)),
        "the walk must finish at the requested destination",
    );
    assert_eq!(
        before.zip(tu_of(&app, actor)).map(|(b, a)| b - a),
        Some(*quoted),
        "a completed walk's per-step charges must total exactly move_tu_cost",
    );
}

#[test]
fn can_move_and_the_dispatch_refuse_the_same_pool() {
    let mut app = headless_app();
    let dest = ground(13, 10);
    let routed = plan(&app, ground(10, 10), dest);
    assert!(routed.is_some(), "the test map must route (10,10)->(13,10)");
    let Some(path) = routed else {
        return;
    };
    let quoted = *move_tu_cost(&path);
    assert!(quoted > 0, "a three-cell route costs real TU");

    let at = Position::new(ground(10, 10));
    let empty = CoverLedger::new();
    let terrain = BareTerrain::new();
    let broke = Tu::new(quoted.saturating_sub(1));
    assert_eq!(
        can_move(
            Mover::new(UNSPAWNED_MOVER, &at, &broke, &STANDING, &NORTH, None),
            &dest,
            &path,
            &empty,
            &terrain.sight(),
        ),
        MoveVerdict::Unaffordable,
        "can_move must refuse a pool one TU below the quote",
    );
    assert_eq!(
        can_move(
            Mover::new(
                UNSPAWNED_MOVER,
                &at,
                &Tu::new(quoted),
                &STANDING,
                &NORTH,
                None
            ),
            &dest,
            &path,
            &empty,
            &terrain.sight(),
        ),
        MoveVerdict::Allowed,
        "can_move must allow a pool that exactly covers the quote",
    );

    let actor = spawn_move_actor(app.world_mut(), 10, 10, quoted.saturating_sub(1));
    app.world_mut()
        .write_message(MoveRequested::new(actor, dest));
    app.update();

    let rejects = drain_rejects(&mut app);
    assert_eq!(
        rejects.first().map(|reject| reject.reason),
        Some(MoveRejection::Unaffordable),
        "the dispatch must reject the same pool can_move refuses: {rejects:?}",
    );
    assert_eq!(
        tu_of(&app, actor),
        Some(quoted.saturating_sub(1)),
        "a refused move spends no TU",
    );
}
