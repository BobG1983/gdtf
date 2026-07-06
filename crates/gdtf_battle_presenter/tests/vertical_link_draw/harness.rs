//! Shared `vertical_link_draw` fixture: the headless renderer app + the real
//! `setup_battle` pour.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::{error::warn, message::Messages},
    prelude::default,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{TileRoles, TopDownAtlases, TopDownRendererPlugin};
use gdtf_battle_sim::{
    battle::{BattleReady, SetupBattleRequested, setup_battle_on_request},
    ganger::{Aiming, Facing},
    prelude::{CellLevel, Direction, Faction},
    rng::{BattleSeed, ShotRng},
    situation::{GangerSpawn, Situation},
    test_support::{
        test_armor_registry, test_gang_registry, test_melee_weapon_registry, test_terrain_registry,
        test_weapon_registry,
    },
};
use gdtf_test_utils::advance_until_resource_exists;

/// Bounded settle headroom for the post-setup state / spawn / glide waits.
pub(crate) const MAX_UPDATES: u32 = 128;

/// Safety-net cap for the async atlas / tile-role loads.
pub(crate) const LOAD_SAFETY_NET: u32 = 10_000;

/// A fixed seed for the deterministic `SetupBattleRequested`.
pub(crate) const SEED: u64 = 0x0D15_EA5E;

/// The workspace-root `assets/` directory (manifest -> up two -> assets).
pub(crate) fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// Builds a headless `DefaultPlugins`/`no_renderer` app driving the real `setup_battle`
/// spawn path (the `ganger_draw.rs` harness).
pub(crate) fn headless_renderer_app() -> App {
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
                file_path: workspace_assets_root().to_string_lossy().into_owned(),
                ..default()
            }),
    )
    .add_message::<SetupBattleRequested>()
    .add_message::<BattleReady>()
    .add_message::<gdtf_battle_sim::occupancy_sync::CoverDestroyed>()
    .add_systems(bevy::app::Update, setup_battle_on_request)
    .add_plugins(TopDownRendererPlugin);
    app.insert_resource(test_weapon_registry());
    // GTW-505: the melee registry (with the `fists` default) so each ganger's melee
    // weapon resolves at setup (fixture gangers author none -> `fists`).
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    // GTW-396/491: the TerrainDefRegistry — setup_battle_on_request reads it to resolve
    // each slab's UUID to its def; the SituationBuilder's slab_at uses the test slab def.
    app.insert_resource(test_terrain_registry());
    // GTW-414/415: the GangRegistry the v2 setup_battle resolves each placed ganger's
    // (gang, member) ref against (without it setup fails closed and no ganger spawns).
    app.insert_resource(test_gang_registry());
    app.set_error_handler(warn);
    app
}

/// Drive `update()`s until `TileRoles` + `TopDownAtlases` are BOTH resident (the async
/// load chain settled), polling each resource's inserted SIGNAL (GTW-305).
pub(crate) fn settle_resources(app: &mut App) {
    advance_until_resource_exists::<TileRoles>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(app, LOAD_SAFETY_NET);
}

/// Build an authored ganger at `at` (standing rifleman, given faction + facing).
pub(crate) fn ganger_at(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
    use gdtf_battle_sim::{ganger::GangerName, test_support::GangerSpawnBuilder};
    GangerSpawnBuilder::new()
        .at(at)
        .name(GangerName::new(format!("Ganger {faction}")))
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .aiming(Aiming::new(false))
        .build()
}

/// Pour `situation` into the battle via the REAL setup path; returns whether setup ran.
pub(crate) fn drive_setup(app: &mut App, situation: Situation) -> bool {
    app.world_mut()
        .resource_mut::<Messages<SetupBattleRequested>>()
        .write(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    for _ in 0..MAX_UPDATES {
        app.update();
        if app.world().get_resource::<ShotRng>().is_some() {
            app.update();
            return true;
        }
    }
    false
}
