//! GTW-282 AC1 — the REAL menu→Load→battle path, with the REAL shipped
//! `skirmish.ron`, drives a player ganger the status panel can actually read.
//!
//! This is the coverage gap that let the `TuMax` defect ship twice: every prior
//! status-panel / selection test either hand-spawned gangers (inserting `TuMax`
//! directly) or seeded an inline `LoadedSituation` under `MinimalPlugins` — none drove
//! the real `skirmish.ron` through `setup_battle` and then read the panel. A real ganger
//! authored without `tu_max` spawned without a `TuMax` component, so the status panel's
//! non-optional `Vitals` query failed `.get`, painting every line "No ganger selected"
//! even though auto-select correctly held the player ganger.
//!
//! The harness is the real-asset [`GdtfLoadTestAppBuilder`] (`DefaultPlugins`,
//! `backends: None`, a live `AssetServer` rooted at the workspace `assets/`), started at
//! [`AppState::Load`] so the REAL situation/theme/tuning/weapon loads run. It then drives
//! the real state machine: down to [`RunningState::Menu`], a `NextState<RunningState>` →
//! [`RunningState::Game`] (the Battlescape button's mapped target — `menu/systems/actions.rs`
//! maps `BATTLESCAPE` → `RunningState::Game`), and on to
//! [`BattleScapeState::BattleRunning`], where the Generation `setup_battle` poured the real
//! gangers and `auto_select_first_player_ganger` selected the player one.
//!
//! `app.world_mut()` / `app.world()` calls are all in the TEST BODY (the accepted headless
//! idiom, `bevy-traps.md` #7 carve-out (a)); no function here takes `&mut World`.

use bevy::{app::App, prelude::*, state::state::State};
use gdtf_app::test_support::{AppState, BattleScapeState, RunningState, StanceText, TuText};
use gdtf_battle_input::SelectedShooter;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

/// A generous budget: the real `DefaultPlugins` async asset loads (situation + theme +
/// tuning + weapons + the presenter's tile sheets / role tables, all sharing the
/// `AssetServer`) plus the full state descent need many headless `update()` polls under
/// contention. Bounded so a machine that never reaches the predicate fails instead of
/// hanging (the `load_situation.rs` `LOAD_BUDGET` precedent).
const BUDGET: u32 = 512;

/// The status panel's no-selection empty-state copy (`status_panel/.../labels/format.rs`
/// `NO_SELECTION`) — the exact string every line is painted when the `Vitals` read fails.
/// The bug symptom is the stance/TU line equalling this; the fix makes it NOT this.
const NO_SELECTION: &str = "No ganger selected";

/// Reads the current [`AppState`].
fn app_state(app: &App) -> AppState {
    app.world().resource::<State<AppState>>().get().clone()
}

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the rendered `Text` of the single entity carrying line-marker `M` (the
/// `status_panel.rs` `line_text` idiom).
fn line_text<M: Component>(app: &mut App) -> Option<String> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    let found: Vec<Entity> = q.iter(app.world()).collect();
    let [entity] = found.as_slice() else {
        return None;
    };
    app.world()
        .get::<Text>(*entity)
        .map(|t| t.as_str().to_owned())
}

/// Whether `SelectedShooter` currently holds a selection.
fn has_selection(app: &App) -> bool {
    app.world()
        .get_resource::<SelectedShooter>()
        .is_some_and(|s| (**s).is_some())
}

/// AC1 — the real `skirmish.ron` battle shows a SELECTABLE player ganger the panel reads.
///
/// Drives the REAL menu→Load→battle path with the REAL shipped situation and asserts:
/// (a) `SelectedShooter` is `Some` (a true auto-select regression would leave it empty),
/// and (b) the status panel's stance + TU lines do NOT show "No ganger selected" and DO
/// show the ganger's stance plus a `cur/max` TU pair — asserting the current TU value's
/// presence and the `/` separator, NOT a brittle exact max.
///
/// Pin-discriminating: RED before the fix (the real ganger lacked `TuMax`, so the panel's
/// `Vitals.get` failed closed and painted `NO_SELECTION`), GREEN after (the authored
/// `tu_max` spawns a `TuMax` the `Vitals` query reads). This real-asset menu→battle→panel
/// chain is what no existing test drove.
#[test]
fn real_skirmish_battle_shows_a_selectable_readable_player_ganger() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Drive the real Load (loads skirmish.ron + theme/tuning/weapons) and the auto-descent
    // (Load → Intro → Running) down to the menu resting in RunningState::Menu.
    let reached_menu = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    assert!(
        reached_menu,
        "the real Load + descent must reach RunningState::Menu within {BUDGET} updates; last \
         AppState was {:?}, RunningState {:?}",
        app_state(&app),
        running_state(&app),
    );

    // Take the Battlescape button's mapped path: BATTLESCAPE → RunningState::Game
    // (`menu/systems/actions.rs`). Headless, a queued NextState is that path's effect.
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);

    // Descend into the battle: Generation's setup_battle pours the real skirmish gangers,
    // then auto-select picks the player ganger, and the panel repaints from its vitals.
    let reached_battle = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        reached_battle,
        "the real battle must reach BattleScapeState::BattleRunning within {BUDGET} updates; last \
         BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // (a) auto-select holds the real player ganger (a true auto-select regression would
    // leave SelectedShooter empty even with the TuMax fix).
    assert!(
        has_selection(&app),
        "auto_select_first_player_ganger must hold the real skirmish player ganger in \
         SelectedShooter",
    );

    // (b) the stance line reads the real ganger's posture — NOT the no-selection empty state.
    let stance = line_text::<StanceText>(&mut app);
    assert!(
        stance.is_some(),
        "the status panel must carry a StanceText line"
    );
    let Some(stance) = stance else { return };
    assert_ne!(
        stance, NO_SELECTION,
        "the stance line must NOT read the no-selection empty state — the panel can read the \
         selected ganger's vitals (got {stance:?})",
    );
    assert!(
        stance.starts_with("Stance: "),
        "the stance line must show the ganger's posture (got {stance:?})",
    );

    // (b) the TU line shows a cur/max pair — assert the CURRENT value + the `/` separator,
    // never a brittle exact max (the skirmish player ganger is authored tu/tu_max = 60).
    let tu = line_text::<TuText>(&mut app);
    assert!(tu.is_some(), "the status panel must carry a TuText line");
    let Some(tu) = tu else { return };
    assert_ne!(
        tu, NO_SELECTION,
        "the TU line must NOT read the no-selection empty state (got {tu:?})",
    );
    assert!(
        tu.contains('/'),
        "the TU line must show a cur/max pair separated by '/' (got {tu:?})",
    );
    assert!(
        tu.contains("60"),
        "the TU line must show the authored current TU value 60 (got {tu:?})",
    );
}
