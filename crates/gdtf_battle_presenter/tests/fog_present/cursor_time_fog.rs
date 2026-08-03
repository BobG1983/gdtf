use bevy::{app::App, ecs::system::RunSystemOnce, prelude::Visibility};
use gdtf_battle_presenter::{
    PlaybackCursor, present_fog, promote_shown_fog, resolve_ganger_visibility,
};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, RecordedAct},
    ganger::Faction,
    prelude::{Cell, CellLevel, Direction, Level},
    test_support::SituationBuilder,
};

use super::harness::*;

fn run<M>(app: &mut App, system: impl bevy::ecs::system::IntoSystem<(), (), M>, what: &str) {
    let ran = app.world_mut().run_system_once(system);
    assert!(ran.is_ok(), "the {what} system must run in the fixture");
}

fn open_gate(app: &mut App) {
    app.world_mut().insert_resource(ActLog::default());
    let Some(mut cursor) = app.world_mut().get_resource_mut::<PlaybackCursor>() else {
        unreachable!("the renderer plugin inits the cursor");
    };
    cursor.reset();
}

fn close_gate(app: &mut App) {
    let entity = app.world_mut().spawn_empty().id();
    let Some(mut log) = app.world_mut().get_resource_mut::<ActLog>() else {
        unreachable!("open_gate inserted an act log");
    };
    log.append(RecordedAct::new(
        entity,
        ActProvenance::Clock,
        ActDeed::TurnBegan {
            now_active: Faction::new(0),
        },
    ));
}

#[test]
fn revealed_reactor_is_shown_while_terrain_fog_stays_frozen() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let player_cell = CellLevel::new(Cell::new(5, 5), l0);
    let reactor_cell = CellLevel::new(Cell::new(6, 6), l0);

    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(player_cell, 0, Direction::East))
        .with_ganger(ganger_at(reactor_cell, 1, Direction::East))
        .player_faction(Faction::new(0))
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");
    assert!(
        settle_terrain_at(&mut app, reactor_cell),
        "the reactor's terrain tile must have drawn",
    );
    let reactor = sim_entity_at(&mut app, reactor_cell);
    assert!(reactor.is_some(), "the reactor sim ganger must exist");
    assert!(
        settle_actor(&mut app, reactor),
        "the reactor sprite must map"
    );

    open_gate(&mut app);
    set_fog(&mut app, &[player_cell], &[]);
    run(&mut app, promote_shown_fog, "promote_shown_fog");
    run(&mut app, present_fog, "present_fog");
    run(
        &mut app,
        resolve_ganger_visibility,
        "resolve_ganger_visibility",
    );
    assert_eq!(
        actor_visibility(&mut app, reactor),
        Some(Visibility::Hidden),
        "before the reveal, the fog-hidden reactor sprite is Hidden",
    );

    close_gate(&mut app);
    set_fog(&mut app, &[player_cell, reactor_cell], &[]);
    run(&mut app, promote_shown_fog, "promote_shown_fog");
    run(&mut app, present_fog, "present_fog");
    run(
        &mut app,
        resolve_ganger_visibility,
        "resolve_ganger_visibility",
    );

    assert_eq!(
        actor_visibility(&mut app, reactor),
        Some(Visibility::Inherited),
        "the reactor revealed mid-walk is shown even while the gate is closed \
         (resolve_ganger_visibility reads LIVE fog — the invisible-shooter guard)",
    );
    let reactor_terrain = terrain_at(&mut app, reactor_cell);
    assert_eq!(
        reactor_terrain.map(|(_, flag)| flag),
        Some(Visibility::Hidden),
        "the reactor's terrain tile stays fogged (frozen shadow) during closed-gate playback",
    );
}
