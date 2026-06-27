//! GTW-219 (GTW-48 S5): headless draw-LOGIC tests for the ganger sprite draw — AC1
//! (an `Added<Position>` ganger on the active level spawns exactly one faction-coloured
//! sprite at `cell_to_world` with the facing-correct atlas index), AC3 (a
//! `Changed<Position>` MOVES the existing sprite and does not spawn a second), AC4 (a
//! `Changed<LifeState>` Downed re-tints and Dead despawns), AC5 (gangers are drawn for
//! the active level ONLY; an `ActiveLevel` change hides off-level and shows on-level).
//! AC2 (the pure 8->4 facing map) is the in-crate unit test in `src/ganger.rs`.
//!
//! These prove the DRAW LOGIC headless; "the right sprites appear on screen facing the
//! right way" is AC7's in-engine QA evidence. The harness is a `DefaultPlugins`/
//! `no_renderer` app (a live `AssetServer` rooted at the workspace `assets/` so
//! `TopDownAtlases` + the `character_roles.ron`-resolved `CharacterRoles` are resident)
//! plus `TopDownRendererPlugin` and the sim lifecycle pieces needed to drive the REAL
//! `setup_battle` spawn path (`SetupBattleRequested` -> `setup_battle_on_request`). The
//! gangers are NOT hand-spawned — they come from a `Situation` poured through the real
//! setup. `app.world_mut()` in the test body is the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::{error::warn, message::Messages},
    math::Vec2,
    prelude::{Entity, Visibility, default},
    render::{RenderPlugin, settings::WgpuSettings},
    sprite::Sprite,
    time::TimeUpdateStrategy,
    transform::components::Transform,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    ActiveLevel, CELL_PX, CharacterRoles, FacingFrame, GangerSprite, GangerSprites, Layer,
    TerrainSprite, TopDownAtlases, TopDownRendererPlugin, cell_to_world_layered, facing_frame,
};
use gdtf_battle_sim::{
    Aiming, BattleReady, BattleSeed, Cell, CellLevel, Direction, Facing, Faction, GangerSpawn,
    Level, LifeState, Position, SetupBattleRequested, ShotRng, Situation, Stance, StanceKind,
    setup_battle_on_request,
    test_support::{
        SituationBuilder, test_armor_registry, test_gang_registry, test_terrain_registry,
        test_weapon_registry,
    },
};
use gdtf_test_utils::advance_until_resource_exists;

/// Bounded settle headroom for the post-setup STATE / SPAWN waits (`drive_setup`'s
/// `ShotRng`-present signal, `settle_terrain_z_at`'s spawned-sprite signal). These wait on a
/// SYNCHRONOUS battle setup + its command flush, NOT on an async asset load, so a true small
/// frame count is the right termination — they are out of GTW-305's async-load scope.
const MAX_UPDATES: u32 = 128;

/// Generous SAFETY-NET cap for the async atlas / character-role loads polled by
/// [`settle_resources`]. It is a safety net against a genuine never-resolve hang, NOT a timing
/// budget: each gate resource is waited on by its inserted SIGNAL (not a fixed frame count),
/// which is what makes these draw tests deterministic under parallel `cargo` load (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// A fixed seed for the deterministic `SetupBattleRequested` (the draw is RNG-free; the
/// seed only feeds the unused battle RNG streams).
const SEED: u64 = 0x0D15_EA5E;

/// The workspace-root `assets/` directory (this crate's manifest -> up two -> assets),
/// the same root the running app uses so the shipped sheets + `character_roles.ron` load.
fn workspace_assets_root() -> PathBuf {
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
    .add_message::<gdtf_battle_sim::CoverDestroyed>()
    .add_systems(bevy::app::Update, setup_battle_on_request)
    .add_plugins(TopDownRendererPlugin);
    // The Load-built WeaponRegistry (GTW-257): setup_battle_on_request reads it to arm
    // each spawned ganger. The canonical shared [`test_weapon_registry`] (GTW-324),
    // inserted up front (the fixture gangers reference its [`TEST_WEAPON_KEY`]).
    app.insert_resource(test_weapon_registry());
    // The Load-built ArmorRegistry (GTW-269): setup_battle_on_request reads it to armor
    // each spawned ganger. The canonical shared [`test_armor_registry`] (GTW-324),
    // inserted up front (the fixture gangers reference its [`TEST_ARMOR_KEY`]).
    app.insert_resource(test_armor_registry());
    // GTW-396: the TerrainRegistry — setup_battle_on_request reads it to resolve cover,
    // slab, and floor piece keys. The canonical test registry supplies the four
    // `"test-wall"` / `"test-slab"` / `"test-cover"` / `"test-floor"` keys the
    // SituationBuilder uses.
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

/// Drives `update()`s until `CharacterRoles` + `TopDownAtlases` are BOTH resident (the async
/// load chain has settled), polling each resource's inserted SIGNAL rather than a fixed frame
/// count (GTW-305). Both resolve over the same async `AssetServer` chain, so waiting for them
/// in sequence drives the app until the last is present. Panics (naming the missing resource)
/// via [`advance_until_resource_exists`] if either is still absent after the safety-net cap — a
/// genuine load failure, surfaced loudly rather than leaving the draw systems silently no-op.
fn settle_resources(app: &mut App) {
    advance_until_resource_exists::<CharacterRoles>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(app, LOAD_SAFETY_NET);
}

/// The resolved `CharacterRoles` resource as a clone, or `None` if absent.
fn character_roles(app: &App) -> Option<CharacterRoles> {
    app.world().get_resource::<CharacterRoles>().cloned()
}

/// Build an authored ganger at `at` with the given faction + facing, otherwise plausible
/// component values (a standing, hip-firing, alive rifleman). Routed through the canonical
/// shared [`GangerSpawnBuilder`] (GTW-324) so the setup path spawns the same component set
/// the draw reads. The builder's TEST armor/weapon keys are `TEST_ARMOR_KEY` /
/// `TEST_WEAPON_KEY`, the same keys the canonical shared [`test_armor_registry`] /
/// [`test_weapon_registry`] resolve. Overrides off the builder defaults: the per-faction
/// `"Ganger {faction}"` name (the faction-distinct sprite-colour assertion reads it) and
/// `aiming(false)` (this is the hip-firing draw fixture; the builder default aims).
fn ganger_at(at: CellLevel, faction: u8, facing: Direction) -> GangerSpawn {
    use gdtf_battle_sim::{GangerName, test_support::GangerSpawnBuilder};
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
fn drive_setup(app: &mut App, situation: Situation) -> bool {
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

/// All `GangerSprite` markers in the world, paired with the sim entity each mirrors and
/// its sprite's atlas index / custom-size / translation. (Visibility is asserted per-sim
/// via [`visibility_of_sim`], so it is not snapshotted here.)
struct DrawnGanger {
    /// The presenter sprite entity.
    sprite_entity: Entity,
    /// The sim ganger entity it mirrors.
    sim_entity:    Entity,
    /// The sprite's texture-atlas index (if it carries an atlas).
    atlas_index:   Option<usize>,
    /// The sprite's `custom_size`.
    custom_size:   Option<Vec2>,
    /// The sprite's world translation.
    translation:   bevy::math::Vec3,
}

/// Snapshot every drawn ganger sprite (marker + sprite + transform).
fn drawn_gangers(app: &mut App) -> Vec<DrawnGanger> {
    let mut q = app
        .world_mut()
        .query::<(Entity, &GangerSprite, &Sprite, &Transform)>();
    q.iter(app.world())
        .map(|(sprite_entity, marker, sprite, transform)| DrawnGanger {
            sprite_entity,
            sim_entity: marker.entity,
            atlas_index: sprite.texture_atlas.as_ref().map(|a| a.index),
            custom_size: sprite.custom_size,
            translation: transform.translation,
        })
        .collect()
}

/// The world-space `z` of the terrain (floor / wall / cover) sprite drawn at `at`, if
/// the static-battlefield draw spawned one there. Every in-range cell is at least a
/// floor tile, so a co-located terrain sprite exists at a spawned ganger's cell.
fn terrain_z_at(app: &mut App, at: CellLevel) -> Option<f32> {
    let mut q = app.world_mut().query::<(&TerrainSprite, &Transform)>();
    q.iter(app.world())
        .find(|(marker, _)| marker.at == at)
        .map(|(_, transform)| transform.translation.z)
}

/// Drive bounded `update()`s until a co-located terrain sprite exists at `at`, returning
/// its `z` (or `None` if none appeared within `MAX_UPDATES`).
///
/// `draw_static_battlefield` (`terrain/draw.rs`) fires only when a `BattleReady` is drained
/// THIS update (or `ActiveLevel.is_changed()`) and `commands.spawn`s the `TerrainSprite`,
/// so the sprite is queryable only AFTER the end-of-update command flush. `drive_setup`'s
/// extra update is tuned for the ganger draw's `Added<Position>`, NOT the terrain draw's
/// `BattleReady`-read + spawn flush, which can land a frame later under parallel test
/// contention. Settling on the terrain z directly makes the AC2 assertion deterministic.
fn settle_terrain_z_at(app: &mut App, at: CellLevel) -> Option<f32> {
    for _ in 0..MAX_UPDATES {
        if let Some(z) = terrain_z_at(app, at) {
            return Some(z);
        }
        app.update();
    }
    terrain_z_at(app, at)
}

/// The sim entity that occupies `at`, found via its `Position` (the setup spawns one
/// ganger per authored cell). `None` if no ganger is there.
fn sim_entity_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &Position)>();
    q.iter(app.world())
        .find(|(_, pos)| ***pos == at)
        .map(|(e, _)| e)
}

/// The expected atlas index for `faction` facing `facing`, read STRUCTURALLY from the
/// data table + the 8->4 map — never a literal.
fn expected_index(roles: &CharacterRoles, faction: u8, facing: Direction) -> usize {
    *roles.base_for(Faction::new(faction)) + *facing_frame(facing)
}

/// AC1 — two `Added<Position>` gangers (distinct factions, distinct facings) on the
/// active level spawn exactly two faction-coloured sprites at `cell_to_world`, each with
/// the facing-correct atlas index read structurally; the two factions resolve to two
/// distinct base indices; `custom_size == Some(Vec2::splat(CELL_PX))`.
#[test]
fn added_gangers_spawn_one_faction_coloured_sprite_each() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let g0_at = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let g1_at = CellLevel::new(Cell::new(12, 9), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(g0_at, 0, Direction::East))
        .with_ganger(ganger_at(g1_at, 1, Direction::North))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    let roles = character_roles(&app);
    assert!(roles.is_some(), "CharacterRoles must be resident");
    let Some(roles) = roles else { return };

    // The two factions resolve to two distinct base indices (visibly distinct actors).
    assert_ne!(
        roles.base_for(Faction::new(0)),
        roles.base_for(Faction::new(1)),
        "the two factions must resolve to two distinct actor base indices",
    );

    let drawn = drawn_gangers(&mut app);
    assert_eq!(
        drawn.len(),
        2,
        "exactly two ganger sprites spawn (one per authored ganger), got {}",
        drawn.len(),
    );

    // Map each drawn sprite back to its authored cell + faction + facing.
    let g0_sim = sim_entity_at(&mut app, g0_at);
    let g1_sim = sim_entity_at(&mut app, g1_at);
    assert!(
        g0_sim.is_some() && g1_sim.is_some(),
        "both authored gangers must have spawned sim entities",
    );

    for d in &drawn {
        // Sizing: every ganger sprite is one cell.
        assert_eq!(
            d.custom_size,
            Some(Vec2::splat(CELL_PX)),
            "every ganger sprite must be custom_size Some(Vec2::splat(CELL_PX))",
        );
        // Every drawn sprite mirrors one of the two authored gangers.
        let mirrors_authored = Some(d.sim_entity) == g0_sim || Some(d.sim_entity) == g1_sim;
        assert!(
            mirrors_authored,
            "every drawn sprite must mirror one of the two authored gangers",
        );
        // Index + position, structural per sim entity.
        if Some(d.sim_entity) == g0_sim {
            assert_eq!(
                d.atlas_index,
                Some(expected_index(&roles, 0, Direction::East)),
                "faction-0 ganger sprite index = faction_0 base + East(RIGHT) offset",
            );
            assert_eq!(
                d.translation,
                cell_to_world_layered(Cell::new(5, 6), Level::new(0), Layer::Actor),
                "faction-0 sprite at the Actor-layer projection of its authored Position",
            );
        } else if Some(d.sim_entity) == g1_sim {
            assert_eq!(
                d.atlas_index,
                Some(expected_index(&roles, 1, Direction::North)),
                "faction-1 ganger sprite index = faction_1 base + North(UP) offset",
            );
            assert_eq!(
                d.translation,
                cell_to_world_layered(Cell::new(12, 9), Level::new(0), Layer::Actor),
                "faction-1 sprite at the Actor-layer projection of its authored Position",
            );
        }
    }
}

/// AC3 — a `Changed<Position>` MOVES the existing presenter sprite (looked up through
/// `GangerSprites`) and does NOT spawn a second.
#[test]
fn changed_position_moves_the_same_sprite() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let start = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(start, 0, Direction::East))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    let drawn_before = drawn_gangers(&mut app);
    assert_eq!(drawn_before.len(), 1, "one ganger sprite before the move");
    let sprite_before = drawn_before[0].sprite_entity;

    let sim = sim_entity_at(&mut app, start);
    assert!(sim.is_some(), "the ganger sim entity must exist");
    let Some(sim) = sim else { return };

    // The presenter sprite the map links to this sim entity, captured before the move.
    let mapped_before = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim));
    assert_eq!(
        mapped_before,
        Some(sprite_before),
        "the map links the sim ganger to its one sprite",
    );

    // Move the ganger (mutate its Position — the real Changed<Position> trigger).
    let dest = CellLevel::new(Cell::new(7, 8), Level::new(0));
    let mut pos_q = app.world_mut().query::<&mut Position>();
    if let Ok(mut pos) = pos_q.get_mut(app.world_mut(), sim) {
        *pos = Position::new(dest);
    }
    // GTW-359 (C4): the move is now GLIDED (a re-targeting tween), not snapped — so the
    // sprite reaches the new cell over a few frames, not in one update. Settle the glide
    // (bounded), then assert it landed exactly on the new cell.
    let dest_world = cell_to_world_layered(Cell::new(7, 8), Level::new(0), Layer::Actor);
    settle_sprite_at(&mut app, sprite_before, dest_world);

    let drawn_after = drawn_gangers(&mut app);
    assert_eq!(
        drawn_after.len(),
        1,
        "still exactly one ganger sprite after the move (not respawned), got {}",
        drawn_after.len(),
    );
    // The SAME presenter sprite entity, now at the new cell.
    assert_eq!(
        drawn_after[0].sprite_entity, sprite_before,
        "the move reuses the SAME presenter sprite entity",
    );
    assert!(
        drawn_after[0].translation.distance(dest_world) < 1.0e-3,
        "the sprite glided to the Actor-layer projection of the new Position (got {:?}, \
         expected {dest_world:?})",
        drawn_after[0].translation,
    );
    // And the map still points at that same sprite.
    let mapped_after = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim));
    assert_eq!(
        mapped_after,
        Some(sprite_before),
        "the map still links the sim ganger to its one (moved) sprite",
    );
}

/// AC4 — a `Changed<LifeState>` Downed re-tints the sprite, and Dead despawns it +
/// drops its `GangerSprites` entry.
#[test]
fn downed_retints_and_dead_despawns() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let at = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(at, 0, Direction::East))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    let sim = sim_entity_at(&mut app, at);
    assert!(sim.is_some(), "the ganger sim entity must exist");
    let Some(sim) = sim else { return };

    // The live sprite's tint, captured before downing.
    let drawn_alive = drawn_gangers(&mut app);
    assert_eq!(drawn_alive.len(), 1, "one live ganger sprite");
    let sprite = drawn_alive[0].sprite_entity;
    let alive_color = sprite_color(&mut app, sprite);

    // Down the ganger (mutate LifeState — the real Changed<LifeState> trigger).
    set_life_state(&mut app, sim, LifeState::Downed);
    app.update();

    // The sprite is still present but re-tinted (the documented Downed delta).
    let drawn_downed = drawn_gangers(&mut app);
    assert_eq!(drawn_downed.len(), 1, "the Downed ganger's sprite remains");
    let downed_color = sprite_color(&mut app, sprite);
    assert!(
        alive_color != downed_color,
        "a Downed ganger's sprite must re-tint (live {alive_color:?} vs downed {downed_color:?})",
    );

    // Kill the ganger (mutate LifeState to Dead — the despawn delta).
    set_life_state(&mut app, sim, LifeState::Dead);
    app.update();

    // The presenter sprite is despawned and its map entry dropped.
    let drawn_dead = drawn_gangers(&mut app);
    assert_eq!(
        drawn_dead.len(),
        0,
        "a Dead ganger's presenter sprite must be despawned",
    );
    let still_mapped = app
        .world()
        .get_resource::<GangerSprites>()
        .map(|m| m.contains(sim));
    assert_eq!(
        still_mapped,
        Some(false),
        "no live GangerSprites entry remains for a Dead ganger",
    );
}

/// AC5 — gangers are drawn for the active level ONLY; an `ActiveLevel` change hides
/// off-level and shows on-level sprites.
#[test]
fn active_level_change_hides_off_level_shows_on_level() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0_at = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let l1_at = CellLevel::new(Cell::new(7, 8), Level::new(1));
    // Author both endpoints as slabs so the (implicit) cell existence is clean — not
    // strictly required for the draw, but keeps the situation valid.
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(l0_at, 0, Direction::East))
        .with_ganger(ganger_at(l1_at, 1, Direction::West))
        .slab_at(l0_at)
        .slab_at(l1_at)
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    let l0_sim = sim_entity_at(&mut app, l0_at);
    let l1_sim = sim_entity_at(&mut app, l1_at);
    assert!(
        l0_sim.is_some() && l1_sim.is_some(),
        "both gangers must have spawned",
    );

    // At active level 0: the level-0 ganger is visible, the level-1 ganger hidden.
    assert_eq!(
        visibility_of_sim(&mut app, l0_sim),
        Some(Visibility::Inherited),
        "the level-0 ganger sprite is visible at active level 0",
    );
    assert_eq!(
        visibility_of_sim(&mut app, l1_sim),
        Some(Visibility::Hidden),
        "the level-1 ganger sprite is hidden at active level 0",
    );

    // Change the active level to 1.
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(Level::new(1));
    app.update();

    // Now the level-1 ganger is shown and the level-0 ganger hidden.
    assert_eq!(
        visibility_of_sim(&mut app, l1_sim),
        Some(Visibility::Inherited),
        "after the change, the level-1 ganger sprite is visible",
    );
    assert_eq!(
        visibility_of_sim(&mut app, l0_sim),
        Some(Visibility::Hidden),
        "after the change, the level-0 ganger sprite is hidden",
    );
}

/// GTW-283 AC2 — a ganger sprite draws ON TOP of (in front of) its OWN floor tile, both
/// at spawn and after a move. Drives the REAL setup spawn + the REAL terrain draw, then
/// asserts (a) the ganger's `z` is strictly greater than the co-located terrain (floor)
/// `z`, (b) `0.0 < ganger.z < 1.0` at level 0 (the lift stays within its own storey band,
/// never crossing into the next storey), and (c) after a `Changed<Position>` move the
/// moved ganger STILL has `z` > the terrain `z` at its NEW cell. RED before the fix (both
/// projected to `z = 0.0`, so neither the strict-greater nor the `> 0.0` bound held),
/// GREEN after. The rasterized "the figure draws over the floor" itself is AC7 in-engine QA.
#[test]
fn ganger_draws_above_its_own_floor_at_spawn_and_after_move() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let start = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(start, 0, Direction::East))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    // Settle the terrain draw first: its BattleReady-read + spawn flush can land a frame
    // behind drive_setup's ganger-tuned update under parallel test contention, so drive
    // bounded update()s until the co-located floor sprite exists (deterministic, no flake).
    let floor_z = settle_terrain_z_at(&mut app, start);
    assert!(
        floor_z.is_some(),
        "a co-located floor tile must be drawn under the ganger's cell",
    );
    let Some(floor_z) = floor_z else { return };

    // The single ganger sprite's z (read after the terrain settle; the ganger has not
    // moved, so its spawn z is unchanged by the extra terrain-settle updates).
    let drawn = drawn_gangers(&mut app);
    assert_eq!(drawn.len(), 1, "one ganger sprite at spawn");
    let ganger_z = drawn[0].translation.z;

    // (a) the ganger draws strictly in FRONT of its own floor tile.
    assert!(
        ganger_z > floor_z,
        "the ganger sprite z ({ganger_z}) must be strictly greater than its own floor z \
         ({floor_z}) — it draws on top of, not behind, its floor",
    );
    // (b) the lift stays within the storey-0 band: 0.0 < z < 1.0.
    assert!(
        ganger_z > 0.0 && ganger_z < 1.0,
        "the level-0 ganger z ({ganger_z}) must satisfy 0.0 < z < 1.0 (within its own storey)",
    );

    // (c) after a move the moved ganger STILL draws above the floor at its NEW cell.
    let sim = sim_entity_at(&mut app, start);
    assert!(sim.is_some(), "the spawned ganger sim entity must exist");
    let Some(sim) = sim else { return };
    let dest = CellLevel::new(Cell::new(7, 8), Level::new(0));
    let mut pos_q = app.world_mut().query::<&mut Position>();
    if let Ok(mut pos) = pos_q.get_mut(app.world_mut(), sim) {
        *pos = Position::new(dest);
    }
    app.update();

    let drawn_after = drawn_gangers(&mut app);
    assert_eq!(
        drawn_after.len(),
        1,
        "still one ganger sprite after the move"
    );
    let moved_z = drawn_after[0].translation.z;
    let dest_floor_z = settle_terrain_z_at(&mut app, dest);
    assert!(
        dest_floor_z.is_some(),
        "a co-located floor tile must be drawn under the moved ganger's new cell",
    );
    let Some(dest_floor_z) = dest_floor_z else {
        return;
    };
    assert!(
        moved_z > dest_floor_z,
        "after the move the ganger z ({moved_z}) must STILL be strictly greater than the floor \
         z at its new cell ({dest_floor_z}) — the bias holds across moves",
    );
}

/// Reframe / re-tint (the contract's "asserted to match" clause, identical phrasing to
/// the Death clause AC4 asserts) — drive the REAL change path on the live presenter
/// sprite:
///
/// - `Changed<Facing>` recomputes the atlas index via the 8->4 map: the sprite re-indexes
///   to `base_for(faction) + facing_frame(new_facing)` read STRUCTURALLY (never a
///   literal), distinct from the spawn frame.
/// - `Changed<Stance>` (-> `Prone`) dims the tint; `Changed<Aiming>` (-> `true`)
///   brightens it — both on the SAME presenter sprite, each a distinct, expected-direction
///   re-tint.
///
/// This pins `reframe_ganger_sprites` + `stance_aiming_tint` on the real path: deleting
/// the reframe system (or no-opping its atlas/color writes) FAILS this test.
#[test]
fn changed_facing_reframes_and_stance_aiming_retints_the_same_sprite() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    // Spawn fixture: faction 0, facing East (RIGHT frame), Standing, not aiming.
    let at = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(at, 0, Direction::East))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    let roles = character_roles(&app);
    assert!(roles.is_some(), "CharacterRoles must be resident");
    let Some(roles) = roles else { return };

    let sim = sim_entity_at(&mut app, at);
    assert!(sim.is_some(), "the ganger sim entity must exist");
    let Some(sim) = sim else { return };

    // The presenter sprite the map links to this sim entity (the SAME one we reframe).
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim));
    assert!(sprite.is_some(), "the ganger must be mapped to a sprite");
    let Some(sprite) = sprite else { return };

    // --- Facing reframe: East (RIGHT) -> North (UP). ---
    let index_east = atlas_index_of_sim(&mut app, sim);
    assert_eq!(
        index_east,
        Some(expected_index(&roles, 0, Direction::East)),
        "spawn frame = faction_0 base + East(RIGHT) offset",
    );

    set_facing(&mut app, sim, Direction::North);
    app.update();

    let index_north = atlas_index_of_sim(&mut app, sim);
    assert_eq!(
        index_north,
        Some(expected_index(&roles, 0, Direction::North)),
        "after Changed<Facing> the SAME sprite re-indexes to faction_0 base + North(UP) \
         offset, read structurally from the table + the 8->4 map",
    );
    assert_ne!(
        index_north, index_east,
        "the reframe actually changed the atlas index (East RIGHT vs North UP)",
    );

    // --- Stance re-tint: Standing -> Prone dims. ---
    let color_standing = sprite_color(&mut app, sprite);
    set_stance(&mut app, sim, StanceKind::Prone);
    app.update();
    let color_prone = sprite_color(&mut app, sprite);
    assert!(
        color_standing != color_prone,
        "a Prone ganger re-tints (standing {color_standing:?} vs prone {color_prone:?})",
    );
    assert!(
        luminance(color_prone) < luminance(color_standing),
        "Prone dims the sprite (prone luminance must be < standing luminance)",
    );

    // --- Aiming re-tint: not-aiming -> aiming brightens (from the Prone baseline). ---
    set_aiming(&mut app, sim, true);
    app.update();
    let color_prone_aiming = sprite_color(&mut app, sprite);
    assert!(
        color_prone != color_prone_aiming,
        "an aiming ganger re-tints (prone {color_prone:?} vs prone+aiming \
         {color_prone_aiming:?})",
    );
    assert!(
        luminance(color_prone_aiming) > luminance(color_prone),
        "aiming brightens the sprite (prone+aiming luminance must be > prone luminance)",
    );
}

/// The linear-RGB luminance proxy (unweighted channel sum) of a sprite color — enough to
/// assert the dim/brighten DIRECTION of the stance/aiming re-tint without pinning exact
/// channel values. `None` colors (no sprite) sort to `0.0`.
fn luminance(color: Option<bevy::color::Color>) -> f32 {
    match color {
        Some(c) => {
            let lin = c.to_linear();
            lin.red + lin.green + lin.blue
        }
        None => 0.0,
    }
}

/// The `Sprite.color` of the presenter sprite `entity`.
fn sprite_color(app: &mut App, entity: Entity) -> Option<bevy::color::Color> {
    let mut q = app.world_mut().query::<&Sprite>();
    q.get(app.world(), entity).ok().map(|s| s.color)
}

/// The world translation of the presenter sprite `entity`, if it exists.
fn sprite_translation(app: &mut App, entity: Entity) -> Option<bevy::math::Vec3> {
    let mut q = app.world_mut().query::<&Transform>();
    q.get(app.world(), entity).ok().map(|t| t.translation)
}

/// Drive bounded `update()`s until the presenter sprite `entity` GLIDES to within a hair
/// of `target` (GTW-359 C4: the move is now a tween, so it settles over frames, not in one
/// update).
///
/// Installs [`TimeUpdateStrategy::ManualDuration`] so each `app.update()` advances
/// `Res<Time>` by a FIXED, generous delta (longer than the tween's own glide duration),
/// making the settle DETERMINISTIC rather than dependent on real wall-clock deltas under
/// parallel test load — a couple of updates land the glide exactly on `target`. The loop
/// is bounded so a never-arriving glide surfaces as a failed assertion, not a hang.
fn settle_sprite_at(app: &mut App, entity: Entity, target: bevy::math::Vec3) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(500),
    ));
    for _ in 0..MAX_UPDATES {
        app.update();
        if sprite_translation(app, entity).is_some_and(|t| t.distance(target) < 1.0e-3) {
            return;
        }
    }
}

/// Set the `LifeState` of sim ganger `sim` (the real `Changed<LifeState>` trigger).
fn set_life_state(app: &mut App, sim: Entity, state: LifeState) {
    let mut q = app.world_mut().query::<&mut LifeState>();
    if let Ok(mut life) = q.get_mut(app.world_mut(), sim) {
        *life = state;
    }
}

/// Set the `Facing` of sim ganger `sim` (the real `Changed<Facing>` reframe trigger).
fn set_facing(app: &mut App, sim: Entity, facing: Direction) {
    let mut q = app.world_mut().query::<&mut Facing>();
    if let Ok(mut f) = q.get_mut(app.world_mut(), sim) {
        *f = Facing::new(facing);
    }
}

/// Set the `Stance` of sim ganger `sim` (the real `Changed<Stance>` re-tint trigger).
fn set_stance(app: &mut App, sim: Entity, stance: StanceKind) {
    let mut q = app.world_mut().query::<&mut Stance>();
    if let Ok(mut s) = q.get_mut(app.world_mut(), sim) {
        *s = Stance::new(stance);
    }
}

/// Set the `Aiming` flag of sim ganger `sim` (the real `Changed<Aiming>` re-tint trigger).
fn set_aiming(app: &mut App, sim: Entity, aiming: bool) {
    let mut q = app.world_mut().query::<&mut Aiming>();
    if let Ok(mut a) = q.get_mut(app.world_mut(), sim) {
        *a = Aiming::new(aiming);
    }
}

/// The presenter sprite's atlas index (looked up through `GangerSprites`), or `None`.
fn atlas_index_of_sim(app: &mut App, sim: Entity) -> Option<usize> {
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim))?;
    let mut q = app.world_mut().query::<&Sprite>();
    q.get(app.world(), sprite)
        .ok()
        .and_then(|s| s.texture_atlas.as_ref().map(|a| a.index))
}

/// The visibility of the presenter sprite mirroring sim ganger `sim` (via the map).
fn visibility_of_sim(app: &mut App, sim: Option<Entity>) -> Option<Visibility> {
    let sim = sim?;
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim))?;
    let mut q = app.world_mut().query::<&Visibility>();
    q.get(app.world(), sprite).ok().copied()
}

/// A compile-time witness that the public `FacingFrame` offsets are reachable from the
/// integration boundary (the structural sum the AC1 index assert relies on).
#[test]
fn facing_frame_offsets_are_public() {
    assert_eq!(*FacingFrame::LEFT, 0);
    assert_eq!(*FacingFrame::DOWN, 1);
    assert_eq!(*FacingFrame::UP, 2);
    assert_eq!(*FacingFrame::RIGHT, 3);
}
