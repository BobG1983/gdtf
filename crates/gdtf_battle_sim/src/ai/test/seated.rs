use bevy::prelude::Entity;

use super::support::{
    ENEMY, PLAYER, active_of, brain_app, drain_moves, ground, place_occupant, spawn_combatant,
};
use crate::{
    acts::{dismount_surcharge, seat_departure},
    ai::plan_advance,
    ganger::{Direction, Tu},
    injuries::MovementCostFactor,
    metric::CellLevel,
    occupancy::OccupancyGrid,
    pathfinder::{Departure, MoveGrids, PlanningView, reachable_within},
    terrain::{
        emplacement::{EmplacementEntrySides, EmplacementFacing, Mounted, MountedBy},
        entity::TerrainCell,
        facing::TerrainFacing,
        floor::FloorCostGrid,
    },
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::{FactionRelation, OmniscientFog},
};

const FRAME_CAP: usize = 80;

// TU the seated enemy carries: enough for a short advance once the exit is paid.
const ENEMY_TU: u8 = 20;

#[test]
fn a_seated_enemy_advances_only_where_its_entry_side_lets_it_go() {
    let mut app = brain_app();
    let seat_cell = ground(2, 5);
    let player_at = ground(40, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        seat_cell,
        ENEMY,
        Direction::East,
        ENEMY_TU,
        6,
    );
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);

    let sides = EmplacementEntrySides::new(vec![TerrainFacing::West]);
    let facing = EmplacementFacing::new(TerrainFacing::default());
    app.world_mut().spawn((
        TerrainCell::new(seat_cell),
        MountedBy::new(enemy),
        sides.clone(),
        facing,
    ));

    assert!(
        app.world().get::<Mounted>(enemy).is_some(),
        "the seat's MountedBy must give the enemy its Mounted before the turn is planned",
    );

    let seated = reachable_for(
        &app,
        enemy,
        &seat_departure(seat_cell, seat_cell, Some(&sides), Some(&facing)),
    );
    let anywhere = reachable_for(&app, enemy, &Departure::anywhere(seat_cell));

    let mut requested: Option<CellLevel> = None;
    for _ in 0..FRAME_CAP {
        app.update();
        if let Some(step) = drain_moves(&mut app)
            .into_iter()
            .find(|step| step.actor == enemy)
        {
            requested = Some(step.dest);
            break;
        }
        if active_of(&app) == PLAYER {
            break;
        }
    }

    assert!(
        requested.is_some_and(|dest| seated.iter().any(|(offered, _)| *offered == dest)),
        "the seated enemy asked to move to {requested:?}, which its one entry side does not \
         reach; the seat-constrained set is {seated:?}",
    );

    let unconstrained = plan_advance(seat_cell, player_at.cell(), &anywhere);
    assert!(
        unconstrained.is_some_and(|dest| !seated.iter().any(|(offered, _)| *offered == dest)),
        "the fixture must discriminate: planning from anywhere picks {unconstrained:?}, which \
         the seat-constrained set {seated:?} must not hold",
    );
}

// The cells this enemy can reach for a given departure, costed the way its turn costs them.
fn reachable_for(
    app: &bevy::prelude::App,
    enemy: Entity,
    departure: &Departure,
) -> Vec<(CellLevel, Tu)> {
    let world = app.world();
    let grid = world.resource::<OccupancyGrid>().clone();
    let links = world.resource::<VerticalLinkGraph>().clone();
    let floor_costs = world.resource::<FloorCostGrid>().clone();
    let tuning = world.resource::<CombatTuning>().clone();
    let fog = world.resource::<OmniscientFog>().clone();
    let budget = world.get::<Tu>(enemy).copied().unwrap_or(Tu::new(0));
    let surcharge = dismount_surcharge(world.get::<Mounted>(enemy), &tuning);
    let planning = PlanningView::new(&fog, |occupant: Entity| {
        if occupant == enemy {
            FactionRelation::OwnSquad
        } else {
            FactionRelation::Other
        }
    });
    reachable_within(
        departure,
        budget,
        surcharge,
        MoveGrids {
            occupancy:   &grid,
            links:       &links,
            floor_costs: &floor_costs,
            tuning:      &tuning,
        },
        MovementCostFactor::IDENTITY,
        &planning,
    )
}
