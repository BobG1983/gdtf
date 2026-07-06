//! GTW-566 C4 (AC5) — the LOUD-TYPO warn: a sim-spawned [`TerrainGraphicKey`] that fails
//! [`TileRole::from_key`](gdtf_battle_presenter::TileRole) classification makes the draw's
//! `resolve_index` emit a `warn!` naming the unresolvable key AND the cell — while the
//! no-panic role-default fallback STILL draws the tile (the pre-GTW-566 behaviour, kept).
//!
//! Driven through the REAL `draw_static_battlefield` system on the same
//! `DefaultPlugins`/`no_renderer` harness as `terrain_draw.rs`. This lives in its OWN test
//! binary because the log capture must be a PROCESS-GLOBAL `tracing` subscriber: the draw
//! system runs on a worker thread of Bevy's multithreaded executor, so a scoped
//! (thread-local) `with_default` subscriber would miss the emission — and a global default
//! shared with dozens of parallel tests risks the `tracing-core` per-callsite
//! interest-cache poison (the documented `capture_logs` flake). One test, one process, one
//! always-on global subscriber + a post-install `rebuild_interest_cache()` = deterministic.

use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    ecs::{error::warn, message::Messages},
    log::{
        tracing::{
            Event, Subscriber,
            field::{Field, Visit},
        },
        tracing_subscriber::{Layer, layer::Context, prelude::*, registry::Registry},
    },
    prelude::default,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{TileRoles, TopDownAtlases, TopDownRendererPlugin};
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
use gdtf_test_utils::advance_until_resource_exists;

/// Generous SAFETY-NET cap for the async atlas / tile-role loads — a hang guard, not a
/// timing budget (each gate resource is polled by its inserted SIGNAL).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The out-of-vocabulary graphic key under test — unique to this test so the captured
/// warn line is unambiguously OURS.
const BOGUS_KEY: &str = "gtw566_not_a_role";

/// Every `message` captured by the process-global subscriber, in emission order.
static CAPTURED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// One-shot guard: the global capture subscriber installs exactly once per process.
static INSTALL: OnceLock<()> = OnceLock::new();

/// A `tracing` layer recording each event's `message` field into [`CAPTURED`] — installed
/// as the PROCESS-GLOBAL default so a worker-thread emission (Bevy's multithreaded
/// executor) is still seen.
struct CaptureLayer;

/// Pulls the `message` field's debug rendering out of a `tracing` event.
struct MessageVisitor {
    /// The captured message text, if a `message` field was visited.
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

/// Install the global capture subscriber ONCE, then rebuild the callsite interest cache so
/// any callsite registered before the install (against the `NoSubscriber` default, cached
/// `never`) re-evaluates against the now-live always-interested subscriber.
fn install_global_capture() {
    INSTALL.get_or_init(|| {
        let subscriber = Registry::default().with(CaptureLayer);
        let _ = bevy::log::tracing::subscriber::set_global_default(subscriber);
        bevy::log::tracing::callsite::rebuild_interest_cache();
    });
}

/// The workspace-root `assets/` directory (this crate's manifest → up two → assets).
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// The `terrain_draw.rs` headless `DefaultPlugins`/`no_renderer` harness: a live
/// `AssetServer` (workspace `assets/`), `TopDownRendererPlugin`, and the two sim message
/// buffers the draw reads. `LogPlugin` stays disabled — the global capture subscriber IS
/// this process's log sink.
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
    // No-renderer harness: route failed param validation to `warn` (the 0.18 skip), not a panic.
    app.set_error_handler(warn);
    app
}

/// GTW-566 C4/AC5 — an out-of-vocabulary authored key WARNS (naming the key + the cell)
/// and STILL draws the `TerrainKind` role-default tile.
///
/// Pin-discriminating both ways: dropping the `warn!` leaves the capture without the line
/// (the log assert fails); dropping the fallback leaves no sprite / a wrong index at the
/// cell (the draw asserts fail).
#[test]
fn out_of_vocabulary_key_warns_and_still_draws_the_role_default() {
    // Install the process-global capture BEFORE the app exists, so every later callsite
    // registers against the live always-interested subscriber.
    install_global_capture();

    let mut app = headless_renderer_app();
    advance_until_resource_exists::<TileRoles>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(&mut app, LOAD_SAFETY_NET);

    let cell = CellLevel::new(Cell::new(8, 7), Level::new(0));

    // A Cover cell in the occupancy grid — the TerrainKind role default the fallback must
    // draw when the per-def key fails classification.
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

    // The sim-spawned per-def fact carrying the TYPO'd graphic key.
    app.world_mut().spawn((
        TerrainCell::new(cell),
        TerrainGraphicKey::new(BOGUS_KEY.to_owned()),
    ));

    // Fire the one-shot draw and settle.
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    // STILL DRAWS: the cell's sprite exists and carries the TerrainKind role default
    // (`roles.cover`) — the no-panic fallback is unchanged (C4).
    let roles = app.world().get_resource::<TileRoles>().cloned();
    assert!(roles.is_some(), "TileRoles must be resident after settle");
    let Some(roles) = roles else { return };
    let index = sprite_index_at(&mut app, cell);
    assert_eq!(
        index,
        Some(*roles.cover),
        "an out-of-vocabulary key must STILL draw the TerrainKind role-default tile (cover)",
    );

    // WARNS LOUDLY: the capture holds a warn naming the unresolvable key AND the cell.
    let captured = CAPTURED
        .lock()
        .map(|buffer| buffer.clone())
        .unwrap_or_default();
    let expected_cell = format!("{cell:?}");
    assert!(
        captured.iter().any(|line| {
            line.contains("unresolvable graphic key")
                && line.contains(BOGUS_KEY)
                && line.contains(&expected_cell)
        }),
        "the draw must warn, naming the unresolvable key (`{BOGUS_KEY}`) and the cell \
         ({expected_cell}); captured: {captured:?}",
    );
}

/// Reads the atlas index of the one `TerrainSprite` at `key`, if present (the material's
/// `atlas_index` — the GTW-348 material path).
fn sprite_index_at(app: &mut App, key: CellLevel) -> Option<usize> {
    use bevy::{asset::Assets, prelude::MeshMaterial2d};
    use gdtf_battle_presenter::{TerrainFogMaterial, TerrainSprite};

    let mut q = app
        .world_mut()
        .query::<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>();
    let handle = q
        .iter(app.world())
        .find(|(t, _)| t.at == key)
        .map(|(_, mat)| mat.id())?;
    let index = app
        .world()
        .get_resource::<Assets<TerrainFogMaterial>>()?
        .get(handle)?
        .atlas_index;
    Some(index)
}
