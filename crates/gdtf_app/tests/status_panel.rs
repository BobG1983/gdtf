//! GTW-252 — the battlescape status HUD panel, driven through the REAL app stack.
//!
//! These headless `GdtfTestAppBuilder` integration tests drive the genuine state
//! machine down to `BattleScapeState::BattleRunning`, where the real status-panel
//! plugin's `OnEnter(BattleRunning)` `spawn_status_panel` runs and its
//! `update_status_panel` repaints the lines under the `BattleInProgress` gate. They
//! cover:
//!
//! - **AC1** — the panel spawns in a live battle and despawns on exit (mirrors the
//!   action-bar's `action_bar_spawns_in_battle_and_despawns_outside`).
//! - **AC2** — each line `Text` reflects the selected ganger's known vitals (assert on
//!   the rendered string content; discriminating — a line wired to the wrong component
//!   would surface the wrong value).
//! - **AC3** — changing the selected ganger's `Tu` / `Stance` via the real components
//!   and re-running `update()` re-renders the lines (the panel is not a one-shot).
//! - **AC4** — with no selection the panel shows its empty state and does not panic /
//!   does not show stale ganger data.
//!
//! The pure format-helper unit tests (AC5) live in-crate in `status_panel/systems/labels.rs`.

use bevy::{ecs::entity::Entity, prelude::*, state::state::State, ui::widget::Button};
use gdtf_app::test_support::{
    AppState, BattleRunningComplete, BattleScapeState, HpText, IdentityText, LifeText,
    RunningState, StanceText, TuText,
};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    Cell, CellLevel, Faction, Hp, Level, LifeState, Position, Stance, StanceKind, Tu, TuMax,
    Wounds, tuning::CombatTuning,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough to drive the deep walk into the battlescape (each leaf scene
/// spends a couple of `FixedUpdate` ticks plus transition propagation), bounded so a
/// machine that never reaches the predicate fails instead of hanging (the
/// `action_bar.rs` budget).
const BUDGET: u32 = 96;

// ---------------------------------------------------------------------------------
// Harness — drive the real stack to BattleRunning, where the panel is live.
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
/// (which `spawn_status_panel` reads) + `CombatTuning`. No `LoadedSituation` → the empty
/// `Situation::default()` battle is set up, which still makes `BattleInProgress` present
/// in `BattleRunning` (the panel's update gate). The action-bar harness precedent.
fn walk_app() -> App {
    let mut app = GdtfTestAppBuilder::new()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
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
/// succeeded (so each test starts from the live battle where the panel is spawned).
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

/// Reads the rendered `Text` string of the single entity carrying line-marker `M`.
fn line_text<M: Component>(app: &mut App) -> Option<String> {
    let entity = single_with::<M>(app)?;
    app.world()
        .get::<Text>(entity)
        .map(|t| t.as_str().to_owned())
}

/// A ganger's vitals for a test spawn — grouped into one struct so the spawn helper
/// stays under clippy's argument-count gate (the `too_many_arguments` group-into-a-struct
/// idiom).
struct GangerSetup {
    /// The ganger's cell.
    cell:    Cell,
    /// The ganger's storey level.
    level:   Level,
    /// The ganger's faction (gang) index.
    faction: Faction,
    /// The ganger's stance posture.
    stance:  StanceKind,
    /// The ganger's current TU pool.
    tu:      Tu,
    /// The ganger's round-start TU ceiling.
    tu_max:  TuMax,
    /// The ganger's current HP pool.
    hp:      Hp,
    /// The ganger's Wounds (life) pool.
    wounds:  Wounds,
    /// The ganger's terminal life state.
    life:    LifeState,
}

/// Spawns a ganger carrying exactly the vital components the panel reads and SELECTS it
/// via the `SelectedShooter` resource (the selection the update system reads). Returns
/// its entity. Setting the resource directly is the faithful minimal selection for these
/// view tests (the cursor-click selection path is covered in `gdtf_battle_input`); the
/// panel only reads `*SelectedShooter` + the on-entity components.
fn spawn_and_select(app: &mut App, setup: GangerSetup) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((
            Position::new(CellLevel::new(setup.cell, setup.level)),
            setup.faction,
            Stance::new(setup.stance),
            setup.tu,
            setup.tu_max,
            setup.hp,
            setup.wounds,
            setup.life,
        ))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    ganger
}

// ---------------------------------------------------------------------------------
// AC1 — panel spawns in a live battle, despawns on exit.
// ---------------------------------------------------------------------------------

/// AC1 — `OnEnter(BattleRunning)` the panel's five vitals lines exist (each a `Text`),
/// and `OnExit(BattleRunning)` the panel is recursively despawned.
#[test]
fn status_panel_spawns_in_battle_and_despawns_outside() {
    let mut app = battle_running_app();

    // Each per-line marker resolves to exactly one entity carrying a Text.
    for found in [
        single_with::<IdentityText>(&mut app),
        single_with::<StanceText>(&mut app),
        single_with::<TuText>(&mut app),
        single_with::<HpText>(&mut app),
        single_with::<LifeText>(&mut app),
    ] {
        assert!(
            found.is_some(),
            "the status panel must spawn exactly one Text per vitals line in BattleRunning",
        );
        let Some(line) = found else { return };
        assert!(
            app.world().get::<Text>(line).is_some(),
            "a vitals line must carry a Text",
        );
        // A panel line is body text, NOT an interactive button.
        assert!(
            app.world().get::<Button>(line).is_none(),
            "a vitals line is plain text, not a button",
        );
    }

    // Leave BattleRunning via the explicit end-signal marker (standing in for the
    // not-yet-wired victory/flee), tripping `move_on` to advance the machine out of
    // BattleRunning, where `OnExit` despawns the panel (the action-bar AC1 precedent).
    app.world_mut().insert_resource(BattleRunningComplete);
    let left = advance_until(
        &mut app,
        |app| battlescape_state(app) != Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        left,
        "an explicit BattleRunningComplete insert must advance the machine out of BattleRunning \
         within {BUDGET} updates",
    );
    assert!(
        single_with::<StanceText>(&mut app).is_none(),
        "the status panel must be despawned once the battle leaves BattleRunning",
    );
}

// ---------------------------------------------------------------------------------
// AC2 — lines reflect the selected ganger.
// ---------------------------------------------------------------------------------

/// AC2 — with a selected ganger at a known cell with known vitals, after `update()`
/// each line `Text` contains the expected rendered value. Discriminating: a line wired
/// to the wrong component would surface a different value here (e.g. the TU line showing
/// HP, or the stance line showing the wrong posture).
#[test]
fn lines_reflect_the_selected_ganger() {
    let mut app = battle_running_app();
    spawn_and_select(
        &mut app,
        GangerSetup {
            cell:    Cell::new(5, 6),
            level:   Level::new(2),
            faction: Faction::new(1),
            stance:  StanceKind::Crouching,
            tu:      Tu::new(7),
            tu_max:  TuMax::new(10),
            hp:      Hp::new(8),
            wounds:  Wounds::new(3),
            life:    LifeState::Alive,
        },
    );
    app.update();

    let identity = line_text::<IdentityText>(&mut app).unwrap_or_default();
    assert!(
        identity.contains('5'),
        "identity shows the cell x: {identity}"
    );
    assert!(
        identity.contains('6'),
        "identity shows the cell y: {identity}"
    );
    assert!(
        identity.contains("Gang 1"),
        "identity shows the faction: {identity}",
    );

    let stance = line_text::<StanceText>(&mut app).unwrap_or_default();
    assert!(
        stance.contains("Crouching"),
        "stance line shows the posture: {stance}",
    );

    let tu = line_text::<TuText>(&mut app).unwrap_or_default();
    assert!(tu.contains("7/10"), "TU line shows current/max: {tu}");

    let hp = line_text::<HpText>(&mut app).unwrap_or_default();
    assert!(hp.contains('8'), "HP line shows current HP: {hp}");
    assert!(hp.contains('3'), "HP line shows the Wounds count: {hp}");

    let life = line_text::<LifeText>(&mut app).unwrap_or_default();
    assert!(life.contains("Alive"), "life line shows the state: {life}");
}

/// AC2 (discriminating, cross-line) — the TU line shows the TU pair, NOT the HP value,
/// and the stance/life lines render the correct word, so a mis-wired line is caught.
/// Uses values where TU and HP would alias if a line were cross-wired.
#[test]
fn lines_do_not_cross_wire_components() {
    let mut app = battle_running_app();
    spawn_and_select(
        &mut app,
        GangerSetup {
            cell:    Cell::new(1, 2),
            level:   Level::new(0),
            faction: Faction::new(0),
            stance:  StanceKind::Prone,
            tu:      Tu::new(4),
            tu_max:  TuMax::new(9),
            hp:      Hp::new(12),
            wounds:  Wounds::new(1),
            life:    LifeState::Downed,
        },
    );
    app.update();

    let tu = line_text::<TuText>(&mut app).unwrap_or_default();
    assert!(tu.contains("4/9"), "TU line is the TU pair: {tu}");
    assert!(
        !tu.contains("12"),
        "TU line must NOT show the HP value (cross-wire guard): {tu}",
    );

    let stance = line_text::<StanceText>(&mut app).unwrap_or_default();
    assert!(stance.contains("Prone"), "stance is Prone: {stance}");

    let life = line_text::<LifeText>(&mut app).unwrap_or_default();
    assert!(life.contains("Downed"), "life is Downed: {life}");
}

// ---------------------------------------------------------------------------------
// AC3 — updates on change.
// ---------------------------------------------------------------------------------

/// AC3 — after spending TU and changing stance on the selected ganger via the real
/// components, a re-`update()` re-renders the corresponding lines (the panel reflects
/// CURRENT state, not a spawn snapshot).
#[test]
fn lines_update_when_the_ganger_changes() {
    let mut app = battle_running_app();
    let ganger = spawn_and_select(
        &mut app,
        GangerSetup {
            cell:    Cell::new(3, 3),
            level:   Level::new(0),
            faction: Faction::new(0),
            stance:  StanceKind::Standing,
            tu:      Tu::new(10),
            tu_max:  TuMax::new(10),
            hp:      Hp::new(10),
            wounds:  Wounds::new(2),
            life:    LifeState::Alive,
        },
    );
    app.update();

    // Baseline: standing, full TU.
    assert!(
        line_text::<StanceText>(&mut app)
            .unwrap_or_default()
            .contains("Standing"),
        "baseline stance is Standing",
    );
    assert!(
        line_text::<TuText>(&mut app)
            .unwrap_or_default()
            .contains("10/10"),
        "baseline TU is full",
    );

    // Spend TU and drop to prone on the real components.
    if let Some(mut tu) = app.world_mut().get_mut::<Tu>(ganger) {
        *tu = Tu::new(3);
    }
    if let Some(mut stance) = app.world_mut().get_mut::<Stance>(ganger) {
        *stance = Stance::new(StanceKind::Prone);
    }
    app.update();

    let tu = line_text::<TuText>(&mut app).unwrap_or_default();
    assert!(tu.contains("3/10"), "TU line reflects the spent pool: {tu}");
    let stance = line_text::<StanceText>(&mut app).unwrap_or_default();
    assert!(
        stance.contains("Prone"),
        "stance line reflects the new posture: {stance}",
    );
}

// ---------------------------------------------------------------------------------
// AC4 — empty selection is handled.
// ---------------------------------------------------------------------------------

/// AC4 — with `SelectedShooter == None`, `update()` shows the empty state on every line
/// and does not panic; and after a ganger was shown then deselected, the lines do NOT
/// show stale ganger data (they revert to the empty state).
#[test]
fn empty_selection_shows_empty_state_and_no_stale_data() {
    let mut app = battle_running_app();

    // No selection (SelectedShooter defaults to None) — the empty state, no panic.
    app.update();
    for line in [
        line_text::<IdentityText>(&mut app),
        line_text::<StanceText>(&mut app),
        line_text::<TuText>(&mut app),
        line_text::<HpText>(&mut app),
        line_text::<LifeText>(&mut app),
    ] {
        let line = line.unwrap_or_default();
        assert!(
            line.contains("No ganger selected"),
            "an unselected line must show the empty state: {line}",
        );
    }

    // Force-select a ganger, render it, then clear the selection: the lines must NOT keep
    // the stale ganger data — they revert to the empty state. The ganger is an ENEMY
    // faction (1, distinct from the default `PlayerFaction` 0) so the landed GTW-255
    // `auto_select_first_player_ganger` (which fills an EMPTY selection with the first
    // PLAYER-faction ganger) does NOT re-select it on clear — isolating the panel's
    // revert-on-clear behavior from that separate auto-fill rule. A direct
    // `SelectedShooter` write bypasses the player-faction SELECT gate, so the enemy
    // ganger can still be force-shown first.
    spawn_and_select(
        &mut app,
        GangerSetup {
            cell:    Cell::new(8, 8),
            level:   Level::new(0),
            faction: Faction::new(1),
            stance:  StanceKind::Crouching,
            tu:      Tu::new(5),
            tu_max:  TuMax::new(5),
            hp:      Hp::new(5),
            wounds:  Wounds::new(5),
            life:    LifeState::Alive,
        },
    );
    app.update();
    assert!(
        line_text::<StanceText>(&mut app)
            .unwrap_or_default()
            .contains("Crouching"),
        "the selected ganger is rendered before clearing",
    );

    app.world_mut().insert_resource(SelectedShooter::cleared());
    app.update();
    let stance = line_text::<StanceText>(&mut app).unwrap_or_default();
    assert!(
        stance.contains("No ganger selected"),
        "after clearing, the line must revert to the empty state (no stale data): {stance}",
    );
}
