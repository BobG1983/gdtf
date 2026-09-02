//! grid authoring, and the shared tile probes.

use std::{path::PathBuf, time::Duration};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    ecs::{entity::Entity, error::warn, message::Messages},
    math::URect,
    prelude::{MeshMaterial2d, default},
    render::{RenderPlugin, settings::WgpuSettings},
    time::TimeUpdateStrategy,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_assets::ContentFamilyAppExt;
use gdtf_battle_presenter::{
    PlaybackCursor, PlaybackTuning, StampedGraphic, TerrainFogMaterial, TerrainSprite,
    TopDownAtlases, TopDownRendererPlugin, source_parts, source_urect,
};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, ActSeq, ActWitnesses, RecordedAct},
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

// The shared renderer build both public builders call.
fn renderer_app_at(assets_root: &std::path::Path) -> App {
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
    .add_plugins(TopDownRendererPlugin);
    app.register_content_family::<SpriteDefsFamily>();
    app.set_error_handler(warn);
    app
}

pub(crate) fn headless_renderer_app_at(assets_root: &std::path::Path) -> App {
    let mut app = renderer_app_at(assets_root);
    app.add_message::<TerrainPieceDestroyed>();
    app
}

pub(crate) fn headless_renderer_app() -> App {
    headless_renderer_app_at(&workspace_assets_root())
}

/// The same renderer app with no raw `TerrainPieceDestroyed` buffer, so only the played buffer
/// can drive a destruction swap.
pub(crate) fn headless_renderer_app_without_raw_destroyed() -> App {
    renderer_app_at(&workspace_assets_root())
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

pub(crate) fn sprite_entity_at(app: &mut App, key: CellLevel) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &TerrainSprite)>();
    q.iter(app.world())
        .find(|(_, t)| t.at == key)
        .map(|(entity, _)| entity)
}

// How many manual frames the bounded advance loop is allowed before it gives up.
const MAX_FRAMES: usize = 16;

/// Sequence of the next act-log entry the cursor will show.
pub(crate) fn shown(app: &App) -> ActSeq {
    app.world().resource::<PlaybackCursor>().shown()
}

/// Whether the cursor is dwelling on an entry rather than moving on.
pub(crate) fn holding(app: &App) -> bool {
    app.world().resource::<PlaybackCursor>().is_holding()
}

/// Whether the raw `TerrainPieceDestroyed` buffer is registered in this app.
pub(crate) fn raw_destroyed_present(app: &App) -> bool {
    app.world()
        .contains_resource::<Messages<TerrainPieceDestroyed>>()
}

// One manual frame, longer than the shortest dwell.
pub(crate) fn hold_step(app: &App) -> Duration {
    let minor = *app.world().resource::<PlaybackTuning>().minor_seconds;
    Duration::from_secs_f32(minor.max(0.0) + 0.05)
}

/// An act log holding a detaining minor deed then one smash per entry, whose last
/// sequence it returns.
pub(crate) fn detained_smash_log(
    app: &mut App,
    smashes: &[(CellLevel, TerrainPieceKind)],
) -> ActSeq {
    let actor = app.world_mut().spawn_empty().id();
    let mut log = ActLog::default();
    let mut last = log.append(RecordedAct::new(
        actor,
        ActProvenance::Commanded,
        ActDeed::BleedStarted,
        ActWitnesses::unseen(),
    ));
    for (at, kind) in smashes {
        last = log.append(RecordedAct::new(
            actor,
            ActProvenance::Commanded,
            ActDeed::TerrainPieceSmashed {
                at:   *at,
                kind: *kind,
            },
            ActWitnesses::unseen(),
        ));
    }
    app.world_mut().insert_resource(log);
    last
}

/// Steps the manual clock until the cursor plays `seq`, running `each_frame` after each update.
pub(crate) fn play_past(app: &mut App, seq: ActSeq, each_frame: impl Fn(&App)) {
    let step = hold_step(app);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(step));
    for _ in 0..MAX_FRAMES {
        app.update();
        each_frame(app);
        if shown(app) > seq {
            return;
        }
    }
    assert!(
        shown(app) > seq,
        "the cursor never played the smash entry {seq:?} within {MAX_FRAMES} manual frames of \
         {step:?} — it is still at {:?}",
        shown(app),
    );
}

/// Despawn the terrain piece standing at a cell, which is what a destruction does.
pub(crate) fn despawn_terrain_entity(app: &mut App, key: CellLevel) {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &TerrainCell)>();
    let standing: Vec<Entity> = query
        .iter(world)
        .filter(|(_, cell)| ***cell == key)
        .map(|(entity, _)| entity)
        .collect();
    for entity in standing {
        assert!(
            world.despawn(entity),
            "the terrain piece at {key:?} must still be alive to despawn",
        );
    }
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
