//! GTW-359 (E7 · GTW-12k) headless POSITIVE QA for the vertical-link tile draw + the
//! cross-storey ganger handoff + the movement tween — the C6 (b)/(c)/(d) clauses.
//!
//! These prove the DRAW LOGIC headless (component state — atlas index / Visibility /
//! Transform — is queryable); the PIXEL proof that the stair / ladder tiles actually
//! render NON-EMPTY is `vertical_link_readback.rs` (the dev screenshot capture is broken,
//! so a real-GPU readback closes that gap, `bevy-traps.md` #8). The harness is the
//! `ganger_draw.rs` pattern: a `DefaultPlugins`/`no_renderer` app with a live
//! `AssetServer` rooted at the workspace `assets/` (so `TileRoles` resolves to the
//! SHIPPED `tile_roles.ron`, `TopDownAtlases` loads) plus `TopDownRendererPlugin` and the
//! real `setup_battle` spawn path. The links + gangers are NOT hand-spawned — they are
//! poured through the real setup from a `Situation` carrying authored
//! [`VerticalLink`](gdtf_battle_sim::VerticalLink)s, so `setup_battle` validates + inserts
//! the `VerticalLinkGraph` the draw reads. `app.world_mut()` in the test body is the
//! accepted headless idiom (`bevy-traps.md` #7 carve-out (a)); no function here takes
//! `&mut World`/`&World`.
//!
//! POSITIVE — every assertion NAMES the indices (stair 77 / ladder 235), cells, and
//! Visibility / intermediate-Transform values it checks, not "something renders".

use std::{path::PathBuf, time::Duration};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::{error::warn, message::Messages},
    math::Vec3,
    prelude::{Entity, Visibility, default},
    render::{RenderPlugin, settings::WgpuSettings},
    sprite::Sprite,
    time::TimeUpdateStrategy,
    transform::components::Transform,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    ActiveLevel, GangerSprites, Layer, TileRoles, TopDownAtlases, TopDownRendererPlugin,
    VerticalLinkSprite, cell_to_world_layered,
};
use gdtf_battle_sim::{
    Aiming, BattleReady, BattleSeed, Cell, CellLevel, Direction, Facing, Faction, GangerSpawn,
    Level, LinkKind, Position, SetupBattleRequested, SimRng, Situation, VerticalLink,
    setup_battle_on_request,
    test_support::{SituationBuilder, test_armor_registry, test_weapon_registry},
};
use gdtf_test_utils::advance_until_resource_exists;

/// Bounded settle headroom for the post-setup state / spawn / glide waits.
const MAX_UPDATES: u32 = 128;

/// Safety-net cap for the async atlas / tile-role loads.
const LOAD_SAFETY_NET: u32 = 10_000;

/// A fixed seed for the deterministic `SetupBattleRequested`.
const SEED: u64 = 0x0D15_EA5E;

/// The LOCKED stair tile index (user OQ-3 ruling) — asserted as a system constant.
const STAIR_INDEX: usize = 77;

/// The LOCKED ladder tile index (user OQ-3 ruling) — asserted as a system constant.
const LADDER_INDEX: usize = 235;

/// The workspace-root `assets/` directory (manifest -> up two -> assets).
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// Builds a headless `DefaultPlugins`/`no_renderer` app driving the real `setup_battle`
/// spawn path (the `ganger_draw.rs` harness).
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
    .add_message::<BattleReady>()
    .add_message::<gdtf_battle_sim::CoverDestroyed>()
    .add_systems(bevy::app::Update, setup_battle_on_request)
    .add_plugins(TopDownRendererPlugin);
    app.insert_resource(test_weapon_registry());
    app.insert_resource(test_armor_registry());
    app.set_error_handler(warn);
    app
}

/// Drive `update()`s until `TileRoles` + `TopDownAtlases` are BOTH resident (the async
/// load chain settled), polling each resource's inserted SIGNAL (GTW-305).
fn settle_resources(app: &mut App) {
    advance_until_resource_exists::<TileRoles>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(app, LOAD_SAFETY_NET);
}

/// Build an authored ganger at `at` (standing rifleman, given faction + facing).
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

/// Pour `situation` into the battle via the REAL setup path; returns whether setup ran.
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

/// The atlas indices of every VISIBLE pooled vertical-link sprite (hidden pooled sprites,
/// which carry stale state, are excluded — the draw shows exactly the on-storey set).
fn visible_link_indices(app: &mut App) -> Vec<usize> {
    let mut q = app
        .world_mut()
        .query::<(&VerticalLinkSprite, &Sprite, &Visibility)>();
    q.iter(app.world())
        .filter(|(_, _, vis)| !matches!(vis, Visibility::Hidden))
        .filter_map(|(_, sprite, _)| sprite.texture_atlas.as_ref().map(|a| a.index))
        .collect()
}

/// Drive bounded `update()`s until at least one VISIBLE vertical-link sprite exists
/// (the link draw fires once `BattleInProgress` + the graph + `TileRoles` + atlases are
/// all live; its spawn lands a frame after the setup flush under parallel contention).
fn settle_link_sprites(app: &mut App) {
    for _ in 0..MAX_UPDATES {
        if !visible_link_indices(app).is_empty() {
            return;
        }
        app.update();
    }
}

/// The sim entity at `at` (the setup spawns one ganger per authored cell).
fn sim_entity_at(app: &mut App, at: CellLevel) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &Position)>();
    q.iter(app.world())
        .find(|(_, pos)| ***pos == at)
        .map(|(e, _)| e)
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

/// The world translation of the presenter sprite mirroring sim ganger `sim` (via the map).
fn translation_of_sim(app: &mut App, sim: Entity) -> Option<Vec3> {
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim))?;
    let mut q = app.world_mut().query::<&Transform>();
    q.get(app.world(), sprite).ok().map(|t| t.translation)
}

/// Drive bounded `update()`s until the presenter sprite mirroring sim ganger `sim` is
/// MAPPED in [`GangerSprites`] and queryable (so [`translation_of_sim`] returns `Some`).
///
/// The presenter `spawn_ganger_sprites` runs in `PresenterSystems::Draw` gated on
/// `BattleInProgress` + `TopDownAtlases`, so its `GangerSprites` map entry can lag the sim
/// spawn by a frame under parallel `cargo dtest` contention. This bounded settle removes
/// that spawn-lag race before a translation is read — mirroring `ganger_draw.rs`'s
/// `settle_sprite_at`; the bound surfaces a never-mapped sprite as a failed assertion, not
/// a hang.
fn settle_sprite_mapped(app: &mut App, sim: Entity) {
    for _ in 0..MAX_UPDATES {
        if translation_of_sim(app, sim).is_some() {
            return;
        }
        app.update();
    }
}

/// C6 (b) — the loaded SHIPPED `TileRoles` resolves stair == 77 + ladder == 235 (the
/// LOCKED system constants, user OQ-3), AND the rendered link-cell tile carries the
/// correct atlas index: a Stair link endpoint draws 77, a Ladder link endpoint draws 235.
///
/// Authors two links on level 0 ⇄ level 1: a STAIR (cells (3,3)) and a LADDER (cells
/// (6,6)). At active level 0 the lower endpoint of each is drawn; the stair sprite must
/// carry index 77 and the ladder sprite index 235 — proving the `LinkKind`-keyed role
/// mapping (C2) on the real draw path.
#[test]
fn loaded_roles_resolve_locked_indices_and_link_cells_carry_them() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    // The loaded SHIPPED tile_roles.ron resolves the LOCKED constants (the locked-constant
    // exemption — these are system constants per the user ruling, not tunable magnitudes).
    let roles = app.world().get_resource::<TileRoles>().cloned();
    assert!(
        roles.is_some(),
        "TileRoles must be resident after the load settles",
    );
    let Some(roles) = roles else { return };
    assert_eq!(
        *roles.stair, STAIR_INDEX,
        "the shipped tile_roles.ron must resolve stair == 77 (LOCKED, user OQ-3)",
    );
    assert_eq!(
        *roles.ladder, LADDER_INDEX,
        "the shipped tile_roles.ron must resolve ladder == 235 (LOCKED, user OQ-3)",
    );

    // A stair link (3,3) L0 <-> L1 and a ladder link (6,6) L0 <-> L1. Author the endpoints
    // as slabs so the link validation (no dangling endpoint) passes.
    let stair_lo = CellLevel::new(Cell::new(3, 3), Level::new(0));
    let stair_hi = CellLevel::new(Cell::new(3, 3), Level::new(1));
    let ladder_lo = CellLevel::new(Cell::new(6, 6), Level::new(0));
    let ladder_hi = CellLevel::new(Cell::new(6, 6), Level::new(1));
    let situation = SituationBuilder::new()
        .slab_at(stair_lo)
        .slab_at(stair_hi)
        .slab_at(ladder_lo)
        .slab_at(ladder_hi)
        .vertical_link(VerticalLink::new(stair_lo, stair_hi, LinkKind::stair()))
        .vertical_link(VerticalLink::new(ladder_lo, ladder_hi, LinkKind::ladder()))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    settle_link_sprites(&mut app);
    let indices = visible_link_indices(&mut app);
    // At active level 0 exactly the two LOWER endpoints draw (the hard cut excludes the
    // level-1 endpoints), one stair (77) and one ladder (235).
    assert_eq!(
        indices.len(),
        2,
        "exactly two link-cell sprites draw at active level 0 (one stair, one ladder), got {indices:?}",
    );
    assert!(
        indices.contains(&STAIR_INDEX),
        "a Stair link cell must draw the stair tile (index 77), got {indices:?}",
    );
    assert!(
        indices.contains(&LADDER_INDEX),
        "a Ladder link cell must draw the ladder tile (index 235), got {indices:?}",
    );
}

/// C6 (b) cont. — the link draw HARD-CUTS to the active storey (AC4): only the on-storey
/// endpoint of a link is drawn. With ONE stair link L0 ⇄ L1, exactly one stair sprite is
/// visible at active level 0; switching to active level 1 still shows exactly one stair
/// sprite (the upper endpoint, mutated in place — not respawned), never both endpoints.
#[test]
fn link_draw_hard_cuts_to_the_active_storey() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let lo = CellLevel::new(Cell::new(4, 4), Level::new(0));
    let hi = CellLevel::new(Cell::new(4, 4), Level::new(1));
    let situation = SituationBuilder::new()
        .slab_at(lo)
        .slab_at(hi)
        .vertical_link(VerticalLink::new(lo, hi, LinkKind::stair()))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    settle_link_sprites(&mut app);
    assert_eq!(
        visible_link_indices(&mut app),
        vec![STAIR_INDEX],
        "at active level 0 exactly the lower stair endpoint draws (the hard cut)",
    );

    // Switch to active level 1 — the upper endpoint draws, still exactly one (mutated in
    // place, never both endpoints at once).
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(Level::new(1));
    app.update();
    assert_eq!(
        visible_link_indices(&mut app),
        vec![STAIR_INDEX],
        "at active level 1 exactly the upper stair endpoint draws (still one, mutated in place)",
    );
}

/// C6 (c) — the CROSS-STOREY ganger Visibility handoff (AC2 CONFIRM, no new system): a
/// ganger whose `Position.z` leaves the active storey goes `Hidden`; back on it goes
/// `Inherited`. Confirms the LANDED `move_ganger_sprites` flip survives the GTW-359 tween
/// restructure (the C3 KEEP clause).
#[test]
fn ganger_visibility_flips_hidden_when_position_leaves_active_storey() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    // The ganger starts on level 0; author both storey cells as slabs.
    let l0 = CellLevel::new(Cell::new(5, 5), Level::new(0));
    let l1 = CellLevel::new(Cell::new(5, 5), Level::new(1));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(l0, 0, Direction::East))
        .slab_at(l0)
        .slab_at(l1)
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    let sim = sim_entity_at(&mut app, l0);
    assert!(sim.is_some(), "the ganger sim entity must exist");
    let Some(sim) = sim else { return };

    // On the active storey (level 0): Inherited.
    assert_eq!(
        visibility_of_sim(&mut app, Some(sim)),
        Some(Visibility::Inherited),
        "a ganger ON the active storey is Visibility::Inherited",
    );

    // Move the ganger UP to level 1 (off the active storey 0) — Position.z leaves it.
    let mut pos_q = app.world_mut().query::<&mut Position>();
    if let Ok(mut pos) = pos_q.get_mut(app.world_mut(), sim) {
        *pos = Position::new(l1);
    }
    app.update();
    assert_eq!(
        visibility_of_sim(&mut app, Some(sim)),
        Some(Visibility::Hidden),
        "after Position.z leaves the active storey the ganger sprite goes Hidden (the handoff)",
    );

    // Move it BACK down to level 0 — Inherited again.
    let mut pos_q = app.world_mut().query::<&mut Position>();
    if let Ok(mut pos) = pos_q.get_mut(app.world_mut(), sim) {
        *pos = Position::new(l0);
    }
    app.update();
    assert_eq!(
        visibility_of_sim(&mut app, Some(sim)),
        Some(Visibility::Inherited),
        "back on the active storey the ganger sprite goes Inherited again",
    );
}

/// C6 (d) — the sprite is at an INTERMEDIATE Transform mid-tween: after a single small
/// time step following a move, the sprite is STRICTLY BETWEEN the old and new cell world
/// positions, NOT snapped to the destination (the AC3 / OQ-5 glide, no snap).
#[test]
fn moved_sprite_is_intermediate_mid_tween_not_snapped() {
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

    let sim = sim_entity_at(&mut app, start);
    assert!(sim.is_some(), "the ganger sim entity must exist");
    let Some(sim) = sim else { return };

    let start_world = cell_to_world_layered(Cell::new(5, 6), Level::new(0), Layer::Actor);
    let dest = CellLevel::new(Cell::new(12, 6), Level::new(0));
    let dest_world = cell_to_world_layered(Cell::new(12, 6), Level::new(0), Layer::Actor);

    // Wait out the presenter spawn-lag: under parallel `cargo dtest` the ganger sprite +
    // its `GangerSprites` map entry can land a frame after the sim spawn, so settle the map
    // before reading the translation (removes the spawn-lag race; the assertion is unchanged).
    settle_sprite_mapped(&mut app, sim);

    // The sprite begins AT the start cell (the spawn-seeded settled tween).
    let before = translation_of_sim(&mut app, sim);
    assert!(
        before.is_some_and(|t| t.distance(start_world) < 1.0e-3),
        "the sprite begins at the start cell (got {before:?}, expected {start_world:?})",
    );

    // Move the ganger; advance time by a SMALL fixed step — much less than the tween's own
    // glide duration — so the glide is mid-flight, NOT settled, this frame.
    let mut pos_q = app.world_mut().query::<&mut Position>();
    if let Ok(mut pos) = pos_q.get_mut(app.world_mut(), sim) {
        *pos = Position::new(dest);
    }
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        30,
    )));
    app.update();

    let mid = translation_of_sim(&mut app, sim);
    assert!(mid.is_some(), "the sprite must still exist mid-tween");
    let Some(mid) = mid else { return };
    // STRICTLY between start and dest on the moving (x) axis — not snapped to either end.
    assert!(
        mid.x > start_world.x && mid.x < dest_world.x,
        "the sprite must be at an INTERMEDIATE x mid-tween (strictly between start {} and dest \
         {}, NOT snapped), got {}",
        start_world.x,
        dest_world.x,
        mid.x,
    );
    // It has moved off the start but not reached the dest (the never-snap property).
    assert!(
        mid.distance(start_world) > 1.0e-3,
        "the sprite must have started gliding (moved off the start cell), got {mid:?}",
    );
    assert!(
        mid.distance(dest_world) > 1.0e-3,
        "the sprite must NOT have snapped to the dest in one small step, got {mid:?}",
    );
}
