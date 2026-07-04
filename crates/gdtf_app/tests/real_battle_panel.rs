//! GTW-282 AC1 — the REAL menu→Load→battle path, with the REAL shipped `skirmish.ron`,
//! drives a player ganger the status panel can actually read.
//!
//! This is the coverage gap that let the `TuMax` defect ship twice: every prior
//! status-panel / selection test either hand-spawned gangers (inserting `TuMax` directly)
//! or seeded an inline `LoadedSituation` under `MinimalPlugins` — none drove the real
//! `skirmish.ron` through `setup_battle` and then read the panel. A real ganger authored
//! without `tu_max` spawned without a `TuMax` component, blanking the panel.
//!
//! GTW-278 reworked the panel to the shared stat block (the TU/HP are `ProgressBar`s, the
//! name/stance are `Text`), so this regression guard now reads the stat-block widgets: the
//! stance/name lines must NOT be the empty state, and the TU bar must have a NON-zero fill
//! (a real ganger lacking `TuMax` would render an empty bar — the same defect, now on the
//! bar instead of the text line).
//!
//! The harness is the real-asset [`GdtfLoadTestAppBuilder`] (`DefaultPlugins`, a live
//! `AssetServer` rooted at the workspace `assets/`), started at [`AppState::Load`] so the
//! REAL situation/theme/tuning/weapon loads run, then driven to
//! [`BattleScapeState::BattleRunning`].
//!
//! `app.world_mut()` / `app.world()` calls are all in the TEST BODY (the accepted headless
//! idiom, `bevy-traps.md` #7 carve-out (a)); no function here takes `&mut World`.

use bevy::{app::App, prelude::*, state::state::State, ui::Val};
use gdtf_app::test_support::{
    AppState, BattleScapeState, InspectPanelRoot, RunningState, StatName, StatStance, StatTuBar,
    app_state,
};
use gdtf_battle_input::SelectedShooter;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};
use gdtf_ui::ProgressBarFill;

/// A generous budget for the real `DefaultPlugins` async asset loads + the full state
/// descent under contention (the `load_situation.rs` `LOAD_BUDGET` precedent).
const BUDGET: u32 = 512;

/// The stat block's no-target empty-state copy (`stat_block/.../update.rs` `NO_TARGET`) —
/// the string the name line is painted when there is no selection.
const NO_TARGET: &str = "No ganger selected";

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

/// Whether `entity` has an ancestor carrying marker `R` (walks the `ChildOf` chain up).
fn descends_from<R: Component>(app: &App, entity: Entity) -> bool {
    let mut current = entity;
    loop {
        if app.world().get::<R>(current).is_some() {
            return true;
        }
        match app.world().get::<ChildOf>(current) {
            Some(parent) => current = parent.parent(),
            None => return false,
        }
    }
}

/// The single STATUS-panel entity carrying marker `M` (NOT a descendant of the inspect panel —
/// the two panels share the stat-block markers, so this discriminates the status panel's).
fn status_entity<M: Component>(app: &mut App) -> Option<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    let all: Vec<Entity> = q.iter(app.world()).collect();
    let found: Vec<Entity> = all
        .into_iter()
        .filter(|&e| !descends_from::<InspectPanelRoot>(app, e))
        .collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// Reads the rendered `Text` of the single STATUS-panel entity carrying line-marker `M`.
fn line_text<M: Component>(app: &mut App) -> Option<String> {
    let entity = status_entity::<M>(app)?;
    app.world()
        .get::<Text>(entity)
        .map(|t| t.as_str().to_owned())
}

/// The fill PERCENT of the STATUS-panel `ProgressBar` carrying track-marker `M` — reads the
/// `ProgressBarFill` child's `Node.width` `Val::Percent`. `None` if the bar / fill is missing.
fn bar_fill_percent<M: Component>(app: &mut App) -> Option<f32> {
    let track = status_entity::<M>(app)?;
    let kids: Vec<Entity> = app
        .world()
        .get::<Children>(track)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    for kid in kids {
        if app.world().get::<ProgressBarFill>(kid).is_some()
            && let Some(node) = app.world().get::<Node>(kid)
            && let Val::Percent(p) = node.width
        {
            return Some(p);
        }
    }
    None
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
/// (a) `SelectedShooter` is `Some`, (b) the stat block's stance + name lines do NOT show the
/// empty state, and (c) the TU `ProgressBar` has a NON-zero fill — the GTW-282 regression
/// guard, now on the bar (a real ganger lacking `TuMax` would render an empty bar).
#[test]
fn real_skirmish_battle_shows_a_selectable_readable_player_ganger() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

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

    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);

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

    // (a) auto-select holds the real player ganger.
    assert!(
        has_selection(&app),
        "auto_select_first_player_ganger must hold the real skirmish player ganger in \
         SelectedShooter",
    );

    // (b) the name + stance lines read the real ganger — NOT the empty state.
    let name = line_text::<StatName>(&mut app);
    assert!(name.is_some(), "the stat block must carry a name line");
    let Some(name) = name else { return };
    assert_ne!(
        name, NO_TARGET,
        "the name line must NOT read the no-target empty state — the panel can read the \
         selected ganger (got {name:?})",
    );

    let stance = line_text::<StatStance>(&mut app);
    assert!(stance.is_some(), "the stat block must carry a stance line");
    let Some(stance) = stance else { return };
    assert!(
        stance.starts_with("Stance: "),
        "the stance line must show the ganger's posture (got {stance:?})",
    );

    // (c) the TU bar has a NON-zero fill — the regression guard (a real ganger missing
    // `TuMax` would render an empty 0% bar). The skirmish player ganger is authored full
    // tu/tu_max, so the fill is 100%.
    let tu_fill = bar_fill_percent::<StatTuBar>(&mut app);
    assert!(
        tu_fill.is_some(),
        "the stat block must carry a TU ProgressBar"
    );
    let Some(tu_fill) = tu_fill else { return };
    assert!(
        tu_fill > 0.0,
        "the TU bar must have a non-zero fill — the panel reads the ganger's Tu/TuMax (got \
         {tu_fill}%)",
    );
}
