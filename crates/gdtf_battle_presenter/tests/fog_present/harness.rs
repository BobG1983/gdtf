//! authoring, and the terrain / actor probes.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup, Update},
    asset::{AssetPlugin, Assets},
    ecs::{error::warn, message::Messages},
    platform::collections::HashSet,
    prelude::{Entity, MeshMaterial2d, Visibility, default},
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    GangerSprites, TerrainFogMaterial, TerrainSprite, TopDownAtlases, TopDownRendererPlugin,
};
use gdtf_battle_sim::{
    battle::{SetupBattleRequested, setup_battle_on_request},
    ganger::{Aiming, Facing, GangerName},
    prelude::{CellLevel, Direction, Faction, Position},
    rng::{BattleSeed, ShotRng},
    situation::{GangerSpawn, Situation},
    test_support::{
        GangerSpawnBuilder, test_armor_registry, test_gang_registry, test_melee_weapon_registry,
        test_terrain_registry, test_weapon_registry,
    },
    tuning::CombatTuning,
    visibility::SquadVisibility,
};
use gdtf_test_utils::advance_until_resource_exists;

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
            .set(AssetPlugin {
                file_path: workspace_assets_root().to_string_lossy().into_owned(),
                ..default()
            }),
    )
    .add_message::<SetupBattleRequested>()
    .add_message::<gdtf_battle_sim::battle::BattleReady>()
    .add_message::<gdtf_battle_sim::occupancy_sync::TerrainPieceDestroyed>()
    .add_systems(Update, setup_battle_on_request)
    .add_plugins(TopDownRendererPlugin);
    gdtf_assets::ContentFamilyAppExt::register_content_family::<
        gdtf_content_families::SpriteDefsFamily,
    >(&mut app);
    app.insert_resource(test_weapon_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app.insert_resource(test_gang_registry());
    app.insert_resource(test_terrain_registry());
    app.insert_resource(CombatTuning::default());
    app.set_error_handler(warn);
    app
}

pub(crate) fn settle_resources(app: &mut App) {
    advance_until_resource_exists::<gdtf_content_families::sprites::SpriteDefRegistry>(app);
    advance_until_resource_exists::<TopDownAtlases>(app);
}

pub(crate) fn ganger_at(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
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

pub(crate) fn set_fog(app: &mut App, visible: &[CellLevel], explored: &[CellLevel]) {
    let visible: HashSet<CellLevel> = visible.iter().copied().collect();
    let mut explored_set: HashSet<CellLevel> = explored.iter().copied().collect();
    explored_set.extend(visible.iter().copied());
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible, explored_set));
}

pub(crate) fn terrain_at(app: &mut App, key: CellLevel) -> Option<(f32, Visibility)> {
    let mut q = app.world_mut().query::<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &Visibility,
    )>();
    let (handle, vis) = q
        .iter(app.world())
        .find(|(t, ..)| t.at == key)
        .map(|(_, mat, vis)| (mat.id(), *vis))?;
    let saturation = app
        .world()
        .get_resource::<Assets<TerrainFogMaterial>>()?
        .get(handle)?
        .saturation;
    Some((saturation, vis))
}

pub(crate) fn sim_entity_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &Position)>();
    q.iter(app.world())
        .find(|(_, pos)| ***pos == at)
        .map(|(e, _)| e)
}

pub(crate) fn actor_visibility(app: &mut App, sim: Option<Entity>) -> Option<Visibility> {
    let sim = sim?;
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim))?;
    let mut q = app.world_mut().query::<&Visibility>();
    q.get(app.world(), sprite).ok().copied()
}

pub(crate) fn settle_terrain_at(app: &mut App, at: CellLevel) -> bool {
    for _ in 0..MAX_UPDATES {
        if terrain_at(app, at).is_some() {
            return true;
        }
        app.update();
    }
    terrain_at(app, at).is_some()
}

pub(crate) fn settle_actor(app: &mut App, sim: Option<Entity>) -> bool {
    for _ in 0..MAX_UPDATES {
        if actor_visibility(app, sim).is_some() {
            return true;
        }
        app.update();
    }
    actor_visibility(app, sim).is_some()
}
