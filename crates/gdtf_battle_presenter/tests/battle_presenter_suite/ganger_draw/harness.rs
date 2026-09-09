//! real setup-battle driver, and fog / actor-settle authoring.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    ecs::{error::warn, message::Messages},
    platform::collections::HashSet,
    prelude::{Entity, default},
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use cobalt_test_utils::{advance_until_resource_exists, asset_plugin_at};
use gdtf_battle_presenter::{CharacterRoles, TopDownAtlases, TopDownRendererPlugin};
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
    visibility::SquadVisibility,
};

use super::probes::visibility_of_sim;

pub(crate) const MAX_UPDATES: u32 = 128;

pub(crate) const SEED: u64 = 0x0D15_EA5E;

pub(crate) fn workspace_assets_root() -> PathBuf {
    let Some(root) = cobalt_ron_assets::workspace_assets_root() else {
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
            .set(asset_plugin_at(&workspace_assets_root())),
    )
    .add_message::<SetupBattleRequested>()
    .add_message::<BattleReady>()
    .add_message::<gdtf_battle_sim::occupancy_sync::TerrainPieceDestroyed>()
    .add_systems(bevy::app::Update, setup_battle_on_request)
    .add_plugins(TopDownRendererPlugin);
    gdtf_assets::ContentFamilyAppExt::register_content_family::<
        gdtf_content_families::SpriteDefsFamily,
    >(&mut app);
    app.insert_resource(test_weapon_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app.insert_resource(test_terrain_registry());
    app.insert_resource(test_gang_registry());
    app.set_error_handler(warn);
    app
}

pub(crate) fn band_only_fog(app: &mut App) {
    app.world_mut().remove_resource::<SquadVisibility>();
}

pub(crate) fn settle_resources(app: &mut App) {
    advance_until_resource_exists::<CharacterRoles>(app);
    advance_until_resource_exists::<TopDownAtlases>(app);
    advance_until_resource_exists::<gdtf_content_families::sprites::SpriteDefRegistry>(app);
}

pub(crate) fn set_fog(app: &mut App, visible: &[CellLevel], explored: &[CellLevel]) {
    let visible: HashSet<CellLevel> = visible.iter().copied().collect();
    let mut explored_set: HashSet<CellLevel> = explored.iter().copied().collect();
    explored_set.extend(visible.iter().copied());
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible, explored_set));
}

pub(crate) fn settle_actor(app: &mut App, sim: Option<Entity>) -> bool {
    for _ in 0..MAX_UPDATES {
        if visibility_of_sim(app, sim).is_some() {
            return true;
        }
        app.update();
    }
    visibility_of_sim(app, sim).is_some()
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
