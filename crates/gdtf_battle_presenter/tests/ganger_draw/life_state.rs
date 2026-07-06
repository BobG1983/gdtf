//! Downed retint + dead despawn (AC4).

use bevy::{app::App, prelude::Entity};
use gdtf_battle_presenter::GangerSprites;
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Direction, Level, LifeState},
    test_support::SituationBuilder,
};

use super::{harness::*, probes::*};

/// Set the `LifeState` of sim ganger `sim` (the real `Changed<LifeState>` trigger).
fn set_life_state(app: &mut App, sim: Entity, state: LifeState) {
    let mut q = app.world_mut().query::<&mut LifeState>();
    if let Ok(mut life) = q.get_mut(app.world_mut(), sim) {
        *life = state;
    }
}

/// AC4 — a `Changed<LifeState>` Downed re-tints the sprite, and Dead despawns it +
/// drops its `GangerSprites` entry.
#[test]
fn downed_retints_and_dead_despawns() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let at = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(at, 0, Direction::East))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    let sim = sim_entity_at(&mut app, at);
    assert!(sim.is_some(), "the ganger sim entity must exist");
    let Some(sim) = sim else { return };

    // The live sprite's tint, captured before downing.
    let drawn_alive = drawn_gangers(&mut app);
    assert_eq!(drawn_alive.len(), 1, "one live ganger sprite");
    let sprite = drawn_alive[0].sprite_entity;
    let alive_color = sprite_color(&mut app, sprite);

    // Down the ganger (mutate LifeState — the real Changed<LifeState> trigger).
    set_life_state(&mut app, sim, LifeState::Downed);
    app.update();

    // The sprite is still present but re-tinted (the documented Downed delta).
    let drawn_downed = drawn_gangers(&mut app);
    assert_eq!(drawn_downed.len(), 1, "the Downed ganger's sprite remains");
    let downed_color = sprite_color(&mut app, sprite);
    assert!(
        alive_color != downed_color,
        "a Downed ganger's sprite must re-tint (live {alive_color:?} vs downed {downed_color:?})",
    );

    // Kill the ganger (mutate LifeState to Dead — the despawn delta).
    set_life_state(&mut app, sim, LifeState::Dead);
    app.update();

    // The presenter sprite is despawned and its map entry dropped.
    let drawn_dead = drawn_gangers(&mut app);
    assert_eq!(
        drawn_dead.len(),
        0,
        "a Dead ganger's presenter sprite must be despawned",
    );
    let still_mapped = app
        .world()
        .get_resource::<GangerSprites>()
        .map(|m| m.contains(sim));
    assert_eq!(
        still_mapped,
        Some(false),
        "no live GangerSprites entry remains for a Dead ganger",
    );
}
