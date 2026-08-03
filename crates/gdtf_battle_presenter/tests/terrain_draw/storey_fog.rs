use bevy::{
    app::App, asset::Assets, ecs::message::Messages, platform::collections::HashSet,
    prelude::MeshMaterial2d,
};
use gdtf_battle_presenter::{ActiveLevel, TerrainFogMaterial, TerrainSprite};
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    occupancy::{TerrainKind, TerrainPlacement},
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::SurfaceGrid,
    visibility::SquadVisibility,
};

use super::harness::*;

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

fn set_fog(app: &mut App, visible: &[CellLevel], explored: &[CellLevel]) {
    let visible_set: HashSet<CellLevel> = visible.iter().copied().collect();
    let mut explored_set: HashSet<CellLevel> = explored.iter().copied().collect();
    explored_set.extend(visible_set.iter().copied());
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible_set, explored_set));
}

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

    set_fog(&mut app, &[active], &[lower]);

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();
    app.update();

    assert_eq!(
        sprite_brightness_at(&mut app, active),
        Some(1.0),
        "the ACTIVE-storey tile must render at full brightness (1.0)",
    );
    let lower_brightness = sprite_brightness_at(&mut app, lower);
    assert!(
        lower_brightness.is_some_and(|b| b < 1.0),
        "a LOWER drawn-storey tile must render darkened (brightness < 1.0); got \
         {lower_brightness:?}",
    );
    assert_eq!(
        sprite_saturation_at(&mut app, lower),
        Some(0.0),
        "a lower-storey EXPLORED cell must KEEP its fog greyscale (saturation 0.0) — the \
         darken multiplies ON TOP of the saturation, never instead of it",
    );
    assert_eq!(
        sprite_saturation_at(&mut app, active),
        Some(1.0),
        "the VISIBLE active-storey cell must be full colour (saturation 1.0)",
    );
}
