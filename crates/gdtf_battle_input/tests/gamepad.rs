//! GTW-259: tests for the gamepad software cursor — the SHARED decision (AC2), the picker
//! arbitration (AC3), the mouse-reclaims-pointer logic (AC4), and the gamepad acts reusing
//! the intent seam (AC6, by construction + the shared decision).
//!
//! HONESTY (the contract / `verification.md`): real `Gamepad` stick / button STATE is
//! device-event-driven in Bevy 0.18 and cannot be cleanly driven headlessly (the same
//! limitation GTW-250 documents). So the raw stick / button READS (`move_gamepad_cursor`,
//! `gamepad_click_act`, `gamepad_turn`) are covered by the pure `move_cursor` helper (its
//! unit tests live in `gamepad.rs`) + the SHARED decision exercised here via the mouse /
//! `SystemState` path + in-engine QA. These headless tests prove the SETTABLE-resource /
//! message logic: the picker honors `ActivePointer`, the mouse reclaims the pointer, and the
//! shared `decide_left_click` / `decide_turn` resolve the contract precedence.
//!
//! Every `app.world_mut()` / `SystemState` use is over an [`App`] — the accepted headless
//! idiom (`bevy-traps.md` #7 carve-out (a)): the AC2 shared-decision tests build the same
//! `picking_app()`-style [`App`] (`MinimalPlugins` + the real [`GdtfBattleInputPlugin`]) the
//! AC3/AC4 tests use, spawn fixtures with `app.world_mut().spawn(...)`, and read the shared
//! decision via a [`SystemState`] constructed against `app.world_mut()`. No helper here takes
//! `&mut World` / `&World` in its signature (the landed sibling `selection.rs` house style).

use bevy::{
    camera::{
        Camera, ComputedCameraValues, OrthographicProjection, Projection, RenderTargetInfo,
        primitives::Frustum,
    },
    ecs::system::SystemState,
    input::ButtonInput,
    math::Vec2,
    prelude::*,
    transform::components::GlobalTransform,
    window::{CursorMoved, PrimaryWindow, Window, WindowResolution},
};
use gdtf_battle_input::{
    ActivePointer, GamepadCursor, GdtfBattleInputPlugin, InspectTarget, LeftClickOutcome,
    SelectedFireMode, SelectedShooter, decide_left_click, decide_turn,
    fire_surface::ShooterFireData, selection::LeftClickReads,
};
use gdtf_battle_presenter::{ActiveLevel, HighlightRequest, WorldCamera, cell_to_world};
use gdtf_battle_sim::{
    Aiming, BattleInProgress, Cell, CellLevel, Direction, Faction, FireMode, FireModeSpec, Level,
    LifeState, Magazine, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
    OccupancyGrid, PlayerFaction, Position, ReloadTu, TerrainKind, Tu, TuMax,
    acts::SetFacingRequested, tuning::CombatTuning,
};

/// The faction the player controls (matches the inserted `PlayerFaction`).
const PLAYER_FACTION: Faction = Faction::new(0);
/// An ENEMY faction (distinct from [`PLAYER_FACTION`]).
const ENEMY_FACTION: Faction = Faction::new(1);
/// The level all tests run on.
const LEVEL: Level = Level::new(0);
/// The synthetic window / camera render-target size (physical px).
const TARGET_SIZE: Vec2 = Vec2::new(1280.0, 720.0);

// =================================================================================
// AC2 — the SHARED `decide_left_click` / `decide_turn` match the contract precedence.
// =================================================================================

/// A `Single`-kind fire-mode spec with a marker `tu_percent` (arbitrary, not pinned
/// tuning).
const fn spec(tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// Builds a focused headless decision app — the same `MinimalPlugins` + real
/// [`GdtfBattleInputPlugin`] harness `picking_app()` uses (the landed `selection.rs` house
/// style: carve-out (a), every mutation in a TEST BODY) — and seeds the resources the SHARED
/// [`decide_left_click`] reads beyond what the plugin `init_resource`s.
///
/// The plugin already `init_resource`s `InspectTarget` / `SelectedShooter` / `PendingActIntent`
/// / `SelectedFireMode`; this seeds the battle-scoped reads the AC2 decision also consults
/// (`OccupancyGrid` / `CombatTuning` / `PlayerFaction`), overrides `SelectedFireMode` with the
/// marker spec the FIRE branch needs, and inserts the `ButtonInput<MouseButton>` the
/// `LeftClickReads` bundle's `mouse` field requires for the `SystemState` to validate (the
/// decision never consults it — the bundle is shared with the mouse surface).
fn decision_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut()
        .insert_resource(SelectedFireMode::new(spec(0.2, 1)));
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    // The `LeftClickReads` bundle reads `ButtonInput<MouseButton>` (the `mouse` field) even
    // though `decide_left_click` does NOT consult it — the bundle is shared with the mouse
    // surface; insert it so the `SystemState` over `LeftClickReads` validates.
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());
    app
}

/// Spawns an armed, alive, loaded, affordable PLAYER-faction shooter at `cell` (carrying the
/// firing components `can_fire` reads + a `Position` for turn-to-face), places it in the
/// `OccupancyGrid`, and returns its entity. Takes `&mut App` (not `&mut World`): every spawn /
/// resource write is `app.world_mut()` in a TEST-helper body (carve-out (a), the
/// `selection.rs` `place_player_ganger` / `spawn_ganger` precedent).
fn spawn_player_shooter(app: &mut App, cell: CellLevel) -> Entity {
    let single = spec(0.2, 1);
    let ganger = app
        .world_mut()
        .spawn((
            PLAYER_FACTION,
            Position::new(cell),
            FireMode::new(vec![single]),
            Aiming::new(false),
            LifeState::Alive,
            Tu::new(255),
            TuMax::new(100),
            Magazine::new(10, MagazineSize::new(30), ReloadTu::new(12)),
        ))
        .id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    ganger
}

/// Spawns an ENEMY-faction occupant at `cell` and returns its entity. Takes `&mut App`
/// (carve-out (a) — `app.world_mut()` in a test-helper body, the `selection.rs` precedent).
fn place_enemy(app: &mut App, cell: CellLevel) -> Entity {
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(enemy));
    enemy
}

/// The `SystemState` param tuple for [`decide_left_click`] (aliased to keep clippy's
/// `type_complexity` happy — the framework-plumbing carve-out for a `SystemState` tuple). The
/// [`InspectTarget`] is a SEPARATE `Res` (GTW-300: it is no longer inside [`LeftClickReads`], so
/// the click systems can hold it as a `ResMut` for the pin write without a B0002 conflict).
type DecideParams<'w, 's> = (
    LeftClickReads<'w>,
    Res<'w, InspectTarget>,
    Query<'w, 's, &'static Faction>,
    Query<'w, 's, ShooterFireData<'static>>,
    Res<'w, SelectedShooter>,
);

/// Calls the SHARED [`decide_left_click`] over the `app`'s world via a `SystemState`
/// constructed in this helper's body (carve-out (a) — `app.world_mut()`, NOT a `&mut World`
/// signature). Exercising the pub decision fn with `Query` params over the real app world.
fn decide(app: &mut App) -> LeftClickOutcome {
    let world = app.world_mut();
    let mut state: SystemState<DecideParams> = SystemState::new(world);
    // `get` now returns a `Result` (Bevy 0.19); these params always validate, so
    // an `Err` is structurally impossible — fall back to the no-op outcome, which
    // would fail the calling assertion loudly rather than panic.
    let Ok((reads, inspect, factions, shooters, selected)) = state.get(world) else {
        return LeftClickOutcome::NoOp;
    };
    decide_left_click(&reads, &inspect, &factions, &shooters, &selected)
}

/// Calls the SHARED [`decide_turn`] over the `app`'s world via a `SystemState` constructed in
/// this helper's body (carve-out (a) — `app.world_mut()`, NOT a `&mut World` signature).
fn turn(app: &mut App) -> Option<SetFacingRequested> {
    let world = app.world_mut();
    let mut state: SystemState<(Res<SelectedShooter>, Res<InspectTarget>, Query<&Position>)> =
        SystemState::new(world);
    // `get` now returns a `Result` (Bevy 0.19); these params always validate.
    let Ok((selected, hovered, positions)) = state.get(world) else {
        return None;
    };
    decide_turn(&selected, &hovered, &positions)
}

/// AC2 — `decide_left_click` resolves the FIRE → SELECT → MOVE → CLEAR precedence into the
/// matching `LeftClickOutcome` (the same decision the mouse AND the gamepad use). Each branch
/// is pin-discriminated against a WRONG variant.
#[test]
fn decide_left_click_matches_the_contract_precedence() {
    // --- FIRE: a fire mode + a player selection + an ENEMY occupant + can_fire passes. ---
    {
        let mut app = decision_app();
        let shooter_cell = CellLevel::new(Cell::new(2, 2), LEVEL);
        let ganger = spawn_player_shooter(&mut app, shooter_cell);
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        let target = CellLevel::new(Cell::new(6, 2), LEVEL);
        let _enemy = place_enemy(&mut app, target);
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(target)));

        let outcome = decide(&mut app);
        assert!(
            matches!(outcome, LeftClickOutcome::Fire(_)),
            "an enemy cell with a fire mode must FIRE, got {outcome:?}",
        );
        let LeftClickOutcome::Fire(request) = outcome else {
            return;
        };
        assert_eq!(request.shooter, ganger, "FIRE shooter = the selection");
        assert_eq!(
            request.target_cell,
            Cell::new(6, 2),
            "FIRE target = hovered"
        );
    }

    // --- SELECT: the hovered cell holds one of YOUR gangers. ---
    {
        let mut app = decision_app();
        let cell = CellLevel::new(Cell::new(5, 5), LEVEL);
        let ganger = spawn_player_shooter(&mut app, cell);
        app.world_mut().insert_resource(SelectedShooter::cleared());
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(cell)));

        assert_eq!(
            decide(&mut app),
            LeftClickOutcome::Select(ganger),
            "a player-occupied cell must SELECT that ganger",
        );
    }

    // --- MOVE: a player selection + an empty, in-bounds, unblocked cell. ---
    {
        let mut app = decision_app();
        let shooter_cell = CellLevel::new(Cell::new(3, 3), LEVEL);
        let ganger = spawn_player_shooter(&mut app, shooter_cell);
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        let dest = CellLevel::new(Cell::new(4, 3), LEVEL);
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(dest)));

        let outcome = decide(&mut app);
        assert!(
            matches!(outcome, LeftClickOutcome::Move(_)),
            "an empty cell with a selection must MOVE, got {outcome:?}",
        );
        let LeftClickOutcome::Move(request) = outcome else {
            return;
        };
        assert_eq!(request.actor, ganger, "MOVE actor = the selection");
        assert_eq!(request.dest, dest, "MOVE dest = the hovered cell");
    }

    // --- NO-OP: nothing hovered (GTW-288). The GTW-286 viewport gate resolves an over-UI /
    //     margin / off-map click to no hovered cell, and a no-hover click must NOT clear the
    //     selection — it is a NoOp (the gamepad shares the same `decide_left_click`). ---
    {
        let mut app = decision_app();
        app.world_mut().insert_resource(SelectedShooter::cleared());
        app.world_mut().insert_resource(InspectTarget::new(None));

        assert_eq!(
            decide(&mut app),
            LeftClickOutcome::NoOp,
            "nothing hovered must be a NO-OP, not CLEAR (GTW-288, the GTW-286 over-UI case)",
        );
    }

    // --- CLEAR: a valid in-grid hovered cell with a NON-player / stale selection where none
    //     of FIRE/SELECT/MOVE/NO-OP applies (GTW-288 keeps CLEAR for the genuine Some-cell
    //     case). A bare entity selection is not player-faction, so MOVE does not apply. ---
    {
        let mut app = decision_app();
        let stale = app.world_mut().spawn_empty().id();
        app.world_mut().insert_resource(SelectedShooter::new(stale));
        // An EMPTY in-grid cell with a non-player (bare-entity) selection: FIRE needs an enemy
        // occupant (none), SELECT needs a player occupant (none), MOVE needs a player-faction
        // selection (the bare `stale` is not), NO-OP needs an enemy occupant (none) -> CLEAR.
        let cell = CellLevel::new(Cell::new(6, 6), LEVEL);
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(cell)));

        assert_eq!(
            decide(&mut app),
            LeftClickOutcome::Clear,
            "an in-grid Some cell with a non-player/stale selection (no FIRE/SELECT/MOVE/NO-OP) \
             must still CLEAR (GTW-288 preserves the genuine clear case)",
        );
    }

    // --- Fall-through pin: a fire mode over an EMPTY cell must NOT lock out MOVE (not FIRE,
    //     not CLEAR). ---
    {
        let mut app = decision_app();
        let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
        let ganger = spawn_player_shooter(&mut app, shooter_cell);
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        let dest = CellLevel::new(Cell::new(11, 10), LEVEL);
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(dest)));

        assert!(
            matches!(decide(&mut app), LeftClickOutcome::Move(_)),
            "a fire mode over an EMPTY cell must fall through to MOVE, not FIRE / CLEAR",
        );
    }
}

/// AC2 — `decide_turn` resolves the actor→hovered direction into a `SetFacingRequested`, and
/// returns `None` for the actor's OWN cell / no hover (the same decision the mouse AND the
/// gamepad use).
#[test]
fn decide_turn_matches_the_contract() {
    // (5,5) -> (8,5) is due East.
    {
        let mut app = decision_app();
        let actor_cell = CellLevel::new(Cell::new(5, 5), LEVEL);
        let ganger = spawn_player_shooter(&mut app, actor_cell);
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(CellLevel::new(
                Cell::new(8, 5),
                LEVEL,
            ))));

        let request = turn(&mut app);
        assert!(
            request.is_some(),
            "a hovered cell off the actor's cell must yield a turn",
        );
        let Some(request) = request else {
            return;
        };
        assert_eq!(request.actor, ganger, "the turn actor = the selection");
        assert_eq!(
            request.facing,
            Direction::East,
            "from_cells((5,5),(8,5)) must be East",
        );
    }

    // Hovering the actor's OWN cell -> from_cells None -> no turn.
    {
        let mut app = decision_app();
        let actor_cell = CellLevel::new(Cell::new(5, 5), LEVEL);
        let ganger = spawn_player_shooter(&mut app, actor_cell);
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        app.world_mut()
            .insert_resource(InspectTarget::new(Some(actor_cell)));

        assert_eq!(
            turn(&mut app),
            None,
            "hovering the actor's OWN cell must yield no turn",
        );
    }

    // No hover -> no turn.
    {
        let mut app = decision_app();
        let ganger = spawn_player_shooter(&mut app, CellLevel::new(Cell::new(5, 5), LEVEL));
        app.world_mut()
            .insert_resource(SelectedShooter::new(ganger));
        app.world_mut().insert_resource(InspectTarget::new(None));

        assert_eq!(turn(&mut app), None, "nothing hovered must yield no turn");
    }
}

// =================================================================================
// AC3 — the picker honors `ActivePointer` (gamepad cursor vs OS cursor).
// =================================================================================

/// Builds a deterministic [`Camera`] whose `viewport_to_world_2d` succeeds headlessly (the
/// `picking.rs` recipe): render-target info + a computed clip-from-view matrix.
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

/// Builds a focused headless picking app (the `picking.rs` recipe): `MinimalPlugins` + the
/// input plugin, `ActiveLevel`, the `BattleInProgress` gate, a synthetic `WorldCamera` and
/// `Window`/`PrimaryWindow`. The OS cursor is left unset (off-window) so the gamepad path is
/// the only cursor source unless a test sets the OS cursor.
fn picking_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    app.world_mut().insert_resource(ActiveLevel::new(LEVEL));
    app.world_mut().insert_resource(BattleInProgress);
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

/// Sets the primary window's OS cursor position in logical px (or clears it).
fn set_os_cursor(app: &mut App, position: Option<Vec2>) {
    let mut windows = app.world_mut().query::<&mut Window>();
    for mut window in windows.iter_mut(app.world_mut()) {
        window.set_cursor_position(position);
    }
}

/// Reads the current `InspectTarget` live hovered cell.
fn hovered(app: &App) -> Option<CellLevel> {
    app.world()
        .get_resource::<InspectTarget>()
        .and_then(InspectTarget::hovered)
}

/// The camera's world unprojection of a screen `cursor` (so the test derives the expected
/// cell from the documented inverse instead of hardcoding the world math).
fn unproject(app: &mut App, cursor: Vec2) -> Option<Vec2> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Camera, &GlobalTransform), With<WorldCamera>>();
    let (camera, transform) = q.iter(app.world()).next()?;
    camera.viewport_to_world_2d(transform, cursor).ok()
}

/// AC3 — with `ActivePointer::Gamepad` and a `GamepadCursor` over a known cell (resources set
/// directly), `pick_hovered_cell` writes THAT cell to `InspectTarget` (the gamepad cursor's,
/// NOT the OS cursor's); flipping back to `Mouse` reverts to the OS-cursor path.
#[test]
fn picker_honors_active_pointer() {
    let mut app = picking_app();

    // A gamepad-cursor screen point that lands on a non-origin in-grid cell (shifted right +
    // down from the centre — the `picking.rs` screen→world sign reasoning).
    let gamepad_screen = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    // A DIFFERENT OS-cursor screen point (shifted further) so the two cursors disagree.
    let os_screen = TARGET_SIZE * 0.5 + Vec2::new(80.0, 64.0);
    set_os_cursor(&mut app, Some(os_screen));

    // Force the gamepad to be the active pointer + park its cursor over the known point.
    app.world_mut().insert_resource(ActivePointer::Gamepad);
    app.world_mut()
        .insert_resource(GamepadCursor::new(gamepad_screen));
    app.update();

    // The expected cell is the documented inverse of the GAMEPAD cursor's unprojection.
    let gamepad_world = unproject(&mut app, gamepad_screen);
    let os_world = unproject(&mut app, os_screen);
    assert!(
        gamepad_world.is_some() && os_world.is_some(),
        "the synthetic camera must unproject both screen points",
    );
    let (Some(gamepad_world), Some(os_world)) = (gamepad_world, os_world) else {
        return;
    };
    let gamepad_cell = gdtf_battle_input::world_to_cell(gamepad_world, LEVEL);
    let os_cell = gdtf_battle_input::world_to_cell(os_world, LEVEL);
    assert!(
        gamepad_cell.is_some() && os_cell.is_some() && gamepad_cell != os_cell,
        "the two cursors must land on distinct in-grid cells for the arbitration to be \
         observable (gamepad {gamepad_cell:?}, os {os_cell:?})",
    );

    assert_eq!(
        hovered(&app),
        gamepad_cell,
        "in Gamepad mode the picker must project the GAMEPAD cursor, not the OS cursor",
    );

    // Flip back to Mouse -> the picker reverts to the OS-cursor path.
    app.world_mut().insert_resource(ActivePointer::Mouse);
    app.update();
    assert_eq!(
        hovered(&app),
        os_cell,
        "flipping to Mouse must revert the picker to the OS cursor",
    );
}

/// The `HighlightRequest`s a probe drained this run (so the test reads exactly what the
/// landed `emit_highlight_request` wrote — proving the highlight follows the gamepad cursor).
#[derive(Resource, Default)]
struct HighlightProbe(Vec<HighlightRequest>);

/// AC3 — the landed GTW-251 `emit_highlight_request` makes the highlight follow the gamepad
/// cursor for free: in Gamepad mode the emitted `HighlightRequest` equals `Some(the gamepad
/// cell)` — so the cell-highlight IS the cursor (no separate reticle).
#[test]
fn highlight_follows_the_gamepad_cursor() {
    let mut app = picking_app();
    // GTW-268 — the emit now gates on the `OccupancyGrid`; `picking_app` does not seed one,
    // so insert an empty grid here and (below) mark the resolved cell blocking.
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.insert_resource(HighlightProbe::default());
    app.add_systems(
        Update,
        (|mut r: MessageReader<HighlightRequest>, mut p: ResMut<HighlightProbe>| {
            p.0.extend(r.read().copied());
        })
        .after(gdtf_battle_input::emit_highlight_request),
    );

    // OS cursor off-window; gamepad is the active pointer over a known cell.
    set_os_cursor(&mut app, None);
    let gamepad_screen = TARGET_SIZE * 0.5 + Vec2::new(40.0, 32.0);
    app.world_mut().insert_resource(ActivePointer::Gamepad);
    app.world_mut()
        .insert_resource(GamepadCursor::new(gamepad_screen));
    app.update();

    let cell = hovered(&app);
    assert!(
        cell.is_some(),
        "the gamepad cursor must resolve an in-grid cell"
    );
    let Some(resolved) = cell else { return };

    // GTW-268 — the emit now highlights ONLY occupied / blocking cells. Make the gamepad's
    // resolved cell blocking (an object) so the highlight follows it; then drop the
    // bare-floor probe reads and re-run so the probe captures the post-gate emit.
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_terrain(resolved, TerrainKind::Cover);
    }
    if let Some(mut probe) = app.world_mut().get_resource_mut::<HighlightProbe>() {
        probe.0.clear();
    }
    app.update();

    let emitted = app
        .world()
        .get_resource::<HighlightProbe>()
        .map(|p| p.0.clone())
        .unwrap_or_default();
    assert_eq!(
        emitted,
        vec![HighlightRequest::new(cell)],
        "the highlight request must follow the gamepad cursor's resolved cell",
    );
    // Sanity: that cell really is `cell_to_world`-projectable (the highlight will draw there).
    if let Some(cell) = cell {
        let _ = cell_to_world(Cell::new(cell.x, cell.y), LEVEL);
    }
}

// =================================================================================
// AC4 — the mouse reclaims the pointer on a CursorMoved message.
// =================================================================================

/// AC4 — with `ActivePointer::Gamepad`, emitting a `CursorMoved` message flips the active
/// pointer back to `Mouse` (last-moved-wins) via the REAL registered `mouse_reclaims_pointer`.
#[test]
fn mouse_reclaims_the_pointer() {
    let mut app = picking_app();
    // `MinimalPlugins` has no `InputPlugin`, so the `Messages<CursorMoved>` buffer is absent
    // (and `mouse_reclaims_pointer` is gated on it). Register it so the system runs and the
    // test can write the OS-cursor-moved message (under `DefaultPlugins` `InputPlugin`
    // provides this buffer).
    app.add_message::<CursorMoved>();
    // Start in Gamepad mode.
    app.world_mut().insert_resource(ActivePointer::Gamepad);
    app.update();
    assert_eq!(
        *app.world().resource::<ActivePointer>(),
        ActivePointer::Gamepad,
        "precondition: the pointer starts on Gamepad",
    );

    // Find the primary window entity to address the CursorMoved message at it.
    let window = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>();
        q.iter(app.world()).next()
    };
    assert!(
        window.is_some(),
        "the picking app must have a primary window"
    );
    let Some(window) = window else {
        return;
    };

    // Emit a CursorMoved (the OS mouse moved) and update — the mouse reclaims the pointer.
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<CursorMoved>>()
        .write(CursorMoved {
            window,
            position: Vec2::new(100.0, 100.0),
            delta: Some(Vec2::new(5.0, 5.0)),
        });
    app.update();

    assert_eq!(
        *app.world().resource::<ActivePointer>(),
        ActivePointer::Mouse,
        "a CursorMoved message must flip the active pointer back to Mouse (last-moved-wins)",
    );
}
