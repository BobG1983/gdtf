//! GTW-762 clause 5 (the invisible-shooter guard): during closed-gate playback the TERRAIN
//! fog is frozen at the cursor's playback position (it reads the `ShownSquadVisibility`
//! shadow), but a ganger sprite that BECOMES visible in the LIVE fog is STILL shown — because
//! the ganger-visibility resolver is DELIBERATELY exempt and reads the live fog.
//!
//! This is the exact GTW-727 "tracer from an invisible shooter" bug the exemption prevents: a
//! reactor that steps into line of sight and fires must be shown firing, even while the fog
//! terrain around it lags a beat behind the cursor. The X-COM model is binding — a shooter
//! within line of sight is always shown.
//!
//! The systems are driven with `run_system_once` so the gate state is controlled exactly (no
//! wall-clock waits): a fresh empty act log + a reset cursor forces the gate OPEN for the
//! setup promote, then one appended act closes it for the assertion window.

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

/// Run a system once and assert it ran (the fixture guarantees its params exist). Generic
/// over the system's input-marker so each function system's own marker is inferred.
fn run<M>(app: &mut App, system: impl bevy::ecs::system::IntoSystem<(), (), M>, what: &str) {
    let ran = app.world_mut().run_system_once(system);
    assert!(ran.is_ok(), "the {what} system must run in the fixture");
}

/// Force the playback gate OPEN: a fresh empty act log (head at the start) + a reset cursor.
fn open_gate(app: &mut App) {
    app.world_mut().insert_resource(ActLog::default());
    let Some(mut cursor) = app.world_mut().get_resource_mut::<PlaybackCursor>() else {
        unreachable!("the renderer plugin inits the cursor");
    };
    cursor.reset();
}

/// Close the playback gate: append one act so the log head moves past the still-at-start
/// cursor — the closed-gate playback window.
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

/// A reactor that becomes visible DURING a closed-gate playback window is shown as a sprite
/// (the resolver reads LIVE fog) while the terrain fog around it stays frozen (`present_fog`
/// reads the frozen shadow) — the GTW-727 invisible-shooter bug cannot reappear.
#[test]
fn revealed_reactor_is_shown_while_terrain_fog_stays_frozen() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let player_cell = CellLevel::new(Cell::new(5, 5), l0);
    let reactor_cell = CellLevel::new(Cell::new(6, 6), l0);

    // A player ganger (faction 0) and an ENEMY reactor (faction 1); player faction 0 so the
    // reactor is fog-gated (shown only where the fog vouches for its cell).
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

    // BEFORE: the reactor is unseen. Live fog shows only the player; gate open, so the shadow
    // promotes to that (reactor unseen). The reactor sprite is hidden and its terrain fogged.
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

    // The walk reveals the reactor in the LIVE fog, but the gate is now CLOSED (the sim's act
    // is queued for playback). The shadow must stay frozen at the reactor-unseen state.
    close_gate(&mut app);
    set_fog(&mut app, &[player_cell, reactor_cell], &[]);
    run(&mut app, promote_shown_fog, "promote_shown_fog");
    run(&mut app, present_fog, "present_fog");
    run(
        &mut app,
        resolve_ganger_visibility,
        "resolve_ganger_visibility",
    );

    // The SPRITE follows the LIVE fog: the revealed reactor is SHOWN (the exemption — a
    // shooter within line of sight is always shown firing).
    assert_eq!(
        actor_visibility(&mut app, reactor),
        Some(Visibility::Inherited),
        "the reactor revealed mid-walk is shown even while the gate is closed \
         (resolve_ganger_visibility reads LIVE fog — the invisible-shooter guard)",
    );
    // The TERRAIN fog follows the frozen SHADOW: the reactor's cell is still fogged (Hidden),
    // proving present_fog reads cursor-time state, not live state, during the closed window.
    let reactor_terrain = terrain_at(&mut app, reactor_cell);
    assert_eq!(
        reactor_terrain.map(|(_, flag)| flag),
        Some(Visibility::Hidden),
        "the reactor's terrain tile stays fogged (frozen shadow) during closed-gate playback",
    );
}
