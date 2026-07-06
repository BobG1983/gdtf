//! Lower-storey darken composes with the fog saturation (GTW-519 C4/C5).

use bevy::{
    app::App, asset::Assets, ecs::message::Messages, platform::collections::HashSet,
    prelude::MeshMaterial2d,
};
use gdtf_battle_presenter::{ActiveLevel, TerrainFogMaterial, TerrainSprite};
use gdtf_battle_sim::{
    BattleInProgress, BattleReady, Cell, CellLevel, CoverLedger, Level, SquadVisibility,
    SurfaceGrid, TerrainKind, TerrainPlacement,
};

use super::harness::*;

/// The `TerrainFogMaterial.brightness` (as a bare `f32`) of the one `TerrainSprite` at `key`,
/// if present — the C4 storey-darken probe. `Brightness` `Deref`s to its `f32`.
fn sprite_brightness_at(app: &mut App, key: CellLevel) -> Option<f32> {
    let mut q = app
        .world_mut()
        .query::<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>();
    let handle = q
        .iter(app.world())
        .find(|(t, _)| t.at == key)
        .map(|(_, mat)| mat.id())?;
    let brightness = app
        .world()
        .get_resource::<Assets<TerrainFogMaterial>>()?
        .get(handle)?
        .brightness;
    Some(*brightness)
}

/// The `TerrainFogMaterial.saturation` of the one `TerrainSprite` at `key`, if present — the
/// C5 fog-composes probe.
fn sprite_saturation_at(app: &mut App, key: CellLevel) -> Option<f32> {
    let mut q = app
        .world_mut()
        .query::<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>();
    let handle = q
        .iter(app.world())
        .find(|(t, _)| t.at == key)
        .map(|(_, mat)| mat.id())?;
    let saturation = app
        .world()
        .get_resource::<Assets<TerrainFogMaterial>>()?
        .get(handle)?
        .saturation;
    Some(saturation)
}

/// Overwrite the `SquadVisibility` fog with the given VISIBLE / EXPLORED cells (VISIBLE is
/// folded into EXPLORED to honour the `visible ⊆ explored` accrual invariant), so `present_fog`
/// (which the draw harness now needs to run for the brightness/saturation drive) has real
/// fog to modulate against.
fn set_fog(app: &mut App, visible: &[CellLevel], explored: &[CellLevel]) {
    let visible_set: HashSet<CellLevel> = visible.iter().copied().collect();
    let mut explored_set: HashSet<CellLevel> = explored.iter().copied().collect();
    explored_set.extend(visible_set.iter().copied());
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible_set, explored_set));
}

/// GTW-519 C4/C5 — after `present_fog` runs, a tile on a LOWER drawn storey has `brightness <
/// 1.0` (darkened) while a tile on the ACTIVE storey has `brightness == 1.0` (full-bright),
/// AND a lower-storey EXPLORED cell still has its fog `saturation == 0.0` preserved
/// (fog COMPOSES with the darken, is not replaced by it).
///
/// At `ActiveLevel` 1: authors a wall at `(4,4)` on storey 0 and on storey 1. The storey-1
/// wall is squad-VISIBLE; the storey-0 wall is EXPLORED-only. Drives the REAL `present_fog`
/// (requires `SquadVisibility` — inserted here; GTW-627 deleted the old `CombatTuning`
/// pseudo-gate) and asserts the four facts.
#[test]
fn lower_storey_darkened_active_full_bright_and_explored_saturation_preserved() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let cell = Cell::new(4, 4);
    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let lower = CellLevel::new(cell, l0);
    let active = CellLevel::new(cell, l1);

    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(lower, TerrainKind::Wall),
            TerrainPlacement::new(active, TerrainKind::Wall),
        ],
    );
    app.world_mut().insert_resource(CoverLedger::new());
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    // The active-storey wall is VISIBLE; the lower-storey wall is EXPLORED-only (remembered).
    set_fog(&mut app, &[active], &[lower]);

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    // One update draws; present_fog runs the same update (.after the draw) — settle a second
    // so the in-place material edit is observable (settle-before-read).
    app.update();
    app.update();

    // C4: the active storey is full-bright.
    assert_eq!(
        sprite_brightness_at(&mut app, active),
        Some(1.0),
        "the ACTIVE-storey tile must render at full brightness (1.0)",
    );
    // C4: a lower drawn storey is darkened (< 1.0).
    let lower_brightness = sprite_brightness_at(&mut app, lower);
    assert!(
        lower_brightness.is_some_and(|b| b < 1.0),
        "a LOWER drawn-storey tile must render darkened (brightness < 1.0); got \
         {lower_brightness:?}",
    );
    // C5: the lower-storey EXPLORED cell still desaturates (fog composes, not replaced) —
    // saturation 0.0 is the EXPLORED greyscale, driven ALONGSIDE the darken.
    assert_eq!(
        sprite_saturation_at(&mut app, lower),
        Some(0.0),
        "a lower-storey EXPLORED cell must KEEP its fog greyscale (saturation 0.0) — the \
         darken multiplies ON TOP of the saturation, never instead of it",
    );
    // And the VISIBLE active tile keeps full colour (saturation 1.0) — sanity that fog still
    // drives the active storey too.
    assert_eq!(
        sprite_saturation_at(&mut app, active),
        Some(1.0),
        "the VISIBLE active-storey cell must be full colour (saturation 1.0)",
    );
}
