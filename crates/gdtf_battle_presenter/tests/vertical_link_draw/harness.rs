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
use gdtf_assets::ContentFamilyAppExt;
use gdtf_battle_presenter::{TopDownAtlases, TopDownRendererPlugin};
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
use gdtf_content_families::{SpriteDefsFamily, sprites::SpriteDefRegistry};
use gdtf_test_utils::advance_until_resource_exists;

pub(crate) const MAX_UPDATES: u32 = 128;

pub(crate) const LOAD_SAFETY_NET: u32 = 10_000;

pub(crate) const SEED: u64 = 0x0D15_EA5E;

pub(crate) fn workspace_assets_root() -> PathBuf {
    let Some(root) = gdtf_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

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
    app.register_content_family::<SpriteDefsFamily>();
    app.insert_resource(test_weapon_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app.insert_resource(test_terrain_registry());
    app.insert_resource(test_gang_registry());
    app.set_error_handler(warn);
    app
}

pub(crate) fn settle_resources(app: &mut App) {
    advance_until_resource_exists::<SpriteDefRegistry>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(app, LOAD_SAFETY_NET);
}

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
