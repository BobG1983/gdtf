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
    prelude::{MeshMaterial2d, default},
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_assets::ContentFamilyAppExt;
use gdtf_battle_presenter::{
    MissingTileTexture, TerrainFogMaterial, TerrainSprite, TopDownAtlases, TopDownRendererPlugin,
};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    battle::BattleReady,
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    entity::TerrainCell,
    occupancy::{OccupancyInput, TerrainKind, TerrainPlacement},
    occupancy_sync::CoverDestroyed,
    piece::TerrainGraphicKey,
    prelude::{BattleInProgress, Cell, CellLevel, Level, OccupancyGrid},
    surface::SurfaceGrid,
};
use gdtf_content_families::{SpriteDefsFamily, sprites::SpriteDefRegistry};
use gdtf_test_utils::advance_until_resource_exists;

const LOAD_SAFETY_NET: u32 = 10_000;

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
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
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
    .add_message::<CoverDestroyed>()
    .add_plugins(TopDownRendererPlugin);
    app.register_content_family::<SpriteDefsFamily>();
    app.set_error_handler(warn);
    app
}

#[test]
fn missing_sprite_def_warns_and_draws_the_magenta_marker() {
    install_global_capture();

    let mut app = headless_renderer_app();
    advance_until_resource_exists::<SpriteDefRegistry>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(&mut app, LOAD_SAFETY_NET);

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
