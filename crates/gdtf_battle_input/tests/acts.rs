//! GTW-227 (GTW-48 S8 / 222b): headless integration tests for the core player ACTS —
//! fire (left-click), posture (stance / aim / facing keys), and fire-mode selection —
//! over the REAL 222a act-intent seam (`GdtfBattleInputPlugin`'s keyboard / fire-click
//! systems -> the ONE `dispatch_act_intents` drain -> the emitted `*Requested`).
//!
//! - AC1 drives the REAL selection path (synth left-click on an armed occupant) and
//!   asserts `SelectedFireMode` defaults to that weapon's `FireMode::single()`.
//! - AC2 synthesizes the bound fire-mode-cycle key and asserts `SelectedFireMode`
//!   advances ONLY among the weapon's offered modes (a one-mode `[Single]` stays; a
//!   two-mode `[Single, Burst]` toggles; a three-mode `[Single, Burst, Full]` walks +
//!   wraps).
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
    ActIntent, BoundKey, GdtfBattleInputPlugin, HoveredCell, Keybinds, PendingActIntent,
    SelectedFireMode, SelectedShooter, next_facing, next_stance,
};
use gdtf_battle_presenter::{ActiveLevel, WorldCamera};
use gdtf_battle_sim::{
    Aiming, BattleInProgress, BattleSeed, Cell, CellLevel, CoverLedger, Direction, Facing, Faction,
    FireMode, FireModeSpec, Level, LifeState, Magazine, MagazineSize, ModeConeMult, ModeKind,
    ModeShots, ModeTuPercent, OccupancyGrid, PlayerFaction, SimRng, Stance, StanceKind,
    SurfaceGrid, Tu, TuMax,
    acts::{
        AimRequest, FireRequested, SetAimingRequested, SetFacingRequested, SetStanceRequested,
        SimActsPlugin,
    },
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
        select_clear:    BoundKey::KeyEscape,
        level_up:        BoundKey::KeyPageUp,
        level_down:      BoundKey::KeyPageDown,
        stance_cycle:    BoundKey::KeyC,
        aim_toggle:      BoundKey::KeyF,
        facing_cycle:    BoundKey::KeyR,
        fire_mode_cycle: BoundKey::KeyQ,
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
/// to an in-grid cell (driving the genuine cursor -> `HoveredCell` -> select/fire chain,
/// not an injected cell). The keybinds are inserted directly (the sanctioned headless
/// idiom — no `AssetServer` under `MinimalPlugins`).
fn acts_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin)
        .add_plugins(SimActsPlugin);
    let level = Level::new(0);
    app.world_mut().insert_resource(ActiveLevel(level));
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
    app.world_mut()
        .insert_resource(SimRng::from_seed(BattleSeed::new(0x5A1C_AC75)));
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
/// components the act systems query: `Faction` (GTW-238 SELECT/FIRE gating),
/// `Stance`/`Facing`/`Aiming`/`FireMode` (cycle + selection) and
/// `LifeState`/`Tu`/`TuMax`/`Magazine` (the `FireActor` `can_fire` reads).
fn spawn_ganger(
    app: &mut App,
    selector: FireMode,
    stance: StanceKind,
    facing: Direction,
) -> Entity {
    let size = MagazineSize::new(30);
    app.world_mut()
        .spawn((
            PLAYER_FACTION,
            Stance::new(stance),
            Facing::new(facing),
            Aiming::new(false),
            selector,
            LifeState::Alive,
            Tu::new(255),
            TuMax::new(100),
            Magazine::new(10, size),
        ))
        .id()
}

/// Places an ENEMY-faction occupant in the occupancy grid at `cell` (so the FIRE branch
/// of the unified left-click decision sees a non-player target there), returning its
/// entity. The enemy carries only a `Faction` — the fire path reads the SHOOTER's firing
/// components, never the target's.
fn place_enemy(app: &mut App, cell: CellLevel) -> Entity {
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(enemy));
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
        .get_resource::<HoveredCell>()
        .and_then(|h| **h)
        .unwrap_or_else(|| CellLevel::new(Cell::new(0, 0), Level::new(0)))
}

/// Sets the primary window cursor position (logical px), or clears it.
fn set_cursor(app: &mut App, position: Option<Vec2>) {
    let mut windows = app.world_mut().query::<&mut Window>();
    for mut window in windows.iter_mut(app.world_mut()) {
        window.set_cursor_position(position);
    }
}

/// Selects `ganger` over the REAL cursor -> `HoveredCell` -> `left_click_act` chain:
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

/// Releases `key` + clears the keyboard edges so a later `press(key)` is a fresh
/// just-pressed (a held key never re-fires `just_pressed` — `clear()` alone leaves it
/// held).
fn release_key(app: &mut App, key: KeyCode) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release(key);
    keys.clear();
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
/// Collected `SetStanceRequested` messages (probe).
#[derive(Resource, Default)]
struct StanceProbe(Vec<SetStanceRequested>);
/// Collected `SetAimingRequested` messages (probe).
#[derive(Resource, Default)]
struct AimProbe(Vec<SetAimingRequested>);
/// Collected `SetFacingRequested` messages (probe).
#[derive(Resource, Default)]
struct FacingProbe(Vec<SetFacingRequested>);

/// Adds the four message-collecting probe systems, each running AFTER the drain so it
/// observes the same update's emitted messages. The probes have their own
/// `MessageReader` cursors (independent of the sim's `dispatch_*`), so they read every
/// message the drain wrote.
fn add_probes(app: &mut App) {
    app.insert_resource(FireProbe::default())
        .insert_resource(StanceProbe::default())
        .insert_resource(AimProbe::default())
        .insert_resource(FacingProbe::default());
    app.add_systems(
        Update,
        (
            |mut r: MessageReader<FireRequested>, mut p: ResMut<FireProbe>| {
                // `FireRequested` is no longer `Copy` (it owns a `FireModeSpec`) — clone.
                p.0.extend(r.read().cloned());
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
// AC2 — fire-mode cycle advances ONLY among the selected weapon's offered modes.
// ---------------------------------------------------------------------------------

/// AC2 — a three-mode `[Single, Burst, Full]` weapon walks `single` -> `burst` ->
/// `full` -> (wrap) `single` across repeated fire-mode-cycle keypresses; a one-mode
/// `[Single]` weapon stays on `single`; a two-mode `[Single, Burst]` weapon toggles
/// `single` <-> `burst` — all via the REAL key path.
#[test]
fn fire_mode_cycle_walks_only_offered_modes() {
    let binds = test_keybinds();

    // [Single, Burst, Full]: single -> burst -> full -> single.
    {
        let mut app = acts_app();
        // Build the three modes locally so we can assert against them (the specs are
        // `Copy` again). The kind discriminates each mode.
        let single = spec(ModeKind::Single, 0.2, 1);
        let burst = spec(ModeKind::Burst, 0.4, 3);
        let full = spec(ModeKind::Full, 0.7, 6);
        let selector = FireMode::new(vec![single, burst, full]);
        let ganger = spawn_ganger(&mut app, selector, StanceKind::Standing, Direction::North);
        select_ganger(&mut app, ganger);
        assert_eq!(fire_mode(&app), Some(single), "starts on single");

        let cycle = binds.fire_mode_cycle();
        for expected in [burst, full, single, burst] {
            press_key(&mut app, cycle);
            app.update();
            release_key(&mut app, cycle);
            assert_eq!(
                fire_mode(&app),
                Some(expected),
                "fire-mode cycle must walk single->burst->full->single",
            );
        }
    }

    // [Single]: stays on single no matter how many times the cycle key is pressed.
    {
        let mut app = acts_app();
        let single = spec(ModeKind::Single, 0.2, 1);
        let selector = FireMode::new(vec![single]);
        let ganger = spawn_ganger(&mut app, selector, StanceKind::Standing, Direction::North);
        select_ganger(&mut app, ganger);
        let cycle = binds.fire_mode_cycle();
        for _ in 0..4 {
            press_key(&mut app, cycle);
            app.update();
            release_key(&mut app, cycle);
            assert_eq!(
                fire_mode(&app),
                Some(single),
                "a one-mode weapon must stay on single",
            );
        }
    }

    // [Single, Burst]: toggles single <-> burst.
    {
        let mut app = acts_app();
        let single = spec(ModeKind::Single, 0.2, 1);
        let burst = spec(ModeKind::Burst, 0.4, 3);
        let selector = FireMode::new(vec![single, burst]);
        let ganger = spawn_ganger(&mut app, selector, StanceKind::Standing, Direction::North);
        select_ganger(&mut app, ganger);
        let cycle = binds.fire_mode_cycle();
        for expected in [burst, single, burst] {
            press_key(&mut app, cycle);
            app.update();
            release_key(&mut app, cycle);
            assert_eq!(
                fire_mode(&app),
                Some(expected),
                "a two-mode weapon must toggle single<->burst",
            );
        }
    }
}

// ---------------------------------------------------------------------------------
// AC3 — a left-click on an in-bounds target emits exactly one FireRequested with the
// expected fields, when can_fire passes.
// ---------------------------------------------------------------------------------

/// AC3 — a left-click on an ENEMY target cell emits EXACTLY one `FireRequested { shooter
/// = *SelectedShooter, mode = *SelectedFireMode, target from HoveredCell }`, driven over
/// the REAL cursor -> `HoveredCell` -> `left_click_act` FIRE-branch chain.
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
    assert_eq!(msg.target_cell, target_cell, "target cell from HoveredCell");
    assert_eq!(
        msg.target_level, target_level,
        "target level from HoveredCell"
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
        app.world_mut()
            .entity_mut(ganger)
            .insert(Magazine::new(0, MagazineSize::new(30)));
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

    // No in-bounds target — an off-window cursor resolves `HoveredCell(None)`, so the
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
            app.world().get_resource::<HoveredCell>().and_then(|h| **h),
            None,
            "an off-window cursor must resolve HoveredCell to None",
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

/// AC5 — the stance key emits one `SetStanceRequested` for `*SelectedShooter` with the
/// next-of-cycle stance, byte-for-byte EQUAL to the message the direct `StanceCycle`
/// intent (the 222c-button surrogate) produces over the SAME seam.
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

// ---------------------------------------------------------------------------------
// AC6 — with NO SelectedShooter, every act key/click is a no-op (zero messages).
// ---------------------------------------------------------------------------------

/// AC6 — with the selection cleared, driving ALL act keys (stance / aim / facing /
/// fire-mode cycle) + a left-click emits ZERO messages of every `*Requested` type and
/// does not panic.
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
    press_key(&mut app, binds.fire_mode_cycle());
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
}
