//! GTW-889 — the ganger-visibility resolver classifies on the SHOWN life state.
//!
//! Reported symptom: "one of my soldiers disappeared and said lacerated even though the
//! shots themselves hadn't played yet". The resolver read the LIVE `LifeState`, so the
//! instant the sim killed a player ganger his fog relation flipped from `OwnSquad` to
//! `Other`, the fog gate closed over his cell, and the sprite vanished — while the shots
//! that killed him were still queued behind the playback cursor.

use bevy::prelude::{Entity, Visibility};
use gdtf_battle_presenter::GangerSprites;
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Direction, Faction, Level, LifeState},
    test_support::SituationBuilder,
};

use super::{harness::*, probes::*};

/// Kill the ganger in the SIM only — the live `LifeState`, leaving the `DrawnLife` mirror
/// on whatever the cursor last showed.
fn set_live_life(app: &mut bevy::app::App, sim: Entity, life: LifeState) {
    app.world_mut().entity_mut(sim).insert(life);
}

/// A player ganger the SIM has already killed stays SHOWN until his death is PLAYED, and
/// disappears when it is.
///
/// The fog is authored EMPTY, so nothing but the `OwnSquad` relation can keep this sprite
/// visible — which makes the relation the only variable under test. Alive-and-drawn-alive
/// he is shown; sim-dead but not yet played he must STILL be shown; once the cursor plays
/// the death the sprite goes.
///
/// Pin-discriminating: classifying on the live `LifeState` (what the resolver did before
/// GTW-889) hides the sprite at the middle assertion and FAILS.
#[test]
fn a_sim_killed_player_ganger_stays_shown_until_the_death_plays() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let at = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(at, 0, Direction::East))
        .player_faction(Faction::new(0))
        .slab_at(at)
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");

    let sim = sim_entity_at(&mut app, at);
    assert!(sim.is_some(), "the ganger must have spawned");
    assert!(
        settle_actor(&mut app, sim),
        "the ganger sprite must have materialized",
    );

    // Fog with NOTHING visible: only the OwnSquad relation (a live PLAYER ganger) can show
    // this sprite, so every verdict below is about the life state the resolver classifies on.
    set_fog(&mut app, &[], &[]);
    app.update();
    assert_eq!(
        visibility_of_sim(&mut app, sim),
        Some(Visibility::Inherited),
        "a live player ganger is shown regardless of fog (the OwnSquad relation)",
    );

    // The SIM kills him. The cursor has not reached the death, so his DrawnLife is still
    // Alive — and the sprite must stay on screen with the shots still to play.
    let Some(sim_entity) = sim else { return };
    set_live_life(&mut app, sim_entity, LifeState::Dead);
    app.update();
    assert_eq!(
        visibility_of_sim(&mut app, sim),
        Some(Visibility::Inherited),
        "a ganger the SIM has killed stays shown until the cursor plays his death",
    );

    // Now the death PLAYS: the drawn mirror reaches Dead and the sprite goes.
    set_drawn_life(&mut app, sim_entity, LifeState::Dead);
    app.update();
    let still_mapped = app
        .world()
        .get_resource::<GangerSprites>()
        .map(|sprites| sprites.contains(sim_entity));
    assert_eq!(
        still_mapped,
        Some(false),
        "once the death is played the sprite is despawned and unmapped",
    );
}
