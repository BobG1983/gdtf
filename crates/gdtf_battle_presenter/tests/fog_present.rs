//! GTW-342 (GTW-48 leaf 6 / the ONLY VIEW leaf of GTW-13): headless draw-LOGIC tests
//! for the presenter FOG WRITER — the three-state terrain treatment (VISIBLE full /
//! EXPLORED dimmed / UNSEEN hidden), the enemy hard-cut + player always-shown actor
//! flags, and the CRITICAL post-level-cycle re-apply (proving the fog runs `.after`
//! `draw_static_battlefield` + `swap_destroyed_cover`).
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
    asset::AssetPlugin,
    ecs::{error::warn, message::Messages},
    platform::collections::HashSet,
    prelude::{Entity, Visibility, default},
    render::{RenderPlugin, settings::WgpuSettings},
    sprite::Sprite,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    ActiveLevel, GangerSprites, TerrainSprite, TopDownAtlases, TopDownRendererPlugin,
};
use gdtf_battle_sim::{
    Aiming, BattleSeed, Cell, CellLevel, CombatTuning, Direction, Facing, Faction, GangerName,
    GangerSpawn, Level, Position, SetupBattleRequested, SimRng, Situation, SquadVisibility,
    setup_battle_on_request,
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_weapon_registry,
    },
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
    // The fog writer reads CombatTuning for explored_dim (a Load-state resource the focused
    // setup path does NOT insert); author the Default (the shipped 0.55 dim flows through).
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

/// The (`Sprite.color`, `Visibility`) of the one `TerrainSprite` at `key`.
fn terrain_at(app: &mut App, key: CellLevel) -> Option<(bevy::prelude::Color, Visibility)> {
    let mut q = app
        .world_mut()
        .query::<(&TerrainSprite, &Sprite, &Visibility)>();
    q.iter(app.world())
        .find(|(t, ..)| t.at == key)
        .map(|(_, sprite, vis)| (sprite.color, *vis))
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

/// AC — terrain three-state: a squad-VISIBLE cell renders at full identity colour; an
/// EXPLORED-but-not-visible cell renders dimmed by `explored_dim` (RGB scaled, alpha
/// intact); an UNSEEN cell does not render its terrain (`Visibility::Hidden`).
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

    let dim = app
        .world()
        .get_resource::<CombatTuning>()
        .map(|t| *t.explored_dim);
    assert!(dim.is_some(), "CombatTuning must be resident");
    let Some(dim) = dim else { return };

    // VISIBLE -> full identity (white modulate), shown.
    let visible_terrain = terrain_at(&mut app, visible_cell);
    assert!(
        visible_terrain.is_some(),
        "the VISIBLE cell must have a terrain sprite"
    );
    let Some((vis_color, vis_flag)) = visible_terrain else {
        return;
    };
    assert_eq!(
        vis_color,
        bevy::prelude::Color::WHITE,
        "a squad-VISIBLE cell renders at full identity colour",
    );
    assert_eq!(vis_flag, Visibility::Inherited, "VISIBLE terrain is shown");

    // EXPLORED -> RGB x explored_dim, alpha untouched, shown.
    let explored_terrain = terrain_at(&mut app, explored_cell);
    assert!(
        explored_terrain.is_some(),
        "the EXPLORED cell must have a terrain sprite"
    );
    let Some((exp_color, exp_flag)) = explored_terrain else {
        return;
    };
    let exp = exp_color.to_srgba();
    let white = bevy::prelude::Color::WHITE.to_srgba();
    assert!(
        white.red.mul_add(-dim, exp.red).abs() < 1e-5,
        "EXPLORED RGB is the identity RGB times explored_dim (read from tuning, not a literal)",
    );
    assert!(
        (exp.alpha - white.alpha).abs() < 1e-5,
        "EXPLORED leaves alpha untouched",
    );
    assert!(
        exp.red < white.red,
        "EXPLORED is dimmer than VISIBLE (a memory, not live sight)",
    );
    assert_eq!(
        exp_flag,
        Visibility::Inherited,
        "EXPLORED terrain is shown (dimmed)",
    );

    // UNSEEN -> hidden.
    let unseen_terrain = terrain_at(&mut app, unseen_cell);
    assert!(
        unseen_terrain.is_some(),
        "the UNSEEN cell must have a terrain sprite"
    );
    let Some((_unseen_color, unseen_flag)) = unseen_terrain else {
        return;
    };
    assert_eq!(
        unseen_flag,
        Visibility::Hidden,
        "a never-seen cell does not render its terrain",
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
        "the level-1 VISIBLE cell must have a respawned sprite"
    );
    let Some((vis_color, vis_flag)) = visible_terrain else {
        return;
    };
    assert_eq!(
        vis_color,
        bevy::prelude::Color::WHITE,
        "the freshly-respawned VISIBLE level-1 terrain is at full colour",
    );
    assert_eq!(
        vis_flag,
        Visibility::Inherited,
        "VISIBLE level-1 terrain shown"
    );

    let unseen_terrain = terrain_at(&mut app, unseen_l1);
    assert!(
        unseen_terrain.is_some(),
        "the level-1 UNSEEN cell must have a respawned sprite"
    );
    let Some((_c, unseen_flag)) = unseen_terrain else {
        return;
    };
    assert_eq!(
        unseen_flag,
        Visibility::Hidden,
        "the freshly-respawned UNSEEN level-1 terrain is hidden — the fog re-applied on the \
         same cycle (proves present_fog runs .after draw_static_battlefield)",
    );
}
