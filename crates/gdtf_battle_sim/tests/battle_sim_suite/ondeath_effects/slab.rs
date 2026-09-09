use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog},
    acts::FireRequested,
    effects::fields::FieldRegistry,
    ganger::Direction,
    metric::{Cell, CellLevel, Level},
    surface::{SlabState, SurfaceGrid},
    test_support::{SituationBuilder, single_mode, test_pieces},
};

use super::harness::*;

/// The cell directly over the shooter's head, so a round fired up strikes it.
fn overhead() -> CellLevel {
    CellLevel::new(Cell::new(5, 5), Level::new(1))
}

fn slab_state(app: &App, at: CellLevel) -> SlabState {
    app.world()
        .get_resource::<SurfaceGrid>()
        .map_or(SlabState::Absent, |grid| grid.slab_state(&at))
}

fn field_present(app: &App, at: CellLevel) -> bool {
    app.world()
        .get_resource::<FieldRegistry>()
        .is_some_and(|r| r.field_at(&at).is_some())
}

fn died_at(app: &App, at: CellLevel) -> bool {
    app.world().get_resource::<ActLog>().is_some_and(|log| {
        log.since(log.oldest_seq())
            .any(|entry| matches!(entry.deed(), ActDeed::DiedAt { at: cell } if *cell == at))
    })
}

/// Fire up at the overhead slab until it is destroyed or the cap runs out.
fn fire_until_destroyed(app: &mut App, shooter: Entity) {
    assert_eq!(
        slab_state(app, overhead()),
        SlabState::Present,
        "the slab must be standing before the firing loop, or its exit test reads Absent on the \
         first pass and the case asserts nothing",
    );
    loop {
        if slab_state(app, overhead()) == SlabState::Absent {
            return;
        }
        app.world_mut().write_message(FireRequested::new(
            shooter,
            single_mode(0.2, 1),
            Cell::new(5, 5),
            Level::new(1),
        ));
        step(app, 4);
    }
}

#[test]
fn a_destroyed_slab_leaves_its_authored_field_at_its_cell() {
    let (mut app, seed) = battle_app(0x5148_0F0F, true);

    let situation = SituationBuilder::new()
        .with_gangers([shooter(ground(5, 5), Direction::East)])
        .slab_piece_at(overhead(), FUEL_SLAB)
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let Some(shooter_e) = ganger_at(&mut app, ground(5, 5)) else {
        unreachable!("setup spawns the shooter at (5,5)");
    };
    assert!(
        !field_present(&app, overhead()),
        "no field stands at the slab cell before it is destroyed"
    );

    fire_until_destroyed(&mut app, shooter_e);

    assert_eq!(
        slab_state(&app, overhead()),
        SlabState::Absent,
        "the slab must be destroyed before its on-death effect can be judged; it reads {:?}",
        slab_state(&app, overhead()),
    );
    assert!(
        field_present(&app, overhead()),
        "the destroyed slab's authored LeaveField did not fan — no field at the slab cell"
    );
}

#[test]
fn a_destroyed_slab_records_a_died_at_for_its_cell() {
    let (mut app, seed) = battle_app(0x5148_0A0A, true);

    let situation = SituationBuilder::new()
        .with_gangers([shooter(ground(5, 5), Direction::East)])
        .slab_piece_at(overhead(), test_pieces::SLAB)
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let Some(shooter_e) = ganger_at(&mut app, ground(5, 5)) else {
        unreachable!("setup spawns the shooter at (5,5)");
    };

    fire_until_destroyed(&mut app, shooter_e);

    assert_eq!(
        slab_state(&app, overhead()),
        SlabState::Absent,
        "the slab must be destroyed before its death record can be judged; it reads {:?}",
        slab_state(&app, overhead()),
    );
    assert!(
        died_at(&app, overhead()),
        "destroying a slab that authors NO on-death must still record ActDeed::DiedAt at its \
         cell"
    );
}
