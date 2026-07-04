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
    PendingActIntent, SelectedFireMode, SelectedShooter,
    contextual::{
        EnterEmplacementAct, ExecuteAct, ExitEmplacementAct, OpenDoorAct, PendingContextualIntents,
        StabilizeAct,
    },
    next_facing, next_stance,
};
use gdtf_battle_presenter::{ActiveLevel, ViewMode, WorldCamera};
use gdtf_battle_sim::{
    Aiming, BattleInProgress, BattleSeed, Cell, CellLevel, Direction, Faction, FireMode,
    FireModeSpec, Handedness, Level, LifeState, LinkKind, Magazine, MagazineSize, ModeConeMult,
    ModeKind, ModeShots, ModeTuPercent, OccupancyGrid, ReloadTu, SquadVisibility, StanceKind, Tu,
    TuMax, VerticalLink,
    acts::{
        AimRequest, EndTurnRequested, EnterEmplacementRequested, ExecuteDownedRequested,
        ExitEmplacementRequested, FireRequested, MoveRequested, OpenDoorRequested, ReloadRequested,
        SetAimingRequested, SetFacingRequested, SetStanceRequested, SimActsPlugin,
        StabilizeDownedRequested,
    },
    build_vertical_link_graph,
    test_support::{
        GangerEntityBuilder, SituationBuilder, TEST_SEED, fight_rng, insert_sim_resources, wield,
    },
    tuning::CombatTuning,
};
use gdtf_test_utils::{MessageProbePlugin, clear_keys, clear_mouse, press_key, press_left, probed};

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
        select_clear:     BoundKey::KeyEscape,
        level_up:         BoundKey::KeyPageUp,
        level_down:       BoundKey::KeyPageDown,
        // GTW-521 — the full-view toggle key.
        toggle_full_view: BoundKey::KeyV,
        stance_cycle:     BoundKey::KeyC,
        aim_toggle:       BoundKey::KeyF,
        facing_cycle:     BoundKey::KeyR,
        // GTW-458 — the Tab/Shift+Tab Prev/Next cycle chord (both bind to Tab).
        select_next:      BoundKey::KeyTab,
        select_prev:      BoundKey::KeyTab,
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
    // The canonical sim-resource litany (GTW-576) `SimActsPlugin`'s dispatch systems
    // read (so they pass param validation): the grids/ledgers empty, an empty fog + link
    // graph, the five seeded RNG streams, empty injury content, `CombatTuning::default`,
    // a uniform `FloorCostGrid`, and `PlayerFaction` on gang 0 (== PLAYER_FACTION — the
    // GTW-238 unified left-click decision gates on it). An unarmed shooter (no `Weapon`
    // marker) makes `fire()` a no-op, so none of this affects the asserted `*Requested`
    // MESSAGE — the buffers/dispatch must merely validate. COMPOSED from
    // `gdtf_battle_sim::test_support` instead of mirroring the litany line-by-line.
    insert_sim_resources(&mut app, BattleSeed::new(TEST_SEED));
    let level = Level::new(0);
    app.world_mut().insert_resource(ActiveLevel::new(level));
    // GTW-521 — `dispatch_act_intents` also mutates the presenter-owned `ViewMode`.
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(BattleInProgress);
    // GTW-507: the melee opposed-Fight stream `dispatch_melee` (in SimActsPlugin's Simulate
    // band) takes as `ResMut<FightRng>` — NOT part of the five-stream litany; insert it so
    // the system's param validation passes in this BattleInProgress-gated harness.
    app.world_mut().insert_resource(fight_rng(TEST_SEED));
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
fn armed_ganger(
    app: &mut App,
    selector: FireMode,
    stance: StanceKind,
    facing: Direction,
) -> Entity {
    let ganger = GangerEntityBuilder::new()
        .faction(PLAYER_FACTION)
        .stance(stance)
        .facing(facing)
        .aiming(false)
        .life_state(LifeState::Alive)
        .tu(255)
        .tu_max(100)
        .spawn(app.world_mut());
    wield(
        app.world_mut(),
        ganger,
        (
            selector,
            Magazine::new(10, MagazineSize::new(30), ReloadTu::new(12)),
            // GTW-443: the wielded-weapon entity carries Handedness — the fire surface's
            // WeaponMagazine query reads `(&Magazine, &Handedness)`, so without it the query
            // would not match and the click would fire nothing.
            Handedness::OneHanded,
        ),
    );
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

/// The current `SelectedFireMode`.
fn fire_mode(app: &App) -> Option<FireModeSpec> {
    app.world().get_resource::<SelectedFireMode>().map(|m| **m)
}

// ---------------------------------------------------------------------------------
// Message probes — collect the messages emitted this run into resources read in the
// test body (each probe runs after the drain, so it sees the same update's emission).
// ---------------------------------------------------------------------------------

/// Adds the generic message probes (GTW-576 `MessageProbePlugin<M>`) for every act
/// `*Requested` the tests assert on. The drain runs in `Last`, so it observes the same
/// update's emissions regardless of where in `Update` the drain/dispatch wrote them; each
/// probe has its own `MessageReader` cursor, independent of the sim's `dispatch_*`.
fn add_probes(app: &mut App) {
    app.add_plugins((
        MessageProbePlugin::<FireRequested>::default(),
        MessageProbePlugin::<MoveRequested>::default(),
        MessageProbePlugin::<SetStanceRequested>::default(),
        MessageProbePlugin::<SetAimingRequested>::default(),
        MessageProbePlugin::<SetFacingRequested>::default(),
        MessageProbePlugin::<ReloadRequested>::default(),
        MessageProbePlugin::<EndTurnRequested>::default(),
        MessageProbePlugin::<ExecuteDownedRequested>::default(),
        MessageProbePlugin::<StabilizeDownedRequested>::default(),
        MessageProbePlugin::<OpenDoorRequested>::default(),
        MessageProbePlugin::<EnterEmplacementRequested>::default(),
        MessageProbePlugin::<ExitEmplacementRequested>::default(),
    ));
}

/// The collected `FireRequested` messages.
fn fires(app: &App) -> Vec<FireRequested> {
    probed::<FireRequested>(app)
}

/// The collected `MoveRequested` messages (GTW-356 two-click move target).
fn moves(app: &App) -> Vec<MoveRequested> {
    probed::<MoveRequested>(app)
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
    let ganger = armed_ganger(
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
    let ganger = armed_ganger(
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
    let target_cell = target.cell();
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
        let ganger = armed_ganger(
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
        let ganger = armed_ganger(
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
        let ganger = armed_ganger(
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
        let ganger = armed_ganger(
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
    let ganger = armed_ganger(
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
    let via_key = probed::<SetStanceRequested>(&app);
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
    let via_intent = probed::<SetStanceRequested>(&app2);
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
    let via_key = probed::<SetAimingRequested>(&app);
    assert_eq!(via_key.len(), 1, "one SetAimingRequested via the aim key");
    assert_eq!(via_key[0].actor, ganger, "actor = *SelectedShooter");
    assert_eq!(
        via_key[0].aim,
        AimRequest::new(true),
        "aim toggles from the ganger's current false to true",
    );

    let (app2, _) = drive_one_act(Drive::Intent(ActIntent::AimToggle));
    let via_intent = probed::<SetAimingRequested>(&app2);
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
    let via_key = probed::<SetFacingRequested>(&app);
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
    let via_intent = probed::<SetFacingRequested>(&app2);
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
    let reloads = probed::<ReloadRequested>(&app);
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
// GTW-294 / GTW-571 — pushing a target onto the per-act contextual queues
// (PendingContextualIntents<ExecuteAct> / <StabilizeAct>) emits exactly one
// ExecuteDownedRequested / StabilizeDownedRequested for the SelectedShooter as the actor
// over the carried downed target; with the selection cleared, nothing is written.
// ---------------------------------------------------------------------------------

/// GTW-294 / GTW-571 — pushing `target` onto `PendingContextualIntents<ExecuteAct>` and
/// `<StabilizeAct>` with a selected shooter emits EXACTLY one
/// `ExecuteDownedRequested { actor, target }` and one
/// `StabilizeDownedRequested { actor, target }` through the per-act generic
/// `drain_contextual_intents` drains, the actor being the `*SelectedShooter` and the
/// target the carried downed entity (the downed-target affordance surrogate, over the
/// GTW-571 per-act contextual seam).
#[test]
fn downed_intents_emit_requests_for_selection_over_carried_target() {
    let mut app = acts_app();
    add_probes(&mut app);
    let actor = armed_ganger(
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
        .resource_mut::<PendingContextualIntents<ExecuteAct>>()
        .push(target);
    app.world_mut()
        .resource_mut::<PendingContextualIntents<StabilizeAct>>()
        .push(target);
    app.update();

    let executes = probed::<ExecuteDownedRequested>(&app);
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

    let stabilizes = probed::<StabilizeDownedRequested>(&app);
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
        .resource_mut::<PendingContextualIntents<ExecuteAct>>()
        .push(target);
    app.world_mut()
        .resource_mut::<PendingContextualIntents<StabilizeAct>>()
        .push(target);
    app.update();

    assert!(
        probed::<ExecuteDownedRequested>(&app).is_empty(),
        "no ExecuteDownedRequested without a selection",
    );
    assert!(
        probed::<StabilizeDownedRequested>(&app).is_empty(),
        "no StabilizeDownedRequested without a selection",
    );
}

// ---------------------------------------------------------------------------------
// GTW-315 / GTW-571 — pushing a door onto PendingContextualIntents<OpenDoorAct> emits
// exactly one OpenDoorRequested for the SelectedShooter as the actor over the carried
// door; with no selection, nothing is written (the drain resolves the actor from the
// selection).
// ---------------------------------------------------------------------------------

/// GTW-315 / GTW-571 — pushing `door` onto `PendingContextualIntents<OpenDoorAct>` with a
/// selected player actor emits EXACTLY one `OpenDoorRequested { actor, door }` through the
/// act's generic `drain_contextual_intents` drain, the actor being the `*SelectedShooter`
/// and the door the carried openable entity (the Open-Door affordance surrogate, over the
/// GTW-571 per-act contextual seam). The sim's `dispatch_open_door` gate (CLOSED +
/// 8-adjacent + affords `OpenDoorTu`) is the authoritative check, not this seam.
#[test]
fn open_door_intent_emits_request_for_selection_over_carried_door() {
    let mut app = acts_app();
    add_probes(&mut app);
    let actor = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, actor);
    // The door target entity — only its identity matters at this seam (the sim's
    // OpenState/adjacency/TU gate is the authoritative check, not this layer).
    let door = app.world_mut().spawn_empty().id();

    app.world_mut()
        .resource_mut::<PendingContextualIntents<OpenDoorAct>>()
        .push(door);
    app.update();

    let opens = probed::<OpenDoorRequested>(&app);
    assert_eq!(
        opens.len(),
        1,
        "one OpenDoorRequested via the open-door intent",
    );
    assert_eq!(
        opens[0],
        OpenDoorRequested::new(actor, door),
        "OpenDoorRequested has actor = *SelectedShooter and door = carried",
    );
}

/// GTW-315 — with the selection cleared (`SelectedShooter(None)`), pushing the open-door
/// intent writes NOTHING (the drain resolves the actor from the selection and is a no-op
/// without one — the same fail-closed shape as the Execute / Reload arms).
#[test]
fn open_door_intent_emits_nothing_without_selection() {
    let mut app = acts_app();
    add_probes(&mut app);
    // A door target exists but there is NO selected actor.
    let door = app.world_mut().spawn_empty().id();
    app.world_mut().insert_resource(SelectedShooter::cleared());

    app.world_mut()
        .resource_mut::<PendingContextualIntents<OpenDoorAct>>()
        .push(door);
    app.update();

    assert!(
        probed::<OpenDoorRequested>(&app).is_empty(),
        "no OpenDoorRequested without a selection",
    );
}

// ---------------------------------------------------------------------------------
// GTW-543 / GTW-571 — pushing an emplacement onto the per-act contextual queues
// (PendingContextualIntents<EnterEmplacementAct> / <ExitEmplacementAct>) emits exactly
// one EnterEmplacementRequested / ExitEmplacementRequested for the SelectedShooter as
// the actor over the carried emplacement; with no selection nothing is written (the
// drain resolves the actor from the selection).
// ---------------------------------------------------------------------------------

/// GTW-543 / GTW-571 — pushing `emplacement` onto
/// `PendingContextualIntents<EnterEmplacementAct>` with a selected player actor emits
/// EXACTLY one `EnterEmplacementRequested { actor, emplacement }` through the act's generic
/// `drain_contextual_intents` drain, the actor being the `*SelectedShooter` and the
/// emplacement the carried terrain entity (the Enter affordance surrogate, over the GTW-571
/// per-act contextual seam). The sim's `dispatch_enter_emplacement` gate (VACANT +
/// 8-adjacent + affords `EnterEmplacementTu`) is the authoritative check, not this seam.
#[test]
fn enter_emplacement_intent_emits_request_for_selection_over_carried_emplacement() {
    let mut app = acts_app();
    add_probes(&mut app);
    let actor = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, actor);
    // The emplacement target entity — only its identity matters at this seam (the sim's
    // EmplacementState/adjacency/TU gate is the authoritative check, not this layer).
    let emplacement = app.world_mut().spawn_empty().id();

    app.world_mut()
        .resource_mut::<PendingContextualIntents<EnterEmplacementAct>>()
        .push(emplacement);
    app.update();

    let enters = probed::<EnterEmplacementRequested>(&app);
    assert_eq!(
        enters.len(),
        1,
        "one EnterEmplacementRequested via the enter-emplacement intent",
    );
    assert_eq!(
        enters[0],
        EnterEmplacementRequested::new(actor, emplacement),
        "EnterEmplacementRequested has actor = *SelectedShooter and emplacement = carried",
    );
}

/// GTW-543 — with the selection cleared (`SelectedShooter(None)`), pushing the enter-emplacement
/// intent writes NOTHING (the drain resolves the actor from the selection and is a no-op without
/// one — the same fail-closed shape as the `OpenDoor` / `Execute` arms).
#[test]
fn enter_emplacement_intent_emits_nothing_without_selection() {
    let mut app = acts_app();
    add_probes(&mut app);
    // An emplacement target exists but there is NO selected actor.
    let emplacement = app.world_mut().spawn_empty().id();
    app.world_mut().insert_resource(SelectedShooter::cleared());

    app.world_mut()
        .resource_mut::<PendingContextualIntents<EnterEmplacementAct>>()
        .push(emplacement);
    app.update();

    assert!(
        probed::<EnterEmplacementRequested>(&app).is_empty(),
        "no EnterEmplacementRequested without a selection",
    );
}

/// GTW-543 / GTW-571 — pushing `emplacement` onto
/// `PendingContextualIntents<ExitEmplacementAct>` with a selected player actor emits EXACTLY
/// one `ExitEmplacementRequested { actor, emplacement }` through the act's generic
/// `drain_contextual_intents` drain, the actor being the `*SelectedShooter` and the
/// emplacement the carried terrain entity (the Exit affordance surrogate, over the GTW-571
/// per-act contextual seam). The sim's `dispatch_exit_emplacement` gate (the recorded
/// occupant IS the actor + affords `ExitEmplacementTu`) is the authoritative check, not this
/// seam.
#[test]
fn exit_emplacement_intent_emits_request_for_selection_over_carried_emplacement() {
    let mut app = acts_app();
    add_probes(&mut app);
    let actor = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, actor);
    let emplacement = app.world_mut().spawn_empty().id();

    app.world_mut()
        .resource_mut::<PendingContextualIntents<ExitEmplacementAct>>()
        .push(emplacement);
    app.update();

    let exits = probed::<ExitEmplacementRequested>(&app);
    assert_eq!(
        exits.len(),
        1,
        "one ExitEmplacementRequested via the exit-emplacement intent",
    );
    assert_eq!(
        exits[0],
        ExitEmplacementRequested::new(actor, emplacement),
        "ExitEmplacementRequested has actor = *SelectedShooter and emplacement = carried",
    );
}

/// GTW-543 — with the selection cleared, pushing the exit-emplacement intent writes NOTHING (the
/// drain resolves the actor from the selection and is a no-op without one).
#[test]
fn exit_emplacement_intent_emits_nothing_without_selection() {
    let mut app = acts_app();
    add_probes(&mut app);
    let emplacement = app.world_mut().spawn_empty().id();
    app.world_mut().insert_resource(SelectedShooter::cleared());

    app.world_mut()
        .resource_mut::<PendingContextualIntents<ExitEmplacementAct>>()
        .push(emplacement);
    app.update();

    assert!(
        probed::<ExitEmplacementRequested>(&app).is_empty(),
        "no ExitEmplacementRequested without a selection",
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

    let ends = probed::<EndTurnRequested>(&app);
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
    let _ganger = armed_ganger(
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
        probed::<SetStanceRequested>(&app).is_empty(),
        "no SetStanceRequested without a selection",
    );
    assert!(
        probed::<SetAimingRequested>(&app).is_empty(),
        "no SetAimingRequested without a selection",
    );
    assert!(
        probed::<SetFacingRequested>(&app).is_empty(),
        "no SetFacingRequested without a selection",
    );
    assert!(
        probed::<ReloadRequested>(&app).is_empty(),
        "no ReloadRequested without a selection",
    );
}

// =================================================================================
// GTW-356 — two-click move targeting + cross-storey UX, over the REAL cursor ->
// InspectTarget -> left_click_act -> dispatch_act_intents chain. AC1 (target then
// commit), AC4 (cross-storey via the level keys), AC5 (default = ActiveLevel), AC6
// (a vertical-link tile is NOT a move target).
// =================================================================================

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
    let ganger = armed_ganger(
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
    let ganger = armed_ganger(
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
    let ganger = armed_ganger(
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
    let ganger = armed_ganger(
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
    let up_storey = (*target.level()).saturating_add(1);
    let up = CellLevel::new(target.cell(), Level::new(up_storey));
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

// =================================================================================
// GTW-521 — the full-view toggle input seam: the ViewMode flip through the ONE
// `dispatch_act_intents` drain, and the bound key that pushes `ActIntent::ToggleFullView`.
// =================================================================================

/// The presenter [`ViewMode`], or [`ViewMode::DownToActive`] if the resource is somehow
/// absent (the harness always inserts it — this keeps the read panic-free).
fn view_mode(app: &App) -> ViewMode {
    app.world()
        .get_resource::<ViewMode>()
        .copied()
        .unwrap_or(ViewMode::DownToActive)
}

/// GTW-521 C4 — pushing [`ActIntent::ToggleFullView`] and running ONE update FLIPS the
/// presenter-owned [`ViewMode`] through the REAL `dispatch_act_intents` drain, and a second
/// toggle flips it back (the round-trip). It does NOT touch [`ActiveLevel`] (C5).
///
/// Drives the genuine intent -> drain -> presenter-resource path (the shared act-intent seam,
/// the same one the level keys / buttons use), not a direct resource write — so it proves the
/// drain arm is wired end-to-end.
#[test]
fn toggle_full_view_intent_flips_view_mode_and_round_trips() {
    let mut app = acts_app();

    // The harness seeds the default (DownToActive) and the ground floor.
    assert_eq!(
        view_mode(&app),
        ViewMode::DownToActive,
        "the harness seeds the default ViewMode (DownToActive)",
    );
    let active_before = active_storey(&app);

    // Push the toggle intent and drain it: DownToActive -> FullView.
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::ToggleFullView);
    app.update();
    assert_eq!(
        view_mode(&app),
        ViewMode::FullView,
        "one ToggleFullView drain must flip ViewMode DownToActive -> FullView",
    );
    // C5 — the toggle must NOT move the active storey.
    assert_eq!(
        active_storey(&app),
        active_before,
        "toggling FullView must NOT change ActiveLevel (C5)",
    );

    // Push it again: FullView -> DownToActive (the round-trip).
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::ToggleFullView);
    app.update();
    assert_eq!(
        view_mode(&app),
        ViewMode::DownToActive,
        "a second ToggleFullView drain must flip ViewMode back FullView -> DownToActive",
    );
    assert_eq!(
        active_storey(&app),
        active_before,
        "the round-trip toggle still must NOT change ActiveLevel (C5)",
    );
}

/// GTW-521 C4 — a `just_pressed` of the BOUND full-view key (`toggle_full_view`) PUSHES
/// exactly one [`ActIntent::ToggleFullView`] onto the shared [`PendingActIntent`] seam
/// (the keyboard-reader precedent — the level keys' `press_key` -> intent test).
///
/// Reads the bound key off [`Keybinds`] (NO `KeyCode` literal), presses it, and — because
/// the drain runs the SAME update — asserts the resulting `ViewMode` flip (the reader pushed
/// the intent, the drain consumed it). Then clears the key so no phantom re-press lingers.
#[test]
fn bound_full_view_key_press_pushes_toggle_full_view() {
    let mut app = acts_app();
    assert_eq!(
        view_mode(&app),
        ViewMode::DownToActive,
        "the harness starts in DownToActive",
    );

    // Press the DATA-DRIVEN bound key (read off Keybinds — no literal). The keyboard band
    // (`full_view_key`) pushes ToggleFullView, and `dispatch_act_intents` (ordered `.after`
    // it) drains it the same update, flipping the ViewMode.
    press_key(&mut app, test_keybinds().toggle_full_view());
    app.update();
    clear_keys(&mut app);
    assert_eq!(
        view_mode(&app),
        ViewMode::FullView,
        "pressing the bound full-view key must push ToggleFullView (drained same update -> \
         FullView)",
    );

    // A second press of the same bound key round-trips back to DownToActive — proving the key
    // pushes the toggle each press (not a one-shot latch).
    press_key(&mut app, test_keybinds().toggle_full_view());
    app.update();
    clear_keys(&mut app);
    assert_eq!(
        view_mode(&app),
        ViewMode::DownToActive,
        "a second bound-key press must push ToggleFullView again (drained -> DownToActive)",
    );
}
