//! grid authoring, and the shared tile probes.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    ecs::error::warn,
    math::URect,
    prelude::{MeshMaterial2d, default},
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_assets::ContentFamilyAppExt;
use gdtf_battle_presenter::{
    StampedGraphic, TerrainFogMaterial, TerrainSprite, TopDownAtlases, TopDownRendererPlugin,
    source_parts, source_urect,
};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    battle::BattleReady,
    cover::{CoverEntry, CoverHp, HeightBand},
    entity::{TerrainCell, TerrainPieceKind},
    occupancy::{OccupancyInput, TerrainPlacement},
    occupancy_sync::TerrainPieceDestroyed,
    piece::{FootfallSound, TerrainGraphicKey},
    prelude::{CellLevel, Level, OccupancyGrid},
};
use gdtf_content_families::{
    SpriteDefsFamily,
    sprites::{SpriteDefRegistry, SpriteName},
};
use gdtf_test_utils::advance_until_resource_exists;

pub(crate) fn workspace_assets_root() -> PathBuf {
    let Some(root) = gdtf_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

pub(crate) fn headless_renderer_app_at(assets_root: &std::path::Path) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: assets_root.to_string_lossy().into_owned(),
                ..default()
            }),
    )
    .add_message::<BattleReady>()
    .add_message::<TerrainPieceDestroyed>()
    .add_plugins(TopDownRendererPlugin);
    app.register_content_family::<SpriteDefsFamily>();
    app.set_error_handler(warn);
    app
}

pub(crate) fn headless_renderer_app() -> App {
    headless_renderer_app_at(&workspace_assets_root())
}

pub(crate) fn settle_resources(app: &mut App) {
    advance_until_resource_exists::<SpriteDefRegistry>(app);
    advance_until_resource_exists::<TopDownAtlases>(app);
}

pub(crate) fn insert_occupancy(app: &mut App, terrain: Vec<TerrainPlacement>) {
    let input = OccupancyInput {
        terrain,
        occupants: Vec::new(),
    };
    let grid = OccupancyGrid::build_from_occupancy_input(
        &input,
        &bevy::platform::collections::HashSet::default(),
    );
    app.world_mut().insert_resource(grid);
}

pub(crate) fn sprite_defs(app: &App) -> Option<SpriteDefRegistry> {
    app.world().get_resource::<SpriteDefRegistry>().cloned()
}

pub(crate) fn def_rect(defs: &SpriteDefRegistry, name: &str) -> Option<URect> {
    let def = defs.def(&SpriteName::new(name.to_owned()))?;
    let (_path, rect) = source_parts(&def.source);
    rect.map(source_urect)
}

pub(crate) const fn low_cover_entry() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(30),
        HeightBand::Low,
        ArmorProtection::new(2),
        ArmorHardness::new(1),
        TerrainPieceKind::Cover,
    )
}

pub(crate) fn sprite_rect_at(app: &mut App, key: CellLevel) -> Option<URect> {
    let mut q = app
        .world_mut()
        .query::<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>();
    let handle = q
        .iter(app.world())
        .find(|(t, _)| t.at == key)
        .map(|(_, mat)| mat.id())?;
    let material = app
        .world()
        .get_resource::<Assets<TerrainFogMaterial>>()?
        .get(handle)?;
    material
        .atlas_layout
        .as_ref()
        .and_then(|layout| layout.textures.get(material.atlas_index).copied())
}

/// Which tile role a cell's sprite is currently stamped with.
pub(crate) fn stamped_graphic_at(app: &mut App, key: CellLevel) -> Option<StampedGraphic> {
    let mut q = app.world_mut().query::<(&TerrainSprite, &StampedGraphic)>();
    q.iter(app.world())
        .find(|(t, _)| t.at == key)
        .map(|(_, stamped)| stamped.clone())
}

pub(crate) fn sprite_entity_at(app: &mut App, key: CellLevel) -> Option<bevy::ecs::entity::Entity> {
    let mut q = app
        .world_mut()
        .query::<(bevy::ecs::entity::Entity, &TerrainSprite)>();
    q.iter(app.world())
        .find(|(_, t)| t.at == key)
        .map(|(entity, _)| entity)
}

pub(crate) const CENTER_WALL_DEF: &str = r#"(
    source: Sheet(
        sheet: "sprites/alt_tileset_terrain.png",
        rect: (x: 0, y: 0, w: 16, h: 16),
    ),
    anchor: (x: 8, y: 8),
)"#;

pub(crate) fn write_sprite_def(root: &std::path::Path, file: &str, payload: &str) {
    let dir = root.join("content").join("sprites");
    let created = std::fs::create_dir_all(&dir);
    assert!(
        created.is_ok(),
        "creating the sprites dir must succeed: {created:?}"
    );
    let written = std::fs::write(dir.join(file), payload);
    assert!(written.is_ok(), "writing {file} must succeed: {written:?}");
}

pub(crate) fn draw_one_wall(app: &mut App, key: CellLevel) {
    insert_occupancy(
        app,
        vec![TerrainPlacement::new(
            key,
            gdtf_battle_sim::occupancy::TerrainKind::Wall,
        )],
    );
    app.world_mut()
        .insert_resource(gdtf_battle_sim::cover::CoverLedger::new());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::surface::SurfaceGrid::new());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::prelude::BattleInProgress);
    spawn_terrain_entity(app, key, "wall", None);
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<BattleReady>>()
        .write(BattleReady);
    app.update();
}

pub(crate) fn spawn_terrain_entity(
    app: &mut App,
    key: CellLevel,
    graphic: &str,
    footfall: Option<&str>,
) {
    let mut entity = app.world_mut().spawn((
        TerrainCell::new(key),
        TerrainGraphicKey::new(graphic.to_owned()),
    ));
    if let Some(footfall) = footfall {
        entity.insert(FootfallSound::new(footfall.to_owned()));
    }
}

pub(crate) fn terrain_sprite_count_on_level(app: &mut App, level: Level) -> usize {
    let z = i32::from(*level);
    let mut q = app.world_mut().query::<&TerrainSprite>();
    q.iter(app.world()).filter(|t| t.at.z == z).count()
}
