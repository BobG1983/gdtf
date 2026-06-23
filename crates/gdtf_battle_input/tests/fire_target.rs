//! GTW-371 (C2 / C4): headless integration tests for the input-crate `populate_fire_target`
//! system — it POPULATES the presenter-owned `FireTargetHighlight` when the SELECTED player
//! shooter hovers a squad-VISIBLE ENEMY it could fire on (mirroring `decide_left_click`'s FIRE
//! rung + the GTW-346 fog gate), exposing the `mode_tu_cost` the shot would charge, and is WIRED
//! in the input plugin.
//!
//! POSITIVE — every assertion NAMES the cell + computes the expected cost from `mode_tu_cost`
//! (no magnitude pin):
//!
//! - C2 / C4b (the producer): hovering a fireable enemy fills `FireTargetHighlight` with the
//!   hovered cell + a cost EXACTLY equal to `mode_tu_cost(SelectedFireMode, TuMax, Aiming,
//!   CombatTuning)` (computed independently in-test). Selection / fire-mode / hover are never
//!   written into the sim.
//! - C4c (clears): NOT hovering a fireable enemy (empty cell / own ganger / non-visible enemy /
//!   no selection) clears the highlight (empty).
//!
//! The click systems run `.before(pick_hovered_cell)`, so an INJECTED `InspectTarget` is read
//! that update before the (headless, camera-less) picker clobbers it to `None`. Every
//! `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom (`bevy-traps.md`
//! #7 carve-out (a)).

use bevy::{input::ButtonInput, platform::collections::HashSet, prelude::*};
use gdtf_battle_input::{GdtfBattleInputPlugin, InspectTarget, SelectedFireMode, SelectedShooter};
use gdtf_battle_presenter::{ActiveLevel, FireTargetHighlight};
use gdtf_battle_sim::{
    Aiming, BattleInProgress, Cell, CellLevel, Faction, FireModeSpec, Level, LifeState, Magazine,
    MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, OccupancyGrid, PlayerFaction,
    Position, ReloadTu, SquadVisibility, Tu, TuMax, VerticalLinkGraph, WieldedBy, mode_tu_cost,
    tuning::CombatTuning,
};

/// The faction the player controls (matches the inserted `PlayerFaction`).
const PLAYER_FACTION: Faction = Faction::new(0);
/// An ENEMY faction (distinct from [`PLAYER_FACTION`]) — the fireable-target faction.
const ENEMY_FACTION: Faction = Faction::new(1);
/// The level the tests run on (the default `ActiveLevel`).
const LEVEL: Level = Level::new(0);

/// A `Single`-kind fire-mode spec with a marker `tu_percent` (arbitrary, not pinned tuning) — so
/// the expected cost is computed from `mode_tu_cost`, never asserted as a shipped magnitude.
const fn spec(tu_percent: f32) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(1),
    )
}

/// Builds the focused headless app: `MinimalPlugins` + the real `GdtfBattleInputPlugin`, plus the
/// resources `populate_fire_target` reads (all-Open occupancy, default tuning, the player
/// faction), the `BattleInProgress` gate witness, an empty fog (so the test marks enemy cells
/// VISIBLE explicitly), and the presenter-owned `FireTargetHighlight` + `ActiveLevel` (which the
/// presenter plugin would normally init — inserted here since this focused harness adds no
/// renderer plugin).
fn fire_target_app() -> App {
    let mut app = App::new();
    // `update_selection_highlight` spawns its reticle via `spawn_scene`, which needs an
    // `AssetServer` + the scene schedule (the GTW-322 spike requirement, the control.rs
    // precedent).
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ))
    .add_plugins(GdtfBattleInputPlugin);
    let w = app.world_mut();
    w.insert_resource(ActiveLevel::new(LEVEL));
    w.insert_resource(BattleInProgress);
    w.insert_resource(OccupancyGrid::default());
    w.insert_resource(VerticalLinkGraph::default());
    w.insert_resource(CombatTuning::default());
    w.insert_resource(PlayerFaction::new(PLAYER_FACTION));
    w.insert_resource(ButtonInput::<MouseButton>::default());
    w.insert_resource(FireTargetHighlight::cleared());
    // An empty fog — `place_enemy` marks the enemy cell VISIBLE so the fog gate passes.
    w.insert_resource(SquadVisibility::new(HashSet::default(), HashSet::default()));
    app
}

/// Spawns an armed, alive PLAYER-faction shooter at `cell` (carrying the `TuMax` / `Aiming` the
/// cost reads + a weapon entity with a magazine), selects it, and returns its entity + its TU max
/// / aiming so the test can compute the expected cost independently.
fn spawn_and_select_shooter(app: &mut App, cell: CellLevel) -> (Entity, TuMax, Aiming) {
    let tu_max = TuMax::new(100);
    let aiming = Aiming::new(false);
    let ganger = app
        .world_mut()
        .spawn((
            PLAYER_FACTION,
            Position::new(cell),
            aiming,
            LifeState::Alive,
            Tu::new(255),
            tu_max,
        ))
        .id();
    app.world_mut().spawn((
        WieldedBy(ganger),
        Magazine::new(10, MagazineSize::new(30), ReloadTu::new(12)),
    ));
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    (ganger, tu_max, aiming)
}

/// Places an ENEMY occupant at `cell` and marks the cell squad-VISIBLE (the realistic
/// fire-on-a-seen-enemy state), and returns its entity.
fn place_enemy(app: &mut App, cell: CellLevel) -> Entity {
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(enemy));
    mark_visible(app, cell);
    enemy
}

/// Marks `cell` squad-VISIBLE (accruing onto the current fog).
fn mark_visible(app: &mut App, cell: CellLevel) {
    let mut visible: HashSet<CellLevel> = app
        .world()
        .get_resource::<SquadVisibility>()
        .map(|fog| fog.visible_cells().copied().collect())
        .unwrap_or_default();
    visible.insert(cell);
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible.clone(), visible));
}

/// Injects the live hovered cell (read by the populate system before the headless picker clobbers
/// it).
fn set_hovered(app: &mut App, cell: Option<CellLevel>) {
    app.world_mut().insert_resource(InspectTarget::new(cell));
}

/// Forces the selected fire mode.
fn set_fire_mode(app: &mut App, mode: FireModeSpec) {
    app.world_mut().insert_resource(SelectedFireMode::new(mode));
}

/// The current `FireTargetHighlight` after an update.
fn highlight(app: &App) -> FireTargetHighlight {
    *app.world().resource::<FireTargetHighlight>()
}

/// C2 / C4b (positive) — hovering a fireable enemy populates the highlight with the hovered cell +
/// a cost EXACTLY equal to `mode_tu_cost` (computed independently in-test, no magnitude pin).
#[test]
fn hovering_fireable_enemy_populates_cell_and_mode_tu_cost() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let enemy_cell = CellLevel::new(Cell::new(13, 11), LEVEL);
    let mode = spec(0.2);

    let (_shooter, tu_max, aiming) = spawn_and_select_shooter(&mut app, shooter_cell);
    place_enemy(&mut app, enemy_cell);
    set_fire_mode(&mut app, mode);
    set_hovered(&mut app, Some(enemy_cell));

    // The expected cost, computed the SAME way the shot will charge it (REUSE, no magnitude pin).
    let tuning = app.world().resource::<CombatTuning>();
    let expected_cost = mode_tu_cost(&mode, &tu_max, &aiming, tuning);

    app.update();

    let h = highlight(&app);
    assert_eq!(
        h.cell(),
        Some(enemy_cell),
        "hovering a fireable enemy populates the highlight with the hovered cell (13,11,L0)",
    );
    assert_eq!(
        h.cost(),
        Some(expected_cost),
        "the highlight cost EXACTLY equals mode_tu_cost(SelectedFireMode, TuMax, Aiming, tuning)",
    );
}

/// C4c — NOT hovering a fireable enemy clears the highlight: an EMPTY cell yields no fire target
/// even with a selection.
#[test]
fn empty_cell_clears_highlight() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let empty_cell = CellLevel::new(Cell::new(20, 20), LEVEL);

    spawn_and_select_shooter(&mut app, shooter_cell);
    set_fire_mode(&mut app, spec(0.2));
    // Seed a stale highlight, then hover an EMPTY cell.
    app.world_mut().insert_resource(FireTargetHighlight::new(
        CellLevel::new(Cell::new(5, 5), LEVEL),
        Tu::new(9),
    ));
    set_hovered(&mut app, Some(empty_cell));

    app.update();

    assert!(
        highlight(&app).is_empty(),
        "hovering an EMPTY cell clears the fire-target highlight (no fireable enemy there)",
    );
}

/// C4c — hovering your OWN ganger clears the highlight (you cannot fire on your own).
#[test]
fn own_ganger_clears_highlight() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);

    spawn_and_select_shooter(&mut app, shooter_cell);
    set_fire_mode(&mut app, spec(0.2));
    // Mark the shooter's own cell visible so only the friend/foe gate (not the fog) decides it.
    mark_visible(&mut app, shooter_cell);
    set_hovered(&mut app, Some(shooter_cell));

    app.update();

    assert!(
        highlight(&app).is_empty(),
        "hovering your OWN ganger clears the highlight (an own-faction occupant is not fireable)",
    );
}

/// C4c — hovering an enemy on a NON-VISIBLE (unseen) cell clears the highlight (the GTW-346 fog
/// gate: you cannot target what the squad cannot see).
#[test]
fn non_visible_enemy_clears_highlight() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let enemy_cell = CellLevel::new(Cell::new(30, 30), LEVEL);

    spawn_and_select_shooter(&mut app, shooter_cell);
    set_fire_mode(&mut app, spec(0.2));
    // Spawn the enemy occupant but DO NOT mark its cell visible — it stays unseen.
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(enemy_cell, Some(enemy));
    set_hovered(&mut app, Some(enemy_cell));

    app.update();

    assert!(
        highlight(&app).is_empty(),
        "hovering an enemy on a NON-VISIBLE cell clears the highlight (GTW-346 fog gate)",
    );
}

/// C4c — with NO selection, hovering even a visible enemy clears the highlight (there is no
/// shooter to fire).
#[test]
fn no_selection_clears_highlight() {
    let mut app = fire_target_app();
    let enemy_cell = CellLevel::new(Cell::new(13, 11), LEVEL);

    place_enemy(&mut app, enemy_cell);
    set_fire_mode(&mut app, spec(0.2));
    // No SelectedShooter set (the plugin inits it to the cleared default).
    app.world_mut().insert_resource(SelectedShooter::cleared());
    set_hovered(&mut app, Some(enemy_cell));

    app.update();

    assert!(
        highlight(&app).is_empty(),
        "with NO selected shooter, hovering an enemy populates no fire target",
    );
}
