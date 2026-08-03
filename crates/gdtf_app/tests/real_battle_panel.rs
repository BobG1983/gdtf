//! `skirmish.ron` through `setup_battle` and then read the panel. A real ganger authored
use bevy::{app::App, prelude::*, state::state::State, ui::Val};
use gdtf_app::test_support::{
    AppState, BattleScapeState, InspectPanelRoot, RunningState, StatName, StatStance, StatTuBar,
    app_state,
};
use gdtf_battle_input::SelectedShooter;
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};
use gdtf_ui::ProgressBarFill;

const BUDGET: u32 = 512;

const NO_TARGET: &str = "No ganger selected";

fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

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

fn line_text<M: Component>(app: &mut App) -> Option<String> {
    let entity = status_entity::<M>(app)?;
    app.world()
        .get::<Text>(entity)
        .map(|t| t.as_str().to_owned())
}

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

fn has_selection(app: &App) -> bool {
    app.world()
        .get_resource::<SelectedShooter>()
        .is_some_and(|s| (**s).is_some())
}

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

    assert!(
        has_selection(&app),
        "auto_select_first_player_ganger must hold the real skirmish player ganger in \
         SelectedShooter",
    );

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
