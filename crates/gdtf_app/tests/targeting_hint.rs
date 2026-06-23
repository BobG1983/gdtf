//! GTW-11 — the battlescape targeting-fog HINT, driven through the REAL app stack.
//!
//! This headless `GdtfTestAppBuilder` integration test drives the genuine state machine down to
//! `BattleScapeState::BattleRunning`, where the real targeting-hint plugin spawns its one Text
//! node and `update_targeting_hint` repaints it under the `BattleInProgress` gate from the SHARED
//! `cell_squad_visible` read. It proves C6(3):
//!
//! - hovering a NON-VISIBLE cell shows the hint reading EXACTLY the canon string
//!   `unseen — hold your fire` (verbatim em-dash + spacing);
//! - hovering a squad-VISIBLE cell HIDES the hint (and clears the text);
//! - an EXPLORED-not-VISIBLE cell is treated as non-VISIBLE (shown), DISTINCT from a VISIBLE cell
//!   (C5).
//!
//! The cursor→cell pick is the one external stubbed (`force_hover`, the `status_panel.rs`
//! precedent — it is tested in `gdtf_battle_input`); everything downstream is the real path. No
//! function here takes `&mut World`/`&World`.

use bevy::{ecs::entity::Entity, prelude::*, state::state::State, ui::Node};
use gdtf_app::test_support::{AppState, BattleScapeState, RunningState, TargetingHintText};
use gdtf_battle_input::{InputSystems, InspectTarget, pick_hovered_cell};
use gdtf_battle_sim::{
    Cell, CellLevel, Level, SquadVisibility, tuning::CombatTuning, weapon::WeaponRegistry,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// The canon targeting-fog hint string (`docs/combat/visibility.md` §"UX edges", VERBATIM).
const CANON_HINT: &str = "unseen — hold your fire";

/// A budget large enough to drive the deep walk into the battlescape, bounded so a machine that
/// never reaches the predicate fails instead of hanging.
const BUDGET: u32 = 96;

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Builds the headless walk app, injecting the persistent `Load` resources the machine needs to
/// traverse `Load` (the `status_panel.rs` precedent).
fn walk_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app
}

/// Drives the app from `Running`/`Menu` down to `BattleScapeState::BattleRunning`.
fn drive_to_battle_running(app: &mut App) -> bool {
    let at_menu = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    if !at_menu {
        return false;
    }
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(
        app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    )
}

/// A test-controlled desired hover cell, copied into `InspectTarget` AFTER the headless picker
/// runs (which would otherwise clobber an injected value to `None`) — the `status_panel.rs` seam.
#[derive(Resource, Clone, Copy, Default)]
struct DesiredHover(Option<CellLevel>);

/// Copies [`DesiredHover`] into [`InspectTarget`]'s live hovered cell — registered
/// `.after(pick_hovered_cell)` in `InputSystems::Gather`, so it is the LAST hovered-cell writer of
/// the frame and the `.after(Gather)` hint update reads it.
fn force_hover(desired: Res<DesiredHover>, mut target: ResMut<InspectTarget>) {
    target.set_hovered(desired.0);
}

/// Drives the walk to `BattleRunning` (asserting the descent) and wires the hover-forcing seam.
fn hint_app() -> App {
    let mut app = walk_app();
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleScapeState::BattleRunning within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    app.world_mut().insert_resource(DesiredHover::default());
    app.add_systems(
        Update,
        force_hover
            .in_set(InputSystems::Gather)
            .after(pick_hovered_cell),
    );
    app
}

/// Sets the desired hover cell and steps one update so the real `update_targeting_hint` reads it.
fn hover(app: &mut App, cell: Option<CellLevel>) {
    app.world_mut().insert_resource(DesiredHover(cell));
    app.update();
}

/// Inserts a [`SquadVisibility`] with `visible` cells VISIBLE and `explored` cells EXPLORED
/// (EXPLORED auto-includes VISIBLE per the accrual invariant).
fn seed_fog(app: &mut App, visible: &[CellLevel], explored: &[CellLevel]) {
    let visible: bevy::platform::collections::HashSet<CellLevel> =
        visible.iter().copied().collect();
    let mut explored_set: bevy::platform::collections::HashSet<CellLevel> =
        explored.iter().copied().collect();
    explored_set.extend(visible.iter().copied());
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible, explored_set));
}

/// The single targeting-hint Text node, if exactly one exists.
fn hint_entity(app: &mut App) -> Option<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<TargetingHintText>>();
    match q.iter(app.world()).collect::<Vec<_>>().as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// The hint node's rendered text + visibility, if it exists.
fn hint_state(app: &mut App) -> Option<(String, Visibility)> {
    let entity = hint_entity(app)?;
    let text = app.world().get::<Text>(entity)?.as_str().to_owned();
    let visibility = *app.world().get::<Visibility>(entity)?;
    Some((text, visibility))
}

/// C6(3) — the hint reads EXACTLY the canon string + is Visible on a non-VISIBLE hovered cell, and
/// is hidden (text cleared) on a VISIBLE cell; an EXPLORED-not-VISIBLE cell shows it (DISTINCT
/// from VISIBLE, C5). Driven by the SAME `cell_squad_visible` read the reticle + the fire-refusal
/// consume.
#[test]
fn targeting_hint_reads_canon_string_on_non_visible_cell() {
    let mut app = hint_app();

    // The hint Text node exists (spawned OnEnter(BattleRunning)) and starts hidden + empty.
    assert!(
        hint_entity(&mut app).is_some(),
        "the targeting-hint Text node must spawn on entering BattleRunning",
    );

    let unseen = CellLevel::new(Cell::new(7, 7), Level::new(0));
    let explored = CellLevel::new(Cell::new(8, 8), Level::new(0));
    let visible = CellLevel::new(Cell::new(9, 9), Level::new(0));
    // `visible` is VISIBLE; `explored` is EXPLORED-not-VISIBLE; `unseen` is in neither set.
    seed_fog(&mut app, &[visible], &[explored]);

    // --- UNSEEN: the hint shows the EXACT canon string. ---
    hover(&mut app, Some(unseen));
    let (text, vis) = hint_state(&mut app).unwrap_or_else(|| (String::new(), Visibility::Hidden));
    assert_eq!(
        text, CANON_HINT,
        "an UNSEEN hovered cell must show the EXACT canon string",
    );
    assert_eq!(
        vis,
        Visibility::Visible,
        "an UNSEEN hovered cell must show the hint (Visible)",
    );

    // --- VISIBLE: the hint hides + clears (DISTINCT from the UNSEEN case). ---
    hover(&mut app, Some(visible));
    let (text, vis) =
        hint_state(&mut app).unwrap_or_else(|| ("STILL-SHOWN".to_owned(), Visibility::Visible));
    assert_eq!(
        vis,
        Visibility::Hidden,
        "a VISIBLE hovered cell must HIDE the hint",
    );
    assert!(
        text.is_empty(),
        "a VISIBLE hovered cell must CLEAR the hint text, got {text:?}",
    );

    // --- EXPLORED-not-VISIBLE: treated as non-VISIBLE — shows the canon string (C5). ---
    hover(&mut app, Some(explored));
    let (text, vis) = hint_state(&mut app).unwrap_or_else(|| (String::new(), Visibility::Hidden));
    assert_eq!(
        text, CANON_HINT,
        "an EXPLORED-not-VISIBLE cell is non-VISIBLE — it must show the canon string (C5)",
    );
    assert_eq!(
        vis,
        Visibility::Visible,
        "an EXPLORED-not-VISIBLE cell must show the hint (refused like UNSEEN, C5)",
    );

    // --- Nothing hovered: the hint hides. ---
    hover(&mut app, None);
    let (_, vis) = hint_state(&mut app).unwrap_or_else(|| (String::new(), Visibility::Visible));
    assert_eq!(
        vis,
        Visibility::Hidden,
        "no hovered cell must HIDE the hint",
    );
}

/// Pin the canon string is laid out as a real UI Text node (a `Node` is present), so the hint is a
/// genuine rendered label — not a detached `Text` with no layout (`bevy-traps.md` #8 lineage).
#[test]
fn targeting_hint_is_a_laid_out_ui_node() {
    let mut app = hint_app();
    let entity = hint_entity(&mut app);
    assert!(
        entity.is_some(),
        "the hint node must exist in BattleRunning"
    );
    let has_node = entity.is_some_and(|e| app.world().get::<Node>(e).is_some());
    assert!(
        has_node,
        "the hint must be a UI Node (laid out), not a detached Text",
    );
}
