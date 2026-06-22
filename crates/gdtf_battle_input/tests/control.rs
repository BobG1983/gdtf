//! GTW-238: headless integration tests for the `PlayerFaction`-gated control surface —
//! the ONE disambiguated left-click decision (`left_click_act`: FIRE -> SELECT -> MOVE
//! -> CLEAR) and the right-click turn-to-face surface (`right_click_turn_to_face`), over
//! the REAL `GdtfBattleInputPlugin` seam (its click systems -> the ONE
//! `dispatch_act_intents` drain -> the emitted `*Requested`).
//!
//! Tests are headless `GdtfBattleInputPlugin` apps: synth `ButtonInput<MouseButton>` +
//! `InspectTarget` + `OccupancyGrid` + `PlayerFaction` + spawned `Faction`/`Position`/firing
//! components, `app.update()`, assert the emitted `*Requested` / `SelectedShooter`. The
//! click systems run `.before(pick_hovered_cell)`, so an INJECTED `InspectTarget` is read
//! that update before the (headless, camera-less) picker clobbers it to `None`.
//!
//! - AC1 — left-click SELECTS only a player ganger (enemy/empty does not select-as-own).
//! - AC2 — left-click empty + player selection -> exactly one `MoveRequested`, no fire.
//! - AC3 — left-click ENEMY + fire mode -> exactly one `FireRequested`, no move, selection
//!   unchanged.
//! - AC4 — left-click EMPTY + fire mode + selection -> falls through to MOVE (no fire).
//! - AC5 — right-click + selection -> `SetFacingRequested` to `from_cells(actor, hovered)`;
//!   the actor's own cell is a no-op.
//! - AC6 — a FORCED enemy selection emits NOTHING on Left or Right.
//! - AC7 — the drain emits each NEW intent (`Move`/`Turn`) exactly once and empties the
//!   queue.
//!
//! Every `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.

use bevy::{input::ButtonInput, prelude::*};
use gdtf_battle_input::{
    ActIntent, GdtfBattleInputPlugin, InspectTarget, PendingActIntent, SelectedFireMode,
    SelectedShooter,
};
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::{
    Aiming, BattleInProgress, Cell, CellLevel, Direction, Faction, FireMode, FireModeSpec, Level,
    LifeState, Magazine, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
    OccupancyGrid, PlayerFaction, Position, ReloadTu, Tu, TuMax, VerticalLinkGraph, WieldedBy,
    acts::{FireRequested, MoveRequested, SetFacingRequested},
    tuning::CombatTuning,
};

/// The faction the player controls (matches the inserted `PlayerFaction`).
const PLAYER_FACTION: Faction = Faction::new(0);
/// An ENEMY faction (distinct from [`PLAYER_FACTION`]) — the FIRE branch's valid target.
const ENEMY_FACTION: Faction = Faction::new(1);
/// The level all control tests run on.
const LEVEL: Level = Level::new(0);

// ---------------------------------------------------------------------------------
// Fixtures + harness.
// ---------------------------------------------------------------------------------

/// Builds the base headless control app: `MinimalPlugins` plus the
/// `GdtfBattleInputPlugin`, the presenter-owned `ActiveLevel`, the `BattleInProgress`
/// gate, an empty `OccupancyGrid`, `CombatTuning`, the `PlayerFaction` the decision gates
/// on, and an empty `ButtonInput<MouseButton>`. No synthetic camera is needed because the
/// click systems run before `pick_hovered_cell`, so an injected `InspectTarget` is read
/// before the headless picker clobbers it.
fn control_app() -> App {
    let mut app = App::new();
    // GTW-322: `update_selection_highlight` (in `GdtfBattleInputPlugin`) spawns its reticle
    // via `Commands::spawn_scene`, which PANICS under `MinimalPlugins` without an
    // `AssetServer` + the scene schedule (the spike-documented requirement).
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ))
    .add_plugins(GdtfBattleInputPlugin);
    app.world_mut().insert_resource(ActiveLevel::new(LEVEL));
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    // GTW-356: the shared left-click decision reads `Res<VerticalLinkGraph>` (the OQ-4
    // link-tile non-target gate, via `LeftClickReads`), and `battle_act_gate()` now gates the
    // click systems on it — seed an empty graph so the click decision runs (these control tests
    // place no vertical links, so every move target is a non-link tile).
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());
    add_probes(&mut app);
    app
}

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

/// Spawns an armed, alive, loaded, affordable PLAYER-faction shooter at `cell` (carrying
/// the firing components `can_fire` reads + a `Position` for right-click turn-to-face),
/// places it in the occupancy grid, and returns its entity.
fn spawn_player_shooter(app: &mut App, cell: CellLevel) -> Entity {
    let single = spec(0.2, 1);
    // The ganger carries its OWN vitals only — the weapon's FireMode + Magazine ride on a
    // related WEAPON entity (`Wields`, GTW-323 slice 3); the `WieldedBy` insert hook
    // populates the ganger's `Wields` synchronously in a bare `World` spawn.
    let ganger = app
        .world_mut()
        .spawn((
            PLAYER_FACTION,
            Position::new(cell),
            Aiming::new(false),
            LifeState::Alive,
            Tu::new(255),
            TuMax::new(100),
        ))
        .id();
    app.world_mut().spawn((
        WieldedBy(ganger),
        FireMode::new(vec![single]),
        Magazine::new(10, MagazineSize::new(30), ReloadTu::new(12)),
    ));
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    ganger
}

/// Spawns an ENEMY-faction occupant at `cell` (only a `Faction` — the fire path reads
/// the SHOOTER's firing components, never the target's) and returns its entity.
fn place_enemy(app: &mut App, cell: CellLevel) -> Entity {
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(enemy));
    enemy
}

/// Injects the `InspectTarget`'s live hovered cell (read by the click systems before the
/// headless picker clobbers it).
fn set_hovered(app: &mut App, cell: Option<CellLevel>) {
    app.world_mut().insert_resource(InspectTarget::new(cell));
}

/// Forces the current `SelectedShooter` to `entity` (the test-injected selection).
fn set_selection(app: &mut App, entity: Entity) {
    app.world_mut()
        .insert_resource(SelectedShooter::new(entity));
}

/// Forces a non-default fire mode so the FIRE branch has a mode to fire (the plugin
/// already inits a single-shot default; this is explicit for the fire-mode tests).
fn set_fire_mode(app: &mut App, mode: FireModeSpec) {
    app.world_mut().insert_resource(SelectedFireMode::new(mode));
}

/// Presses (just-pressed edge) a mouse button.
fn press(app: &mut App, button: MouseButton) {
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(button);
}

/// Releases + clears the mouse edges so the NEXT `press` is a fresh just-pressed (under
/// `MinimalPlugins` no `InputPlugin` clears the edges per frame, so a two-click sequence must
/// clear between presses).
fn clear_mouse(app: &mut App) {
    let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    mouse.release(MouseButton::Left);
    mouse.clear();
}

/// The current `PathPreviewTarget` (the GTW-356 two-click move target the preview previews TO).
fn move_target(app: &App) -> Option<CellLevel> {
    app.world()
        .get_resource::<gdtf_battle_input::PathPreviewTarget>()
        .and_then(|t| **t)
}

/// The current `SelectedShooter`.
fn selected(app: &App) -> Option<Entity> {
    app.world()
        .get_resource::<SelectedShooter>()
        .and_then(|s| **s)
}

// ---------------------------------------------------------------------------------
// Message probes — each runs AFTER the drain, so it sees the same update's emission.
// ---------------------------------------------------------------------------------

/// Collected `FireRequested` messages (probe).
#[derive(Resource, Default)]
struct FireProbe(Vec<FireRequested>);
/// Collected `MoveRequested` messages (probe).
#[derive(Resource, Default)]
struct MoveProbe(Vec<MoveRequested>);
/// Collected `SetFacingRequested` messages (probe).
#[derive(Resource, Default)]
struct FacingProbe(Vec<SetFacingRequested>);

/// Adds the three message-collecting probe systems, each running AFTER the drain so it
/// observes the same update's emitted messages (its own `MessageReader` cursor).
fn add_probes(app: &mut App) {
    app.insert_resource(FireProbe::default())
        .insert_resource(MoveProbe::default())
        .insert_resource(FacingProbe::default());
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
        .map_or_else(Vec::new, |p| p.0.clone())
}

/// The collected `MoveRequested` messages.
fn moves(app: &App) -> Vec<MoveRequested> {
    app.world()
        .get_resource::<MoveProbe>()
        .map_or_else(Vec::new, |p| p.0.clone())
}

/// The collected `SetFacingRequested` messages.
fn facings(app: &App) -> Vec<SetFacingRequested> {
    app.world()
        .get_resource::<FacingProbe>()
        .map_or_else(Vec::new, |p| p.0.clone())
}

// ---------------------------------------------------------------------------------
// AC1 — left-click SELECTS only a player ganger.
// ---------------------------------------------------------------------------------

/// AC1 — a Left press over a PLAYER-faction occupant selects it; a Left press over a
/// NON-player occupant (or an empty cell) does NOT become a player-own selection.
#[test]
fn left_click_selects_only_a_player_ganger() {
    // Player occupant -> selected.
    {
        let mut app = control_app();
        let cell = CellLevel::new(Cell::new(5, 5), LEVEL);
        let ganger = spawn_player_shooter(&mut app, cell);
        set_hovered(&mut app, Some(cell));
        press(&mut app, MouseButton::Left);
        app.update();
        assert_eq!(
            selected(&app),
            Some(ganger),
            "a Left press over a player-faction occupant must select it",
        );
    }
    // Enemy occupant -> NOT selected-as-own (no fire mode forces fire; no prior selection
    // -> CLEAR).
    {
        let mut app = control_app();
        let cell = CellLevel::new(Cell::new(7, 7), LEVEL);
        let enemy = place_enemy(&mut app, cell);
        set_hovered(&mut app, Some(cell));
        press(&mut app, MouseButton::Left);
        app.update();
        assert_ne!(
            selected(&app),
            Some(enemy),
            "an enemy occupant must never become a player-own selection",
        );
    }
    // Empty cell -> not a selection.
    {
        let mut app = control_app();
        set_hovered(&mut app, Some(CellLevel::new(Cell::new(9, 9), LEVEL)));
        press(&mut app, MouseButton::Left);
        app.update();
        assert_eq!(
            selected(&app),
            None,
            "a Left press over an empty cell must not select",
        );
    }
}

// ---------------------------------------------------------------------------------
// AC2 (GTW-356) — left-click empty + player selection is TWO-CLICK: click-1 SETS the
// move target (no dispatch); click-2 on the SAME cell COMMITS the move.
// ---------------------------------------------------------------------------------

/// AC2 — with a player-faction `SelectedShooter`, the FIRST Left press on an empty,
/// in-bounds, unblocked cell SETS `PathPreviewTarget` to that cell and emits NO
/// `MoveRequested`; the SECOND Left press on the SAME cell emits exactly one `MoveRequested
/// { actor = selection, dest = hovered }`, clears the target, and emits zero `FireRequested`
/// (GTW-356 two-click flow — the single-click immediate-move was REPLACED, not weakened).
#[test]
fn two_click_empty_with_selection_targets_then_moves() {
    let mut app = control_app();
    let shooter_cell = CellLevel::new(Cell::new(3, 3), LEVEL);
    let ganger = spawn_player_shooter(&mut app, shooter_cell);
    set_selection(&mut app, ganger);

    let dest = CellLevel::new(Cell::new(4, 3), LEVEL);
    set_hovered(&mut app, Some(dest));

    // Click-1: SET the target, dispatch NOTHING.
    press(&mut app, MouseButton::Left);
    app.update();
    assert!(
        moves(&app).is_empty(),
        "click-1 on a valid target must emit NO MoveRequested (it only sets the target)",
    );
    assert_eq!(
        move_target(&app),
        Some(dest),
        "click-1 must SET PathPreviewTarget to the clicked cell",
    );

    // Click-2 on the SAME cell: COMMIT.
    clear_mouse(&mut app);
    set_hovered(&mut app, Some(dest));
    press(&mut app, MouseButton::Left);
    app.update();

    let emitted = moves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "click-2 on the SAME cell must emit exactly one MoveRequested (commit)",
    );
    assert_eq!(emitted[0].actor, ganger, "move actor = the selection");
    assert_eq!(emitted[0].dest, dest, "move dest = the targeted cell");
    assert_eq!(
        move_target(&app),
        None,
        "committing the move must CLEAR PathPreviewTarget",
    );
    assert!(
        fires(&app).is_empty(),
        "a MOVE commit must emit no FireRequested",
    );
}

/// AC2 (GTW-356 re-target) — with a target already pending, a click on a DIFFERENT valid
/// cell RE-TARGETS the preview (does NOT commit): no `MoveRequested`, and the target follows
/// the new cell. (The same-cell-commit / different-cell-retarget interpretation, FLAGGED.)
#[test]
fn click_on_a_different_cell_retargets_without_committing() {
    let mut app = control_app();
    let shooter_cell = CellLevel::new(Cell::new(3, 3), LEVEL);
    let ganger = spawn_player_shooter(&mut app, shooter_cell);
    set_selection(&mut app, ganger);

    let first = CellLevel::new(Cell::new(4, 3), LEVEL);
    set_hovered(&mut app, Some(first));
    press(&mut app, MouseButton::Left);
    app.update();
    assert_eq!(
        move_target(&app),
        Some(first),
        "click-1 sets the first target"
    );

    // A click on a DIFFERENT valid cell re-targets, never commits.
    clear_mouse(&mut app);
    let second = CellLevel::new(Cell::new(5, 3), LEVEL);
    set_hovered(&mut app, Some(second));
    press(&mut app, MouseButton::Left);
    app.update();

    assert!(
        moves(&app).is_empty(),
        "a click on a DIFFERENT cell must RE-TARGET, never commit (no MoveRequested)",
    );
    assert_eq!(
        move_target(&app),
        Some(second),
        "the target must follow the newly-clicked cell",
    );
}

// ---------------------------------------------------------------------------------
// AC3 — left-click ENEMY + fire mode -> FIRE, mutually exclusive.
// ---------------------------------------------------------------------------------

/// AC3 — with a fire mode selected, a player-faction selection, and a Left press on an
/// ENEMY-occupied cell where `can_fire` passes, exactly one `FireRequested` is emitted,
/// NO `MoveRequested`, and `SelectedShooter` is UNCHANGED that edge.
#[test]
fn left_click_enemy_with_fire_mode_fires_and_is_mutually_exclusive() {
    let mut app = control_app();
    let shooter_cell = CellLevel::new(Cell::new(2, 2), LEVEL);
    let ganger = spawn_player_shooter(&mut app, shooter_cell);
    set_selection(&mut app, ganger);
    set_fire_mode(&mut app, spec(0.2, 1));

    let target = CellLevel::new(Cell::new(6, 2), LEVEL);
    let _enemy = place_enemy(&mut app, target);
    set_hovered(&mut app, Some(target));

    press(&mut app, MouseButton::Left);
    app.update();

    assert_eq!(
        fires(&app).len(),
        1,
        "exactly one FireRequested on a Left press over an enemy with a fire mode",
    );
    assert!(
        moves(&app).is_empty(),
        "a FIRE edge must emit no MoveRequested",
    );
    assert_eq!(
        selected(&app),
        Some(ganger),
        "a FIRE edge must leave SelectedShooter unchanged",
    );
    let fire = fires(&app);
    assert_eq!(fire[0].shooter, ganger, "the fire shooter = the selection");
    assert_eq!(
        fire[0].target_cell,
        Cell::new(6, 2),
        "the fire target = the hovered cell",
    );
}

// ---------------------------------------------------------------------------------
// AC4 (GTW-356) — left-click EMPTY + fire mode + selection -> falls through to the
// two-click MOVE path: click-1 SETS the target (no fire); click-2 same cell COMMITS.
// ---------------------------------------------------------------------------------

/// AC4 — with a fire mode selected, a player-faction selection, and a Left press on an
/// EMPTY in-bounds unblocked cell, the fire mode does NOT lock out move: click-1 falls
/// through to SET the move target (no `FireRequested`, no `MoveRequested`); click-2 on the
/// SAME cell commits exactly one `MoveRequested` (the flagged fall-through precedence, now
/// over the two-click flow).
#[test]
fn empty_with_fire_mode_falls_through_to_two_click_move() {
    let mut app = control_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let ganger = spawn_player_shooter(&mut app, shooter_cell);
    set_selection(&mut app, ganger);
    set_fire_mode(&mut app, spec(0.2, 1));

    let dest = CellLevel::new(Cell::new(11, 10), LEVEL);
    set_hovered(&mut app, Some(dest));

    // Click-1: falls through FIRE (empty cell) to SET the move target — no fire, no move.
    press(&mut app, MouseButton::Left);
    app.update();
    assert!(
        fires(&app).is_empty(),
        "the fire mode must NOT lock out move on an empty cell (no FireRequested)",
    );
    assert!(
        moves(&app).is_empty(),
        "click-1 only sets the target (no immediate MoveRequested)",
    );
    assert_eq!(
        move_target(&app),
        Some(dest),
        "click-1 with a fire mode over an empty cell still SETS the move target",
    );

    // Click-2 on the SAME cell: COMMIT the move.
    clear_mouse(&mut app);
    set_hovered(&mut app, Some(dest));
    press(&mut app, MouseButton::Left);
    app.update();
    assert_eq!(
        moves(&app).len(),
        1,
        "click-2 on the same cell commits exactly one MoveRequested",
    );
    assert!(
        fires(&app).is_empty(),
        "the commit must still emit no FireRequested",
    );
}

// ---------------------------------------------------------------------------------
// AC5 — right-click + selection -> SetFacingRequested to from_cells(actor, hovered).
// ---------------------------------------------------------------------------------

/// AC5 — with a player-faction selection on `Position` cell `(5,5)` and the `InspectTarget`
/// hovered cell on
/// `(8,5)`, one Right press emits exactly one `SetFacingRequested { facing = East }`
/// (`Direction::from_cells((5,5),(8,5)) == East`). Hovering the actor's OWN cell emits no
/// intent.
#[test]
fn right_click_turns_to_face_the_hovered_cell() {
    // (5,5) -> (8,5) is due East.
    {
        let mut app = control_app();
        let actor_cell = CellLevel::new(Cell::new(5, 5), LEVEL);
        let ganger = spawn_player_shooter(&mut app, actor_cell);
        set_selection(&mut app, ganger);
        set_hovered(&mut app, Some(CellLevel::new(Cell::new(8, 5), LEVEL)));

        press(&mut app, MouseButton::Right);
        app.update();

        let emitted = facings(&app);
        assert_eq!(
            emitted.len(),
            1,
            "exactly one SetFacingRequested on a Right press with a player selection",
        );
        assert_eq!(emitted[0].actor, ganger, "the turn actor = the selection");
        assert_eq!(
            emitted[0].facing,
            Direction::East,
            "from_cells((5,5),(8,5)) must be East",
        );
    }
    // Hovering the actor's OWN cell -> from_cells None -> no intent.
    {
        let mut app = control_app();
        let actor_cell = CellLevel::new(Cell::new(5, 5), LEVEL);
        let ganger = spawn_player_shooter(&mut app, actor_cell);
        set_selection(&mut app, ganger);
        set_hovered(&mut app, Some(actor_cell));

        press(&mut app, MouseButton::Right);
        app.update();

        assert!(
            facings(&app).is_empty(),
            "hovering the actor's own cell must emit no SetFacingRequested",
        );
    }
}

// ---------------------------------------------------------------------------------
// AC6 — gating covers Left AND Right: a forced ENEMY selection emits NOTHING.
// ---------------------------------------------------------------------------------

/// AC6 — a FORCED enemy-faction `SelectedShooter` (a non-player entity injected as the
/// selection) emits NOTHING on a Left press (no Move/Fire) AND nothing on a Right press
/// (no `SetFacingRequested`), and is never treated as a player actor.
#[test]
fn forced_enemy_selection_emits_nothing_on_left_or_right() {
    // Forced enemy selection -> Left press over an empty cell emits nothing.
    {
        let mut app = control_app();
        // An enemy ganger with a Position (so right-click's Position lookup succeeds and
        // the ONLY block is the faction gate).
        let enemy_cell = CellLevel::new(Cell::new(20, 20), LEVEL);
        let enemy = app
            .world_mut()
            .spawn((ENEMY_FACTION, Position::new(enemy_cell)))
            .id();
        set_selection(&mut app, enemy);
        set_hovered(&mut app, Some(CellLevel::new(Cell::new(21, 20), LEVEL)));

        press(&mut app, MouseButton::Left);
        app.update();
        assert!(
            moves(&app).is_empty() && fires(&app).is_empty(),
            "a forced enemy selection must emit no Move/Fire on a Left press",
        );

        // Right press over a distinct cell — no turn either.
        set_hovered(&mut app, Some(CellLevel::new(Cell::new(25, 20), LEVEL)));
        press(&mut app, MouseButton::Right);
        app.update();
        assert!(
            facings(&app).is_empty(),
            "a forced enemy selection must emit no SetFacingRequested on a Right press",
        );
    }
}

// ---------------------------------------------------------------------------------
// AC7 — the drain emits each NEW intent exactly once and empties the queue.
// ---------------------------------------------------------------------------------

/// AC7 — queueing an `ActIntent::Move` and an `ActIntent::Turn` (via the public
/// `PendingActIntent::push`) and running ONE update emits exactly one `MoveRequested` and
/// one `SetFacingRequested`, and the queue is `is_empty()` afterward (take-and-clear).
#[test]
fn drain_emits_each_new_intent_exactly_once() {
    let mut app = control_app();
    let actor = spawn_player_shooter(&mut app, CellLevel::new(Cell::new(1, 1), LEVEL));
    let dest = CellLevel::new(Cell::new(2, 1), LEVEL);

    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::Move(MoveRequested::new(actor, dest)));
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::Turn(SetFacingRequested::new(
            actor,
            Direction::East,
        )));

    app.update();

    assert_eq!(
        moves(&app).len(),
        1,
        "the drain must emit exactly one MoveRequested",
    );
    assert_eq!(
        facings(&app).len(),
        1,
        "the drain must emit exactly one SetFacingRequested",
    );
    assert!(
        app.world()
            .get_resource::<PendingActIntent>()
            .is_some_and(PendingActIntent::is_empty),
        "the intent queue must be empty after the drain (take-and-clear)",
    );
}
