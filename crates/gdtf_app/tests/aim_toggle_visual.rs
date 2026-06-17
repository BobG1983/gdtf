//! GTW-253 — the Aim action-bar button's active (toggled-on) visual, driven through
//! the REAL app stack.
//!
//! These headless `GdtfTestAppBuilder` integration tests drive the genuine state
//! machine down to `BattleScapeState::BattleRunning`, where the real action-bar
//! plugin's `OnEnter(BattleRunning)` `spawn_action_bar` spawns the `AimToggleButton`
//! and its `Update` `sync_aim_button_active` (gated on `BattleInProgress`) mirrors the
//! selected ganger's `Aiming` onto the `gdtf_ui` `ActiveButton` paint marker. They cover
//! AC4:
//!
//! - a selected ganger with `Aiming(true)` → the `AimToggleButton` HAS `ActiveButton`;
//! - flipping to `Aiming(false)` → it does NOT;
//! - no selection → no `ActiveButton`.
//!
//! Discriminating: the marker is read off the REAL `AimToggleButton` entity the bar
//! spawns and the REAL `Aiming` sim component, so a sync wired to the wrong
//! component/entity would surface the wrong result. The `gdtf_ui` paint mechanism (AC1)
//! and interaction/disabled rules (AC2/AC3) live as `gdtf_ui` in-crate tests; the theme
//! round-trip (AC5) lives in `gdtf_ui::theme`.
//!
//! Every `app.world_mut()` mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)). No function here takes `&mut World`/`&World`.

use bevy::{ecs::entity::Entity, prelude::*, state::state::State};
use gdtf_app::test_support::{AimToggleButton, AppState, BattleScapeState, RunningState};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    Aiming, Cell, CellLevel, Faction, Level, Position, tuning::CombatTuning, weapon::WeaponRegistry,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::{ActiveButton, theme::default_theme};

/// A budget large enough to drive the deep walk into the battlescape, bounded so a
/// machine that never reaches the predicate fails instead of hanging (the
/// `action_bar.rs` / `status_panel.rs` budget).
const BUDGET: u32 = 96;

// ---------------------------------------------------------------------------------
// Harness — drive the real stack to BattleRunning, where the bar is live.
// ---------------------------------------------------------------------------------

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

/// Builds the headless walk app, injecting the persistent `Load` resources the machine
/// needs to traverse `Load` (no `AssetServer` under `MinimalPlugins`) — `default_theme()`
/// (which `spawn_action_bar` reads) + `CombatTuning`. No `LoadedSituation` → the empty
/// `Situation::default()` battle is set up, which still makes `BattleInProgress` present
/// in `BattleRunning` (the sync system's gate) and spawns NO gangers (so the test owns
/// the only gangers). The `action_bar.rs` harness precedent.
fn walk_app() -> App {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // GTW-257: the Load->Intro gate also requires a WeaponRegistry (empty-default
    // situation here, so an empty registry clears the gate).
    app.world_mut().insert_resource(WeaponRegistry::default());
    app
}

/// Drives the app from `Running`/`Menu` down to the first update on which
/// [`BattleScapeState::BattleRunning`] is active. Returns whether it was reached.
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

/// Drives the walk to `BattleRunning` and returns the app, asserting the descent
/// succeeded (so each test starts from the live battle where the bar is spawned).
fn battle_running_app() -> App {
    let mut app = walk_app();
    assert!(
        drive_to_battle_running(&mut app),
        "the walk should reach BattleScapeState::BattleRunning within {BUDGET} updates; last \
         observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    app
}

/// Looks up the single entity carrying marker `M`, if exactly one exists (the
/// `action_bar.rs` `single_with` idiom).
fn single_with<M: Component>(app: &mut App) -> Option<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    let found: Vec<Entity> = q.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// Whether the single `AimToggleButton` entity currently carries the `gdtf_ui`
/// `ActiveButton` paint marker. Returns `false` if the marker is absent and panics
/// nowhere — it is read off the REAL bar button entity.
fn aim_button_is_active(app: &mut App) -> bool {
    let Some(button) = single_with::<AimToggleButton>(app) else {
        return false;
    };
    app.world().get::<ActiveButton>(button).is_some()
}

/// Spawns a ganger with the given faction and an [`Aiming`] component, and SELECTS it
/// via the `SelectedShooter` resource (the selection `sync_aim_button_active` reads).
/// Setting the resource directly is the faithful minimal selection for this view test
/// (the cursor-click selection path is covered in `gdtf_battle_input`). Returns its
/// entity. A `Position` is included so the landed GTW-255 auto-select (which reads
/// `&Position`) tolerates the ganger.
fn spawn_and_select(app: &mut App, faction: Faction, aiming: bool) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((
            Position::new(CellLevel::new(Cell::new(3, 3), Level::new(0))),
            faction,
            Aiming::new(aiming),
        ))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    ganger
}

// ---------------------------------------------------------------------------------
// AC4 — the Aim button reflects the selected ganger's `Aiming`.
// ---------------------------------------------------------------------------------

/// AC4 — a selected (player-faction) ganger with `Aiming(true)` → after `update()` the
/// `AimToggleButton` HAS `ActiveButton`; flipping to `Aiming(false)` → after `update()`
/// it does NOT.
///
/// Pin-discriminating: a sync wired to the wrong component (or the wrong entity) would
/// not flip the marker with the ganger's aim, so the second assert would still see the
/// active marker.
#[test]
fn aim_button_active_follows_selected_ganger_aiming() {
    let mut app = battle_running_app();

    // Player faction (0 = the default `PlayerFaction`) so the selection is the natural
    // controllable one; `Aiming(true)`.
    let ganger = spawn_and_select(&mut app, Faction::new(0), true);
    app.update();
    assert!(
        aim_button_is_active(&mut app),
        "with the selected ganger aiming, the Aim button must carry ActiveButton",
    );

    // Flip the selected ganger to NOT aiming on the real component.
    if let Some(mut aiming) = app.world_mut().get_mut::<Aiming>(ganger) {
        *aiming = Aiming::new(false);
    }
    app.update();
    assert!(
        !aim_button_is_active(&mut app),
        "with the selected ganger no longer aiming, the Aim button must NOT carry \
         ActiveButton",
    );
}

/// AC4 — with NO selection, the Aim button does NOT carry `ActiveButton` (it shows OFF)
/// and `update()` does not panic.
///
/// The previously-shown ganger is an ENEMY faction (1, distinct from the default
/// `PlayerFaction` 0) so the landed GTW-255 `auto_select_first_player_ganger` (which
/// fills an EMPTY selection with the first PLAYER-faction ganger) does NOT re-select it
/// on clear — isolating the sync's no-selection behaviour. A direct `SelectedShooter`
/// write bypasses the player-faction SELECT gate, so the enemy ganger can still be
/// force-shown aiming first, proving the marker then CLEARS on deselect (no stale ON).
#[test]
fn no_selection_clears_active_marker() {
    let mut app = battle_running_app();

    // Force-select an aiming ENEMY-faction ganger so the marker is ON first.
    spawn_and_select(&mut app, Faction::new(1), true);
    app.update();
    assert!(
        aim_button_is_active(&mut app),
        "the force-selected aiming ganger shows the Aim button active before clearing",
    );

    // Clear the selection. Auto-select will not re-pick the enemy ganger, and there is
    // no player-faction ganger, so the selection stays empty → the marker clears.
    app.world_mut().insert_resource(SelectedShooter::cleared());
    app.update();
    assert!(
        !aim_button_is_active(&mut app),
        "with no selection, the Aim button must NOT carry ActiveButton (no stale ON state)",
    );
}
