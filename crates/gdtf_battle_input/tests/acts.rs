//! GTW-227 (GTW-48 S8 / 222b): headless integration tests for the core player ACTS —
//! fire (left-click), posture (stance / aim / facing keys), and fire-mode selection —
//! over the REAL 222a act-intent seam (`GdtfBattleInputPlugin`'s keyboard / fire-click
//! systems -> the ONE `dispatch_act_intents` drain -> the emitted `*Requested`).
//!
//! - AC1 drives the REAL selection path (synth left-click on an armed occupant) and
//!   asserts `SelectedFireMode` defaults to that weapon's `FireMode::single()`.
//! - (GTW-254) The blind fire-mode cycle was REMOVED — the `gdtf_app` popup picker
//!   replaced it, so the old AC2 key-walk test is gone (the picker's behavior is covered
//!   by the `gdtf_app` `action_bar.rs` integration tests). `sync_fire_mode_on_select`
//!   (AC1) is still the picker's default-on-select dependency.
//! - AC3 synthesizes a left-click on an in-bounds target for an alive / loaded /
//!   affordable shooter and asserts EXACTLY one `FireRequested` with the expected
//!   shooter / mode / target fields is emitted (a probe `MessageReader`).
//! - AC4 varies ONE failing `can_fire` input at a time (Downed; empty magazine; TU one
//!   below the charge; an out-of-bounds target) and asserts ZERO `FireRequested`.
//! - AC5 synthesizes each posture key and asserts one `SetStanceRequested` /
//!   `SetAimingRequested` / `SetFacingRequested` for `*SelectedShooter` with the
//!   next-of-cycle value — byte-for-byte EQUAL to the message the SAME `ActIntent`
//!   pushed directly (the 222c-button surrogate) produces.
//! - AC6 clears the selection and drives all act keys + a left-click, asserting ZERO
//!   messages of every `*Requested` type and no panic.
//!
//! Every `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.

use bevy::{
    camera::{
        Camera, ComputedCameraValues, OrthographicProjection, Projection, RenderTargetInfo,
        primitives::Frustum,
    },
    input::ButtonInput,
    prelude::*,
    transform::components::GlobalTransform,
    window::{PrimaryWindow, Window, WindowResolution},
};
use gdtf_battle_input::{
    ActIntent, BoundKey, GdtfBattleInputPlugin, InspectTarget, Keybinds, PathPreviewTarget,
    PendingActIntent, SelectedFireMode, SelectedShooter, next_facing, next_stance,
};
use gdtf_battle_presenter::{ActiveLevel, WorldCamera};
use gdtf_battle_sim::{
    Aiming, BattleInProgress, BattleSeed, BraceStairCells, Cell, CellLevel, CoverLedger, Direction,
    Facing, Faction, FireMode, FireModeSpec, FloorCostGrid, InjuryRng, Level, LifeState, LinkKind,
    LootRng, Magazine, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
    OccupancyGrid, PlayerFaction, ProcgenRng, ReloadTu, SeverityRng, ShotRng, SlabLedger,
    SquadVisibility, Stance, StanceKind, SurfaceGrid, Tu, TuMax, VerticalLink, VerticalLinkGraph,
    WieldedBy,
    acts::{
        AimRequest, EndTurnRequested, ExecuteDownedRequested, FireRequested, MoveRequested,
        ReloadRequested, SetAimingRequested, SetFacingRequested, SetStanceRequested, SimActsPlugin,
        StabilizeDownedRequested,
    },
    build_vertical_link_graph,
    test_support::SituationBuilder,
    tuning::CombatTuning,
};

/// The faction the player controls in these tests (matches `PlayerFaction`).
const PLAYER_FACTION: Faction = Faction::new(0);
/// An ENEMY faction (distinct from [`PLAYER_FACTION`]) — a non-player target the FIRE
/// branch of the unified left-click decision (GTW-238) fires on.
const ENEMY_FACTION: Faction = Faction::new(1);

/// The synthetic window/camera render-target size (physical px), large enough that
/// distinct cursor offsets land on distinct in-grid cells (the GTW-221 `picking.rs`
/// harness size).
const TARGET_SIZE: Vec2 = Vec2::new(1280.0, 720.0);

// ---------------------------------------------------------------------------------
// Fixtures.
// ---------------------------------------------------------------------------------

/// A fully-bound [`Keybinds`] table for the act-key tests — built in the test body
/// (not parsed from the editable shipped `.ron`).
const fn test_keybinds() -> Keybinds {
    Keybinds {
        select_clear: BoundKey::KeyEscape,
        level_up:     BoundKey::KeyPageUp,
        level_down:   BoundKey::KeyPageDown,
        stance_cycle: BoundKey::KeyC,
        aim_toggle:   BoundKey::KeyF,
        facing_cycle: BoundKey::KeyR,
    }
}

/// One fire-mode spec of an explicit [`ModeKind`] — the kind is what the cycle
/// identifies a mode by (the cone/TU/shots magnitudes are arbitrary, not pinned
/// tuning).
const fn spec(kind: ModeKind, tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        kind,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// A three-mode `[Single, Burst, Full]` selector with three distinct modes.
fn sbf_selector() -> FireMode {
    FireMode::new(vec![
        spec(ModeKind::Single, 0.2, 1),
        spec(ModeKind::Burst, 0.4, 3),
        spec(ModeKind::Full, 0.7, 6),
    ])
}

/// A cursor offset (from the window centre) that resolves to a non-origin in-grid cell
/// for the SHOOTER. Centre maps to world (0,0) = cell (0,0); an in-grid cell needs
/// world.x >= 0 (cursor RIGHT) and world.y <= 0 (cursor DOWN — screen-y grows down,
/// world-y grows up). The GTW-221 picking-test sign reasoning.
const SHOOTER_CURSOR_OFFSET: Vec2 = Vec2::new(40.0, 32.0);
/// A SECOND, distinct cursor offset resolving to a DIFFERENT in-grid cell — the FIRE
/// target (so the fire click targets a cell other than the shooter's own).
const TARGET_CURSOR_OFFSET: Vec2 = Vec2::new(200.0, 160.0);

/// Builds a deterministic [`Camera`] whose `viewport_to_world_2d` succeeds WITHOUT a
/// render pipeline (the GTW-221 `picking.rs` synthetic-camera recipe).
fn synthetic_camera() -> Camera {
    let mut projection = Projection::Orthographic(OrthographicProjection::default_2d());
    projection.update(TARGET_SIZE.x, TARGET_SIZE.y);
    Camera {
        computed: ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: TARGET_SIZE.as_uvec2(),
                scale_factor:  1.0,
            }),
            clip_from_view: projection.get_clip_from_view(),
            ..ComputedCameraValues::default()
        },
        ..Camera::default()
    }
}

/// Builds the base headless app for the act tests: `MinimalPlugins` + the
/// `GdtfBattleInputPlugin` + `SimActsPlugin` (the message-buffer + dispatch registration
/// the AC3/AC4 probe reads against) + the presenter-owned `ActiveLevel`, the
/// `BattleInProgress` gate, an empty `OccupancyGrid`, the sim resources `SimActsPlugin`
/// reads, `CombatTuning`, a `Keybinds` table, seeded `ButtonInput` buffers, and a
/// SYNTHETIC `WorldCamera` + `Window` so the REAL `pick_hovered_cell` resolves a cursor
/// to an in-grid cell (driving the genuine cursor -> `InspectTarget` -> select/fire chain,
/// not an injected cell). The keybinds are inserted directly (the sanctioned headless
/// idiom — no `AssetServer` under `MinimalPlugins`).
fn acts_app() -> App {
    let mut app = App::new();
    // GTW-322: `update_selection_highlight` (in `GdtfBattleInputPlugin`) spawns its reticle
    // via `Commands::spawn_scene`, which needs an `AssetServer` + the scene schedule. Under
    // `MinimalPlugins` that flush PANICS without `AssetPlugin` + `ScenePlugin` (the
    // spike-documented requirement); adding them keeps the harness a no-op asset world while
    // the converted spawn path resolves.
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(bevy::scene::ScenePlugin)
        .add_plugins(GdtfBattleInputPlugin)
        .add_plugins(SimActsPlugin);
    let level = Level::new(0);
    app.world_mut().insert_resource(ActiveLevel::new(level));
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut().insert_resource(CombatTuning::default());
    // GTW-238: the unified left-click decision gates on (and reads) PlayerFaction.
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    // The sim resources `SimActsPlugin`'s dispatch systems read (so they pass param
    // validation). The grids are empty defaults + a seeded RNG; an unarmed shooter
    // (no `Weapon` marker) makes `fire()` a no-op, so they never affect the asserted
    // `*Requested` MESSAGE — only its registered buffers / dispatch must validate.
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(CoverLedger::new());
    // GTW-365: `dispatch_fire` reads `ResMut<SlabLedger>` (the slab-hit depletion path),
    // so the harness seeds an empty ledger for the dispatch param to validate.
    app.world_mut().insert_resource(SlabLedger::new());
    // GTW-392: `dispatch_fire` reads `Res<BraceStairCells>` — seed an empty set (no
    // stair entities in this harness) so the terrain-brace gate param validates.
    app.world_mut().insert_resource(BraceStairCells::empty());
    // GTW-354: the constrained `dispatch_move` reads `Res<VerticalLinkGraph>` +
    // `Res<SquadVisibility>` for its route gate, so the harness seeds them (empty link
    // graph + empty fog — these AC tests assert the input `*Requested` MESSAGE, never a
    // move outcome, so the route gate result is irrelevant; they need only validate).
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());
    app.world_mut().insert_resource(SquadVisibility::default());
    // GTW-396: `dispatch_move` reads `Res<FloorCostGrid>` — seed a uniform grid at the
    // default open cost so the dispatch params validate (these tests assert the input
    // *Requested MESSAGE, never a move outcome).
    {
        let open = CombatTuning::default().move_costs.open;
        app.world_mut()
            .insert_resource(FloorCostGrid::new(open, []));
    }
    // GTW-14: five per-subsystem RNG streams from the test seed.
    let sim_seed = BattleSeed::new(0x5A1C_AC75);
    app.world_mut()
        .insert_resource(ShotRng::from_root(sim_seed));
    app.world_mut()
        .insert_resource(SeverityRng::from_root(sim_seed));
    app.world_mut()
        .insert_resource(LootRng::from_root(sim_seed));
    app.world_mut()
        .insert_resource(InjuryRng::from_root(sim_seed));
    app.world_mut()
        .insert_resource(ProcgenRng::from_root(sim_seed));
    app.world_mut().insert_resource(test_keybinds());
    app.world_mut()
        .insert_resource(ButtonInput::<KeyCode>::default());
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());

    // The synthetic world camera + window (GTW-221 picking-test recipe), so
    // `pick_hovered_cell` unprojects a cursor into a real in-grid cell.
    app.world_mut().spawn((
        Camera2d,
        WorldCamera,
        synthetic_camera(),
        GlobalTransform::IDENTITY,
        Projection::Orthographic(OrthographicProjection::default_2d()),
        Frustum::default(),
    ));
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(TARGET_SIZE.x as u32, TARGET_SIZE.y as u32),
            ..default()
        },
        PrimaryWindow,
    ));
    app
}

/// Spawns an armed, alive, loaded, affordable PLAYER-faction ganger with the given
/// fire-mode selector, stance, and facing, and returns its entity. Carries exactly the
/// GANGER components the act systems query: `Faction` (GTW-238 SELECT/FIRE gating),
/// `Stance`/`Facing`/`Aiming` (cycle + selection) and `LifeState`/`Tu`/`TuMax` (the
/// `FireActor` `can_fire` vitals). The weapon's `FireMode` selector + `Magazine` ride on
/// a related WEAPON entity (`Wields`, GTW-323 slice 3 — the fire-mode default + the
/// `can_fire` magazine read both resolve through `ganger → Wields → weapon` now). The
/// `WieldedBy` insert hook populates the ganger's `Wields` synchronously in a bare
/// `World` spawn.
fn spawn_ganger(
    app: &mut App,
    selector: FireMode,
    stance: StanceKind,
    facing: Direction,
) -> Entity {
    let size = MagazineSize::new(30);
    let ganger = app
        .world_mut()
        .spawn((
            PLAYER_FACTION,
            Stance::new(stance),
            Facing::new(facing),
            Aiming::new(false),
            LifeState::Alive,
            Tu::new(255),
            TuMax::new(100),
        ))
        .id();
    app.world_mut().spawn((
        WieldedBy::new(ganger),
        selector,
        Magazine::new(10, size, ReloadTu::new(12)),
    ));
    ganger
}

/// Empties the wielded WEAPON entity's magazine (GTW-323 slice 3: the fire guard reads
/// `ganger → Wields → weapon → Magazine`, so an empty-magazine test must empty the WEAPON,
/// not the ganger). No-op if the ganger wields no weapon.
fn empty_wielded_magazine(app: &mut App, ganger: Entity) {
    if let Some(weapon) = app
        .world()
        .get::<gdtf_battle_sim::Wields>(ganger)
        .and_then(gdtf_battle_sim::Wields::weapon)
    {
        app.world_mut().entity_mut(weapon).insert(Magazine::new(
            0,
            MagazineSize::new(30),
            ReloadTu::new(12),
        ));
    }
}

/// Places an ENEMY-faction occupant in the occupancy grid at `cell` (so the FIRE branch
/// of the unified left-click decision sees a non-player target there), returning its
/// entity. The enemy carries only a `Faction` — the fire path reads the SHOOTER's firing
/// components, never the target's.
///
/// GTW-11 — it ALSO marks `cell` squad-VISIBLE in the fog, the realistic battle state (you fire
/// on a SEEN enemy): without this the new targeting-fog rung in `decide_left_click` would refuse
/// every fire (fail-closed on a cell absent from the default-empty `SquadVisibility`). The
/// can_fire-FAILURE tests still emit nothing — they fail `can_fire` for OTHER reasons (no TU,
/// empty magazine, out of bounds), which a visible cell does not change.
fn place_enemy(app: &mut App, cell: CellLevel) -> Entity {
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(enemy));
    // Mark the enemy cell squad-VISIBLE so the fire commit passes the GTW-11 fog gate.
    if let Some(fog) = app.world_mut().get_resource::<SquadVisibility>() {
        let mut visible: bevy::platform::collections::HashSet<CellLevel> =
            fog.visible_cells().copied().collect();
        let mut explored: bevy::platform::collections::HashSet<CellLevel> =
            fog.explored_cells().copied().collect();
        visible.insert(cell);
        explored.insert(cell);
        app.world_mut()
            .insert_resource(SquadVisibility::new(visible, explored));
    }
    enemy
}

/// Sets the primary window cursor to the given window-centre offset and resolves it via
/// one `update()` (driving the REAL `pick_hovered_cell`), returning the resolved hovered
/// cell. The cursor stays at this position until moved, so a later action `update()`
/// re-resolves the SAME cell — making select/fire vs `pick_hovered_cell` ordering moot
/// (the value is identical before and after pick).
fn hover_at(app: &mut App, offset: Vec2) -> CellLevel {
    set_cursor(app, Some(TARGET_SIZE * 0.5 + offset));
    app.update();
    app.world()
        .get_resource::<InspectTarget>()
        .and_then(InspectTarget::hovered)
        .unwrap_or_else(|| CellLevel::new(Cell::new(0, 0), Level::new(0)))
}

/// Sets the primary window cursor position (logical px), or clears it.
fn set_cursor(app: &mut App, position: Option<Vec2>) {
    let mut windows = app.world_mut().query::<&mut Window>();
    for mut window in windows.iter_mut(app.world_mut()) {
        window.set_cursor_position(position);
    }
}

/// Selects `ganger` over the REAL cursor -> `InspectTarget` -> `left_click_act` chain:
/// places the cursor over the shooter cell, places `ganger` there in the occupancy grid,
/// then a fresh left-click + `update()` selects it. The cursor stays over the shooter
/// cell (so the value is stable). Returns the resolved shooter cell.
fn select_ganger(app: &mut App, ganger: Entity) -> CellLevel {
    let shooter_cell = hover_at(app, SHOOTER_CURSOR_OFFSET);
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(shooter_cell, Some(ganger));
    press_left(app);
    app.update();
    clear_mouse(app);
    shooter_cell
}

/// Presses (just-pressed edge) a key.
fn press_key(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
}

/// Presses (just-pressed edge) the left mouse button.
fn press_left(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
}

/// Releases + clears the mouse edges so a later press is a fresh just-pressed.
fn clear_mouse(app: &mut App) {
    let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    mouse.release(MouseButton::Left);
    mouse.clear();
}

/// The current `SelectedFireMode`.
fn fire_mode(app: &App) -> Option<FireModeSpec> {
    app.world().get_resource::<SelectedFireMode>().map(|m| **m)
}

// ---------------------------------------------------------------------------------
// Message probes — collect the messages emitted this run into resources read in the
// test body (each probe runs after the drain, so it sees the same update's emission).
// ---------------------------------------------------------------------------------

/// Collected `FireRequested` messages (probe).
#[derive(Resource, Default)]
struct FireProbe(Vec<FireRequested>);
/// Collected `MoveRequested` messages (probe, GTW-356 two-click move target).
#[derive(Resource, Default)]
struct MoveProbe(Vec<MoveRequested>);
/// Collected `SetStanceRequested` messages (probe).
#[derive(Resource, Default)]
struct StanceProbe(Vec<SetStanceRequested>);
/// Collected `SetAimingRequested` messages (probe).
#[derive(Resource, Default)]
struct AimProbe(Vec<SetAimingRequested>);
/// Collected `SetFacingRequested` messages (probe).
#[derive(Resource, Default)]
struct FacingProbe(Vec<SetFacingRequested>);
/// Collected `ReloadRequested` messages (probe, GTW-275).
#[derive(Resource, Default)]
struct ReloadProbe(Vec<ReloadRequested>);
/// Collected `EndTurnRequested` messages (probe, GTW-309).
#[derive(Resource, Default)]
struct EndTurnProbe(Vec<EndTurnRequested>);
/// Collected `ExecuteDownedRequested` messages (probe, GTW-294).
#[derive(Resource, Default)]
struct ExecuteProbe(Vec<ExecuteDownedRequested>);
/// Collected `StabilizeDownedRequested` messages (probe, GTW-294).
#[derive(Resource, Default)]
struct StabilizeProbe(Vec<StabilizeDownedRequested>);

/// Adds the message-collecting probe systems, each running AFTER the drain so it
/// observes the same update's emitted messages. The probes have their own
/// `MessageReader` cursors (independent of the sim's `dispatch_*`), so they read every
/// message the drain wrote.
fn add_probes(app: &mut App) {
    app.insert_resource(FireProbe::default())
        .insert_resource(MoveProbe::default())
        .insert_resource(StanceProbe::default())
        .insert_resource(AimProbe::default())
        .insert_resource(FacingProbe::default())
        .insert_resource(ReloadProbe::default())
        .insert_resource(EndTurnProbe::default())
        .insert_resource(ExecuteProbe::default())
        .insert_resource(StabilizeProbe::default());
    app.add_systems(
        Update,
        (
            |mut r: MessageReader<FireRequested>, mut p: ResMut<FireProbe>| {
                // `FireRequested` is no longer `Copy` (it owns a `FireModeSpec`) — clone.
                p.0.extend(r.read().cloned());
            },
            |mut r: MessageReader<MoveRequested>, mut p: ResMut<MoveProbe>| {
                p.0.extend(r.read().copied());
            },
            |mut r: MessageReader<SetStanceRequested>, mut p: ResMut<StanceProbe>| {
                p.0.extend(r.read().copied());
            },
            |mut r: MessageReader<SetAimingRequested>, mut p: ResMut<AimProbe>| {
                p.0.extend(r.read().copied());
            },
            |mut r: MessageReader<SetFacingRequested>, mut p: ResMut<FacingProbe>| {
                p.0.extend(r.read().copied());
            },
            |mut r: MessageReader<ReloadRequested>, mut p: ResMut<ReloadProbe>| {
                p.0.extend(r.read().copied());
            },
            |mut r: MessageReader<EndTurnRequested>, mut p: ResMut<EndTurnProbe>| {
                p.0.extend(r.read().copied());
            },
            |mut r: MessageReader<ExecuteDownedRequested>, mut p: ResMut<ExecuteProbe>| {
                p.0.extend(r.read().copied());
            },
            |mut r: MessageReader<StabilizeDownedRequested>, mut p: ResMut<StabilizeProbe>| {
                p.0.extend(r.read().copied());
            },
        )
            .after(gdtf_battle_input::dispatch_act_intents),
    );
}

/// The collected `FireRequested` messages.
fn fires(app: &App) -> Vec<FireRequested> {
    app.world()
        .get_resource::<FireProbe>()
        .map(|p| p.0.clone())
        .unwrap_or_default()
}

/// The collected `MoveRequested` messages (GTW-356 two-click move target).
fn moves(app: &App) -> Vec<MoveRequested> {
    app.world()
        .get_resource::<MoveProbe>()
        .map(|p| p.0.clone())
        .unwrap_or_default()
}

/// The current `PathPreviewTarget` (the GTW-356 two-click move target the preview previews TO).
fn move_target(app: &App) -> Option<CellLevel> {
    app.world()
        .get_resource::<PathPreviewTarget>()
        .and_then(|t| **t)
}

// ---------------------------------------------------------------------------------
// AC1 — SelectedFireMode defaults to the selected weapon's single() on selecting an
// armed ganger, via the REAL selection path.
// ---------------------------------------------------------------------------------

/// AC1 — selecting an armed ganger sets `SelectedFireMode` to that weapon's
/// `FireMode::single()`.
#[test]
fn selecting_armed_ganger_defaults_fire_mode_to_single() {
    let mut app = acts_app();
    let selector = sbf_selector();
    let ganger = spawn_ganger(
        &mut app,
        selector.clone(),
        StanceKind::Standing,
        Direction::North,
    );

    select_ganger(&mut app, ganger);

    assert_eq!(
        fire_mode(&app),
        Some(selector.single()),
        "selecting an armed ganger must default SelectedFireMode to its FireMode::single()",
    );
}

// ---------------------------------------------------------------------------------
// (GTW-254) The blind fire-mode cycle was REMOVED — the `gdtf_app` popup picker
// replaced it. Its old key-walk test (`fire_mode_cycle_walks_only_offered_modes`) is
// gone; the picker's open -> list-offered-modes -> select-sets-`SelectedFireMode`
// behavior is covered by the `gdtf_app` `action_bar.rs` integration tests on the real
// picker. AC1 (default-to-`single()` on select) above still covers
// `sync_fire_mode_on_select`, which the picker also relies on.
// ---------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------
// AC3 — a left-click on an in-bounds target emits exactly one FireRequested with the
// expected fields, when can_fire passes.
// ---------------------------------------------------------------------------------

/// AC3 — a left-click on an ENEMY target cell emits EXACTLY one `FireRequested { shooter
/// = *SelectedShooter, mode = *SelectedFireMode, target from the hovered cell }`, driven over
/// the REAL cursor -> `InspectTarget` -> `left_click_act` FIRE-branch chain.
#[test]
fn left_click_emits_one_fire_requested() {
    let mut app = acts_app();
    add_probes(&mut app);
    let selector = sbf_selector();
    let ganger = spawn_ganger(
        &mut app,
        selector.clone(),
        StanceKind::Standing,
        Direction::North,
    );

    // Select the shooter (cursor over its own cell), then move the cursor to a distinct
    // in-grid target cell. The cursor stays put across the fire `update()`, so
    // `pick_hovered_cell` re-resolves the same target both before and after the
    // fire/select systems (ordering-independent).
    select_ganger(&mut app, ganger);
    let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    let target_cell = Cell::new(target.x, target.y);
    let target_level = Level::new(0);
    // GTW-238 FIRE requires an ENEMY occupant at the target — place one there.
    place_enemy(&mut app, target);

    press_left(&mut app);
    app.update();

    let emitted = fires(&app);
    assert_eq!(
        emitted.len(),
        1,
        "exactly one FireRequested must be emitted by a left-click on an in-bounds target",
    );
    let msg = &emitted[0];
    assert_eq!(msg.shooter, ganger, "shooter = *SelectedShooter");
    assert_eq!(msg.mode, selector.single(), "mode = *SelectedFireMode");
    assert_eq!(
        msg.target_cell, target_cell,
        "target cell from the hovered cell"
    );
    assert_eq!(
        msg.target_level, target_level,
        "target level from the hovered cell"
    );
}

// ---------------------------------------------------------------------------------
// AC4 — can_fire blocks emission: vary one failing input at a time -> zero
// FireRequested.
// ---------------------------------------------------------------------------------

/// AC4 — a Downed shooter, an empty magazine, insufficient TU each fail the shared
/// `can_fire` guard, and a cursor with NO in-bounds target (off-window) yields no target
/// — each causes NO `FireRequested`. Every case starts from the passing AC3 fixture and
/// breaks ONE input.
///
/// Note: the `can_fire` `in_bounds` predicate cannot fail through this surface — the
/// real `pick_hovered_cell` only ever resolves IN-grid cells (it is `None` off-grid), so
/// the "out-of-bounds target" failure manifests as "no in-bounds target -> no fire" (the
/// off-window cursor case). `can_fire`'s `in_bounds` is unit-tested directly in
/// `gdtf_battle_sim::magazine`.
#[test]
fn can_fire_failure_blocks_fire_requested() {
    // Downed shooter.
    {
        let mut app = acts_app();
        add_probes(&mut app);
        let ganger = spawn_ganger(
            &mut app,
            sbf_selector(),
            StanceKind::Standing,
            Direction::North,
        );
        select_ganger(&mut app, ganger);
        let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
        place_enemy(&mut app, target);
        app.world_mut().entity_mut(ganger).insert(LifeState::Downed);
        press_left(&mut app);
        app.update();
        assert!(
            fires(&app).is_empty(),
            "a Downed shooter must emit no FireRequested"
        );
    }

    // Empty magazine.
    {
        let mut app = acts_app();
        add_probes(&mut app);
        let ganger = spawn_ganger(
            &mut app,
            sbf_selector(),
            StanceKind::Standing,
            Direction::North,
        );
        select_ganger(&mut app, ganger);
        let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
        place_enemy(&mut app, target);
        // GTW-323 slice 3: the magazine lives on the related WEAPON entity now, so empty
        // THAT (the fire guard reads `ganger → Wields → weapon → Magazine`), not the ganger.
        empty_wielded_magazine(&mut app, ganger);
        press_left(&mut app);
        app.update();
        assert!(
            fires(&app).is_empty(),
            "an empty magazine must emit no FireRequested"
        );
    }

    // Insufficient TU — one below the selected mode's charge.
    {
        let mut app = acts_app();
        add_probes(&mut app);
        let ganger = spawn_ganger(
            &mut app,
            sbf_selector(),
            StanceKind::Standing,
            Direction::North,
        );
        select_ganger(&mut app, ganger);
        let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
        place_enemy(&mut app, target);
        // The selected mode is single() (set on selection). Compute its exact charge and
        // set TU one below it via the SHARED mode_tu_cost source.
        let charge = gdtf_battle_sim::mode_tu_cost(
            &sbf_selector().single(),
            &TuMax::new(100),
            &Aiming::new(false),
            &CombatTuning::default(),
        );
        assert!(*charge > 0, "the mode charge must be positive for the test");
        app.world_mut()
            .entity_mut(ganger)
            .insert(Tu::new(charge.saturating_sub(1)));
        press_left(&mut app);
        app.update();
        assert!(
            fires(&app).is_empty(),
            "TU one below the mode charge must emit no FireRequested",
        );
    }

    // No in-bounds target — an off-window cursor resolves the hovered cell to `None`, so the
    // fire surface has no target and emits nothing (the in-bounds boundary at this layer).
    {
        let mut app = acts_app();
        add_probes(&mut app);
        let ganger = spawn_ganger(
            &mut app,
            sbf_selector(),
            StanceKind::Standing,
            Direction::North,
        );
        select_ganger(&mut app, ganger);
        set_cursor(&mut app, None);
        app.update();
        assert_eq!(
            app.world()
                .get_resource::<InspectTarget>()
                .and_then(InspectTarget::hovered),
            None,
            "an off-window cursor must resolve the hovered cell to None",
        );
        press_left(&mut app);
        app.update();
        assert!(
            fires(&app).is_empty(),
            "no in-bounds target (off-window cursor) must emit no FireRequested",
        );
    }
}

// ---------------------------------------------------------------------------------
// AC5 — posture keys emit the matching *Requested for *SelectedShooter with the
// next-of-cycle value, byte-for-byte equal to the same intent pushed directly.
// ---------------------------------------------------------------------------------

/// The two ways to drive one posture act — a synthesized KEY press, or a direct
/// `ActIntent` push (the 222c-button surrogate over the SAME seam). Not `Copy`:
/// [`Drive::Intent`] holds an `ActIntent`, which is no longer `Copy`.
#[derive(Clone)]
enum Drive {
    /// Synthesize a just-pressed of `key`.
    Key(KeyCode),
    /// Push `intent` directly onto the shared queue (the 222c-button surrogate).
    Intent(ActIntent),
}

/// Builds a fresh acts app, selects an armed ganger (at the standing/north start), and
/// applies `drive` (a key press OR a direct intent push), then `update()`s once.
/// Returns `(app, ganger)` for the caller to read the relevant probe.
fn drive_one_act(drive: Drive) -> (App, Entity) {
    let mut app = acts_app();
    add_probes(&mut app);
    let ganger = spawn_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, ganger);
    match drive {
        Drive::Key(key) => press_key(&mut app, key),
        Drive::Intent(intent) => app
            .world_mut()
            .resource_mut::<PendingActIntent>()
            .push(intent),
    }
    app.update();
    (app, ganger)
}

/// AC5 — the stance-cycle KEY emits one `SetStanceRequested` for `*SelectedShooter` with
/// the next-of-cycle stance, byte-for-byte EQUAL to the message the direct `StanceCycle`
/// intent produces over the SAME seam. (The keyboard keeps the blind cycle; the GTW-267
/// action-bar replaced its BLIND-cycle button with direct-set `SetStance` toggles.)
#[test]
fn stance_key_emits_next_of_cycle_and_matches_direct_intent() {
    let key = test_keybinds().stance_cycle();
    let (app, ganger) = drive_one_act(Drive::Key(key));
    let via_key = app
        .world()
        .get_resource::<StanceProbe>()
        .map_or_else(Vec::new, |p| p.0.clone());
    assert_eq!(
        via_key.len(),
        1,
        "one SetStanceRequested via the stance key"
    );
    assert_eq!(via_key[0].actor, ganger, "actor = *SelectedShooter");
    assert_eq!(
        via_key[0].stance,
        next_stance(StanceKind::Standing),
        "the next-of-cycle stance",
    );

    let (app2, _) = drive_one_act(Drive::Intent(ActIntent::StanceCycle));
    let via_intent = app2
        .world()
        .get_resource::<StanceProbe>()
        .map_or_else(Vec::new, |p| p.0.clone());
    assert_eq!(via_intent.len(), 1, "one SetStanceRequested via the intent");
    assert_eq!(
        via_key[0], via_intent[0],
        "key and button must produce byte-for-byte equal SetStanceRequested",
    );
}

/// AC5 — the aim key emits one `SetAimingRequested` toggling the actor's aim, byte-for-
/// byte EQUAL to the direct `AimToggle` intent's message.
#[test]
fn aim_key_toggles_and_matches_direct_intent() {
    let key = test_keybinds().aim_toggle();
    let (app, ganger) = drive_one_act(Drive::Key(key));
    let via_key = app
        .world()
        .get_resource::<AimProbe>()
        .map_or_else(Vec::new, |p| p.0.clone());
    assert_eq!(via_key.len(), 1, "one SetAimingRequested via the aim key");
    assert_eq!(via_key[0].actor, ganger, "actor = *SelectedShooter");
    assert_eq!(
        via_key[0].aim,
        AimRequest::new(true),
        "aim toggles from the ganger's current false to true",
    );

    let (app2, _) = drive_one_act(Drive::Intent(ActIntent::AimToggle));
    let via_intent = app2
        .world()
        .get_resource::<AimProbe>()
        .map_or_else(Vec::new, |p| p.0.clone());
    assert_eq!(via_intent.len(), 1, "one SetAimingRequested via the intent");
    assert_eq!(
        via_key[0], via_intent[0],
        "key and button must produce byte-for-byte equal SetAimingRequested",
    );
}

/// AC5 — the facing key emits one `SetFacingRequested` for the next-of-cycle facing,
/// byte-for-byte EQUAL to the direct `FacingCycle` intent's message.
#[test]
fn facing_key_emits_next_of_cycle_and_matches_direct_intent() {
    let key = test_keybinds().facing_cycle();
    let (app, ganger) = drive_one_act(Drive::Key(key));
    let via_key = app
        .world()
        .get_resource::<FacingProbe>()
        .map_or_else(Vec::new, |p| p.0.clone());
    assert_eq!(
        via_key.len(),
        1,
        "one SetFacingRequested via the facing key"
    );
    assert_eq!(via_key[0].actor, ganger, "actor = *SelectedShooter");
    assert_eq!(
        via_key[0].facing,
        next_facing(Direction::North),
        "the next-of-cycle facing",
    );

    let (app2, _) = drive_one_act(Drive::Intent(ActIntent::FacingCycle));
    let via_intent = app2
        .world()
        .get_resource::<FacingProbe>()
        .map_or_else(Vec::new, |p| p.0.clone());
    assert_eq!(via_intent.len(), 1, "one SetFacingRequested via the intent");
    assert_eq!(
        via_key[0], via_intent[0],
        "key and button must produce byte-for-byte equal SetFacingRequested",
    );
}

/// GTW-275 AC4 — pushing `ActIntent::Reload` emits exactly one `ReloadRequested` for
/// the `*SelectedShooter` through the `gdtf_battle_input` seam (the weapon panel's Reload
/// button surrogate, over the SAME `dispatch_act_intents` drain the other intents use).
#[test]
fn reload_intent_emits_one_reload_requested_for_the_selection() {
    let (app, ganger) = drive_one_act(Drive::Intent(ActIntent::Reload));
    let reloads = app
        .world()
        .get_resource::<ReloadProbe>()
        .map_or_else(Vec::new, |p| p.0.clone());
    assert_eq!(
        reloads.len(),
        1,
        "one ReloadRequested via the reload intent",
    );
    assert_eq!(
        reloads[0].actor, ganger,
        "the ReloadRequested actor is the *SelectedShooter",
    );
}

// ---------------------------------------------------------------------------------
// GTW-294 — pushing ActIntent::Execute / ActIntent::Stabilize emits exactly one
// ExecuteDownedRequested / StabilizeDownedRequested for the SelectedShooter as the actor
// over the carried downed target; with the selection cleared, nothing is written.
// ---------------------------------------------------------------------------------

/// GTW-294 — pushing `ActIntent::Execute(target)` and `ActIntent::Stabilize(target)` with
/// a selected shooter emits EXACTLY one `ExecuteDownedRequested { actor, target }` and one
/// `StabilizeDownedRequested { actor, target }` through the `dispatch_act_intents` drain,
/// the actor being the `*SelectedShooter` and the target the carried downed entity (the
/// downed-target affordance surrogate, over the SAME seam the other intents use).
#[test]
fn downed_intents_emit_requests_for_selection_over_carried_target() {
    let mut app = acts_app();
    add_probes(&mut app);
    let actor = spawn_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, actor);
    // The downed target entity — only its identity matters at this seam (the sim's
    // faction/adjacency gate is the authoritative check, not this layer).
    let target = app.world_mut().spawn(ENEMY_FACTION).id();

    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::Execute(target));
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::Stabilize(target));
    app.update();

    let executes = app
        .world()
        .get_resource::<ExecuteProbe>()
        .map_or_else(Vec::new, |p| p.0.clone());
    assert_eq!(
        executes.len(),
        1,
        "one ExecuteDownedRequested via the execute intent",
    );
    assert_eq!(
        executes[0],
        ExecuteDownedRequested::new(actor, target),
        "ExecuteDownedRequested has actor = *SelectedShooter and target = carried",
    );

    let stabilizes = app
        .world()
        .get_resource::<StabilizeProbe>()
        .map_or_else(Vec::new, |p| p.0.clone());
    assert_eq!(
        stabilizes.len(),
        1,
        "one StabilizeDownedRequested via the stabilize intent",
    );
    assert_eq!(
        stabilizes[0],
        StabilizeDownedRequested::new(actor, target),
        "StabilizeDownedRequested has actor = *SelectedShooter and target = carried",
    );
}

/// GTW-294 — with the selection cleared (`SelectedShooter(None)`), pushing both downed
/// intents writes NOTHING (the drain resolves the actor from the selection and is a no-op
/// without one — the same fail-closed shape as the Reload arm).
#[test]
fn downed_intents_emit_nothing_without_selection() {
    let mut app = acts_app();
    add_probes(&mut app);
    // A downed target exists but there is NO selected actor.
    let target = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut().insert_resource(SelectedShooter::cleared());

    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::Execute(target));
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::Stabilize(target));
    app.update();

    assert!(
        app.world()
            .get_resource::<ExecuteProbe>()
            .is_none_or(|p| p.0.is_empty()),
        "no ExecuteDownedRequested without a selection",
    );
    assert!(
        app.world()
            .get_resource::<StabilizeProbe>()
            .is_none_or(|p| p.0.is_empty()),
        "no StabilizeDownedRequested without a selection",
    );
}

// ---------------------------------------------------------------------------------
// GTW-309 — pushing ActIntent::EndTurn emits exactly one (fieldless) EndTurnRequested
// through the same drain, with NO selection (a GLOBAL turn signal).
// ---------------------------------------------------------------------------------

/// GTW-309 — pushing the GLOBAL `ActIntent::EndTurn` emits EXACTLY one fieldless
/// `EndTurnRequested` through the `dispatch_act_intents` drain (the action-bar End-Turn
/// button surrogate). Unlike the per-ganger acts, end-turn needs NO `SelectedShooter`, so
/// this drives it with the selection cleared and still asserts exactly one message.
#[test]
fn end_turn_intent_emits_one_end_turn_requested_without_selection() {
    let mut app = acts_app();
    add_probes(&mut app);
    // No ganger spawned, selection cleared — end-turn is GLOBAL, not per-actor.
    app.world_mut().insert_resource(SelectedShooter::cleared());

    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::EndTurn);
    app.update();

    let ends = app
        .world()
        .get_resource::<EndTurnProbe>()
        .map_or_else(Vec::new, |p| p.0.clone());
    assert_eq!(
        ends.len(),
        1,
        "one EndTurnRequested via the end-turn intent, with no selection",
    );
    assert_eq!(
        ends[0], EndTurnRequested,
        "the drained message is the fieldless EndTurnRequested unit value",
    );
}

// ---------------------------------------------------------------------------------
// AC6 — with NO SelectedShooter, every act key/click is a no-op (zero messages).
// ---------------------------------------------------------------------------------

/// AC6 — with the selection cleared, driving ALL act keys (stance / aim / facing) + a
/// left-click emits ZERO messages of every `*Requested` type and does not panic.
#[test]
fn no_selection_makes_every_act_a_no_op() {
    let mut app = acts_app();
    add_probes(&mut app);
    let binds = test_keybinds();

    // An armed ganger EXISTS in the world but is NOT selected.
    let _ganger = spawn_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    app.world_mut().insert_resource(SelectedShooter::cleared());
    // Hover an in-bounds cell (via the real picker) so only the selection — not the
    // hover — gates the click.
    let hovered = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    assert!(
        app.world()
            .get_resource::<OccupancyGrid>()
            .and_then(|g| g.occupant(&hovered))
            .is_none(),
        "precondition: the hovered cell is empty (no occupant to select)",
    );

    press_key(&mut app, binds.stance_cycle());
    press_key(&mut app, binds.aim_toggle());
    press_key(&mut app, binds.facing_cycle());
    // The Reload act has no key binding — push its intent directly (the button surrogate)
    // to prove the drain is a no-op with no selection (GTW-275).
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::Reload);
    press_left(&mut app);
    app.update();

    assert!(
        fires(&app).is_empty(),
        "no FireRequested without a selection"
    );
    assert!(
        app.world()
            .get_resource::<StanceProbe>()
            .is_none_or(|p| p.0.is_empty()),
        "no SetStanceRequested without a selection",
    );
    assert!(
        app.world()
            .get_resource::<AimProbe>()
            .is_none_or(|p| p.0.is_empty()),
        "no SetAimingRequested without a selection",
    );
    assert!(
        app.world()
            .get_resource::<FacingProbe>()
            .is_none_or(|p| p.0.is_empty()),
        "no SetFacingRequested without a selection",
    );
    assert!(
        app.world()
            .get_resource::<ReloadProbe>()
            .is_none_or(|p| p.0.is_empty()),
        "no ReloadRequested without a selection",
    );
}

// =================================================================================
// GTW-356 — two-click move targeting + cross-storey UX, over the REAL cursor ->
// InspectTarget -> left_click_act -> dispatch_act_intents chain. AC1 (target then
// commit), AC4 (cross-storey via the level keys), AC5 (default = ActiveLevel), AC6
// (a vertical-link tile is NOT a move target).
// =================================================================================

/// Releases + clears the KEY edges so a later press is a fresh just-pressed (under
/// `MinimalPlugins` no `InputPlugin` clears the edges per frame).
fn clear_keys(app: &mut App) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release_all();
    keys.clear();
}

/// Presses Left + `update()`s once (resolving the click on the stable cursor cell), then
/// clears the mouse edge so the NEXT press is a fresh just-pressed — one left-click of the
/// two-click sequence. The cursor must already be set (via [`hover_at`] / a prior call).
fn click_left(app: &mut App) {
    press_left(app);
    app.update();
    clear_mouse(app);
}

/// The presenter [`ActiveLevel`]'s storey index as an `i32` (matching a `CellLevel`'s `z`),
/// or storey 0 if the resource is somehow absent.
fn active_storey(app: &App) -> i32 {
    app.world()
        .get_resource::<ActiveLevel>()
        .map_or(0, |l| i32::from(***l))
}

/// Switches the presenter [`ActiveLevel`] UP one storey via the REAL level key (`PageUp` ->
/// `ActIntent::LevelUp` -> `dispatch_act_intents` mutates `ActiveLevel`), returning the
/// resulting active storey index. Drives the genuine keyboard -> intent -> level-step path,
/// not an injected level (GTW-356 cross-storey uses the EXISTING level switch — no new
/// mechanism).
fn switch_level_up(app: &mut App) -> i32 {
    press_key(app, test_keybinds().level_up());
    app.update();
    clear_keys(app);
    active_storey(app)
}

/// AC1 — with a player ganger selected, a FIRST left-click on a VALID move target SETS
/// `PathPreviewTarget` and emits NO `MoveRequested`; a SECOND left-click on the SAME cell
/// commits exactly one `MoveRequested { shooter, target }`, over the REAL cursor ->
/// `InspectTarget` -> `left_click_act` two-click chain.
#[test]
fn two_click_sets_target_then_commits_move() {
    let mut app = acts_app();
    add_probes(&mut app);
    // A player ganger with NO firing components fails FIRE closed, so a click on an empty cell
    // is a MOVE (the `move_path_app` precedent). Select it over the real cursor chain.
    let ganger = spawn_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, ganger);

    // Hover a DISTINCT, empty, in-grid target cell (not the shooter's own cell).
    let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);

    // Click-1: SET the target, dispatch NOTHING.
    click_left(&mut app);
    assert!(
        moves(&app).is_empty(),
        "click-1 on a valid target must emit NO MoveRequested (it only sets the target)",
    );
    assert_eq!(
        move_target(&app),
        Some(target),
        "click-1 must SET PathPreviewTarget to the clicked cell",
    );

    // Click-2 on the SAME cell (cursor unmoved): COMMIT.
    click_left(&mut app);
    let emitted = moves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "click-2 on the SAME cell must emit exactly one MoveRequested (commit)",
    );
    assert_eq!(emitted[0].actor, ganger, "the move actor = the selection");
    assert_eq!(emitted[0].dest, target, "the move dest = the targeted cell");
    assert_eq!(
        move_target(&app),
        None,
        "committing the move must CLEAR PathPreviewTarget",
    );
}

/// AC5 — with NO level switch, the two-click move's `dest.z` is the DEFAULT `ActiveLevel`
/// (storey 0): the committed `MoveRequested.dest.level` equals the active level the cursor
/// resolves through `world_to_cell`.
#[test]
fn two_click_default_targets_active_level() {
    let mut app = acts_app();
    add_probes(&mut app);
    let ganger = spawn_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, ganger);

    let active = active_storey(&app);
    let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    assert_eq!(
        target.z, active,
        "with no switch the hovered cell resolves at the DEFAULT active storey",
    );

    click_left(&mut app); // click-1: set target.
    click_left(&mut app); // click-2: commit.
    let emitted = moves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "the same-cell second click commits a move"
    );
    assert_eq!(
        emitted[0].dest.z, active,
        "the default move dest.z is the active storey (no switch)",
    );
}

/// AC4 — after switching `ActiveLevel` UP one storey via the EXISTING level key, the two-click
/// move on a destination-storey tile yields a `MoveRequested` whose `dest.z` == the switched
/// storey (the cross-storey encoding already flows through `world_to_cell`'s `ActiveLevel`; no
/// new level-switch, no 3D message extension).
#[test]
fn two_click_after_level_switch_targets_switched_storey() {
    let mut app = acts_app();
    add_probes(&mut app);
    let ganger = spawn_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, ganger);

    // Switch UP one storey via the real PageUp key -> ActIntent::LevelUp -> ActiveLevel.
    let switched = switch_level_up(&mut app);
    assert_ne!(
        switched, 0,
        "the level key must raise ActiveLevel off storey 0",
    );

    // Now the cursor resolves the target cell at the SWITCHED storey.
    let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    assert_eq!(
        target.z, switched,
        "the hovered cell resolves at the switched active storey",
    );

    click_left(&mut app); // click-1: set target on the switched storey.
    click_left(&mut app); // click-2: commit.
    let emitted = moves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "the same-cell second click commits a move"
    );
    assert_eq!(
        emitted[0].dest.z, switched,
        "the committed move dest.z == the SWITCHED storey (cross-storey already flows)",
    );
}

/// AC6 — a left-click on a VERTICAL-LINK tile is NOT a targeting action: it emits NO
/// `MoveRequested` with the link tile as dest AND sets NO `PathPreviewTarget` (GTW-356 OQ-4).
/// Two clicks on the link tile confirm it never commits.
#[test]
fn click_on_a_link_tile_is_not_a_move_target() {
    let mut app = acts_app();
    add_probes(&mut app);
    let ganger = spawn_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, ganger);

    // Resolve the target cell, then author a vertical-link DEPARTING it (a stair up one storey),
    // so `VerticalLinkGraph::links_from(target)` reports it as a link tile. The active level is
    // storey 0 here (no switch), so the up endpoint is storey 1.
    let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    let up_storey = u8::try_from(target.z.saturating_add(1)).unwrap_or(1);
    let up = CellLevel::new(Cell::new(target.x, target.y), Level::new(up_storey));
    let link = VerticalLink::new(target, up, LinkKind::stair());
    let graph = SituationBuilder::new()
        .slab_at(target)
        .slab_at(up)
        .vertical_link(link)
        .build();
    if let Ok(graph) = build_vertical_link_graph(&graph) {
        app.world_mut().insert_resource(graph);
    }

    // Two clicks on the link tile: it is a no-op each time — no target, no move.
    click_left(&mut app);
    click_left(&mut app);
    assert!(
        moves(&app).is_empty(),
        "a click on a vertical-link tile must emit NO MoveRequested (OQ-4: not a move target)",
    );
    assert_eq!(
        move_target(&app),
        None,
        "a click on a vertical-link tile must set NO PathPreviewTarget (OQ-4: not a target)",
    );
}
