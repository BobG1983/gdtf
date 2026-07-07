//! Shared `ganger_draw` fixture: the headless renderer app (plain + fog-aware), the
//! real setup-battle driver, and fog / actor-settle authoring.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::{error::warn, message::Messages},
    platform::collections::HashSet,
    prelude::{Entity, default},
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
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
use gdtf_test_utils::advance_until_resource_exists;

use super::probes::visibility_of_sim;

/// Bounded settle headroom for the post-setup STATE / SPAWN waits (`drive_setup`'s
/// `ShotRng`-present signal, `settle_terrain_z_at`'s spawned-sprite signal). These wait on a
/// SYNCHRONOUS battle setup + its command flush, NOT on an async asset load, so a true small
/// frame count is the right termination — they are out of GTW-305's async-load scope.
pub(crate) const MAX_UPDATES: u32 = 128;

/// Generous SAFETY-NET cap for the async atlas / character-role loads polled by
/// [`settle_resources`]. It is a safety net against a genuine never-resolve hang, NOT a timing
/// budget: each gate resource is waited on by its inserted SIGNAL (not a fixed frame count),
/// which is what makes these draw tests deterministic under parallel `cargo` load (GTW-305).
pub(crate) const LOAD_SAFETY_NET: u32 = 10_000;

/// A fixed seed for the deterministic `SetupBattleRequested` (the draw is RNG-free; the
/// seed only feeds the unused battle RNG streams).
pub(crate) const SEED: u64 = 0x0D15_EA5E;

/// The workspace-root `assets/` directory (this crate's manifest -> up two -> assets),
/// the same root the running app uses so the shipped sheets + `character_roles.ron` load.
pub(crate) fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// Builds a headless `DefaultPlugins`/`no_renderer` app with a live `AssetServer`
/// (workspace `assets/`), the `TopDownRendererPlugin`, and the sim lifecycle pieces the
/// real `setup_battle` spawn path needs (the `SetupBattleRequested` / `BattleReady`
/// buffers plus `setup_battle_on_request`). It does NOT re-add `BattleSimPlugin` (a
/// design-fidelity violation) — only the minimal pieces to drive a real ganger spawn.
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
    // The real spawn path: drain SetupBattleRequested, run setup_battle, insert
    // BattleInProgress + the four grids, write BattleReady. The five RNG streams (GTW-14)
    // are inserted by setup_battle_on_request from the message seed.
    .add_message::<SetupBattleRequested>()
    .add_message::<BattleReady>()
    // The S4 terrain draw (also registered by TopDownRendererPlugin) reads
    // MessageReader<CoverDestroyed> once BattleInProgress is live, so the buffer must
    // exist or that system fails param validation ("Message not initialized"). This
    // harness drives a battle, so it registers the buffer the S4 draw needs — the
    // ganger draw itself reads no messages.
    .add_message::<gdtf_battle_sim::occupancy_sync::CoverDestroyed>()
    .add_systems(bevy::app::Update, setup_battle_on_request)
    .add_plugins(TopDownRendererPlugin);
    // GTW-665: the sprite-defs family — the SAME one-line host registration the game's
    // Load plugin performs; the S4 terrain draw (whose floor tiles the storey tests
    // assert under the gangers) resolves graphic names against the SpriteDefRegistry
    // it publishes.
    gdtf_assets::ContentFamilyAppExt::register_content_family::<
        gdtf_content_families::SpriteDefsFamily,
    >(&mut app);
    // The Load-built WeaponRegistry (GTW-257): setup_battle_on_request reads it to arm
    // each spawned ganger. The canonical shared [`test_weapon_registry`] (GTW-324),
    // inserted up front (the fixture gangers reference its [`TEST_WEAPON_KEY`]).
    app.insert_resource(test_weapon_registry());
    // GTW-505: the melee registry (with the `fists` default) so each ganger's melee
    // weapon resolves at setup (fixture gangers author none -> `fists`).
    app.insert_resource(test_melee_weapon_registry());
    // The Load-built ArmorRegistry (GTW-269): setup_battle_on_request reads it to armor
    // each spawned ganger. The canonical shared [`test_armor_registry`] (GTW-324),
    // inserted up front (the fixture gangers reference its [`TEST_ARMOR_KEY`]).
    app.insert_resource(test_armor_registry());
    // GTW-396/491: the TerrainDefRegistry — setup_battle_on_request reads it to resolve
    // each terrain piece's UUID to its sim/presenter def. The canonical test registry
    // supplies the wall / slab / cover / floor defs the SituationBuilder uses.
    app.insert_resource(test_terrain_registry());
    // GTW-414/415: the GangRegistry — setup_battle_on_request resolves each placed
    // ganger's (gang, member) ref against it. The canonical `test_gang_registry` holds
    // every fixture member (the `ganger_at` / default-builder gangs); without it setup
    // fails closed (GangNotFound) and no ganger spawns.
    app.insert_resource(test_gang_registry());
    // Bevy 0.19 routes a FAILED system-param validation to the global error handler
    // (default panics); 0.18 silently SKIPPED. This no-renderer harness lacks the
    // render-provided resources some DefaultPlugins systems want (e.g. bevy_light's
    // update_gizmo_meshes -> Assets<GizmoAsset>), so `warn` restores the 0.18 skip
    // behavior instead of an intermittent headless panic.
    app.set_error_handler(warn);
    app
}

/// Put the GTW-627 ganger-visibility resolver in BAND-ONLY mode: remove the
/// setup-inserted [`SquadVisibility`], so the classifier takes its ABSENT-fog branch (the
/// band fact alone decides — no fog composition, no mode flag).
///
/// The real `setup_battle` inserts an (empty) `SquadVisibility` alongside a default
/// `PlayerFaction`, which makes the resolver COMPOSE the fog fact — an empty fog would
/// hide every non-player ganger. The storey-filter tests pin the BAND axis in isolation,
/// so they clear the fog sets and exercise the classifier's band-only mode directly
/// (pre-GTW-627 the same isolation came from the deleted `CombatTuning` pseudo-gate
/// keeping the fog writer inert).
pub(crate) fn band_only_fog(app: &mut App) {
    app.world_mut().remove_resource::<SquadVisibility>();
}

/// Drives `update()`s until `CharacterRoles` + `TopDownAtlases` + the GTW-665
/// `SpriteDefRegistry` are ALL resident (the async
/// load chain has settled), polling each resource's inserted SIGNAL rather than a fixed frame
/// count (GTW-305). All resolve over the same async `AssetServer` chain, so waiting for them
/// in sequence drives the app until the last is present. Panics (naming the missing resource)
/// via [`advance_until_resource_exists`] if any is still absent after the safety-net cap — a
/// genuine load failure, surfaced loudly rather than leaving the draw systems silently no-op.
pub(crate) fn settle_resources(app: &mut App) {
    advance_until_resource_exists::<CharacterRoles>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<gdtf_content_families::sprites::SpriteDefRegistry>(
        app,
        LOAD_SAFETY_NET,
    );
}

/// Insert a [`SquadVisibility`] fog with the given VISIBLE / EXPLORED cells (mirrors the
/// `fog_present.rs` helper). The setup path inserts an empty `SquadVisibility` (GTW-341);
/// this OVERWRITES it with the test's sets, preserving the accrual invariant
/// (`visible ⊆ explored`).
pub(crate) fn set_fog(app: &mut App, visible: &[CellLevel], explored: &[CellLevel]) {
    let visible: HashSet<CellLevel> = visible.iter().copied().collect();
    let mut explored_set: HashSet<CellLevel> = explored.iter().copied().collect();
    explored_set.extend(visible.iter().copied());
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible, explored_set));
}

/// Drive bounded `update()`s until the presenter sprite mirroring `sim` is mapped AND its
/// [`Visibility`] component is queryable (the ganger sprite spawns via a DEFERRED
/// `commands.spawn_scene` — GTW-322 — so its components can lag several updates under
/// parallel load; settling on the queryable sprite keeps the actor assertions deterministic).
pub(crate) fn settle_actor(app: &mut App, sim: Option<Entity>) -> bool {
    for _ in 0..MAX_UPDATES {
        if visibility_of_sim(app, sim).is_some() {
            return true;
        }
        app.update();
    }
    visibility_of_sim(app, sim).is_some()
}

/// Build an authored ganger at `at` with the given faction + facing, otherwise plausible
/// component values (a standing, hip-firing, alive rifleman). Routed through the canonical
/// shared [`GangerSpawnBuilder`] (GTW-324) so the setup path spawns the same component set
/// the draw reads. The builder's TEST armor/weapon keys are `TEST_ARMOR_KEY` /
/// `TEST_WEAPON_KEY`, the same keys the canonical shared [`test_armor_registry`] /
/// [`test_weapon_registry`] resolve. Overrides off the builder defaults: the per-faction
/// `"Ganger {faction}"` name (the faction-distinct sprite-colour assertion reads it) and
/// `aiming(false)` (this is the hip-firing draw fixture; the builder default aims).
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

/// Pour `situation` into the battle via the REAL setup path: write a
/// `SetupBattleRequested` and update until `BattleReady` was emitted (the setup ran and
/// inserted `BattleInProgress` + spawned the gangers). Returns whether setup completed.
pub(crate) fn drive_setup(app: &mut App, situation: Situation) -> bool {
    app.world_mut()
        .resource_mut::<Messages<SetupBattleRequested>>()
        .write(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    // Update enough to: run setup (spawns + inserts BattleInProgress), flush the spawn
    // Commands, and let the draw's Added<Position> fire.
    for _ in 0..MAX_UPDATES {
        app.update();
        // ShotRng (one of the five GTW-14 streams) is inserted by setup_battle_on_request
        // only on the Ok setup path. It (and the ganger spawns) land in the same
        // end-of-update command flush, so the draw's `Added<Position>` is observable only
        // on the NEXT update — drive one more so the spawn system catches the
        // freshly-spawned gangers before the caller asserts on the sprites.
        if app.world().get_resource::<ShotRng>().is_some() {
            app.update();
            return true;
        }
    }
    false
}
