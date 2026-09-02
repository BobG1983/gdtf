//! Missing sprite def: warns and draws the magenta marker, never invisible.
use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::Assets,
    ecs::{error::warn, message::Messages},
    log::{
        tracing::{
            Event, Subscriber,
            field::{Field, Visit},
        },
        tracing_subscriber::{Layer, layer::Context, prelude::*, registry::Registry},
    },
    math::Vec2,
    prelude::{MeshMaterial2d, default},
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_assets::ContentFamilyAppExt;
use gdtf_battle_presenter::{
    Brightness, CELL_PX, MissingTileTexture, TerrainFogMaterial, TerrainSprite, TopDownAtlases,
    TopDownRendererPlugin,
};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    battle::BattleReady,
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    entity::{TerrainCell, TerrainPieceKind},
    occupancy::{OccupancyInput, TerrainKind, TerrainPlacement},
    occupancy_sync::TerrainPieceDestroyed,
    piece::TerrainGraphicKey,
    prelude::{BattleInProgress, Cell, CellLevel, Level, OccupancyGrid},
    surface::SurfaceGrid,
};
use gdtf_content_families::{SpriteDefsFamily, sprites::SpriteDefRegistry};
use gdtf_test_utils::advance_until_resource_exists;

const MISSING_NAME: &str = "no_such_sprite_fixture";

static CAPTURED: Mutex<Vec<String>> = Mutex::new(Vec::new());

static INSTALL: OnceLock<()> = OnceLock::new();

struct CaptureLayer;

struct MessageVisitor {
    message: Option<String>,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}"));
        }
    }
}

impl<S: Subscriber> Layer<S> for CaptureLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor { message: None };
        event.record(&mut visitor);
        if let Some(message) = visitor.message
            && let Ok(mut buffer) = CAPTURED.lock()
        {
            buffer.push(message);
        }
    }
}

fn install_global_capture() {
    INSTALL.get_or_init(|| {
        let subscriber = Registry::default().with(CaptureLayer);
        let _ = bevy::log::tracing::subscriber::set_global_default(subscriber);
        bevy::log::tracing::callsite::rebuild_interest_cache();
    });
}

fn workspace_assets_root() -> PathBuf {
    let Some(root) = gdtf_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

fn headless_renderer_app() -> App {
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
            .set(bevy::asset::AssetPlugin {
                file_path: workspace_assets_root().to_string_lossy().into_owned(),
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

#[test]
fn missing_sprite_def_warns_and_draws_the_magenta_marker() {
    install_global_capture();

    let mut app = headless_renderer_app();
    advance_until_resource_exists::<SpriteDefRegistry>(&mut app);
    advance_until_resource_exists::<TopDownAtlases>(&mut app);

    let cell = CellLevel::new(Cell::new(8, 7), Level::new(0));

    let input = OccupancyInput {
        terrain:   vec![TerrainPlacement::new(cell, TerrainKind::Cover)],
        occupants: Vec::new(),
    };
    let grid = OccupancyGrid::build_from_occupancy_input(
        &input,
        &bevy::platform::collections::HashSet::default(),
    );
    app.world_mut().insert_resource(grid);
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(
        cell,
        CoverEntry::seeded(
            CoverHp::new(30),
            HeightBand::Low,
            ArmorProtection::new(2),
            ArmorHardness::new(1),
            TerrainPieceKind::Cover,
        ),
    );
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    app.world_mut().spawn((
        TerrainCell::new(cell),
        TerrainGraphicKey::new(MISSING_NAME.to_owned()),
    ));

    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let missing = app
        .world()
        .get_resource::<MissingTileTexture>()
        .map(MissingTileTexture::handle);
    assert!(
        missing.is_some(),
        "the MissingTileTexture marker must be minted at Startup",
    );
    let Some(missing) = missing else { return };
    let material = material_at(&mut app, cell);
    assert!(
        material.is_some(),
        "the missing-def cell must STILL draw a tile (never invisible — C4)",
    );
    let Some(material) = material else { return };
    assert_eq!(
        material.image.id(),
        missing.id(),
        "the missing-def tile must carry the magenta MissingTileTexture image (C4)",
    );
    assert!(
        material.atlas_layout.is_none(),
        "the missing marker draws the whole 1×1 magenta image (identity UV, no layout)",
    );

    let captured = CAPTURED
        .lock()
        .map(|buffer| buffer.clone())
        .unwrap_or_default();
    let expected_cell = format!("{cell:?}");
    assert!(
        captured.iter().any(|line| {
            line.contains("no sprite def named")
                && line.contains(MISSING_NAME)
                && line.contains(&expected_cell)
        }),
        "the draw must warn, naming the unresolvable sprite name (`{MISSING_NAME}`) and the \
         cell ({expected_cell}); captured: {captured:?}",
    );
}

// The empty storey-0 battle both marker cases draw.
fn empty_ground_battle(app: &mut App) {
    let input = OccupancyInput {
        terrain:   Vec::new(),
        occupants: Vec::new(),
    };
    let grid = OccupancyGrid::build_from_occupancy_input(
        &input,
        &bevy::platform::collections::HashSet::default(),
    );
    app.world_mut().insert_resource(grid);
    app.world_mut().insert_resource(CoverLedger::new());
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);
}

#[test]
fn a_cell_with_no_piece_and_no_theme_default_draws_the_marker_material() {
    let mut app = headless_renderer_app();
    advance_until_resource_exists::<SpriteDefRegistry>(&mut app);
    advance_until_resource_exists::<TopDownAtlases>(&mut app);

    let cell = CellLevel::new(Cell::new(3, 3), Level::new(0));
    empty_ground_battle(&mut app);

    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let missing = app
        .world()
        .get_resource::<MissingTileTexture>()
        .map(MissingTileTexture::handle);
    assert!(
        missing.is_some(),
        "the MissingTileTexture marker must be minted at Startup",
    );
    let Some(missing) = missing else { return };
    let material = material_at(&mut app, cell);
    assert!(
        material.is_some(),
        "a storey-0 cell with no piece must still draw a tile",
    );
    let Some(material) = material else { return };

    assert_eq!(
        material.image.id(),
        missing.id(),
        "a storey-0 cell with no piece and no theme default draws the magenta \
         MissingTileTexture, not whatever def happens to be named `floor`",
    );
    assert!(
        material.atlas_layout.is_none(),
        "the marker draws the whole 1x1 magenta image (identity UV, no layout)",
    );
    assert_eq!(
        material.custom_size,
        Some(Vec2::splat(CELL_PX)),
        "the marker tile is drawn at the same cell size as every other terrain tile",
    );
    assert!(
        (material.saturation - 1.0).abs() < f32::EPSILON,
        "the marker is built at full colour; got saturation {}",
        material.saturation,
    );
    assert_eq!(
        material.brightness,
        Brightness::FULL,
        "the marker is built at full brightness, so a reader sees the authoring gap",
    );
}

#[test]
fn a_cell_whose_piece_resolves_does_not_draw_the_marker() {
    let mut app = headless_renderer_app();
    advance_until_resource_exists::<SpriteDefRegistry>(&mut app);
    advance_until_resource_exists::<TopDownAtlases>(&mut app);

    let cell = CellLevel::new(Cell::new(3, 3), Level::new(0));
    empty_ground_battle(&mut app);
    app.world_mut().spawn((
        TerrainCell::new(cell),
        TerrainGraphicKey::new("floor".to_owned()),
    ));

    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let missing = app
        .world()
        .get_resource::<MissingTileTexture>()
        .map(MissingTileTexture::handle);
    assert!(missing.is_some(), "the marker texture must be minted");
    let Some(missing) = missing else { return };
    let material = material_at(&mut app, cell);
    assert!(material.is_some(), "the cell must draw a tile");
    let Some(material) = material else { return };

    assert_ne!(
        material.image.id(),
        missing.id(),
        "a cell whose piece's graphic resolves must NOT draw the marker — the marker is not \
         drawn everywhere",
    );
    assert!(
        material.atlas_layout.is_some(),
        "a resolved sprite carries its sheet region as an atlas layout",
    );
}

fn material_at(app: &mut App, key: CellLevel) -> Option<TerrainFogMaterial> {
    let mut q = app
        .world_mut()
        .query::<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>();
    let handle = q
        .iter(app.world())
        .find(|(t, _)| t.at == key)
        .map(|(_, mat)| mat.id())?;
    app.world()
        .get_resource::<Assets<TerrainFogMaterial>>()?
        .get(handle)
        .cloned()
}
