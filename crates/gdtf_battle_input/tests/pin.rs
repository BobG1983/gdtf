//! GTW-300 slice 3 — headless integration tests for CLICK-TO-PIN the inspect panel.
//!
//! These drive the REAL `left_click_act` system (the same shared decision the gamepad's
//! `gamepad_click_act` routes through, so the gamepad gets pin parity for free) and assert on
//! the `InspectTarget` resource's pin, proving each contract clause:
//!
//! - clicking COVER pins the panel on that cell;
//! - clicking an ENEMY fighter pins the panel on that cell;
//! - clicking an EMPTY in-grid tile unpins (hover resumes);
//! - clicking your OWN ganger SELECTS it and KEEPS the pin;
//! - clicking to FIRE on an enemy KEEPS the pin (does not clear it);
//! - while PINNED, moving the cursor (the live hovered cell) does NOT change the panel's
//!   EFFECTIVE target — the behavioral check the contract requires for the view interaction.
//!
//! Each test is pin-DISCRIMINATING: removing the `decide_pin`/`apply_pin` wiring (or mis-deciding
//! a branch) flips the asserted pin and fails the test.
//!
//! Every `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.

use bevy::{input::ButtonInput, prelude::*};
use gdtf_battle_input::{
    GdtfBattleInputPlugin, InspectMode, InspectTarget, SelectedFireMode, SelectedShooter,
};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::{Aiming, TuMax},
    magazine::{Magazine, ReloadTu},
    occupancy::TerrainKind,
    prelude::{
        BattleInProgress, Cell, CellLevel, Faction, Level, LifeState, OccupancyGrid, Position, Tu,
    },
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    weapon::{
        FireMode, FireModeSpec, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
    },
};
use gdtf_test_utils::{clear_mouse, press_left};

/// The faction the player controls (matches the inserted `PlayerFaction`).
const PLAYER_FACTION: Faction = Faction::new(0);
/// An ENEMY faction (distinct from [`PLAYER_FACTION`]) — the pin-on-enemy + fire target.
const ENEMY_FACTION: Faction = Faction::new(1);
/// The level all pin tests run on.
const LEVEL: Level = Level::new(0);

/// Builds the base headless pin app: `MinimalPlugins` + the `GdtfBattleInputPlugin`, the
/// presenter-owned `ActiveLevel`, the `BattleInProgress` gate, an empty `OccupancyGrid`,
/// `CombatTuning`, the `PlayerFaction` the decision gates on, and an empty
/// `ButtonInput<MouseButton>`. No synthetic camera is needed: the click systems run
/// `.before(pick_hovered_cell)`, so an injected `InspectTarget` hovered cell is read before the
/// headless picker (which has no camera and so resolves `None`) would clobber it.
fn pin_app() -> App {
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
    // GTW-521 — `dispatch_act_intents` also mutates the presenter-owned `ViewMode`.
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    // GTW-356: the shared left-click decision reads `Res<VerticalLinkGraph>` (the OQ-4
    // link-tile gate) via `LeftClickReads`, and `battle_act_gate()` now gates the click systems
    // on it — seed an empty graph so the click decision (and its pin effect) runs.
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());
    app
}

/// A `Single`-kind fire-mode spec (arbitrary `tu_percent`, one shot).
const fn spec() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    )
}

/// Spawns an armed, alive, loaded, affordable PLAYER-faction shooter at `cell` (the firing
/// components `can_fire` reads + a `Position`), registers it in the grid, and returns it.
fn spawn_player_shooter(app: &mut App, cell: CellLevel) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((
            PLAYER_FACTION,
            Position::new(cell),
            FireMode::new(vec![spec()]),
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

/// Spawns a PLAYER-faction occupant (selectable) at `cell` and returns it.
fn place_player_ganger(app: &mut App, cell: CellLevel) -> Entity {
    let ganger = app.world_mut().spawn(PLAYER_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    ganger
}

/// Spawns an ENEMY-faction occupant at `cell` and returns it.
fn place_enemy(app: &mut App, cell: CellLevel) -> Entity {
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(enemy));
    enemy
}

/// Marks `cell` as COVER terrain (so `is_blocked` is true — a wall / cover the pin targets).
fn place_cover(app: &mut App, cell: CellLevel) {
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_terrain(cell, TerrainKind::Cover);
}

/// Sets the live hovered cell WITHOUT disturbing any pin — mutates the existing resource in
/// place (the picker's per-update writer), so a click can resolve "where you clicked" while a
/// prior pin is still held. (Re-inserting `InspectTarget::new` would WIPE the pin.)
fn set_hovered(app: &mut App, cell: Option<CellLevel>) {
    app.world_mut()
        .resource_mut::<InspectTarget>()
        .set_hovered(cell);
}

/// Forces the current `SelectedShooter` to `entity`.
fn set_selection(app: &mut App, entity: Entity) {
    app.world_mut()
        .insert_resource(SelectedShooter::new(entity));
}

/// Forces a non-default fire mode so the FIRE branch has a mode to fire.
fn set_fire_mode(app: &mut App) {
    app.world_mut()
        .insert_resource(SelectedFireMode::new(spec()));
}

/// The current pin on the `InspectTarget`.
fn pinned(app: &App) -> Option<CellLevel> {
    app.world()
        .get_resource::<InspectTarget>()
        .and_then(InspectTarget::pinned)
}

/// The current `SelectedShooter`.
fn selected(app: &App) -> Option<Entity> {
    app.world()
        .get_resource::<SelectedShooter>()
        .and_then(|s| **s)
}

/// The current EFFECTIVE inspect mode (what the panel would describe).
fn effective(app: &App) -> Option<InspectMode> {
    app.world()
        .get_resource::<InspectTarget>()
        .map(InspectTarget::effective)
}

// ---------------------------------------------------------------------------------
// Clause: clicking COVER pins the panel on that target.
// ---------------------------------------------------------------------------------

/// Clicking a COVER cell PINS the inspect panel on that cell — drives the real `left_click_act`
/// through `decide_pin`'s cover rung. (No selection / fire mode needed: the pin is a parallel
/// VIEW concern, so a structural object pins regardless of the act state.)
#[test]
fn clicking_cover_pins_that_cell() {
    let mut app = pin_app();
    let cell = CellLevel::new(Cell::new(8, 5), LEVEL);
    place_cover(&mut app, cell);
    set_hovered(&mut app, Some(cell));

    press_left(&mut app);
    app.update();

    assert_eq!(
        pinned(&app),
        Some(cell),
        "clicking a cover cell must pin the inspect panel on it",
    );
}

// ---------------------------------------------------------------------------------
// Clause: clicking an ENEMY fighter pins the panel on that target.
// ---------------------------------------------------------------------------------

/// Clicking an ENEMY-occupied cell (with no fire mode, so it does NOT fire) PINS the panel on
/// that cell — the `decide_pin` enemy rung. The act effect here is the GTW-287 NO-OP; the pin is
/// independent.
#[test]
fn clicking_an_enemy_pins_that_cell() {
    let mut app = pin_app();
    let cell = CellLevel::new(Cell::new(3, 9), LEVEL);
    place_enemy(&mut app, cell);
    set_hovered(&mut app, Some(cell));

    press_left(&mut app);
    app.update();

    assert_eq!(
        pinned(&app),
        Some(cell),
        "clicking an enemy fighter must pin the inspect panel on its cell",
    );
}

// ---------------------------------------------------------------------------------
// Clause: clicking an empty tile unpins -> hover resumes normally.
// ---------------------------------------------------------------------------------

/// Clicking an EMPTY in-grid tile UNPINS the panel and hover resumes: starting from a held pin,
/// a click on a bare cell clears the pin so `effective()` follows the live hovered cell again.
#[test]
fn clicking_an_empty_tile_unpins_and_hover_resumes() {
    let mut app = pin_app();

    // Establish a pin first (click cover), then move to and click a bare empty cell.
    let cover = CellLevel::new(Cell::new(8, 5), LEVEL);
    place_cover(&mut app, cover);
    set_hovered(&mut app, Some(cover));
    press_left(&mut app);
    app.update();
    assert_eq!(
        pinned(&app),
        Some(cover),
        "precondition: the cover is pinned"
    );

    // Click an empty, unblocked, unoccupied cell.
    let empty = CellLevel::new(Cell::new(1, 1), LEVEL);
    set_hovered(&mut app, Some(empty));
    press_left(&mut app);
    app.update();

    assert_eq!(pinned(&app), None, "clicking an empty tile must unpin");
    // After unpin, `effective()` is back in Hovered mode (the panel follows the cursor again).
    // The exact cell is left to the picker (which runs `.after` the click and, with no headless
    // camera, resolves None); the load-bearing fact is that it is NO LONGER Pinned.
    assert!(
        matches!(effective(&app), Some(InspectMode::Hovered(_))),
        "after unpin the panel follows hover (no longer Pinned)",
    );
}

// ---------------------------------------------------------------------------------
// Clause: clicking own ganger keeps the pin (does not clear).
// ---------------------------------------------------------------------------------

/// Clicking your OWN ganger SELECTS it AND KEEPS the pin — the two effects compose on one click
/// (the contract's "composes correctly"). Starting from a held pin, a click on a player ganger
/// sets `SelectedShooter` (the existing SELECT effect) while leaving the pin untouched.
#[test]
fn clicking_own_ganger_selects_and_keeps_the_pin() {
    let mut app = pin_app();

    // Pin on an enemy first.
    let enemy_cell = CellLevel::new(Cell::new(3, 9), LEVEL);
    place_enemy(&mut app, enemy_cell);
    set_hovered(&mut app, Some(enemy_cell));
    press_left(&mut app);
    app.update();
    assert_eq!(
        pinned(&app),
        Some(enemy_cell),
        "precondition: the enemy is pinned",
    );

    // Now click your OWN ganger.
    let own_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let own = place_player_ganger(&mut app, own_cell);
    set_hovered(&mut app, Some(own_cell));
    press_left(&mut app);
    app.update();

    assert_eq!(
        selected(&app),
        Some(own),
        "clicking your own ganger must SELECT it (the existing effect)",
    );
    assert_eq!(
        pinned(&app),
        Some(enemy_cell),
        "selecting your own ganger must KEEP the pin (not clear it)",
    );
}

// ---------------------------------------------------------------------------------
// Clause: clicking to fire keeps the pin (does not clear).
// ---------------------------------------------------------------------------------

/// Clicking to FIRE on an enemy KEEPS the pin: with a fire mode + a player selection, a click on
/// an enemy FIRES (the existing effect) and the pin lands on that enemy's cell (the pin rung
/// fires for any enemy click), never clearing. The contract: a fire click must not unpin.
#[test]
fn clicking_to_fire_keeps_the_pin() {
    let mut app = pin_app();
    set_fire_mode(&mut app);

    // A player shooter selected, an enemy in range/line — a fire click.
    let shooter_cell = CellLevel::new(Cell::new(5, 5), LEVEL);
    let shooter = spawn_player_shooter(&mut app, shooter_cell);
    set_selection(&mut app, shooter);

    let enemy_cell = CellLevel::new(Cell::new(5, 10), LEVEL);
    place_enemy(&mut app, enemy_cell);
    set_hovered(&mut app, Some(enemy_cell));

    press_left(&mut app);
    app.update();

    // The pin lands on the fired-upon enemy cell (the enemy rung), and is NOT cleared.
    assert_eq!(
        pinned(&app),
        Some(enemy_cell),
        "a fire click must leave the panel pinned on the enemy (never unpin)",
    );
}

// ---------------------------------------------------------------------------------
// Clause: hover no longer changes the panel while pinned (the behavioral view check).
// ---------------------------------------------------------------------------------

/// While PINNED, moving the cursor (the live hovered cell) does NOT change the panel's EFFECTIVE
/// target — the contract's required behavioral check for the view interaction. After a pin, the
/// picker keeps writing the hovered cell underneath, but `effective()` stays `Pinned`.
#[test]
fn hover_does_not_change_the_panel_while_pinned() {
    let mut app = pin_app();

    // Pin on cover.
    let cover = CellLevel::new(Cell::new(8, 5), LEVEL);
    place_cover(&mut app, cover);
    set_hovered(&mut app, Some(cover));
    press_left(&mut app);
    app.update();
    assert_eq!(
        effective(&app),
        Some(InspectMode::Pinned(cover)),
        "precondition: the panel is pinned on the cover cell",
    );

    // Release the click edge so the next update is a hover-only change, NOT another click (under
    // MinimalPlugins `ButtonInput` is never ticked, so a held just_pressed would re-fire).
    clear_mouse(&mut app);

    // The cursor now hovers a DIFFERENT cell (no click): the live hovered cell tracks it, but the
    // EFFECTIVE mode stays Pinned — the panel does not follow hover while pinned.
    let elsewhere = CellLevel::new(Cell::new(30, 30), LEVEL);
    assert_ne!(elsewhere, cover, "the cursor moved off the pinned cell");
    set_hovered(&mut app, Some(elsewhere));
    app.update();

    assert_eq!(
        effective(&app),
        Some(InspectMode::Pinned(cover)),
        "hover must NOT change the panel's effective target while pinned",
    );
}
