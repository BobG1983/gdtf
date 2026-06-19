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
    ecs::message::Messages,
    prelude::default,
    render::{RenderPlugin, settings::WgpuSettings},
    sprite::Sprite,
    time::TimeUpdateStrategy,
    transform::components::Transform,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    EffectRoles, FxFlash, TopDownAtlases, TopDownRendererPlugin, cell_to_world, sim_pos_to_world,
};
use gdtf_battle_sim::{
    ArmorBroken, BattleInProgress, Bleeding, BodyPart, Cell, CellLevel, CoverDestroyed, Level,
    Position, ShotDir, ShotFired, ShotKind, SimPos, Wounds,
};

/// Generous settle headroom so a slow CI box never flakes on the async atlas /
/// effect-role loads.
const MAX_UPDATES: u32 = 128;

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
    app
}

/// Drives `update()`s until `EffectRoles` + `TopDownAtlases` are resident (the async
/// load chain has settled). Returns whether they settled within `MAX_UPDATES`.
fn settle_resources(app: &mut App) -> bool {
    for _ in 0..MAX_UPDATES {
        app.update();
        let has_roles = app.world().get_resource::<EffectRoles>().is_some();
        let has_atlases = app.world().get_resource::<TopDownAtlases>().is_some();
        if has_roles && has_atlases {
            return true;
        }
    }
    false
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

/// AC1 — a `Bleeding { ganger }` spawns EXACTLY ONE `FxFlash` at the ganger's cell with the
/// table's `bleed` atlas index (read structurally from `EffectRoles`, never a literal), and a
/// `*Wounds`-relation tint. A `Bleeding` for an entity with no `Position`/`Wounds` spawns no
/// flash and does not panic (fail-closed).
#[test]
fn bleeding_spawns_one_flash_at_the_ganger_cell_with_the_bleed_index() {
    let mut app = headless_renderer_app();
    assert!(
        settle_resources(&mut app),
        "EffectRoles + TopDownAtlases must resolve within {MAX_UPDATES} updates",
    );
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
    assert!(settle_resources(&mut app), "resources must resolve");
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
    assert!(settle_resources(&mut app), "resources must resolve");
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

/// The atlas indices of every `FxFlash` sprite currently in the world (unordered).
fn flash_indices(app: &mut App) -> Vec<usize> {
    let mut q = app.world_mut().query::<(&FxFlash, &Sprite)>();
    q.iter(app.world())
        .filter_map(|(_, sprite)| sprite.texture_atlas.as_ref().map(|atlas| atlas.index))
        .collect()
}

/// Whether ANY `FxFlash` sprite sits at (approximately) `world` — a position-presence
/// check across all flashes (the muzzle / tracer / impact spawn three at once, so the
/// SINGLE-flash helper does not apply).
fn any_flash_at(app: &mut App, world: bevy::math::Vec3) -> bool {
    let mut q = app.world_mut().query::<(&FxFlash, &Transform)>();
    q.iter(app.world())
        .any(|(_, t)| t.translation.distance(world) < 0.01)
}

/// Counts the `FlashTtl` lifetimes currently in the world (every FX flash carries one).
fn ttl_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&gdtf_battle_presenter::FlashTtl>();
    q.iter(app.world()).count()
}

/// GTW-290 — a `ShotFired` spawns THREE transient flashes: a muzzle flash at
/// `sim_pos_to_world(muzzle)`, a tracer beam, and a generic impact mark at
/// `cell_to_world(impact)` — each carrying `FlashTtl`, at the table's muzzle/tracer/impact
/// indices (read structurally from `EffectRoles`, never a literal). All three despawn once
/// the TTL elapses.
#[test]
fn shot_fired_spawns_muzzle_tracer_and_impact_flashes() {
    let mut app = headless_renderer_app();
    assert!(settle_resources(&mut app), "resources must resolve");
    app.world_mut().insert_resource(BattleInProgress);

    let roles = effect_roles(&app);
    assert!(roles.is_some(), "EffectRoles must be resident");
    let Some(roles) = roles else { return };

    // A struck ganger entity (the ShotKind::Ganger payload) + a known fire geometry.
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
    };

    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .write(shot);
    app.update();

    // Three flashes — muzzle, tracer, impact — each on a FlashTtl.
    assert_eq!(
        fx_count(&mut app),
        3,
        "a ShotFired must spawn three flashes (muzzle + tracer + impact)",
    );
    assert_eq!(
        ttl_count(&mut app),
        3,
        "every firing-FX flash must carry a FlashTtl one-shot clock",
    );

    // The muzzle flash sits at sim_pos_to_world(muzzle); the impact mark at
    // cell_to_world(impact). The tracer's midpoint is between them.
    assert!(
        any_flash_at(&mut app, sim_pos_to_world(muzzle)),
        "a flash must sit at the muzzle world position (sim_pos_to_world(muzzle))",
    );
    assert!(
        any_flash_at(&mut app, cell_to_world(impact_cell, impact_level)),
        "a flash must sit at the impact world position (cell_to_world(impact))",
    );

    // The indices are the table's muzzle/tracer/impact roles (structural, not pinned).
    let indices = flash_indices(&mut app);
    assert!(
        indices.contains(&*roles.muzzle_flash)
            && indices.contains(&*roles.tracer)
            && indices.contains(&*roles.impact),
        "the three flashes must use the table's muzzle/tracer/impact indices, got {indices:?}",
    );

    // All three are one-shot: they despawn once the TTL elapses.
    advance_past_ttl(&mut app);
    assert_eq!(
        fx_count(&mut app),
        0,
        "all three firing-FX flashes must despawn once their FlashTtl elapses",
    );
}

/// GTW-290 — a MISS still draws the firing FX: muzzle + tracer + generic impact mark (the
/// tracer terminates at the impact cell). A miss carries no struck object but the same
/// geometry, so the presenter draws the same three flashes.
#[test]
fn shot_fired_miss_still_spawns_muzzle_tracer_and_impact() {
    let mut app = headless_renderer_app();
    assert!(settle_resources(&mut app), "resources must resolve");
    app.world_mut().insert_resource(BattleInProgress);

    let shot = ShotFired {
        shooter:      app.world_mut().spawn_empty().id(),
        muzzle:       SimPos::new(1.0, 1.0, 0.0),
        trajectory:   ShotDir::from_direction(bevy::math::Vec3::new(0.0, 1.0, 0.0)),
        impact_cell:  Cell::new(1, 9),
        impact_level: Level::new(0),
        kind:         ShotKind::Miss,
    };

    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .write(shot);
    app.update();

    assert_eq!(
        fx_count(&mut app),
        3,
        "a miss still draws muzzle + tracer + impact (the tracer terminates at the cell)",
    );
}

/// AC4 — `FlashTtl` expiry makes the flash one-shot: a spawned flash is despawned by
/// `expire_flashes` after its TTL elapses, and a further empty update spawns nothing (the
/// reader drained its buffer — nothing lingers).
#[test]
fn flash_expires_after_its_ttl_and_nothing_lingers() {
    let mut app = headless_renderer_app();
    assert!(settle_resources(&mut app), "resources must resolve");
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
    assert!(settle_resources(&mut app), "resources must resolve");
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
