//! GTW-220 (GTW-48 S6): headless draw-LOGIC tests for the transient FX-flash layer —
//! AC1 (a `Bleeding` spawns one cell-positioned `FxFlash` at the table's bleed index, with a
//! `*Wounds`-relation tint, and fail-closes with no `Position` / `Wounds`), AC2 (an
//! `ArmorBroken` spawns one at the table's armor-break index), AC3 (a `CoverDestroyed` spawns
//! one at `cell_to_world(at)` at the cover-destroyed index), AC4 (`FlashTtl` expiry despawns
//! the flash; a further empty update spawns nothing), AC5 (two `Bleeding` in one frame spawn
//! two independent flashes).
//!
//! These prove the DRAW + EXPIRY LOGIC headless; "the flashes visibly pop + fade" is AC7's
//! deferred S9-capstone in-engine QA. The harness is a `DefaultPlugins`/`no_renderer` app (a
//! live `AssetServer` rooted at the workspace `assets/` so `TopDownAtlases` + the
//! `effect_roles.ron`-resolved `EffectRoles` are resident) plus `TopDownRendererPlugin` and
//! the three sim FX message buffers the readers drain (`Bleeding` / `ArmorBroken` /
//! `CoverDestroyed`). The gangers + `BattleInProgress` are authored, and the FX messages are
//! written, DIRECTLY via `app.world_mut()` in the test body — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::{error::warn, message::Messages},
    prelude::{Text2d, default},
    render::{RenderPlugin, settings::WgpuSettings},
    sprite::Sprite,
    text::TextColor,
    time::TimeUpdateStrategy,
    transform::components::Transform,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    CharacterRoles, EffectRoles, FctValence, FloatingCombatText, FxFlash, FxTuning, GangerSprite,
    GangerSprites, ProjectileTravel, ShotImpactResolved, ShotProjectile, TopDownAtlases,
    TopDownRendererPlugin, cell_to_world, severity_color, sim_pos_to_world, valence_color,
};
use gdtf_battle_sim::{
    AppliedDamage, ArmorBroken, BattleInProgress, Bleeding, BodyPart, Cell, CellLevel,
    CoverDestroyed, DamageType, Direction, Facing, Faction, HitReport, HitResult, HpDamage,
    IntegrityWear, Level, LifeState, Matchup, PenetratingDamage, Position, Severity, ShotDir,
    ShotFired, ShotKind, SimPos, Wounds,
};
use gdtf_test_utils::advance_until_resource_exists;

/// Generous SAFETY-NET cap for the async atlas / effect-role / tuning / character
/// loads. It is a safety net against a genuine never-resolve hang, NOT a timing
/// budget: each gate resource is waited on by its inserted SIGNAL (not a fixed
/// frame count), which is what makes these FX tests deterministic under parallel
/// `cargo` load (GTW-305). A fixed 128-update budget previously starved under
/// contention and silently left the `run_if`-gated spawn systems no-op (0 vs 1
/// projectile).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The workspace-root `assets/` directory (this crate's manifest → up two → assets),
/// the same root the running app uses so the shipped sheets + `effect_roles.ron` load.
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// Builds a headless `DefaultPlugins`/`no_renderer` app with a live `AssetServer`
/// (workspace `assets/`), the `TopDownRendererPlugin`, and the three sim FX message buffers
/// the readers drain. It does NOT add the sim's lifecycle systems — the test authors the
/// gangers + `BattleInProgress` directly and writes the FX messages itself.
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
    // The FX readers drain these buffers; the sim's plugins register them in the real app,
    // but this focused harness adds only the ones the readers need (the GTW-290 ShotFired
    // included).
    .add_message::<Bleeding>()
    .add_message::<ArmorBroken>()
    .add_message::<CoverDestroyed>()
    .add_message::<ShotFired>()
    .add_plugins(TopDownRendererPlugin);
    // Bevy 0.19 routes a FAILED system-param validation to the global error handler
    // (default panics); 0.18 silently SKIPPED. This no-renderer harness lacks the
    // render-provided resources some DefaultPlugins systems want (e.g. bevy_light's
    // update_gizmo_meshes -> Assets<GizmoAsset>), so `warn` restores the 0.18 skip
    // behavior instead of an intermittent headless panic.
    app.set_error_handler(warn);
    app
}

/// Drives `update()`s until `EffectRoles` + `TopDownAtlases` + `FxTuning` + `CharacterRoles`
/// are ALL resident (the async load chain has settled), polling each resource's inserted
/// SIGNAL rather than a fixed frame count (GTW-305). All four resolve over the same async
/// `AssetServer` chain, so waiting for them in sequence drives the app until the LAST one
/// is present. Panics (naming the missing resource) if any is still absent after the
/// safety-net cap — a genuine load failure, surfaced loudly rather than silently leaving the
/// `run_if`-gated spawn systems no-op.
///
/// `FxTuning` (GTW-306) is part of the settle gate because `spawn_shot_projectiles` now
/// `run_if(resource_exists::<FxTuning>)` — without waiting for it the projectile-spawn tests
/// flake (the system silently does not run until the hot-reloadable tuning has resolved).
/// `CharacterRoles` is part of the gate so the entity-aim test's real `spawn_ganger_sprites`
/// path (gated `run_if(resource_exists::<CharacterRoles>)`) actually runs and registers the
/// hit ganger's presenter sprite in `GangerSprites` — the lookup the entity-aim branch reads.
fn settle_resources(app: &mut App) {
    advance_until_resource_exists::<EffectRoles>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<FxTuning>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<CharacterRoles>(app, LOAD_SAFETY_NET);
}

/// Reads the resolved `EffectRoles` resource as a clone, or `None` if it is absent (the
/// caller asserts it is `Some` — `settle_resources` already gated on its presence).
fn effect_roles(app: &App) -> Option<EffectRoles> {
    app.world().get_resource::<EffectRoles>().cloned()
}

/// Spawns a ganger entity carrying a `Position` at `cell`/`level` plus `Wounds(wounds)`, and
/// returns its `Entity` (the readers look it up by that entity).
fn spawn_ganger(app: &mut App, cell: Cell, level: Level, wounds: u8) -> bevy::ecs::entity::Entity {
    let at = CellLevel::new(cell, level);
    app.world_mut()
        .spawn((Position::new(at), Wounds::new(wounds)))
        .id()
}

/// Counts the `FxFlash` entities currently in the world.
fn fx_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&FxFlash>();
    q.iter(app.world()).count()
}

/// The (translation, atlas index) of the SINGLE `FxFlash` sprite — asserts exactly one exists.
/// Returns `None` if zero or more than one flash exists (the caller asserts `Some`).
fn single_flash(app: &mut App) -> Option<(bevy::math::Vec3, Option<usize>)> {
    let mut q = app.world_mut().query::<(&FxFlash, &Sprite, &Transform)>();
    let mut found: Option<(bevy::math::Vec3, Option<usize>)> = None;
    for (_, sprite, transform) in q.iter(app.world()) {
        if found.is_some() {
            // More than one flash — the caller wants exactly one.
            return None;
        }
        found = Some((
            transform.translation,
            sprite.texture_atlas.as_ref().map(|atlas| atlas.index),
        ));
    }
    found
}

/// Advances the clock past the flash TTL so `expire_flashes` ticks each live flash past
/// expiry, then restores `Automatic`.
///
/// Sets `TimeUpdateStrategy::ManualDuration(250ms)` (the Bevy test idiom) — 250ms is the
/// virtual clock's default `max_delta` clamp, so a single larger jump would be capped at
/// 250ms anyway. It then runs a SMALL FIXED number of `update()`s (each advancing the virtual
/// `Time` by 250ms, the `delta` `expire_flashes` ticks with), comfortably exceeding the
/// sub-second `FlashTtl` window across the batch. Bounded (no spin), and no new message is
/// written across these updates, so no new flash spawns.
fn advance_past_ttl(app: &mut App) {
    /// The per-update manual delta (== the virtual clock's default `max_delta` clamp).
    const STEP: std::time::Duration = std::time::Duration::from_millis(250);
    /// Enough 250ms steps to clear any sub-second `FlashTtl` window with margin.
    const STEPS: u32 = 8;

    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(STEP));
    for _ in 0..STEPS {
        app.update();
    }
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
}

/// Runs exactly ONE `update()` with the clock delta pinned to ZERO, then restores `Automatic`
/// time — the firing-FX read frame, made delta-deterministic.
///
/// The firing tests write a `ShotFired` and then read the spawned projectile expecting it AT the
/// muzzle. Under `DefaultPlugins`' default `Automatic` time the first update after the
/// variable-length [`settle_resources`] carries a non-deterministic wall-clock delta, so
/// `advance_projectiles` could move the bolt a contention-dependent distance off the muzzle and
/// flake the assertion (GTW-305, same parallel-`cargo` non-determinism family as the load flakes).
/// A `ManualDuration(0)` delta keeps the bolt exactly where `spawn_shot_projectiles` placed it,
/// regardless of scheduling, without weakening any assertion.
fn fire_with_zero_delta(app: &mut App) {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ));
    app.update();
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
}

/// AC1 — a `Bleeding { ganger }` spawns EXACTLY ONE `FxFlash` at the ganger's cell with the
/// table's `bleed` atlas index (read structurally from `EffectRoles`, never a literal), and a
/// `*Wounds`-relation tint. A `Bleeding` for an entity with no `Position`/`Wounds` spawns no
/// flash and does not panic (fail-closed).
#[test]
fn bleeding_spawns_one_flash_at_the_ganger_cell_with_the_bleed_index() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(5, 6);
    let level = Level::new(0);
    let ganger = spawn_ganger(&mut app, cell, level, 3);

    app.world_mut()
        .resource_mut::<Messages<Bleeding>>()
        .write(Bleeding::new(ganger));
    app.update();

    let roles = effect_roles(&app);
    assert!(roles.is_some(), "EffectRoles must be resident after settle");
    let Some(roles) = roles else { return };

    assert_eq!(fx_count(&mut app), 1, "exactly one bleed flash must spawn");
    let flash = single_flash(&mut app);
    assert!(flash.is_some(), "exactly one FxFlash sprite must exist");
    let Some((translation, index)) = flash else {
        return;
    };
    assert_eq!(
        translation,
        cell_to_world(cell, level),
        "the bleed flash must sit at cell_to_world(ganger's cell, level)",
    );
    assert_eq!(
        index,
        Some(*roles.bleed),
        "the bleed flash's atlas index must equal the table's bleed role index",
    );

    // Fail-closed: a Bleeding for an entity with NO Position/Wounds spawns nothing, no panic.
    // First clear the existing flash so the count is unambiguous.
    advance_past_ttl(&mut app);
    assert_eq!(fx_count(&mut app), 0, "the first flash must have expired");
    let bare = app.world_mut().spawn_empty().id();
    app.world_mut()
        .resource_mut::<Messages<Bleeding>>()
        .write(Bleeding::new(bare));
    app.update();
    assert_eq!(
        fx_count(&mut app),
        0,
        "a Bleeding for a Position/Wounds-less entity must spawn no flash (fail-closed)",
    );
}

/// AC2 — an `ArmorBroken { ganger, part }` spawns exactly one `FxFlash` at the ganger's cell
/// with the table's `armor_break` index.
#[test]
fn armor_broken_spawns_one_flash_at_the_armor_break_index() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(9, 2);
    let level = Level::new(0);
    let ganger = spawn_ganger(&mut app, cell, level, 4);

    app.world_mut()
        .resource_mut::<Messages<ArmorBroken>>()
        .write(ArmorBroken::new(ganger, BodyPart::Torso));
    app.update();

    let roles = effect_roles(&app);
    assert!(roles.is_some(), "EffectRoles must be resident");
    let Some(roles) = roles else { return };

    assert_eq!(
        fx_count(&mut app),
        1,
        "exactly one armor-break flash must spawn"
    );
    let flash = single_flash(&mut app);
    assert!(flash.is_some(), "exactly one FxFlash sprite must exist");
    let Some((translation, index)) = flash else {
        return;
    };
    assert_eq!(
        translation,
        cell_to_world(cell, level),
        "the armor-break flash must sit at cell_to_world(ganger's cell, level)",
    );
    assert_eq!(
        index,
        Some(*roles.armor_break),
        "the armor-break flash's atlas index must equal the table's armor_break role index",
    );
}

/// AC3 — a `CoverDestroyed { at }` spawns exactly one `FxFlash` at `cell_to_world(at)` with
/// the table's `cover_destroyed` index.
#[test]
fn cover_destroyed_spawns_one_flash_at_the_cover_destroyed_index() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(3, 4);
    let level = Level::new(0);
    let at = CellLevel::new(cell, level);

    app.world_mut()
        .resource_mut::<Messages<CoverDestroyed>>()
        .write(CoverDestroyed::new(at));
    app.update();

    let roles = effect_roles(&app);
    assert!(roles.is_some(), "EffectRoles must be resident");
    let Some(roles) = roles else { return };

    assert_eq!(
        fx_count(&mut app),
        1,
        "exactly one cover-destroyed flash must spawn"
    );
    let flash = single_flash(&mut app);
    assert!(flash.is_some(), "exactly one FxFlash sprite must exist");
    let Some((translation, index)) = flash else {
        return;
    };
    assert_eq!(
        translation,
        cell_to_world(cell, level),
        "the cover-destroyed flash must sit at cell_to_world(at's cell, level)",
    );
    assert_eq!(
        index,
        Some(*roles.cover_destroyed),
        "the cover-destroyed flash's index must equal the table's cover_destroyed role index",
    );
}

/// The (world translation, atlas index) of every `ShotProjectile` sprite (the GTW-306
/// traveling projectiles), unordered.
fn projectile_positions_and_indices(app: &mut App) -> Vec<(bevy::math::Vec3, Option<usize>)> {
    let mut q = app
        .world_mut()
        .query::<(&ShotProjectile, &Sprite, &Transform)>();
    q.iter(app.world())
        .map(|(_, sprite, transform)| {
            (
                transform.translation,
                sprite.texture_atlas.as_ref().map(|atlas| atlas.index),
            )
        })
        .collect()
}

/// GTW-307 — a `ShotFired` spawns NO standalone muzzle flash (it was removed: it rendered
/// oversized at the shooter's feet and read poorly), only a traveling DIRECTIONAL projectile (a
/// `ShotProjectile` starting at the muzzle, on a `ProjectileTravel` toward the impact). The
/// projectile uses the per-damage-type directional tile (orange row for Kinetic) and IS the
/// fire signal; the old stretched tracer is also gone.
#[test]
fn shot_fired_spawns_directional_projectile_and_no_muzzle_flash() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let roles = effect_roles(&app);
    assert!(roles.is_some(), "EffectRoles must be resident");
    let Some(roles) = roles else { return };

    // A struck ganger entity (the ShotKind::Ganger payload) + a known fire geometry, fired
    // EAST (the +x heading -> compass column 0 of the row).
    let struck = app.world_mut().spawn_empty().id();
    let muzzle = SimPos::new(2.0, 5.0, 0.0);
    let impact_cell = Cell::new(8, 5);
    let impact_level = Level::new(0);
    let shot = ShotFired {
        shooter: app.world_mut().spawn_empty().id(),
        muzzle,
        trajectory: ShotDir::from_direction(bevy::math::Vec3::new(1.0, 0.0, 0.0)),
        impact_cell,
        impact_level,
        kind: ShotKind::Ganger(struck),
        damage: DamageType::Kinetic,
        report: None,
    };

    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .write(shot);
    // Pin the firing update's clock delta to ZERO so the just-spawned projectile cannot travel a
    // wall-clock-dependent distance off the muzzle on the read frame: under `Automatic` time, the
    // first update after the variable-length `settle_resources` carries a non-deterministic delta,
    // which flaked the "starts at the muzzle" assertion under parallel `cargo` load (GTW-305). A
    // zero delta keeps `advance_projectiles` from moving the bolt regardless of scheduling, without
    // weakening the assertion (the spawn-at-muzzle truth it pins is delta-independent).
    fire_with_zero_delta(&mut app);

    // No standalone muzzle FxFlash spawns on fire (the muzzle flash was removed in GTW-307;
    // the impact-frame flashes spawn later, when the projectile ARRIVES, not on fire).
    assert_eq!(
        fx_count(&mut app),
        0,
        "a ShotFired must spawn NO muzzle FxFlash on fire (the muzzle flash was removed)",
    );

    // Exactly one traveling directional projectile, starting at the muzzle, at the orange-row
    // east tile (Kinetic -> orange; +x heading -> column 0).
    let projectiles = projectile_positions_and_indices(&mut app);
    assert_eq!(
        projectiles.len(),
        1,
        "a ShotFired must spawn exactly one traveling projectile",
    );
    if let Some((pos, index)) = projectiles.first() {
        assert!(
            pos.distance(sim_pos_to_world(muzzle)) < 0.01,
            "the projectile must START at the muzzle world position",
        );
        assert_eq!(
            *index,
            Some(*roles.orange.directions[0]),
            "the Kinetic east shot must use the orange row's column-0 (E) directional tile",
        );
    }
}

/// GTW-307 — a MISS still draws the firing FX: a traveling projectile (the projectile
/// terminates at the impact cell). A miss carries no struck object but the same geometry, so
/// the presenter draws the same projectile — and, like a hit, NO standalone muzzle flash.
#[test]
fn shot_fired_miss_still_spawns_projectile_and_no_muzzle_flash() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let shot = ShotFired {
        shooter:      app.world_mut().spawn_empty().id(),
        muzzle:       SimPos::new(1.0, 1.0, 0.0),
        trajectory:   ShotDir::from_direction(bevy::math::Vec3::new(0.0, 1.0, 0.0)),
        impact_cell:  Cell::new(1, 9),
        impact_level: Level::new(0),
        kind:         ShotKind::Miss,
        damage:       DamageType::Kinetic,
        report:       None,
    };

    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .write(shot);
    app.update();

    assert_eq!(
        fx_count(&mut app),
        0,
        "a miss draws NO muzzle flash on fire (the muzzle flash was removed)",
    );
    assert_eq!(
        projectile_positions_and_indices(&mut app).len(),
        1,
        "a miss still draws a traveling projectile (it terminates at the impact cell)",
    );
}

/// Spawns a REAL sim ganger (the components `spawn_ganger_sprites` queries — `Position`,
/// `Faction`, `Facing`, `LifeState`) at `cell`/`level` and drives one `update()` so the
/// presenter's real spawn system builds its sprite and registers the `sim Entity -> sprite
/// Entity` link in `GangerSprites`. Returns the sim `Entity` (the `ShotKind::Ganger`
/// payload). `BattleInProgress` must already be resident (the spawn gate).
fn spawn_sim_ganger_with_sprite(
    app: &mut App,
    cell: Cell,
    level: Level,
) -> bevy::ecs::entity::Entity {
    let at = CellLevel::new(cell, level);
    let sim = app
        .world_mut()
        .spawn((
            Position::new(at),
            Faction::new(0),
            Facing::new(Direction::East),
            LifeState::Alive,
        ))
        .id();
    // Drive the real spawn_ganger_sprites system (gated on CharacterRoles + TopDownAtlases +
    // BattleInProgress, all resident after settle) so the presenter sprite + GangerSprites
    // mapping exist before the shot is fired.
    app.update();
    sim
}

/// The `GangerSprites`-mapped presenter sprite's rendered world `Transform.translation` for
/// `sim` ganger, or `None` if it was not spawned/mapped (the caller asserts `Some`).
fn ganger_sprite_world(app: &App, sim: bevy::ecs::entity::Entity) -> Option<bevy::math::Vec3> {
    let sprite_entity = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|sprites| sprites.sprite_for(sim))?;
    app.world()
        .get::<Transform>(sprite_entity)
        .map(|transform| transform.translation)
}

/// The arrival (target) world point of the SINGLE `ShotProjectile` in flight — its
/// `ProjectileTravel` terminus. Returns `None` unless exactly one projectile exists (the
/// caller asserts `Some`).
fn single_projectile_arrival(app: &mut App) -> Option<bevy::math::Vec3> {
    let mut q = app.world_mut().query::<&ProjectileTravel>();
    let mut found: Option<bevy::math::Vec3> = None;
    for travel in q.iter(app.world()) {
        if found.is_some() {
            return None;
        }
        found = Some(travel.arrival());
    }
    found
}

/// GTW-306 (C2 / C5) — a `ShotFired` that struck a GANGER aims its traveling projectile at
/// that hit entity's CURRENT rendered world position (its presenter `Transform`, looked up
/// through `GangerSprites`), NOT at the impact `(cell, level)`. This drives the REAL
/// `spawn_ganger_sprites` path so the hit ganger has a mapped sprite, places that ganger in a
/// DIFFERENT cell from the message's `impact_cell`, and asserts the projectile's
/// `ProjectileTravel` terminus equals the ganger sprite's rendered translation — and is
/// distinctly NOT `cell_to_world(impact_cell, impact_level)`. Reverting the entity-aim logic
/// to always use `cell_to_world(impact_cell)` would flip both assertions (the path is no
/// longer dead-code to the suite).
#[test]
fn shot_fired_at_a_ganger_aims_at_the_hit_entitys_rendered_position() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // The struck ganger sits at a cell DISTINCT from the shot's impact cell, so the
    // entity-aim endpoint and the impact-cell fallback are unambiguously different points.
    let ganger_cell = Cell::new(8, 5);
    let level = Level::new(0);
    let struck = spawn_sim_ganger_with_sprite(&mut app, ganger_cell, level);

    // The ganger's sprite must have been spawned + mapped by the real spawn system.
    let ganger_world = ganger_sprite_world(&app, struck);
    assert!(
        ganger_world.is_some(),
        "the struck ganger's presenter sprite must be spawned + mapped in GangerSprites",
    );
    let Some(ganger_world) = ganger_world else {
        return;
    };

    // Fire AT that ganger, but with an impact cell DELIBERATELY elsewhere — proving the bolt
    // tracks the entity, not the message's impact cell.
    let muzzle = SimPos::new(1.0, 1.0, 0.0);
    let impact_cell = Cell::new(2, 2);
    let impact_level = Level::new(0);
    let shot = ShotFired {
        shooter: app.world_mut().spawn_empty().id(),
        muzzle,
        trajectory: ShotDir::from_direction(bevy::math::Vec3::new(1.0, 0.0, 0.0)),
        impact_cell,
        impact_level,
        kind: ShotKind::Ganger(struck),
        damage: DamageType::Kinetic,
        report: None,
    };
    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .write(shot);
    app.update();

    let arrival = single_projectile_arrival(&mut app);
    assert!(
        arrival.is_some(),
        "exactly one traveling projectile must spawn for the ganger hit",
    );
    let Some(arrival) = arrival else { return };

    // The projectile must fly to the hit ganger's RENDERED position (entity-aim), the (x, y)
    // of its sprite Transform — NOT the impact cell. (Compare x/y: the projectile's z is the
    // FX layer, distinct from the ganger sprite's Actor-layer z bias, so a 2D compare isolates
    // the aim choice from the z-layering.)
    assert!(
        arrival.truncate().distance(ganger_world.truncate()) < 0.01,
        "the bolt must aim at the hit ganger's rendered position {ganger_world:?}, got {arrival:?}",
    );
    let impact_world = cell_to_world(impact_cell, impact_level);
    assert!(
        arrival.truncate().distance(impact_world.truncate()) > 0.01,
        "the bolt must NOT aim at the impact cell {impact_world:?} for a ganger hit (entity-aim)",
    );
}

/// AC4 — `FlashTtl` expiry makes the flash one-shot: a spawned flash is despawned by
/// `expire_flashes` after its TTL elapses, and a further empty update spawns nothing (the
/// reader drained its buffer — nothing lingers).
#[test]
fn flash_expires_after_its_ttl_and_nothing_lingers() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let ganger = spawn_ganger(&mut app, Cell::new(1, 1), Level::new(0), 2);
    app.world_mut()
        .resource_mut::<Messages<Bleeding>>()
        .write(Bleeding::new(ganger));
    app.update();
    assert_eq!(fx_count(&mut app), 1, "one flash must spawn");

    // Advance Time past the TTL and update -> the flash is despawned.
    advance_past_ttl(&mut app);
    assert_eq!(
        fx_count(&mut app),
        0,
        "the flash must be despawned once its FlashTtl elapses",
    );

    // A further update with NO new message spawns nothing — the reader drained its buffer.
    app.update();
    assert_eq!(
        fx_count(&mut app),
        0,
        "a further empty update must spawn no new flash (one-shot per message)",
    );
}

/// AC5 — two `Bleeding` for one ganger in a frame spawn TWO INDEPENDENT flashes (no
/// coalescing), and both later expire under AC4's clock.
#[test]
fn two_bleeding_messages_spawn_two_independent_flashes() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let ganger = spawn_ganger(&mut app, Cell::new(7, 7), Level::new(0), 1);
    {
        let mut buf = app.world_mut().resource_mut::<Messages<Bleeding>>();
        buf.write(Bleeding::new(ganger));
        buf.write(Bleeding::new(ganger));
    }
    app.update();
    assert_eq!(
        fx_count(&mut app),
        2,
        "two Bleeding for one ganger in a frame must spawn two independent flashes (no coalescing)",
    );

    // Both expire under the same clock.
    advance_past_ttl(&mut app);
    assert_eq!(
        fx_count(&mut app),
        0,
        "both independent flashes must expire once their FlashTtl elapses",
    );
}

/// The `(text, alpha-1 color)` of every live `FloatingCombatText` pop — the rendered string and
/// its `TextColor` (full-alpha at spawn, before the first fade tick). Unordered.
fn fct_pops(app: &mut App) -> Vec<(String, bevy::prelude::Color)> {
    let mut q = app
        .world_mut()
        .query::<(&FloatingCombatText, &Text2d, &TextColor)>();
    q.iter(app.world())
        .map(|(_, text, color)| ((**text).clone(), color.0))
        .collect()
}

/// Whether the live pops contain a pop with exactly `text` whose color's RGB matches `color`'s
/// (alpha-agnostic, since the pop fades — but at spawn, pre-tick, it is still full alpha).
fn has_fct_pop(
    pops: &[(String, bevy::prelude::Color)],
    text: &str,
    color: bevy::prelude::Color,
) -> bool {
    let want = color.to_srgba();
    pops.iter().any(|(t, c)| {
        let got = c.to_srgba();
        t == text
            && (got.red - want.red).abs() < 0.001
            && (got.green - want.green).abs() < 0.001
            && (got.blue - want.blue).abs() < 0.001
    })
}

/// A ganger-hit `HitReport` for `part` with `hp` HP loss / `pen` penetration / `severity` tier
/// / `life_after` state, struck on `struck` — the report the FCT reader classifies.
const fn ganger_hit_report(
    struck: bevy::ecs::entity::Entity,
    part: BodyPart,
    hp: i32,
    pen: i32,
    severity: Severity,
    life_after: LifeState,
) -> HitReport {
    HitReport {
        kind:            ShotKind::Ganger(struck),
        part:            Some(part),
        applied:         Some(AppliedDamage {
            matchup: Matchup::Neutral,
            hit: HitResult {
                penetrating: PenetratingDamage::new(pen),
                hp_damage:   HpDamage::new(hp),
                wear:        IntegrityWear::new(0),
            },
            severity,
            life_after,
            broken: None,
            worn: None,
        }),
        cover_destroyed: None,
        slab_destroyed:  None,
        ground_accrued:  None,
        injury:          None,
    }
}

/// Advances the app a FIXED number of `step`-sized manual updates (each advancing the virtual
/// clock by `step`), then restores `Automatic` time — the deterministic way to fly a staggered
/// volley's bolts to their impacts and watch the per-shot FCT pops appear over time.
fn step_app(app: &mut App, step: std::time::Duration, updates: u32) {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(step));
    for _ in 0..updates {
        app.update();
    }
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
}

/// The number of live `FloatingCombatText` pops currently in the world.
fn fct_pop_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&FloatingCombatText>();
    q.iter(app.world()).count()
}

/// Drains and returns the `ShotImpactResolved` signals (GTW-328) emitted SINCE the last drain —
/// the per-shot impact-resolved messages `animate_impact` writes. The combat-text LOG drains this
/// exact buffer to build its shot-outcome lines, so the count of these signals over a stepped run
/// is the number of outcome lines the log gains: zero means no shot has resolved its impact yet.
fn drain_impacts(app: &mut App) -> Vec<ShotImpactResolved> {
    app.world_mut()
        .resource_mut::<Messages<ShotImpactResolved>>()
        .drain()
        .collect()
}

/// Advances the app a fixed number of `step`-sized manual updates, DRAINING the
/// `ShotImpactResolved` buffer after EACH update and accumulating the total emitted across the
/// run, then restores `Automatic` time. The per-update drain is what makes the accumulation
/// reliable: the buffer double-buffers, so a message left un-drained across two updates is
/// dropped — draining each frame captures every staggered impact as it resolves.
fn step_counting_impacts(app: &mut App, step: std::time::Duration, updates: u32) -> usize {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(step));
    let mut total = 0;
    for _ in 0..updates {
        app.update();
        total += drain_impacts(app).len();
    }
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
    total
}

/// GTW-302 (slice 3) / GTW-327 (slice 2) — the REAL dispatch path: a `ShotFired` carrying a
/// damaging, lethal ganger-hit `HitReport` drives the firing pipeline to spawn the
/// floating-combat-text pops (HP number RED, wound AMBER, penetration verdict, DOWN/DEAD lethal
/// RED), anchored at the hit ganger's cell — now spawned at the shot's IMPACT (after the bolt
/// flies), not on the drain frame. Pin-discriminates each pop's text + color.
#[test]
fn shot_fired_with_a_lethal_hit_spawns_the_classified_fct_pops() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // A struck ganger carrying a Position (the FCT reader anchors the pops at its cell).
    let cell = Cell::new(6, 4);
    let level = Level::new(0);
    let struck = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(cell, level)))
        .id();

    // A Critical, DEAD, penetrating torso hit dealing 9 HP.
    let report = ganger_hit_report(
        struck,
        BodyPart::Torso,
        9,
        6,
        Severity::Critical,
        LifeState::Dead,
    );
    let shot = ShotFired {
        shooter:      app.world_mut().spawn_empty().id(),
        muzzle:       SimPos::new(1.0, 1.0, 0.0),
        trajectory:   ShotDir::from_direction(bevy::math::Vec3::new(1.0, 0.0, 0.0)),
        impact_cell:  cell,
        impact_level: level,
        kind:         ShotKind::Ganger(struck),
        damage:       DamageType::Kinetic,
        report:       Some(report),
    };
    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .write(shot);
    // Drain the ShotFired (spawn the bolt) on a zero-delta frame, then fly it to its impact —
    // the pops are spawned at the IMPACT now (GTW-327), so a single drain frame is not enough.
    fire_with_zero_delta(&mut app);
    // A handful of generous steps flies the bolt the muzzle->cell distance to arrival + seeds the
    // impact (which spawns the pops). One 50ms step is shorter than the FCT lifetime, so they
    // are still alive when read.
    step_app(&mut app, std::time::Duration::from_millis(50), 8);

    let pops = fct_pops(&mut app);
    // HP number (RED), wound (Critical amber), penetration verdict (GREY "Penetrated"), DEAD
    // (lethal RED) — four distinct pops.
    assert!(
        has_fct_pop(&pops, "-9", valence_color(FctValence::Damage)),
        "the 9-HP hit must pop a RED \"-9\", got {pops:?}",
    );
    assert!(
        has_fct_pop(&pops, "Torso Critical", severity_color(Severity::Critical)),
        "a Critical torso wound must pop \"Torso Critical\" in the Critical amber, got {pops:?}",
    );
    assert!(
        has_fct_pop(&pops, "Penetrated", valence_color(FctValence::Neutral)),
        "a penetrating hit must pop a GREY \"Penetrated\", got {pops:?}",
    );
    assert!(
        has_fct_pop(&pops, "DEAD", valence_color(FctValence::Lethal)),
        "a Dead outcome must pop a lethal-RED \"DEAD\", got {pops:?}",
    );

    // The pops are anchored at the hit ganger's cell (x/y of cell_to_world; the FCT z is the
    // Highlight band, distinct from the cell z, so compare the planar position).
    let anchor = cell_to_world(cell, level);
    let mut q = app.world_mut().query::<(&FloatingCombatText, &Transform)>();
    let any_at_cell = q
        .iter(app.world())
        .any(|(_, transform)| (transform.translation.x - anchor.x).abs() < 0.001);
    assert!(
        any_at_cell,
        "the FCT pops must anchor at the hit ganger's cell x ({})",
        anchor.x,
    );
}

/// GTW-302 (slice 3) / GTW-327 (slice 2) — a clean MISS `ShotFired` (a non-connecting shot)
/// spawns NO floating-combat-text pop at all on the real registered-system dispatch path, EVEN
/// after its tracer flies to the impact: a missed shot gets no pop (per user feedback, there is
/// no "Miss" popup text). The miss still rides a (numberless) bolt to its impact, so flying it to
/// completion proves the impact-spawn path emits nothing for an empty-pop shot.
#[test]
fn shot_fired_clean_miss_pops_nothing() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(2, 9);
    let level = Level::new(0);
    let shot = ShotFired {
        shooter:      app.world_mut().spawn_empty().id(),
        muzzle:       SimPos::new(1.0, 1.0, 0.0),
        trajectory:   ShotDir::from_direction(bevy::math::Vec3::new(0.0, 1.0, 0.0)),
        impact_cell:  cell,
        impact_level: level,
        kind:         ShotKind::Miss,
        damage:       DamageType::Kinetic,
        report:       Some(HitReport::no_effect(ShotKind::Miss)),
    };
    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .write(shot);
    // Drain (spawn the bolt) then fly it all the way to its impact — even the impact-spawn path
    // must emit no pop for a miss.
    fire_with_zero_delta(&mut app);
    step_app(&mut app, std::time::Duration::from_millis(50), 8);

    let pops = fct_pops(&mut app);
    assert!(
        pops.is_empty(),
        "a clean miss must spawn no FCT pop, got {pops:?}"
    );
}

/// GTW-327 (slice 2) — the BUG FIX, deterministic + headless: a MULTI-ROUND volley's
/// floating-combat-text pops appear STAGGERED at each shot's own impact, NOT all at once on the
/// drain frame. Two rounds (each a connecting ganger hit) are written in one frame, the same way
/// a burst / full-auto fires; their tracers fly staggered by `InterShotSeconds` (GTW-308), and
/// each shot's pops are spawned only when THAT shot's bolt arrives — so at t=0 there are zero
/// pops, after the first shot's (short) flight the first shot's pops are up, and only ~one
/// `InterShotSeconds` later (the second bolt's launch delay) do the second shot's pops appear.
///
/// Each round is a connecting hit (so it classifies to ≥ 1 pop); the assertions check MONOTONIC
/// growth at the staggered times (0 → first shot's pops → strictly more after the second), which
/// is the robust shape of "shot-by-shot, not all at once" regardless of how many pops each
/// classified report yields. A final assert proves a pop persists for (most of) its tuned
/// lifetime rather than vanishing on the next frame — the slice-1 lifetime fix still holds here.
#[test]
fn a_multi_round_volley_pops_its_fct_staggered_per_impact() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // The hot-reloadable stagger step + pop lifetime the system uses (read off the resident
    // FxTuning so the test is not pinned to a literal magnitude the user may retune).
    let tuning = app.world().get_resource::<FxTuning>().copied();
    assert!(
        tuning.is_some(),
        "FxTuning must be resident after settle_resources",
    );
    let Some(tuning) = tuning else { return };
    let inter_shot = std::time::Duration::from_secs_f32(*tuning.inter_shot_seconds);
    let ttl = std::time::Duration::from_secs_f32(*tuning.fct_ttl_seconds);
    // Sanity: the stagger gap must exceed the pop lifetime check granularity — the default
    // InterShotSeconds (0.35s) is well above the per-step deltas below.
    assert!(
        inter_shot >= std::time::Duration::from_millis(100),
        "this test assumes a stagger step (InterShotSeconds {inter_shot:?}) comfortably larger \
         than a flight step — the shipped default is 0.35s",
    );

    // Two struck gangers on the SAME cell (so their pops would overlap if dumped together), each
    // a connecting hit (so each classifies to >= 1 pop — the count grows when each shot lands).
    let cell = Cell::new(5, 5);
    let level = Level::new(0);
    let muzzle = SimPos::new(4.0, 5.0, 0.0); // one cell west of the target — a short flight.
    let write_round = |app: &mut App, struck: bevy::ecs::entity::Entity| {
        let report = ganger_hit_report(
            struck,
            BodyPart::Torso,
            4,
            6,
            Severity::None,
            LifeState::Alive,
        );
        let shot = ShotFired {
            shooter: app.world_mut().spawn_empty().id(),
            muzzle,
            trajectory: ShotDir::from_direction(bevy::math::Vec3::new(1.0, 0.0, 0.0)),
            impact_cell: cell,
            impact_level: level,
            kind: ShotKind::Ganger(struck),
            damage: DamageType::Kinetic,
            report: Some(report),
        };
        app.world_mut()
            .resource_mut::<Messages<ShotFired>>()
            .write(shot);
    };
    let struck_a = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(cell, level)))
        .id();
    let struck_b = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(cell, level)))
        .id();
    write_round(&mut app, struck_a);
    write_round(&mut app, struck_b);

    // Drain both ShotFired on a zero-delta frame: BOTH bolts spawn (held at the muzzle), and
    // CRUCIALLY no pop is spawned yet — the bug was dumping every pop here.
    fire_with_zero_delta(&mut app);
    assert_eq!(
        fct_pop_count(&mut app),
        0,
        "at the drain frame NO pop may exist — the whole point of the fix is that the numbers \
         do not all appear at once on the ShotFired-drain frame",
    );

    // Fly the FIRST bolt to its impact: a short flight (one cell at the tuned velocity), well
    // under one InterShotSeconds. After it, the first shot's pop(s) are up; the second bolt is
    // still parked at the muzzle (its launch delay = one InterShotSeconds has not elapsed).
    let short_step = std::time::Duration::from_millis(30);
    step_app(&mut app, short_step, 4);
    let after_first = fct_pop_count(&mut app);
    assert!(
        after_first >= 1,
        "after the first bolt's flight its FCT pop(s) must be up (got {after_first})",
    );

    // Now advance PAST the second bolt's launch delay (one InterShotSeconds) + its flight: the
    // second shot's pops appear, so the live count STRICTLY GROWS — the volley read shot-by-shot.
    step_app(&mut app, inter_shot, 2);
    let after_second = fct_pop_count(&mut app);
    assert!(
        after_second > after_first,
        "after the second bolt's staggered impact MORE pops must be live than after the first \
         ({after_second} must exceed {after_first}) — the second shot's numbers appeared later",
    );

    // The lifetime fix (slice 1) still holds on this path: a freshly-spawned pop persists across
    // a frame far shorter than its tuned lifetime rather than vanishing immediately. Step a small
    // delta (well under the ttl) and confirm the second shot's pops are still alive.
    assert!(
        ttl >= std::time::Duration::from_millis(500),
        "the tuned FCT lifetime ({ttl:?}) is expected to be at least 0.5s (slice-1 fix)",
    );
    step_app(&mut app, std::time::Duration::from_millis(50), 1);
    assert!(
        fct_pop_count(&mut app) >= after_first,
        "a freshly-impacted pop must persist for its tuned lifetime, not vanish on the next frame",
    );
}

/// GTW-328 (slice A) — the BUG FIX, deterministic + headless: a MULTI-ROUND volley's per-shot
/// `ShotImpactResolved` SIGNALS (the shared per-shot impact moment the combat-text LOG keys its
/// outcome lines off) arrive STAGGERED at each shot's own impact, NOT all at once on the
/// `ShotFired`-drain frame. This is the exact analogue of the GTW-327 FCT staggered test, but on
/// the LOG's signal: the log builds one shot-outcome line per `ShotImpactResolved`, so this proves
/// a burst's outcome lines appear one-per-impact in cadence rather than dumped on the fire frame.
///
/// Two connecting ganger-hit rounds are written in one frame (the way a burst / full-auto fires);
/// their bolts fly staggered by `InterShotSeconds` (GTW-308), and each shot's `ShotImpactResolved`
/// is emitted only when THAT shot's bolt arrives. So at the drain frame ZERO signals exist, after
/// the first shot's (short) flight exactly the first shot's signal has fired, and only ~one
/// `InterShotSeconds` later (the second bolt's launch delay) does the second shot's signal fire —
/// monotonic growth across stepped time, the robust shape of "shot-by-shot, not all at once".
#[test]
fn a_multi_round_volley_emits_shot_impact_resolved_staggered_per_impact() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // The hot-reloadable stagger step the system uses (read off the resident FxTuning so the test
    // is not pinned to a literal magnitude the user may retune).
    let tuning = app.world().get_resource::<FxTuning>().copied();
    assert!(
        tuning.is_some(),
        "FxTuning must be resident after settle_resources",
    );
    let Some(tuning) = tuning else { return };
    let inter_shot = std::time::Duration::from_secs_f32(*tuning.inter_shot_seconds);
    assert!(
        inter_shot >= std::time::Duration::from_millis(100),
        "this test assumes a stagger step (InterShotSeconds {inter_shot:?}) comfortably larger \
         than a flight step — the shipped default is 0.35s",
    );

    // Two struck gangers on the SAME cell, each a connecting hit (so each yields a shot-outcome
    // signal). The shooter is named via its Entity in the signal (the log resolves it downstream).
    let cell = Cell::new(5, 5);
    let level = Level::new(0);
    let muzzle = SimPos::new(4.0, 5.0, 0.0); // one cell west of the target — a short flight.
    let write_round = |app: &mut App, struck: bevy::ecs::entity::Entity| {
        let report = ganger_hit_report(
            struck,
            BodyPart::Torso,
            4,
            6,
            Severity::None,
            LifeState::Alive,
        );
        let shot = ShotFired {
            shooter: app.world_mut().spawn_empty().id(),
            muzzle,
            trajectory: ShotDir::from_direction(bevy::math::Vec3::new(1.0, 0.0, 0.0)),
            impact_cell: cell,
            impact_level: level,
            kind: ShotKind::Ganger(struck),
            damage: DamageType::Kinetic,
            report: Some(report),
        };
        app.world_mut()
            .resource_mut::<Messages<ShotFired>>()
            .write(shot);
    };
    let struck_a = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(cell, level)))
        .id();
    let struck_b = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(cell, level)))
        .id();
    write_round(&mut app, struck_a);
    write_round(&mut app, struck_b);

    // Drain both ShotFired on a zero-delta frame: BOTH bolts spawn (held at the muzzle), and
    // CRUCIALLY no impact-resolved signal fires yet — the bug was the LOG dumping every outcome
    // line on THIS drain frame (it used to drain ShotFired directly).
    fire_with_zero_delta(&mut app);
    assert_eq!(
        drain_impacts(&mut app).len(),
        0,
        "at the drain frame NO ShotImpactResolved may fire — the whole point of the fix is that \
         the shot-outcome lines do not all appear at once on the ShotFired-drain frame",
    );

    // Fly the FIRST bolt to its impact (a short flight, well under one InterShotSeconds): exactly
    // the first shot's impact-resolved signal fires; the second bolt is still parked at the muzzle.
    let short_step = std::time::Duration::from_millis(30);
    let after_first = step_counting_impacts(&mut app, short_step, 4);
    assert!(
        after_first >= 1,
        "after the first bolt's flight its ShotImpactResolved must have fired (got {after_first})",
    );

    // Advance PAST the second bolt's launch delay (one InterShotSeconds) + its flight: the second
    // shot's signal fires, so the cumulative count STRICTLY GROWS — the volley resolved shot-by-shot.
    let after_second = after_first + step_counting_impacts(&mut app, inter_shot, 2);
    assert!(
        after_second > after_first,
        "after the second bolt's staggered impact MORE ShotImpactResolved must have fired in \
         total ({after_second} must exceed {after_first}) — the second shot's outcome resolved later",
    );

    // Both shots, exactly once each: a two-round volley resolves two outcome signals across the run.
    assert_eq!(
        after_second, 2,
        "a two-round volley must emit exactly two ShotImpactResolved (one per shot), staggered, \
         got {after_second}",
    );
}

/// Whether sim ganger `sim` currently has a LIVE presenter sprite — its `GangerSprites` entry is
/// still mapped (GTW-331; the despawn drops the map entry, so a missing entry == despawned).
fn ganger_sprite_alive(app: &App, sim: bevy::ecs::entity::Entity) -> bool {
    app.world()
        .get_resource::<GangerSprites>()
        .is_some_and(|sprites| sprites.contains(sim))
}

/// Set the `LifeState` of sim ganger `sim` directly (the sim-drain `Changed<LifeState>` trigger
/// the presenter reacts to). GTW-331: this is the moment the bug despawned the sprite — long
/// before the killing tracer lands.
fn set_life_state(app: &mut App, sim: bevy::ecs::entity::Entity, state: LifeState) {
    let mut q = app.world_mut().query::<&mut LifeState>();
    if let Ok(mut life) = q.get_mut(app.world_mut(), sim) {
        *life = state;
    }
}

/// GTW-331 — the BUG FIX, deterministic + headless: a SHOT that KILLS a ganger must keep the
/// ganger's sprite ALIVE at sim-drain time (when the sim flips its `LifeState` to `Dead`, well
/// before the staggered killing tracer arrives) and despawn it ONLY after the killing shot's
/// impact resolves (when the bolt reaches the body).
///
/// This drives the FULL FX pipeline: the real `spawn_ganger_sprites` registers the victim's
/// sprite, a `ShotFired` carrying a lethal (`life_after == Dead`) ganger-hit report spawns the
/// bolt, and the victim's `LifeState` is set `Dead` on the SAME drain frame (the sim-drain the
/// bug reacted to). The assertions: (1) at the drain frame the sprite is STILL alive (the bug:
/// it was already despawned), (2) the bolt is still in flight so it stays alive, (3) once the
/// bolt flies to its impact and `animate_impact` resolves the kill, the sprite is despawned and
/// its `GangerSprites` entry dropped.
///
/// RED before the fix: `update_ganger_life_state`'s old `Dead` arm despawned at the drain frame,
/// so assertion (1) (the sprite still alive at drain) fails immediately.
#[test]
fn a_shot_kill_keeps_the_sprite_until_the_killing_impact_lands() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // The struck ganger sits at a cell a short flight from the muzzle, with a REAL presenter
    // sprite registered by the production spawn system.
    let cell = Cell::new(8, 5);
    let level = Level::new(0);
    let victim = spawn_sim_ganger_with_sprite(&mut app, cell, level);
    assert!(
        ganger_sprite_alive(&app, victim),
        "the victim's presenter sprite must be spawned + mapped before the shot",
    );

    // A lethal (DEAD) penetrating torso hit on the victim, fired from one cell west — a short
    // flight, so the bolt takes several updates to reach the body.
    let muzzle = SimPos::new(4.0, 5.0, 0.0);
    let report = ganger_hit_report(
        victim,
        BodyPart::Torso,
        9,
        6,
        Severity::Critical,
        LifeState::Dead,
    );
    let shot = ShotFired {
        shooter: app.world_mut().spawn_empty().id(),
        muzzle,
        trajectory: ShotDir::from_direction(bevy::math::Vec3::new(1.0, 0.0, 0.0)),
        impact_cell: cell,
        impact_level: level,
        kind: ShotKind::Ganger(victim),
        damage: DamageType::Kinetic,
        report: Some(report),
    };
    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .write(shot);

    // The sim-drain frame: drain the ShotFired (spawn the bolt, still parked/short into flight)
    // AND flip the victim's LifeState to Dead, the SAME frame — exactly what the sim does at
    // drain. The bug despawned the sprite here.
    set_life_state(&mut app, victim, LifeState::Dead);
    fire_with_zero_delta(&mut app);

    // (1) THE BUG: at the drain frame the sprite must STILL be alive — its killing tracer has
    // not reached it yet.
    assert!(
        ganger_sprite_alive(&app, victim),
        "a shot-killed ganger's sprite must STILL exist at the sim-drain frame (the bug despawned \
         it here, before the killing tracer arrives)",
    );

    // (2) The bolt is in flight; a couple of small steps keep it short of the impact, so the
    // sprite stays alive across the flight (the kill is pending the incoming impact).
    step_app(&mut app, std::time::Duration::from_millis(10), 1);
    assert!(
        ganger_sprite_alive(&app, victim),
        "the sprite must stay alive while the killing bolt is still in flight",
    );

    // (3) Fly the bolt the rest of the way to its impact: animate_impact resolves the kill, and
    // the sprite is despawned + its map entry dropped — at the IMPACT, not at the drain.
    step_app(&mut app, std::time::Duration::from_millis(50), 8);
    assert!(
        !ganger_sprite_alive(&app, victim),
        "once the killing shot's impact resolves the sprite must be despawned (drop its map entry)",
    );
    let mut q = app.world_mut().query::<&GangerSprite>();
    let mirrors_victim = q.iter(app.world()).any(|marker| marker.entity == victim);
    assert!(
        !mirrors_victim,
        "no GangerSprite entity may still mirror the killed victim after the impact despawn",
    );
}

/// GTW-331 — the fix must NOT drop a NON-shot death: a ganger that dies WITHOUT a tracer (its
/// `LifeState` set `Dead` directly — a bleed-out / direct kill, no `ShotFired`, no projectile)
/// must STILL have its sprite despawned PROMPTLY at the sim event (there is no incoming impact to
/// wait for). Guards the deferral from silently dropping deaths that have no killing shot.
#[test]
fn a_non_shot_death_despawns_the_sprite_promptly() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // A real presenter sprite, no shot fired at all (no projectile in flight, no PendingImpact).
    let cell = Cell::new(3, 3);
    let level = Level::new(0);
    let dying = spawn_sim_ganger_with_sprite(&mut app, cell, level);
    assert!(
        ganger_sprite_alive(&app, dying),
        "the ganger's presenter sprite must be spawned + mapped before it dies",
    );

    // A non-shot death: set LifeState Dead directly (a bleed-out-style death). No tracer is
    // pending, so the life-state path must despawn it on the next update.
    set_life_state(&mut app, dying, LifeState::Dead);
    app.update();

    assert!(
        !ganger_sprite_alive(&app, dying),
        "a non-shot death (no pending tracer) must despawn the sprite promptly at the sim event",
    );
    let mut q = app.world_mut().query::<&GangerSprite>();
    let mirrors_dead = q.iter(app.world()).any(|marker| marker.entity == dying);
    assert!(
        !mirrors_dead,
        "no GangerSprite entity may still mirror the non-shot-dead ganger",
    );
}

/// GTW-302 (slice 4) — the REAL dispatch path: a `Bleeding { ganger }` drives the registered
/// `read_consequence_fct` system to spawn the AMBER `"Bleeding"` floating-combat-text pop over
/// the bleeding ganger's cell (ALONGSIDE the existing `read_bleeding` blood flash). Pins the
/// pop's text + valence on the registered-system path.
#[test]
fn bleeding_pops_the_amber_bleeding_fct_tag() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(4, 7);
    let level = Level::new(0);
    let ganger = spawn_ganger(&mut app, cell, level, 2);

    app.world_mut()
        .resource_mut::<Messages<Bleeding>>()
        .write(Bleeding::new(ganger));
    app.update();

    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "Bleeding", valence_color(FctValence::Wound)),
        "a Bleeding consequence must pop an AMBER \"Bleeding\" tag, got {pops:?}",
    );

    // The pop is anchored at the bleeding ganger's cell (planar x — the FCT z is the Highlight
    // band, distinct from the cell z).
    let anchor = cell_to_world(cell, level);
    let mut q = app.world_mut().query::<(&FloatingCombatText, &Transform)>();
    let any_at_cell = q
        .iter(app.world())
        .any(|(_, transform)| (transform.translation.x - anchor.x).abs() < 0.001);
    assert!(
        any_at_cell,
        "the Bleeding pop must anchor at the bleeding ganger's cell x ({})",
        anchor.x,
    );
}

/// GTW-302 (slice 4) — the REAL dispatch path: an `ArmorBroken { ganger, part }` drives the
/// registered `read_consequence_fct` system to spawn the RED `"Armor Broken"` floating-combat-
/// text pop over the ganger's cell (ALONGSIDE the existing `read_armor_broken` spark flash).
/// Pins the destroy-crossing tag's text + RED valence; the numeric `"Armor -N"` is DEFERRED
/// (the message carries no integrity-delta amount), so NO `"Armor -"` pop appears.
#[test]
fn armor_broken_pops_the_red_armor_broken_fct_tag() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(9, 3);
    let level = Level::new(0);
    let ganger = spawn_ganger(&mut app, cell, level, 5);

    app.world_mut()
        .resource_mut::<Messages<ArmorBroken>>()
        .write(ArmorBroken::new(ganger, BodyPart::Torso));
    app.update();

    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "Armor Broken", valence_color(FctValence::Damage)),
        "an ArmorBroken consequence must pop a RED \"Armor Broken\" tag, got {pops:?}",
    );
    // The numeric "Armor -N" variant is DEFERRED (no integrity delta on the message) — no
    // "Armor -" pop should appear.
    assert!(
        !pops.iter().any(|(t, _)| t.starts_with("Armor -")),
        "no numeric \"Armor -N\" pop is built this slice (no integrity delta), got {pops:?}",
    );
}
