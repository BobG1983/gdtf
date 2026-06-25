//! GTW-342 / GTW-348: headless draw-LOGIC tests for the presenter FOG WRITER — the
//! three-state terrain treatment (VISIBLE full colour `saturation` 1.0 / EXPLORED
//! full-brightness GREYSCALE `saturation` 0.0 / UNSEEN hidden), the enemy hard-cut +
//! player always-shown actor flags, and the CRITICAL post-level-cycle re-apply (proving
//! the fog runs `.after` `draw_static_battlefield` + `swap_destroyed_cover`).
//!
//! GTW-348 moved terrain from the `Sprite` path to a `Mesh2d` +
//! `MeshMaterial2d<TerrainFogMaterial>` so EXPLORED can render DESATURATED (the sprite
//! pipeline's multiply tint cannot desaturate). So the terrain assertions read each tile's
//! `TerrainFogMaterial.saturation` (from `Assets<TerrainFogMaterial>`) + `Visibility`,
//! NOT `Sprite.color`.
//!
//! These prove the WRITER LOGIC headless; "the fog horizon + enemy pop-in actually
//! render" is the in-engine QA evidence (the green suite + gate are blind to it — a
//! `Visible` sprite can still draw nothing, `bevy-traps.md` #8).
//!
//! The harness is the SAME `DefaultPlugins`/`no_renderer` app the terrain / ganger draw
//! tests use (a live `AssetServer` rooted at the workspace `assets/`, the
//! `TopDownRendererPlugin`, the sim lifecycle pieces for the REAL `setup_battle` spawn).
//! The gangers come from a `Situation` poured through the real setup; the
//! `SquadVisibility` fog + `CombatTuning` are authored DIRECTLY via `app.world_mut()` in
//! the test body — the accepted headless idiom (`bevy-traps.md` #7 carve-out (a)). No
//! function here takes `&mut World`/`&World`.

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
    ActiveLevel, GangerSprites, TerrainFogMaterial, TerrainSprite, TopDownAtlases,
    TopDownRendererPlugin,
};
use gdtf_battle_sim::{
    Aiming, BattleSeed, Cell, CellLevel, CombatTuning, CoverLedger, Direction, Facing, Faction,
    FovObserver, GangerName, GangerSpawn, Level, LifeState, OccupancyGrid, Position,
    SetupBattleRequested, SimRng, Situation, SquadVisibility, StairEyeOffset, Stance, StanceKind,
    SurfaceGrid, setup_battle_on_request,
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_weapon_registry,
    },
    union_fov,
};
use gdtf_test_utils::advance_until_resource_exists;

/// Bounded settle headroom for the post-setup SPAWN waits (a synchronous battle setup +
/// command flush, not an async load).
const MAX_UPDATES: u32 = 128;

/// Generous SAFETY-NET cap for the async atlas / tile-role loads (polls each resource's
/// inserted SIGNAL, not a fixed frame count — GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// A fixed seed for the deterministic `SetupBattleRequested` (the fog write is RNG-free).
const SEED: u64 = 0x0D15_EA5E;

/// The workspace-root `assets/` directory (this crate's manifest -> up two -> assets).
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// Builds the headless `DefaultPlugins`/`no_renderer` app with a live `AssetServer`, the
/// `TopDownRendererPlugin`, and the sim lifecycle pieces the real `setup_battle` spawn
/// path needs.
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
    app.insert_resource(test_armor_registry());
    // The dense-FOV helper (dense_visible_from_observer -> union_fov) reads CombatTuning for
    // view_range (a Load-state resource the focused setup path does NOT insert); author the
    // Default. GTW-348: the fog WRITER itself no longer reads CombatTuning (EXPLORED is
    // greyscale, not dimmed), so this is only for the union_fov helper.
    app.insert_resource(CombatTuning::default());
    app.set_error_handler(warn);
    app
}

/// Drive `update()`s until both async render resources have settled.
fn settle_resources(app: &mut App) {
    advance_until_resource_exists::<gdtf_battle_presenter::TileRoles>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(app, LOAD_SAFETY_NET);
}

/// An authored ganger at `at` (faction, facing) routed through the canonical shared
/// builder, named per faction.
fn ganger_at(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
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
fn drive_setup(app: &mut App, situation: Situation) -> bool {
    app.world_mut()
        .resource_mut::<Messages<SetupBattleRequested>>()
        .write(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    for _ in 0..MAX_UPDATES {
        app.update();
        if app.world().get_resource::<SimRng>().is_some() {
            app.update();
            return true;
        }
    }
    false
}

/// Insert a `SquadVisibility` fog with the given VISIBLE / EXPLORED cells. The setup path
/// inserts an empty `SquadVisibility` (GTW-341); this OVERWRITES it with the test's sets.
fn set_fog(app: &mut App, visible: &[CellLevel], explored: &[CellLevel]) {
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
fn terrain_at(app: &mut App, key: CellLevel) -> Option<(f32, Visibility)> {
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
fn sim_entity_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &Position)>();
    q.iter(app.world())
        .find(|(_, pos)| ***pos == at)
        .map(|(e, _)| e)
}

/// The `Visibility` of the presenter sprite mirroring sim ganger `sim`.
fn actor_visibility(app: &mut App, sim: Option<Entity>) -> Option<Visibility> {
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
fn settle_terrain_at(app: &mut App, at: CellLevel) -> bool {
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
fn settle_actor(app: &mut App, sim: Option<Entity>) -> bool {
    for _ in 0..MAX_UPDATES {
        if actor_visibility(app, sim).is_some() {
            return true;
        }
        app.update();
    }
    actor_visibility(app, sim).is_some()
}

/// AC (GTW-348) — terrain three-state: a squad-VISIBLE cell renders at full colour
/// (`TerrainFogMaterial.saturation == 1.0`); an EXPLORED-but-not-visible cell renders
/// full-brightness GREYSCALE (`saturation == 0.0` — colour-loss, not brightness-loss, as
/// the memory cue); an UNSEEN cell does not render its terrain (`Visibility::Hidden`).
///
/// Pin-discriminating: it asserts EXPLORED maps to `0.0` (greyscale) and VISIBLE to `1.0`
/// (colour); if EXPLORED were mapped to the wrong saturation (e.g. still dimmed, or left at
/// `1.0`), the EXPLORED assert fails.
#[test]
fn terrain_renders_visible_explored_unseen() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let visible_cell = CellLevel::new(Cell::new(5, 5), l0);
    let explored_cell = CellLevel::new(Cell::new(6, 6), l0);
    let unseen_cell = CellLevel::new(Cell::new(7, 7), l0);

    // One player ganger so setup spawns a valid battle (its cell is fogged-VISIBLE).
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(visible_cell, 0, Direction::East))
        .player_faction(Faction::new(0))
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");
    assert!(
        settle_terrain_at(&mut app, unseen_cell),
        "the terrain field must have drawn (every in-range cell is at least floor)",
    );

    // Author the fog: visible_cell VISIBLE, explored_cell EXPLORED-only, unseen_cell neither.
    set_fog(&mut app, &[visible_cell], &[explored_cell]);
    app.update();

    // VISIBLE -> full colour (saturation 1.0), shown.
    let visible_terrain = terrain_at(&mut app, visible_cell);
    assert!(
        visible_terrain.is_some(),
        "the VISIBLE cell must have a terrain tile + material"
    );
    let Some((vis_saturation, vis_flag)) = visible_terrain else {
        return;
    };
    assert!(
        (vis_saturation - 1.0).abs() < f32::EPSILON,
        "a squad-VISIBLE cell renders at full colour (saturation 1.0); got {vis_saturation}",
    );
    assert_eq!(vis_flag, Visibility::Inherited, "VISIBLE terrain is shown");

    // EXPLORED -> full-brightness greyscale (saturation 0.0), shown.
    let explored_terrain = terrain_at(&mut app, explored_cell);
    assert!(
        explored_terrain.is_some(),
        "the EXPLORED cell must have a terrain tile + material"
    );
    let Some((exp_saturation, exp_flag)) = explored_terrain else {
        return;
    };
    assert!(
        exp_saturation.abs() < f32::EPSILON,
        "an EXPLORED cell renders full-brightness GREYSCALE (saturation 0.0 — colour-loss as \
         the memory cue, not brightness-loss); got {exp_saturation}",
    );
    assert!(
        exp_saturation < vis_saturation,
        "EXPLORED is desaturated relative to VISIBLE (the memory cue is colour-loss)",
    );
    assert_eq!(
        exp_flag,
        Visibility::Inherited,
        "EXPLORED terrain is shown (greyscale, full brightness)",
    );

    // UNSEEN -> hidden.
    let unseen_terrain = terrain_at(&mut app, unseen_cell);
    assert!(
        unseen_terrain.is_some(),
        "the UNSEEN cell must have a terrain tile + material"
    );
    let Some((_unseen_saturation, unseen_flag)) = unseen_terrain else {
        return;
    };
    assert_eq!(
        unseen_flag,
        Visibility::Hidden,
        "a never-seen cell does not render its terrain",
    );
}

/// Compute the squad VISIBLE set the REAL way (GTW-347): run [`union_fov`]'s dense
/// Chebyshev disc scan from a single conscious player observer at `at`, over the world's
/// live sim grids (`OccupancyGrid` / `SurfaceGrid` / `CoverLedger`). Returns the visible
/// cells as a `Vec` (the test then authors them into the fog and asserts the presenter
/// renders them) — the same dense floor the presenter draws.
///
/// `is_dead` is the no-corpse predicate (no occupant is a corpse on the flat fixtures).
fn dense_visible_from_observer(app: &App, at: CellLevel) -> Vec<CellLevel> {
    let occupancy = app.world().resource::<OccupancyGrid>().clone();
    let surface = app.world().resource::<SurfaceGrid>().clone();
    let cover = app.world().resource::<CoverLedger>().clone();
    let tuning = app.world().resource::<CombatTuning>().clone();
    // A standing, East-facing conscious observer at `at` — the borrow-view union_fov needs.
    let position = Position::new(at);
    let stance = Stance::new(StanceKind::Standing);
    let facing = Facing::new(Direction::East);
    let observers = [FovObserver {
        position:         &position,
        stance:           &stance,
        facing:           &facing,
        life:             LifeState::Alive,
        stair_eye_offset: StairEyeOffset::new(0.0),
    }];
    let visible = union_fov(&observers, &occupancy, &surface, &cover, &tuning, |_| false);
    visible.into_iter().collect()
}

/// AC (GTW-347 clause 6 — the regression-fix render proof): with the **dense** VISIBLE set
/// `union_fov` produces around a player observer, the presenter renders the open-floor
/// `TerrainSprite`s on visible cells at full colour / `Visibility::Inherited` (NOT the
/// all-black bug), while a truly-unseen (out-of-range) floor cell stays `Hidden`.
///
/// This wires the REAL sim FOV (the dense disc scan over the live grids) into the REAL
/// presenter fog writer — proving the fix end to end: pre-GTW-347 the dense set was the
/// sparse authored/occupied subset, so an all-Open floor revealed nothing and the whole
/// terrain layer rendered Hidden (the all-black floor).
#[test]
fn dense_floor_set_renders_lit_floor_around_observer() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    // The player observer's cell, and an in-range open-floor neighbour the dense scan
    // reveals (both are open floor — no terrain authored at them).
    let observer_cell = CellLevel::new(Cell::new(10, 10), l0);
    let near_floor = CellLevel::new(Cell::new(11, 10), l0);
    // A floor cell far beyond the default view_range (14 Chebyshev) — never revealed.
    let far_floor = CellLevel::new(Cell::new(40, 40), l0);

    // One player ganger at the observer cell so setup spawns a valid battle; the rest of
    // the grid is open floor (the presenter floors every in-range Open cell).
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(observer_cell, 0, Direction::East))
        .player_faction(Faction::new(0))
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");
    assert!(
        settle_terrain_at(&mut app, far_floor),
        "the open-floor terrain field must have drawn (every in-range cell is at least floor)",
    );

    // Build the fog the REAL way: union_fov's dense disc scan from the player observer.
    let visible = dense_visible_from_observer(&app, observer_cell);
    assert!(
        visible.contains(&observer_cell) && visible.contains(&near_floor),
        "the dense scan reveals the observer's open-floor cell + an in-range floor neighbour",
    );
    assert!(
        !visible.contains(&far_floor),
        "a floor cell beyond view_range is NOT in the dense VISIBLE set",
    );
    set_fog(&mut app, &visible, &[]);
    app.update();

    // The presenter renders the VISIBLE open-floor cells LIT (full colour, shown) — the
    // lit disc around the squad, NOT the all-black bug.
    let observer_terrain = terrain_at(&mut app, observer_cell);
    let near_terrain = terrain_at(&mut app, near_floor);
    assert!(
        observer_terrain.is_some() && near_terrain.is_some(),
        "the observer + neighbour floor cells must have terrain sprites",
    );
    let (Some((obs_saturation, obs_flag)), Some((near_saturation, near_flag))) =
        (observer_terrain, near_terrain)
    else {
        return;
    };
    assert!(
        (obs_saturation - 1.0).abs() < f32::EPSILON,
        "the observer's open-floor cell renders at full colour (saturation 1.0 — lit, not black); \
         got {obs_saturation}",
    );
    assert_eq!(
        obs_flag,
        Visibility::Inherited,
        "the observer's open-floor cell is shown",
    );
    assert!(
        (near_saturation - 1.0).abs() < f32::EPSILON,
        "an in-range open-floor cell renders LIT at full colour (the dense fog reveals the \
         rendered floor); got {near_saturation}",
    );
    assert_eq!(
        near_flag,
        Visibility::Inherited,
        "an in-range open-floor cell is shown",
    );

    // The out-of-range floor cell stays Hidden (the fog horizon — dark beyond view_range).
    let far_terrain = terrain_at(&mut app, far_floor);
    assert!(
        far_terrain.is_some(),
        "the far open-floor cell must have a terrain tile (every cell is at least floor)",
    );
    let Some((_far_saturation, far_flag)) = far_terrain else {
        return;
    };
    assert_eq!(
        far_flag,
        Visibility::Hidden,
        "a floor cell beyond view_range stays Hidden — the fog horizon, not an all-black map",
    );
}

/// AC — actor hard-cut: an enemy on a non-VISIBLE cell is `Visibility::Hidden` (no ghost);
/// on a VISIBLE cell it shows; a player ganger always shows.
#[test]
fn enemy_hard_cuts_player_always_shown() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let player_cell = CellLevel::new(Cell::new(5, 5), l0);
    let enemy_cell = CellLevel::new(Cell::new(20, 20), l0);

    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(player_cell, 0, Direction::East))
        .with_ganger(ganger_at(enemy_cell, 1, Direction::West))
        .player_faction(Faction::new(0))
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");

    let player_sim = sim_entity_at(&mut app, player_cell);
    let enemy_sim = sim_entity_at(&mut app, enemy_cell);
    assert!(
        player_sim.is_some() && enemy_sim.is_some(),
        "both gangers must have spawned",
    );
    // Settle the DEFERRED ganger-sprite spawn (GTW-322) so both actor sprites exist before
    // the fog assertions (the spawn_scene materialization can lag under parallel load).
    assert!(
        settle_actor(&mut app, player_sim) && settle_actor(&mut app, enemy_sim),
        "both ganger sprites must have materialized",
    );

    // Fog: the player's cell is VISIBLE, the enemy's cell is NOT (only the player cell).
    set_fog(&mut app, &[player_cell], &[]);
    app.update();

    assert_eq!(
        actor_visibility(&mut app, player_sim),
        Some(Visibility::Inherited),
        "a player ganger is always shown",
    );
    assert_eq!(
        actor_visibility(&mut app, enemy_sim),
        Some(Visibility::Hidden),
        "an enemy on a non-VISIBLE cell is hard-hidden (no ghost)",
    );

    // Now make the enemy's cell VISIBLE: it pops in (hard cut, no fade).
    set_fog(&mut app, &[player_cell, enemy_cell], &[]);
    app.update();

    assert_eq!(
        actor_visibility(&mut app, enemy_sim),
        Some(Visibility::Inherited),
        "an enemy on a VISIBLE cell shows",
    );
    assert_eq!(
        actor_visibility(&mut app, player_sim),
        Some(Visibility::Inherited),
        "the player ganger still shows",
    );
}

/// AC (the CRITICAL ordering proof) — after an `ActiveLevel` CHANGE (which despawns +
/// respawns every `TerrainSprite`), the fog RE-APPLIES to the freshly-spawned terrain on
/// the SAME update. If the fog ran BEFORE `draw_static_battlefield`, it would colour the
/// stale/despawned entities and the new storey's terrain would render un-fogged.
#[test]
fn fog_reapplies_after_level_cycle() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let l1 = Level::new(1);
    // A cell that is UNSEEN on the level we cycle TO (so the freshly-drawn terrain there
    // must be hidden by the re-applied fog).
    let unseen_l1 = CellLevel::new(Cell::new(8, 8), l1);
    let visible_l1 = CellLevel::new(Cell::new(9, 9), l1);

    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(
            CellLevel::new(Cell::new(5, 5), l0),
            0,
            Direction::East,
        ))
        .player_faction(Faction::new(0))
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");
    assert!(
        settle_terrain_at(&mut app, CellLevel::new(Cell::new(5, 5), l0)),
        "the level-0 terrain field must have drawn",
    );

    // Author the level-1 fog: visible_l1 VISIBLE, unseen_l1 neither.
    set_fog(&mut app, &[visible_l1], &[]);

    // Cycle to level 1 (despawns + respawns terrain for the new storey on this update).
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    // One update: draw_static_battlefield respawns level-1 terrain, THEN (ordered .after)
    // present_fog colours it. Drive until the level-1 terrain exists (the respawn lands in
    // the end-of-update command flush) and then ensure the fog has run on it.
    assert!(
        settle_terrain_at(&mut app, unseen_l1),
        "after the level change, the level-1 terrain field must have respawned",
    );
    app.update();

    // The freshly-respawned level-1 terrain has the fog applied THIS cycle: the VISIBLE
    // cell is shown at full colour, the UNSEEN cell is hidden — proving present_fog ran
    // .after the despawn+respawn (not on the stale level-0 entities).
    let visible_terrain = terrain_at(&mut app, visible_l1);
    assert!(
        visible_terrain.is_some(),
        "the level-1 VISIBLE cell must have a respawned tile + material"
    );
    let Some((vis_saturation, vis_flag)) = visible_terrain else {
        return;
    };
    assert!(
        (vis_saturation - 1.0).abs() < f32::EPSILON,
        "the freshly-respawned VISIBLE level-1 terrain is at full colour (saturation 1.0); \
         got {vis_saturation}",
    );
    assert_eq!(
        vis_flag,
        Visibility::Inherited,
        "VISIBLE level-1 terrain shown"
    );

    let unseen_terrain = terrain_at(&mut app, unseen_l1);
    assert!(
        unseen_terrain.is_some(),
        "the level-1 UNSEEN cell must have a respawned tile + material"
    );
    let Some((_s, unseen_flag)) = unseen_terrain else {
        return;
    };
    assert_eq!(
        unseen_flag,
        Visibility::Hidden,
        "the freshly-respawned UNSEEN level-1 terrain is hidden — the fog re-applied on the \
         same cycle (proves present_fog runs .after draw_static_battlefield)",
    );
}
