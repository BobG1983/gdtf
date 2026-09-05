//! `recompute_visibility`'s own predicate, driven through the system.

use bevy::ecs::system::RunSystemOnce;

use super::{super::recompute::recompute_visibility, support::*};
use crate::{battle::PlayerFaction, ganger::Faction, visibility::CellVisible};

const PLAYER: u8 = 0;
const ENEMY: u8 = 1;

fn observer_cell() -> CellLevel {
    key(2, 5, 0)
}

fn occluder_cell() -> CellLevel {
    key(3, 5, 0)
}

fn far_cell() -> CellLevel {
    key(8, 5, 0)
}

// The occluder is published at the standing band, as the grid holds it before the sync republishes.
fn far_cell_visible(occluder_life: LifeState) -> Option<CellVisible> {
    let mut world = World::new();

    let occluder = world
        .spawn((
            position(3, 5, 0),
            stance(StanceKind::Standing),
            facing(Direction::East),
            occluder_life,
            Faction::new(ENEMY),
        ))
        .id();
    world.spawn((
        position(2, 5, 0),
        stance(StanceKind::Standing),
        facing(Direction::East),
        LifeState::Alive,
        Faction::new(PLAYER),
    ));

    let mut occupancy = OccupancyGrid::new();
    place_occupant(&mut occupancy, occluder_cell(), occluder, HeightBand::High);

    world.insert_resource(occupancy);
    world.insert_resource(SurfaceGrid::new());
    world.insert_resource(CoverLedger::new());
    world.insert_resource(CombatTuning::default());
    world.insert_resource(PlayerFaction::new(Faction::new(PLAYER)));
    world.insert_resource(SquadVisibility::default());

    world.run_system_once(recompute_visibility).ok()?;
    Some(
        world
            .get_resource::<SquadVisibility>()?
            .is_cell_visible(&far_cell()),
    )
}

#[test]
fn a_downed_occluder_is_seen_past_and_a_living_one_is_not() {
    assert_eq!(
        far_cell_visible(LifeState::Downed).map(|visible| *visible),
        Some(true),
        "with the occluder downed: the observer at {:?} is standing, so its ray crosses {:?} at \
         the HIGH band. The grid still publishes that occupant at HIGH, so the only thing that \
         can clear the ray is the predicate naming a downed ganger, and {:?} must come back \
         visible",
        observer_cell(),
        occluder_cell(),
        far_cell(),
    );
    assert_eq!(
        far_cell_visible(LifeState::Alive).map(|visible| *visible),
        Some(false),
        "with the occluder alive: the same layout with a living occupant at {:?} is tested at its \
         published HIGH band, the HIGH ray impacts it, and {:?} must stay out of the visible set",
        occluder_cell(),
        far_cell(),
    );
}
