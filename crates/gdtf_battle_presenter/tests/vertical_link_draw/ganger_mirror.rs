//! Cross-storey ganger sprite handoff + the movement tween intermediate
//! (C6c/C6d).

use std::time::Duration;

use bevy::{
    app::App,
    math::Vec3,
    prelude::{Entity, Visibility},
    time::TimeUpdateStrategy,
    transform::components::Transform,
};
use gdtf_battle_presenter::{GangerSprites, Layer, cell_to_world_layered};
use gdtf_battle_sim::{
    Cell, CellLevel, Direction, Level, Position, test_support::SituationBuilder,
};

use super::harness::*;

/// The sim entity at `at` (the setup spawns one ganger per authored cell).
fn sim_entity_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &Position)>();
    q.iter(app.world())
        .find(|(_, pos)| ***pos == at)
        .map(|(e, _)| e)
}

/// The visibility of the presenter sprite mirroring sim ganger `sim` (via the map).
fn visibility_of_sim(app: &mut App, sim: Option<Entity>) -> Option<Visibility> {
    let sim = sim?;
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim))?;
    let mut q = app.world_mut().query::<&Visibility>();
    q.get(app.world(), sprite).ok().copied()
}

/// The world translation of the presenter sprite mirroring sim ganger `sim` (via the map).
fn translation_of_sim(app: &mut App, sim: Entity) -> Option<Vec3> {
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim))?;
    let mut q = app.world_mut().query::<&Transform>();
    q.get(app.world(), sprite).ok().map(|t| t.translation)
}

/// Drive bounded `update()`s until the presenter sprite mirroring sim ganger `sim` is
/// MAPPED in [`GangerSprites`] and queryable (so [`translation_of_sim`] returns `Some`).
///
/// The presenter `spawn_ganger_sprites` runs in `PresenterSystems::Draw` gated on
/// `BattleInProgress` + `TopDownAtlases`, so its `GangerSprites` map entry can lag the sim
/// spawn by a frame under parallel `cargo dtest` contention. This bounded settle removes
/// that spawn-lag race before a translation is read — mirroring `ganger_draw.rs`'s
/// `settle_sprite_at`; the bound surfaces a never-mapped sprite as a failed assertion, not
/// a hang.
fn settle_sprite_mapped(app: &mut App, sim: Entity) {
    for _ in 0..MAX_UPDATES {
        if translation_of_sim(app, sim).is_some() {
            return;
        }
        app.update();
    }
}

/// Drive bounded `update()`s until the presenter sprite mirroring sim ganger `sim` reports the
/// `expected` [`Visibility`] (or the bound elapses).
///
/// The cross-storey Visibility flip is `move_ganger_sprites` in `PresenterSystems::Draw`, so after
/// a `Position` mutation the flip can trail the sim write by a frame under parallel `cargo dtest`
/// contention. This bounded settle removes that flip-lag race before the Visibility is asserted —
/// it never weakens the check: the following `assert_eq!` still fails if the value never converges,
/// the bound only stops a transient one-frame-off read (and a never-converging value still surfaces
/// as the assertion failing on the final read, not a hang).
fn settle_visibility(app: &mut App, sim: Entity, expected: Visibility) {
    for _ in 0..MAX_UPDATES {
        if visibility_of_sim(app, Some(sim)) == Some(expected) {
            return;
        }
        app.update();
    }
}

/// C6 (c) — the CROSS-STOREY ganger Visibility handoff (AC2 CONFIRM, no new system): a
/// ganger whose `Position.z` leaves the active storey goes `Hidden`; back on it goes
/// `Inherited`. Confirms the LANDED `move_ganger_sprites` flip survives the GTW-359 tween
/// restructure (the C3 KEEP clause).
#[test]
fn ganger_visibility_flips_hidden_when_position_leaves_active_storey() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    // The ganger starts on level 0; author both storey cells as slabs.
    let l0 = CellLevel::new(Cell::new(5, 5), Level::new(0));
    let l1 = CellLevel::new(Cell::new(5, 5), Level::new(1));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(l0, 0, Direction::East))
        .slab_at(l0)
        .slab_at(l1)
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    let sim = sim_entity_at(&mut app, l0);
    assert!(sim.is_some(), "the ganger sim entity must exist");
    let Some(sim) = sim else { return };

    // Wait out the presenter spawn-lag: under parallel `cargo dtest` the ganger sprite + its
    // `GangerSprites` map entry land in `PresenterSystems::Draw` and can trail the sim spawn by a
    // frame, so settle the map before reading the Visibility (removes the spawn-lag race that left
    // `sprite_for` -> None -> the assertion seeing None instead of Inherited; the asserts are unchanged).
    settle_sprite_mapped(&mut app, sim);

    // On the active storey (level 0): Inherited.
    assert_eq!(
        visibility_of_sim(&mut app, Some(sim)),
        Some(Visibility::Inherited),
        "a ganger ON the active storey is Visibility::Inherited",
    );

    // Move the ganger UP to level 1 (off the active storey 0) — Position.z leaves it.
    let mut pos_q = app.world_mut().query::<&mut Position>();
    if let Ok(mut pos) = pos_q.get_mut(app.world_mut(), sim) {
        *pos = Position::new(l1);
    }
    app.update();
    // Settle the Draw-schedule flip before reading (removes the flip-lag race; the assert stands).
    settle_visibility(&mut app, sim, Visibility::Hidden);
    assert_eq!(
        visibility_of_sim(&mut app, Some(sim)),
        Some(Visibility::Hidden),
        "after Position.z leaves the active storey the ganger sprite goes Hidden (the handoff)",
    );

    // Move it BACK down to level 0 — Inherited again.
    let mut pos_q = app.world_mut().query::<&mut Position>();
    if let Ok(mut pos) = pos_q.get_mut(app.world_mut(), sim) {
        *pos = Position::new(l0);
    }
    app.update();
    // Settle the Draw-schedule flip before reading (removes the flip-lag race; the assert stands).
    settle_visibility(&mut app, sim, Visibility::Inherited);
    assert_eq!(
        visibility_of_sim(&mut app, Some(sim)),
        Some(Visibility::Inherited),
        "back on the active storey the ganger sprite goes Inherited again",
    );
}

/// C6 (d) — the sprite is at an INTERMEDIATE Transform mid-tween: after a single small
/// time step following a move, the sprite is STRICTLY BETWEEN the old and new cell world
/// positions, NOT snapped to the destination (the AC3 / OQ-5 glide, no snap).
#[test]
fn moved_sprite_is_intermediate_mid_tween_not_snapped() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let start = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(start, 0, Direction::East))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    let sim = sim_entity_at(&mut app, start);
    assert!(sim.is_some(), "the ganger sim entity must exist");
    let Some(sim) = sim else { return };

    let start_world = cell_to_world_layered(Cell::new(5, 6), Level::new(0), Layer::Actor);
    let dest = CellLevel::new(Cell::new(12, 6), Level::new(0));
    let dest_world = cell_to_world_layered(Cell::new(12, 6), Level::new(0), Layer::Actor);

    // Wait out the presenter spawn-lag: under parallel `cargo dtest` the ganger sprite +
    // its `GangerSprites` map entry can land a frame after the sim spawn, so settle the map
    // before reading the translation (removes the spawn-lag race; the assertion is unchanged).
    settle_sprite_mapped(&mut app, sim);

    // The sprite begins AT the start cell (the spawn-seeded settled tween).
    let before = translation_of_sim(&mut app, sim);
    assert!(
        before.is_some_and(|t| t.distance(start_world) < 1.0e-3),
        "the sprite begins at the start cell (got {before:?}, expected {start_world:?})",
    );

    // Move the ganger; advance time by a SMALL fixed step — much less than the tween's own
    // glide duration — so the glide is mid-flight, NOT settled, this frame.
    let mut pos_q = app.world_mut().query::<&mut Position>();
    if let Ok(mut pos) = pos_q.get_mut(app.world_mut(), sim) {
        *pos = Position::new(dest);
    }
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        30,
    )));
    app.update();

    let mid = translation_of_sim(&mut app, sim);
    assert!(mid.is_some(), "the sprite must still exist mid-tween");
    let Some(mid) = mid else { return };
    // STRICTLY between start and dest on the moving (x) axis — not snapped to either end.
    assert!(
        mid.x > start_world.x && mid.x < dest_world.x,
        "the sprite must be at an INTERMEDIATE x mid-tween (strictly between start {} and dest \
         {}, NOT snapped), got {}",
        start_world.x,
        dest_world.x,
        mid.x,
    );
    // It has moved off the start but not reached the dest (the never-snap property).
    assert!(
        mid.distance(start_world) > 1.0e-3,
        "the sprite must have started gliding (moved off the start cell), got {mid:?}",
    );
    assert!(
        mid.distance(dest_world) > 1.0e-3,
        "the sprite must NOT have snapped to the dest in one small step, got {mid:?}",
    );
}
