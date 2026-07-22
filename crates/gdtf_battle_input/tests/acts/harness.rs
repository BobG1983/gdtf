//! Shared acts fixture: the headless acts app, the keybinds / camera / window
//! fixtures, the armed-ganger spawner, the cursor / selection drivers, and the
//! `*Requested` message probes.

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
use gdtf_battle_input::{BoundKey, GdtfBattleInputPlugin, InspectTarget, Keybinds};
use gdtf_battle_presenter::{ActiveLevel, ViewMode, WorldCamera};
use gdtf_battle_sim::{
    acts::{
        EndTurnRequested, EnterEmplacementRequested, ExecuteDownedRequested,
        ExitEmplacementRequested, FireRequested, MoveRequested, OpenDoorRequested, ReloadRequested,
        SetAimingRequested, SetFacingRequested, SetStanceRequested, SimActsPlugin,
        StabilizeDownedRequested,
    },
    magazine::{LoadedRounds, Magazine, ReloadTu},
    prelude::{
        BattleInProgress, Cell, CellLevel, Direction, Faction, Level, LifeState, OccupancyGrid,
        StanceKind,
    },
    rng::BattleSeed,
    test_support::{GangerEntityBuilder, TEST_SEED, fight_rng, insert_sim_resources, wield},
    weapon::{
        FireMode, FireModeSpec, Handedness, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent,
    },
};
use gdtf_test_utils::{MessageProbePlugin, clear_mouse, press_left, probed};

/// The faction the player controls in these tests (matches `PlayerFaction`).
pub(crate) const PLAYER_FACTION: Faction = Faction::new(0);
/// An ENEMY faction (distinct from [`PLAYER_FACTION`]) — a non-player target the FIRE
/// branch of the unified left-click decision (GTW-238) fires on.
pub(crate) const ENEMY_FACTION: Faction = Faction::new(1);

/// The synthetic window/camera render-target size (physical px), large enough that
/// distinct cursor offsets land on distinct in-grid cells (the GTW-221 `picking.rs`
/// harness size).
pub(crate) const TARGET_SIZE: Vec2 = Vec2::new(1280.0, 720.0);

// ---------------------------------------------------------------------------------
// Fixtures.
// ---------------------------------------------------------------------------------

/// A fully-bound [`Keybinds`] table for the act-key tests — built in the test body
/// (not parsed from the editable shipped `.ron`).
pub(crate) const fn test_keybinds() -> Keybinds {
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
pub(crate) const fn spec(kind: ModeKind, tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        kind,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// A three-mode `[Single, Burst, Full]` selector with three distinct modes.
pub(crate) fn sbf_selector() -> FireMode {
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
pub(crate) const SHOOTER_CURSOR_OFFSET: Vec2 = Vec2::new(40.0, 32.0);
/// A SECOND, distinct cursor offset resolving to a DIFFERENT in-grid cell — the FIRE
/// target (so the fire click targets a cell other than the shooter's own).
pub(crate) const TARGET_CURSOR_OFFSET: Vec2 = Vec2::new(200.0, 160.0);

/// Builds a deterministic [`Camera`] whose `viewport_to_world_2d` succeeds WITHOUT a
/// render pipeline (the GTW-221 `picking.rs` synthetic-camera recipe).
pub(crate) fn synthetic_camera() -> Camera {
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
/// not an injected cell). The keybinds are inserted directly (the headless
/// idiom — no `AssetServer` under `MinimalPlugins`).
pub(crate) fn acts_app() -> App {
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
pub(crate) fn armed_ganger(
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
            Magazine::new(
                LoadedRounds::new(10),
                MagazineSize::new(30),
                ReloadTu::new(12),
            ),
            // GTW-443: the wielded-weapon entity carries Handedness — the fire surface's
            // WeaponMagazine query reads `(&Magazine, &Handedness)`, so without it the query
            // would not match and the click would fire nothing.
            Handedness::OneHanded,
        ),
    );
    ganger
}

/// Sets the primary window cursor to the given window-centre offset and resolves it via
/// one `update()` (driving the REAL `pick_hovered_cell`), returning the resolved hovered
/// cell. The cursor stays at this position until moved, so a later action `update()`
/// re-resolves the SAME cell — making select/fire vs `pick_hovered_cell` ordering moot
/// (the value is identical before and after pick).
pub(crate) fn hover_at(app: &mut App, offset: Vec2) -> CellLevel {
    set_cursor(app, Some(TARGET_SIZE * 0.5 + offset));
    app.update();
    app.world()
        .get_resource::<InspectTarget>()
        .and_then(InspectTarget::hovered)
        .unwrap_or_else(|| CellLevel::new(Cell::new(0, 0), Level::new(0)))
}

/// Sets the primary window cursor position (logical px), or clears it.
pub(crate) fn set_cursor(app: &mut App, position: Option<Vec2>) {
    let mut windows = app.world_mut().query::<&mut Window>();
    for mut window in windows.iter_mut(app.world_mut()) {
        window.set_cursor_position(position);
    }
}

/// Selects `ganger` over the REAL cursor -> `InspectTarget` -> `left_click_act` chain:
/// places the cursor over the shooter cell, places `ganger` there in the occupancy grid,
/// then a fresh left-click + `update()` selects it. The cursor stays over the shooter
/// cell (so the value is stable). Returns the resolved shooter cell.
pub(crate) fn select_ganger(app: &mut App, ganger: Entity) -> CellLevel {
    let shooter_cell = hover_at(app, SHOOTER_CURSOR_OFFSET);
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(shooter_cell, Some(ganger));
    press_left(app);
    app.update();
    clear_mouse(app);
    shooter_cell
}

// ---------------------------------------------------------------------------------
// Message probes — collect the messages emitted this run into resources read in the
// test body (each probe runs after the drain, so it sees the same update's emission).
// ---------------------------------------------------------------------------------

/// Adds the generic message probes (GTW-576 `MessageProbePlugin<M>`) for every act
/// `*Requested` the tests assert on. The drain runs in `Last`, so it observes the same
/// update's emissions regardless of where in `Update` the drain/dispatch wrote them; each
/// probe has its own `MessageReader` cursor, independent of the sim's `dispatch_*`.
pub(crate) fn add_probes(app: &mut App) {
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
pub(crate) fn fires(app: &App) -> Vec<FireRequested> {
    probed::<FireRequested>(app)
}

/// The presenter [`ActiveLevel`]'s storey index as an `i32` (matching a `CellLevel`'s `z`),
/// or storey 0 if the resource is somehow absent.
pub(crate) fn active_storey(app: &App) -> i32 {
    app.world()
        .get_resource::<ActiveLevel>()
        .map_or(0, |l| i32::from(***l))
}
