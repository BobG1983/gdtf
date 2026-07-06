//! Shared `fog_present` fixture: the fog-writer app, the real setup pour, fog
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
    Aiming, BattleSeed, CellLevel, CombatTuning, Direction, Facing, Faction, GangerName,
    GangerSpawn, Position, SetupBattleRequested, ShotRng, Situation, SquadVisibility,
    setup_battle_on_request,
    test_support::{
        GangerSpawnBuilder, test_armor_registry, test_gang_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
};
use gdtf_test_utils::advance_until_resource_exists;

/// Bounded settle headroom for the post-setup SPAWN waits (a synchronous battle setup +
/// command flush, not an async load).
pub(crate) const MAX_UPDATES: u32 = 128;

/// Generous SAFETY-NET cap for the async atlas / tile-role loads (polls each resource's
/// inserted SIGNAL, not a fixed frame count — GTW-305).
pub(crate) const LOAD_SAFETY_NET: u32 = 10_000;

/// A fixed seed for the deterministic `SetupBattleRequested` (the fog write is RNG-free).
pub(crate) const SEED: u64 = 0x0D15_EA5E;

/// The workspace-root `assets/` directory (this crate's manifest -> up two -> assets).
pub(crate) fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// Builds the headless `DefaultPlugins`/`no_renderer` app with a live `AssetServer`, the
/// `TopDownRendererPlugin`, and the sim lifecycle pieces the real `setup_battle` spawn
/// path needs.
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
    .add_message::<gdtf_battle_sim::BattleReady>()
    .add_message::<gdtf_battle_sim::CoverDestroyed>()
    .add_systems(Update, setup_battle_on_request)
    .add_plugins(TopDownRendererPlugin);
    app.insert_resource(test_weapon_registry());
    // GTW-505: the melee registry (with the `fists` default) so each ganger's melee
    // weapon resolves at setup (fixture gangers author none -> `fists`).
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    // GTW-414/415: the GangRegistry the v2 setup_battle resolves each placed ganger's
    // (gang, member) ref against (without it setup fails closed and no ganger spawns).
    app.insert_resource(test_gang_registry());
    // The dense-FOV helper (dense_visible_from_observer -> union_fov) reads CombatTuning for
    // view_range (a Load-state resource the focused setup path does NOT insert); author the
    // Default. This is ONLY for the union_fov helper: the fog writer neither reads it
    // (GTW-348) nor gates on it (GTW-627 deleted the pseudo-gate).
    app.insert_resource(CombatTuning::default());
    app.set_error_handler(warn);
    app
}

/// Drive `update()`s until both async render resources have settled.
pub(crate) fn settle_resources(app: &mut App) {
    advance_until_resource_exists::<gdtf_battle_presenter::TileRoles>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(app, LOAD_SAFETY_NET);
}

/// An authored ganger at `at` (faction, facing) routed through the canonical shared
/// builder, named per faction.
pub(crate) fn ganger_at(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
    GangerSpawnBuilder::new()
        .at(at)
        .name(GangerName::new(format!("Ganger {faction}")))
        .faction(Faction::new(faction))
        .facing(Facing::new(facing))
        .aiming(Aiming::new(false))
        .build()
}

/// Pour `situation` into the battle via the REAL setup path; returns whether setup
/// completed (and the spawned gangers' `Added<Position>` has been observed by the draw).
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

/// Insert a `SquadVisibility` fog with the given VISIBLE / EXPLORED cells. The setup path
/// inserts an empty `SquadVisibility` (GTW-341); this OVERWRITES it with the test's sets.
pub(crate) fn set_fog(app: &mut App, visible: &[CellLevel], explored: &[CellLevel]) {
    let visible: HashSet<CellLevel> = visible.iter().copied().collect();
    // EXPLORED is the superset (the accrual invariant visible ⊆ explored).
    let mut explored_set: HashSet<CellLevel> = explored.iter().copied().collect();
    explored_set.extend(visible.iter().copied());
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible, explored_set));
}

/// The (`TerrainFogMaterial.saturation`, `Visibility`) of the one `TerrainSprite` at `key`
/// (GTW-348 — terrain renders through a material, so the fog is read off `saturation`, not
/// `Sprite.color`).
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

/// The sim entity occupying `at` (the setup spawns one ganger per authored cell).
pub(crate) fn sim_entity_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &Position)>();
    q.iter(app.world())
        .find(|(_, pos)| ***pos == at)
        .map(|(e, _)| e)
}

/// The `Visibility` of the presenter sprite mirroring sim ganger `sim`.
pub(crate) fn actor_visibility(app: &mut App, sim: Option<Entity>) -> Option<Visibility> {
    let sim = sim?;
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim))?;
    let mut q = app.world_mut().query::<&Visibility>();
    q.get(app.world(), sprite).ok().copied()
}

/// Drive bounded `update()`s until a `TerrainSprite` exists at `at` (the terrain draw's
/// `BattleReady`-read + spawn flush can land a frame behind the ganger-tuned setup).
pub(crate) fn settle_terrain_at(app: &mut App, at: CellLevel) -> bool {
    for _ in 0..MAX_UPDATES {
        if terrain_at(app, at).is_some() {
            return true;
        }
        app.update();
    }
    terrain_at(app, at).is_some()
}

/// Drive bounded `update()`s until the presenter sprite mirroring `sim` is mapped AND its
/// `Visibility` component is queryable.
///
/// The ganger sprite is spawned via a DEFERRED `commands.spawn_scene` (GTW-322), so its
/// components materialize on a later `SpawnScene` schedule — under parallel `cargo` load
/// that can lag several updates behind `drive_setup`'s single extra update. Settling on
/// the queryable sprite (not a fixed frame count) keeps the actor assertions deterministic.
pub(crate) fn settle_actor(app: &mut App, sim: Option<Entity>) -> bool {
    for _ in 0..MAX_UPDATES {
        if actor_visibility(app, sim).is_some() {
            return true;
        }
        app.update();
    }
    actor_visibility(app, sim).is_some()
}
